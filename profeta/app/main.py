import os, time, gc
from datetime import datetime, timedelta
from contextlib import asynccontextmanager

import torch
import numpy as np
import pandas as pd
import uvicorn
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel, Field, field_validator, model_validator

from app.models.chronos_predictor import ChronosPredictor
from app.utils.data_processor import DataProcessor
from app.utils.temporal_aggregation import TemporalAggregator
from app.utils.logger import log
from app.config import cfg

predictor: ChronosPredictor | None = None
data_proc: DataProcessor | None = None

_TUNIT = {"second": 1, "minute": 60, "hour": 3600, "day": 86400, "week": 604800, "month": 2592000}
_WKDAY = ("Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun")


class _GPUCtx:
    __slots__ = ()
    def __enter__(self): return self
    def __exit__(self, *_): torch.cuda.is_available() and (torch.cuda.empty_cache(), gc.collect())


def _gpu_mem():
    if not torch.cuda.is_available(): return {}
    d = torch.cuda.current_device()
    a, r = torch.cuda.memory_allocated(d), torch.cuda.memory_reserved(d)
    t = torch.cuda.get_device_properties(d).total_memory
    return {"allocated_mb": round(a/1e6, 1), "reserved_mb": round(r/1e6, 1),
            "total_gb": round(t/1e9, 1), "free_mb": round((t-r)/1e6, 1),
            "utilization_percent": round(r/t*100, 1)}


def _strip_tz(ts):
    return ts.replace(tzinfo=None) if ts and getattr(ts, 'tzinfo', None) else ts


def _fmt_dur(s):
    if s >= 2592000: return f"{s/2592000:.1f}mo"
    if s >= 86400: return f"{s/86400:.1f}d"
    if s >= 3600: return f"{s/3600:.1f}h"
    if s >= 60: return f"{s/60:.1f}min"
    return f"{s:.0f}s"


def _calc_freq(ts_list):
    if len(ts_list) < 2: return cfg.freq_fallback
    diffs = [(ts_list[i] - ts_list[i-1]).total_seconds() for i in range(1, len(ts_list))]
    return float(np.median(diffs))


def _gen_forecast_ts(last, freq_s, n):
    return [(last + timedelta(seconds=freq_s*(i+1))).isoformat() + "Z" for i in range(n)]


def _parse_horizon(dur, freq, dflt, cap):
    if not dur: return dflt, dflt, False
    try:
        p = dur.lower().split()
        secs = float(p[0]) * _TUNIT.get(p[1].rstrip('s'), 60)
        req = max(1, int(secs / freq))
        return min(req, cap), req, req > cap
    except (ValueError, IndexError):
        return dflt, dflt, False


def _agg_daily(ts_list, vals, method="mean"):
    df = pd.DataFrame({"ts": ts_list, "v": vals})
    df["d"] = df["ts"].dt.date
    fn = getattr(df.groupby("d")["v"], method if method in ("mean","max","min") else "mean")
    agg = fn()
    return [pd.Timestamp(d) for d in agg.index], agg.tolist()


def _check_levels(lvls):
    if not all(0 < x < 1 for x in lvls):
        raise ValueError("confidence levels must be in (0,1)")
    return sorted(lvls)


class TimeSeriesData(BaseModel):
    values: list[float]
    timestamps: list[str] | None = None
    series_name: str | None = "series_1"

    @field_validator('values')
    @classmethod
    def _v(cls, v):
        if len(v) < 2: raise ValueError("need >=2 values")
        for x in v:
            if not isinstance(x, (int, float)): raise ValueError(f"bad val: {x}")
            if isinstance(x, float) and np.isnan(x): raise ValueError("nan not allowed")
        return v


class ForecastRequest(BaseModel):
    data: TimeSeriesData
    prediction_length: int = Field(default_factory=lambda: cfg.pred_len, ge=1, le=100)
    num_samples: int = Field(default_factory=lambda: cfg.sim, ge=10, le=1000)
    confidence_levels: list[float] = Field(default_factory=lambda: cfg.conf.copy())

    @field_validator('confidence_levels')
    @classmethod
    def _c(cls, v): return _check_levels(v)


class ForecastResponse(BaseModel):
    series_name: str
    forecast_values: dict[str, list[float]]
    prediction_length: int
    num_samples: int
    model_info: dict
    processing_time: float


class HealthResponse(BaseModel):
    status: str
    model_loaded: bool
    model_name: str
    version: str


class MetricPoint(BaseModel):
    timestamp: str | datetime
    value: float

    @field_validator('timestamp')
    @classmethod
    def _ts(cls, v):
        if isinstance(v, str): v = pd.to_datetime(v, utc=True)
        return v.replace(tzinfo=None) if getattr(v, 'tzinfo', None) else v


class MetricsData(BaseModel):
    series_name: str
    server_id: str | None = None
    metric_type: str
    unit: str | None = None
    data_points: list[MetricPoint] = Field(..., min_length=2)

    @model_validator(mode='after')
    def _ord(self):
        ts = [_strip_tz(p.timestamp) for p in self.data_points]
        for i in range(1, len(ts)):
            if ts[i] <= ts[i-1]: raise ValueError("timestamps must be chronological")
        return self


class MetricsForecastRequest(BaseModel):
    metrics: MetricsData
    prediction_length: int = Field(default_factory=lambda: cfg.pred_len, ge=1, le=1024)
    prediction_duration: str | None = None
    num_samples: int = Field(default_factory=lambda: cfg.sim, ge=10, le=1000)
    confidence_levels: list[float] = Field(default_factory=lambda: cfg.conf_extended.copy())
    include_analysis: bool = True

    @field_validator('confidence_levels')
    @classmethod
    def _c(cls, v): return _check_levels(v)


class ForecastAnalysis(BaseModel):
    historical_stats: dict[str, float]
    prediction_stats: dict[str, float]
    trend_analysis: dict[str, str | float]
    quality_metrics: dict[str, float]


class MetricsForecastResponse(BaseModel):
    series_name: str
    server_id: str | None
    metric_type: str
    unit: str | None
    sampling_frequency: dict[str, float | str]
    historical_duration: str
    prediction_duration: str
    historical_data: dict
    forecast_values: dict[str, list[float]]
    forecast_timestamps: list[str]
    analysis: ForecastAnalysis | None
    model_info: dict
    processing_time: float
    timestamp: str


class LongTermMetricsRequest(BaseModel):
    metrics: MetricsData
    period_type: str
    aggregation_method: str = "mean"
    prediction_horizon: str = "4 hours"
    num_samples: int = Field(default_factory=lambda: cfg.sim_long, ge=50, le=1000)
    confidence_levels: list[float] = Field(default_factory=lambda: cfg.conf_extended.copy())
    include_analysis: bool = True
    include_raw_data: bool = False


class AggregationMetadata(BaseModel):
    original_points: int
    aggregated_points: int
    aggregation_ratio: float
    original_frequency: str
    target_frequency: str
    aggregation_method: str
    description: str
    time_span: dict | None


class LongTermForecastResponse(BaseModel):
    series_name: str
    server_id: str | None
    period_type: str
    aggregation: AggregationMetadata
    historical_data: dict
    forecast_values: dict[str, list[float]]
    forecast_timestamps: list[str]
    analysis: ForecastAnalysis | None
    patterns_detected: dict
    model_info: dict
    processing_time: float
    timestamp: str


def _stats(arr, q=False):
    r = {"count": len(arr), "mean": float(np.mean(arr)), "std": float(np.std(arr)),
         "min": float(arr.min()), "max": float(arr.max()), "median": float(np.median(arr))}
    if q:
        r["q25"] = float(np.percentile(arr, 25))
        r["q75"] = float(np.percentile(arr, 75))
    return r


def _trend(arr: np.ndarray) -> float:
    c = arr[np.isfinite(arr)]
    if len(c) < 2: return 0.0
    try: return float(np.polyfit(range(len(c)), c, 1)[0])
    except (np.linalg.LinAlgError, ValueError, TypeError): return 0.0


def _analyze(hist, pred):
    h, p = np.array(hist), np.array(pred)
    tail_sz = cfg.trend_tail
    tail = h[-tail_sz:] if len(h) >= tail_sz else h
    ht, pt = _trend(tail), _trend(p)
    hs, ps = _stats(h, True), _stats(p)

    thresh = cfg.jitter(cfg.trend_thresh)
    interp = "stable"
    if abs(ht) > thresh: interp = "increasing" if ht > 0 else "decreasing"
    cons = "consistent" if (ht > 0) == (pt > 0) else "reversal_expected"

    mc = (ps["mean"] - hs["mean"]) / hs["mean"] * 100 if hs["mean"] else 0
    vc = (ps["std"] - hs["std"]) / hs["std"] * 100 if hs["std"] else 0
    stab = 1 - ps["std"] / ps["mean"] if ps["mean"] else 0

    return ForecastAnalysis(
        historical_stats=hs, prediction_stats=ps,
        trend_analysis={"historical_trend": ht, "prediction_trend": pt,
                        "trend_interpretation": interp, "trend_consistency": cons},
        quality_metrics={"mean_change_percent": mc, "volatility_change_percent": vc,
                         "prediction_stability": stab}
    )


def _patterns(ts_list, vals, ptype, avg):
    out = {"cycle_detected": True, "average_value": float(avg),
           "volatility": float(np.std(vals)), "trend": "stable"}

    peak_h = cfg.jitter(cfg.peak_hour_k)
    low_h = cfg.jitter(cfg.low_hour_k)
    peak_d = cfg.jitter(cfg.peak_day_k)
    low_d = cfg.jitter(cfg.low_day_k)

    if ptype == "day":
        by_h = {}
        for t, v in zip(ts_list, vals):
            by_h.setdefault(t.hour, []).append(v)
        out["peak_hours"] = [h for h, vv in by_h.items() if np.mean(vv) > avg * peak_h]
        out["low_hours"] = [h for h, vv in by_h.items() if np.mean(vv) < avg * low_h]
    else:
        by_wd = {i: [] for i in range(7)}
        for t, v in zip(ts_list, vals):
            by_wd[t.weekday()].append(v)
        out["peak_days"] = [_WKDAY[d] for d, vv in by_wd.items() if vv and np.mean(vv) > avg * peak_d]
        out["low_days"] = [_WKDAY[d] for d, vv in by_wd.items() if vv and np.mean(vv) < avg * low_d]

    return out


def _forecast_longterm(req: LongTermMetricsRequest, period: str, max_pts: int, dflt_len: int, max_len: int):
    global predictor
    if not predictor or not predictor.is_loaded:
        raise HTTPException(503, "model unavailable")

    t0 = time.time()
    pts = req.metrics.data_points
    if len(pts) > max_pts:
        raise HTTPException(400, f"too many points: {len(pts)}, max {max_pts}")

    raw = [{"timestamp": p.timestamp.isoformat() if hasattr(p.timestamp, 'isoformat') else p.timestamp,
            "value": p.value} for p in pts]

    agg, meta = TemporalAggregator.aggregate(raw, period, req.aggregation_method)
    ts_list = [pd.to_datetime(p["timestamp"]) for p in agg]
    vals = [p["value"] for p in agg]

    freq = orig_freq = _calc_freq(ts_list)
    pred_len, req_len, trunc = _parse_horizon(req.prediction_horizon, freq, dflt_len, max_len)

    auto_daily = False
    min_days = cfg.jitter(cfg.auto_daily_days)
    if trunc and len(ts_list) >= 2:
        span_s = (_strip_tz(ts_list[-1]) - _strip_tz(ts_list[0])).total_seconds()
        if span_s / 86400 >= min_days:
            try:
                ts_list, vals = _agg_daily(ts_list, vals, req.aggregation_method)
                freq = 86400
                pred_len, req_len, trunc = _parse_horizon(req.prediction_horizon, freq, dflt_len, max_len)
                auto_daily = True
                meta["auto_daily_aggregation"] = True
                meta["original_frequency"] = _fmt_dur(orig_freq)
                meta["aggregated_frequency"] = "1d"
                meta["aggregated_points"] = len(vals)
            except Exception as e:
                log.warning(f"daily agg failed: {e}")

    adj = predictor.adjust(len(vals), pred_len, req.num_samples, True)
    if not adj['can_fit']:
        raise HTTPException(400, "insufficient GPU memory")

    if adj['context_length'] < len(vals):
        vals = vals[-adj['context_length']:]
        ts_list = ts_list[-adj['context_length']:]

    with _GPUCtx():
        res = predictor.predict(vals, adj['prediction_length'], adj['num_samples'], req.confidence_levels)

    mi = res["model_info"].copy()
    if adj['adjustments_made']:
        mi['auto_adjustments'] = {'applied': True, 'changes': adj['adjustments_made'],
                                  'memory_saved_mb': round(adj['memory_saved_mb'], 2)}
    if auto_daily:
        mi['auto_daily_aggregation'] = {'applied': True, 'original_frequency': _fmt_dur(orig_freq),
                                        'new_frequency': '1d', 'data_points_after': len(vals),
                                        'message': f'auto-aggregated to daily for horizon {req.prediction_horizon}'}
    if trunc:
        mi['horizon_warning'] = {'truncated': True, 'requested': req.prediction_horizon,
                                 'actual': _fmt_dur(pred_len * freq),
                                 'reason': f'freq {_fmt_dur(freq)}, max {max_len} pts',
                                 'suggestion': 'need wider temporal coverage'}

    last = _strip_tz(ts_list[-1])
    span = _strip_tz(ts_list[-1]) - _strip_tz(ts_list[0])

    hist = {"timestamps": [t.isoformat() for t in ts_list], "values": vals,
            "count": len(vals), "duration": _fmt_dur(span.total_seconds())}
    if period == "week":
        hist["resolution"] = f"{meta['target_frequency']} aggregated"

    analysis = _analyze(vals, res["quantiles"]["0.5"]) if req.include_analysis else None

    return LongTermForecastResponse(
        series_name=req.metrics.series_name, server_id=req.metrics.server_id, period_type=period,
        aggregation=AggregationMetadata(**meta), historical_data=hist,
        forecast_values=res["quantiles"], forecast_timestamps=_gen_forecast_ts(last, freq, pred_len),
        analysis=analysis, patterns_detected=_patterns(ts_list, vals, period, np.mean(vals)),
        model_info=mi, processing_time=time.time() - t0, timestamp=datetime.now().isoformat()
    )


@asynccontextmanager
async def lifespan(app):
    global predictor, data_proc
    mdl = cfg.model_name
    predictor = ChronosPredictor(mdl)
    data_proc = DataProcessor()
    log.info(f"loaded: {mdl}")
    yield
    if predictor: predictor.cleanup()


app = FastAPI(title="Profeta API", description="Time series forecasting with Chronos",
              version="1.0.0", lifespan=lifespan)
app.add_middleware(CORSMiddleware, allow_origins=["*"], allow_credentials=True,
                   allow_methods=["*"], allow_headers=["*"])


@app.get("/")
async def root():
    return {"message": "Profeta API", "version": "1.0.0", "docs": "/docs"}


@app.get("/health", response_model=HealthResponse)
async def health():
    ok = predictor and predictor.is_loaded
    return HealthResponse(
        status="healthy" if ok else "unhealthy",
        model_loaded=predictor.is_loaded if predictor else False,
        model_name=getattr(predictor, 'model_name', 'n/a'),
        version="1.0.0"
    )


@app.post("/predict", response_model=ForecastResponse)
async def predict(req: ForecastRequest):
    if not predictor or not predictor.is_loaded:
        raise HTTPException(503, "model unavailable")
    try:
        proc = data_proc.prepare(req.data.values, req.data.timestamps)
        with _GPUCtx():
            res = predictor.predict(proc["values"], req.prediction_length, req.num_samples, req.confidence_levels)
        return ForecastResponse(
            series_name=req.data.series_name,
            forecast_values=res["quantiles"],
            prediction_length=req.prediction_length,
            num_samples=req.num_samples,
            model_info=res["model_info"],
            processing_time=res["processing_time"]
        )
    except Exception as e:
        log.error(f"predict err: {e}")
        raise HTTPException(500, str(e))


@app.post("/predict/batch")
async def predict_batch(reqs: list[ForecastRequest]):
    if not predictor or not predictor.is_loaded:
        raise HTTPException(503, "model unavailable")
    if len(reqs) > cfg.batch_cap:
        raise HTTPException(400, f"max {cfg.batch_cap} series per batch")

    out = []
    with _GPUCtx():
        for r in reqs:
            proc = data_proc.prepare(r.data.values, r.data.timestamps)
            res = predictor.predict(proc["values"], r.prediction_length, r.num_samples, r.confidence_levels)
            out.append(ForecastResponse(
                series_name=r.data.series_name,
                forecast_values=res["quantiles"],
                prediction_length=r.prediction_length,
                num_samples=r.num_samples,
                model_info=res["model_info"],
                processing_time=res["processing_time"]
            ))
    return {"results": out, "total_processed": len(out)}


@app.get("/model/info")
async def model_info():
    if not predictor or not predictor.is_loaded:
        raise HTTPException(503, "model unavailable")
    return predictor.info()


@app.post("/metrics/forecast", response_model=MetricsForecastResponse)
async def forecast_metrics(req: MetricsForecastRequest):
    if not predictor or not predictor.is_loaded:
        raise HTTPException(503, "model unavailable")

    t0 = time.time()
    m = req.metrics
    ts_list = [p.timestamp for p in m.data_points]
    vals = [p.value for p in m.data_points]
    freq = _calc_freq(ts_list)
    pred_len, _, _ = _parse_horizon(req.prediction_duration, freq, req.prediction_length, cfg.metrics_cap)

    with _GPUCtx():
        res = predictor.predict(vals, pred_len, req.num_samples, req.confidence_levels)

    first, last = _strip_tz(ts_list[0]), _strip_tz(ts_list[-1])
    hist_dur = _fmt_dur((last - first).total_seconds())
    pred_dur = _fmt_dur(pred_len * freq)

    mi = res["model_info"]
    mi["prediction_method"] = predictor.model_name
    mi["confidence_levels"] = req.confidence_levels

    return MetricsForecastResponse(
        series_name=m.series_name, server_id=m.server_id, metric_type=m.metric_type, unit=m.unit,
        sampling_frequency={"seconds": freq, "description": f"{_fmt_dur(freq)}/pt"},
        historical_duration=hist_dur, prediction_duration=pred_dur,
        historical_data={"timestamps": [t.isoformat() for t in ts_list], "values": vals,
                         "count": len(vals), "duration": hist_dur},
        forecast_values=res["quantiles"],
        forecast_timestamps=_gen_forecast_ts(last, freq, pred_len),
        analysis=_analyze(vals, res["quantiles"]["0.5"]) if req.include_analysis else None,
        model_info=mi, processing_time=time.time() - t0, timestamp=datetime.now().isoformat()
    )


@app.get("/gpu/status")
async def gpu_status():
    st = {"cuda_available": torch.cuda.is_available(),
          "cuda_version": getattr(torch.version, 'cuda', None),
          "device_count": torch.cuda.device_count() if torch.cuda.is_available() else 0}
    if torch.cuda.is_available():
        d = torch.cuda.current_device()
        st["current_device"] = d
        st["device_name"] = torch.cuda.get_device_name(d)
        st["memory"] = _gpu_mem()
        if predictor and predictor.is_loaded:
            st["model_device"] = predictor.device
            st["model_dtype"] = str(predictor.torch_dtype)
    return st


@app.post("/gpu/cleanup")
async def gpu_cleanup():
    if not torch.cuda.is_available():
        raise HTTPException(400, "CUDA not available")
    d = torch.cuda.current_device()
    before = torch.cuda.memory_reserved(d)
    torch.cuda.empty_cache()
    torch.cuda.synchronize()
    return {"status": "ok", "freed_mb": round((before - torch.cuda.memory_reserved(d)) / 1e6, 1)}


@app.post("/metrics/forecast/daily", response_model=LongTermForecastResponse)
async def forecast_daily(req: LongTermMetricsRequest):
    req.period_type = "day"
    return _forecast_longterm(req, "day", cfg.daily_pts_cap, cfg.daily_len, cfg.daily_len_cap)


@app.post("/metrics/forecast/weekly", response_model=LongTermForecastResponse)
async def forecast_weekly(req: LongTermMetricsRequest):
    req.period_type = "week"
    return _forecast_longterm(req, "week", cfg.weekly_pts_cap, cfg.weekly_len, cfg.weekly_len_cap)


@app.get("/examples/data-format")
async def examples():
    return {
        "simple": {
            "data": {"values": [100, 120, 110, 130, 125, 140, 135, 150], "series_name": "sales"},
            "prediction_length": 6, "num_samples": 100
        },
        "with_timestamps": {
            "data": {
                "values": [100, 120, 110, 130, 125, 140],
                "timestamps": ["2024-01-01", "2024-02-01", "2024-03-01", "2024-04-01", "2024-05-01", "2024-06-01"],
                "series_name": "monthly_metrics"
            },
            "prediction_length": 3, "confidence_levels": [0.1, 0.5, 0.9]
        }
    }


if __name__ == "__main__":
    uvicorn.run("app.main:app", host="0.0.0.0", port=8000, reload=True)

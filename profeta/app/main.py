import gc, time
from contextlib import asynccontextmanager
from datetime import datetime, timedelta

import numpy as np, pandas as pd, torch, uvicorn
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel, Field, field_validator, model_validator

from app.config import cfg
from app.models.chronos_predictor import ChronosPredictor
from app.utils.data_processor import DataProcessor
from app.utils.logger import log
from app.utils.temporal_aggregation import TemporalAggregator

predictor, data_proc = None, None
_TU = {"second": 1, "minute": 60, "hour": 3600, "day": 86400, "week": 604800, "month": 2592000}
_WD = ("Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun")
_DT = ((2592000, "mo"), (86400, "d"), (3600, "h"), (60, "min"))


class _G:
    __slots__ = ()
    def __enter__(s): return s
    def __exit__(s, *_): torch.cuda.is_available() and (torch.cuda.empty_cache(), gc.collect())

def _ens(): 
    if not (predictor and predictor.is_loaded): raise HTTPException(503, "model unavailable")

_gm = lambda: (d := torch.cuda.current_device()) and {"allocated_mb": round(torch.cuda.memory_allocated(d) / 1e6, 1), "reserved_mb": round((r := torch.cuda.memory_reserved(d)) / 1e6, 1), "total_gb": round((t := torch.cuda.get_device_properties(d).total_memory) / 1e9, 1), "free_mb": round((t - r) / 1e6, 1), "utilization_percent": round(r / t * 100, 1)} if torch.cuda.is_available() else {}
_sz = lambda ts: ts.replace(tzinfo=None) if ts and getattr(ts, "tzinfo", None) else ts
_fd = lambda s: next((f"{s/th:.1f}{u}" for th, u in _DT if s >= th), f"{s:.0f}s")
_fr = lambda ts: float(np.median([(ts[i] - ts[i-1]).total_seconds() for i in range(1, len(ts))])) if len(ts) >= 2 else cfg.freq_fallback
_gt = lambda l, f, n: [(l + timedelta(seconds=f * (i + 1))).isoformat() + "Z" for i in range(n)]
_cl = lambda l: sorted(l) if all(0 < x < 1 for x in l) else (_ for _ in ()).throw(ValueError("confidence levels must be in (0,1)"))

def _ph(d, f, df, c):
    if not d: return df, df, False
    try: p = d.lower().split(); r = max(1, int(float(p[0]) * _TU.get(p[1].rstrip("s"), 60) / f)); return min(r, c), r, r > c
    except: return df, df, False

def _ad(ts, v, m="mean"):
    df = pd.DataFrame({"ts": ts, "v": v}); df["d"] = df["ts"].dt.date; ag = getattr(df.groupby("d")["v"], m if m in ("mean", "max", "min") else "mean")()
    return [pd.Timestamp(x) for x in ag.index], ag.tolist()


class TimeSeriesData(BaseModel):
    values: list[float]; timestamps: list[str] | None = None; series_name: str | None = "series_1"
    @field_validator("values")
    @classmethod
    def _v(c, v):
        if len(v) < 2: raise ValueError("need >=2 values")
        if any(not isinstance(x, (int, float)) or (isinstance(x, float) and np.isnan(x)) for x in v): raise ValueError("invalid or nan value")
        return v

class ForecastRequest(BaseModel):
    data: TimeSeriesData; prediction_length: int = Field(default_factory=lambda: cfg.pred_len, ge=1, le=100)
    num_samples: int = Field(default_factory=lambda: cfg.sim, ge=10, le=1000); confidence_levels: list[float] = Field(default_factory=lambda: cfg.conf.copy())
    @field_validator("confidence_levels")
    @classmethod
    def _v(c, v): return _cl(v)

class ForecastResponse(BaseModel):
    series_name: str; forecast_values: dict[str, list[float]]; prediction_length: int; num_samples: int; model_info: dict; processing_time: float

class HealthResponse(BaseModel):
    status: str; model_loaded: bool; model_name: str; version: str

class MetricPoint(BaseModel):
    timestamp: str | datetime; value: float
    @field_validator("timestamp")
    @classmethod
    def _v(c, v): ts = pd.to_datetime(v, utc=True) if isinstance(v, str) else v; return ts.replace(tzinfo=None) if getattr(ts, "tzinfo", None) else ts

class MetricsData(BaseModel):
    series_name: str; server_id: str | None = None; metric_type: str; unit: str | None = None; data_points: list[MetricPoint] = Field(..., min_length=2)
    @model_validator(mode="after")
    def _v(s): ts = [_sz(p.timestamp) for p in s.data_points]; any(ts[i] <= ts[i-1] for i in range(1, len(ts))) and (_ for _ in ()).throw(ValueError("timestamps must be chronological")); return s

class MetricsForecastRequest(BaseModel):
    metrics: MetricsData; prediction_length: int = Field(default_factory=lambda: cfg.pred_len, ge=1, le=1024); prediction_duration: str | None = None
    num_samples: int = Field(default_factory=lambda: cfg.sim, ge=10, le=1000); confidence_levels: list[float] = Field(default_factory=lambda: cfg.conf_extended.copy()); include_analysis: bool = True
    @field_validator("confidence_levels")
    @classmethod
    def _v(c, v): return _cl(v)

class ForecastAnalysis(BaseModel):
    historical_stats: dict[str, float]; prediction_stats: dict[str, float]; trend_analysis: dict[str, str | float]; quality_metrics: dict[str, float]

class MetricsForecastResponse(BaseModel):
    series_name: str; server_id: str | None; metric_type: str; unit: str | None; sampling_frequency: dict[str, float | str]; historical_duration: str; prediction_duration: str
    historical_data: dict; forecast_values: dict[str, list[float]]; forecast_timestamps: list[str]; analysis: ForecastAnalysis | None; model_info: dict; processing_time: float; timestamp: str

class LongTermMetricsRequest(BaseModel):
    metrics: MetricsData; period_type: str; aggregation_method: str = "mean"; prediction_horizon: str = "4 hours"
    num_samples: int = Field(default_factory=lambda: cfg.sim_long, ge=50, le=1000); confidence_levels: list[float] = Field(default_factory=lambda: cfg.conf_extended.copy()); include_analysis: bool = True; include_raw_data: bool = False

class AggregationMetadata(BaseModel):
    original_points: int; aggregated_points: int; aggregation_ratio: float; original_frequency: str; target_frequency: str; aggregation_method: str; description: str; time_span: dict | None

class LongTermForecastResponse(BaseModel):
    series_name: str; server_id: str | None; period_type: str; aggregation: AggregationMetadata; historical_data: dict; forecast_values: dict[str, list[float]]; forecast_timestamps: list[str]
    analysis: ForecastAnalysis | None; patterns_detected: dict; model_info: dict; processing_time: float; timestamp: str


def _st(a, q=False):
    b = {"count": len(a), "mean": float(np.mean(a)), "std": float(np.std(a)), "min": float(a.min()), "max": float(a.max()), "median": float(np.median(a))}
    return {**b, "q25": float(np.percentile(a, 25)), "q75": float(np.percentile(a, 75))} if q else b

def _tr(a):
    c = a[np.isfinite(a)]
    if len(c) < 2: return 0.0
    try: return float(np.polyfit(range(len(c)), c, 1)[0])
    except: return 0.0

def _an(h, p):
    ha, pa = np.array(h), np.array(p); ht, pt = _tr(ha[-cfg.trend_tail:] if len(ha) >= cfg.trend_tail else ha), _tr(pa); hs, ps = _st(ha, q=True), _st(pa); th = cfg.jitter(cfg.trend_thresh)
    return ForecastAnalysis(historical_stats=hs, prediction_stats=ps, trend_analysis={"historical_trend": ht, "prediction_trend": pt, "trend_interpretation": "stable" if abs(ht) <= th else ("increasing" if ht > 0 else "decreasing"), "trend_consistency": "consistent" if (ht > 0) == (pt > 0) else "reversal_expected"},
        quality_metrics={"mean_change_percent": (ps["mean"] - hs["mean"]) / hs["mean"] * 100 if hs["mean"] else 0, "volatility_change_percent": (ps["std"] - hs["std"]) / hs["std"] * 100 if hs["std"] else 0, "prediction_stability": 1 - ps["std"] / ps["mean"] if ps["mean"] else 0})

def _pa(ts, vs, pt, av):
    o = {"cycle_detected": True, "average_value": float(av), "volatility": float(np.std(vs)), "trend": "stable"}
    if pt == "day": by = {h: [v for t, v in zip(ts, vs) if t.hour == h] for h in range(24)}; pk, lk = cfg.jitter(cfg.peak_hour_k), cfg.jitter(cfg.low_hour_k); o["peak_hours"], o["low_hours"] = [h for h, vv in by.items() if vv and np.mean(vv) > av * pk], [h for h, vv in by.items() if vv and np.mean(vv) < av * lk]
    else: by = {d: [v for t, v in zip(ts, vs) if t.weekday() == d] for d in range(7)}; pk, lk = cfg.jitter(cfg.peak_day_k), cfg.jitter(cfg.low_day_k); o["peak_days"], o["low_days"] = [_WD[d] for d, vv in by.items() if vv and np.mean(vv) > av * pk], [_WD[d] for d, vv in by.items() if vv and np.mean(vv) < av * lk]
    return o

def _fl(rq, pr, mx, dl, ml):
    global predictor; _ens(); t0, pts = time.time(), rq.metrics.data_points
    if len(pts) > mx: raise HTTPException(400, f"too many points: {len(pts)}, max {mx}")
    raw = [{"timestamp": p.timestamp.isoformat() if hasattr(p.timestamp, "isoformat") else p.timestamp, "value": p.value} for p in pts]
    ag, mt = TemporalAggregator.aggregate(raw, pr, rq.aggregation_method); tl, vs = [pd.to_datetime(p["timestamp"]) for p in ag], [p["value"] for p in ag]
    fq = of = _fr(tl); pl, _, tr = _ph(rq.prediction_horizon, fq, dl, ml); ad = False
    if tr and len(tl) >= 2 and (_sz(tl[-1]) - _sz(tl[0])).total_seconds() / 86400 >= cfg.jitter(cfg.auto_daily_days):
        try: tl, vs = _ad(tl, vs, rq.aggregation_method); fq, ad = 86400, True; pl, _, tr = _ph(rq.prediction_horizon, fq, dl, ml); mt.update({"auto_daily_aggregation": True, "original_frequency": _fd(of), "aggregated_frequency": "1d", "aggregated_points": len(vs)})
        except Exception as e: log.warning(f"daily agg failed: {e}")
    aj = predictor.adjust(len(vs), pl, rq.num_samples, True)
    if not aj["can_fit"]: raise HTTPException(400, "insufficient GPU memory")
    if aj["context_length"] < len(vs): vs, tl = vs[-aj["context_length"]:], tl[-aj["context_length"]:]
    with _G(): rs = predictor.predict(vs, aj["prediction_length"], aj["num_samples"], rq.confidence_levels)
    mi = rs["model_info"].copy()
    aj["adjustments_made"] and mi.update(auto_adjustments={"applied": True, "changes": aj["adjustments_made"], "memory_saved_mb": round(aj["memory_saved_mb"], 2)})
    ad and mi.update(auto_daily_aggregation={"applied": True, "original_frequency": _fd(of), "new_frequency": "1d", "data_points_after": len(vs), "message": f"auto-aggregated to daily for horizon {rq.prediction_horizon}"})
    tr and mi.update(horizon_warning={"truncated": True, "requested": rq.prediction_horizon, "actual": _fd(pl * fq), "reason": f"freq {_fd(fq)}, max {ml} pts", "suggestion": "need wider temporal coverage"})
    ls, sp = _sz(tl[-1]), _sz(tl[-1]) - _sz(tl[0]); hs = {"timestamps": [t.isoformat() for t in tl], "values": vs, "count": len(vs), "duration": _fd(sp.total_seconds())}; pr == "week" and hs.update(resolution=f"{mt['target_frequency']} aggregated")
    return LongTermForecastResponse(series_name=rq.metrics.series_name, server_id=rq.metrics.server_id, period_type=pr, aggregation=AggregationMetadata(**mt), historical_data=hs, forecast_values=rs["quantiles"],
        forecast_timestamps=_gt(ls, fq, pl), analysis=_an(vs, rs["quantiles"]["0.5"]) if rq.include_analysis else None, patterns_detected=_pa(tl, vs, pr, np.mean(vs)), model_info=mi, processing_time=time.time() - t0, timestamp=datetime.now().isoformat())


@asynccontextmanager
async def lifespan(_):
    global predictor, data_proc; predictor, data_proc = ChronosPredictor(cfg.model_name), DataProcessor(); log.info(f"loaded: {cfg.model_name}"); yield; predictor and predictor.cleanup()

app = FastAPI(title="Profeta API", version="1.0.0", lifespan=lifespan)
app.add_middleware(CORSMiddleware, allow_origins=["*"], allow_credentials=True, allow_methods=["*"], allow_headers=["*"])

@app.get("/")
async def root(): return {"message": "Profeta API", "version": "1.0.0", "docs": "/docs"}

@app.get("/health", response_model=HealthResponse)
async def health(): ok = predictor and predictor.is_loaded; return HealthResponse(status="healthy" if ok else "unhealthy", model_loaded=predictor.is_loaded if predictor else False, model_name=getattr(predictor, "model_name", "n/a"), version="1.0.0")

@app.post("/predict", response_model=ForecastResponse)
async def predict(rq: ForecastRequest):
    _ens()
    try:
        with _G(): rs = predictor.predict(data_proc.prepare(rq.data.values, rq.data.timestamps)["values"], rq.prediction_length, rq.num_samples, rq.confidence_levels)
        return ForecastResponse(series_name=rq.data.series_name, forecast_values=rs["quantiles"], prediction_length=rq.prediction_length, num_samples=rq.num_samples, model_info=rs["model_info"], processing_time=rs["processing_time"])
    except Exception as e: log.error(f"predict err: {e}"); raise HTTPException(500, str(e))

@app.post("/predict/batch")
async def predict_batch(rqs: list[ForecastRequest]):
    _ens()
    if len(rqs) > cfg.batch_cap: raise HTTPException(400, f"max {cfg.batch_cap} series per batch")
    with _G(): o = [ForecastResponse(series_name=r.data.series_name, forecast_values=(rs := predictor.predict(data_proc.prepare(r.data.values, r.data.timestamps)["values"], r.prediction_length, r.num_samples, r.confidence_levels))["quantiles"], prediction_length=r.prediction_length, num_samples=r.num_samples, model_info=rs["model_info"], processing_time=rs["processing_time"]) for r in rqs]
    return {"results": o, "total_processed": len(o)}

@app.get("/model/info")
async def model_info(): _ens(); return predictor.info()

@app.post("/metrics/forecast", response_model=MetricsForecastResponse)
async def forecast_metrics(rq: MetricsForecastRequest):
    _ens(); t0, m = time.time(), rq.metrics; tl, vs = [p.timestamp for p in m.data_points], [p.value for p in m.data_points]; fq = _fr(tl); pl = _ph(rq.prediction_duration, fq, rq.prediction_length, cfg.metrics_cap)[0]
    with _G(): rs = predictor.predict(vs, pl, rq.num_samples, rq.confidence_levels)
    fi, ls = _sz(tl[0]), _sz(tl[-1]); mi = {**rs["model_info"], "prediction_method": predictor.model_name, "confidence_levels": rq.confidence_levels}
    return MetricsForecastResponse(series_name=m.series_name, server_id=m.server_id, metric_type=m.metric_type, unit=m.unit, sampling_frequency={"seconds": fq, "description": f"{_fd(fq)}/pt"}, historical_duration=(hd := _fd((ls - fi).total_seconds())), prediction_duration=_fd(pl * fq),
        historical_data={"timestamps": [t.isoformat() for t in tl], "values": vs, "count": len(vs), "duration": hd}, forecast_values=rs["quantiles"], forecast_timestamps=_gt(ls, fq, pl), analysis=_an(vs, rs["quantiles"]["0.5"]) if rq.include_analysis else None, model_info=mi, processing_time=time.time() - t0, timestamp=datetime.now().isoformat())

@app.get("/gpu/status")
async def gpu_status():
    st = {"cuda_available": torch.cuda.is_available(), "cuda_version": getattr(torch.version, "cuda", None), "device_count": torch.cuda.device_count() if torch.cuda.is_available() else 0}
    if torch.cuda.is_available(): d = torch.cuda.current_device(); st |= {"current_device": d, "device_name": torch.cuda.get_device_name(d), "memory": _gm()}; predictor and predictor.is_loaded and st.update(model_device=predictor.device, model_dtype=str(predictor.torch_dtype))
    return st

@app.post("/gpu/cleanup")
async def gpu_cleanup():
    if not torch.cuda.is_available(): raise HTTPException(400, "CUDA not available")
    d, bf = torch.cuda.current_device(), torch.cuda.memory_reserved(torch.cuda.current_device()); torch.cuda.empty_cache(); torch.cuda.synchronize(); return {"status": "ok", "freed_mb": round((bf - torch.cuda.memory_reserved(d)) / 1e6, 1)}

@app.post("/metrics/forecast/daily", response_model=LongTermForecastResponse)
async def forecast_daily(rq: LongTermMetricsRequest): rq.period_type = "day"; return _fl(rq, "day", cfg.daily_pts_cap, cfg.daily_len, cfg.daily_len_cap)

@app.post("/metrics/forecast/weekly", response_model=LongTermForecastResponse)
async def forecast_weekly(rq: LongTermMetricsRequest): rq.period_type = "week"; return _fl(rq, "week", cfg.weekly_pts_cap, cfg.weekly_len, cfg.weekly_len_cap)

@app.get("/examples/data-format")
async def examples(): return {"simple": {"data": {"values": [100, 120, 110, 130, 125, 140, 135, 150], "series_name": "sales"}, "prediction_length": 6, "num_samples": 100}, "with_timestamps": {"data": {"values": [100, 120, 110, 130, 125, 140], "timestamps": ["2024-01-01", "2024-02-01", "2024-03-01", "2024-04-01", "2024-05-01", "2024-06-01"], "series_name": "monthly_metrics"}, "prediction_length": 3, "confidence_levels": [0.1, 0.5, 0.9]}}

if __name__ == "__main__": uvicorn.run("app.main:app", host="0.0.0.0", port=8000, reload=True)

from datetime import datetime, timedelta
import numpy as np, pandas as pd
from app.utils.logger import log

_AC = {"day": ("5min", 288, "Agregación 5:1 para análisis diario"), "week": ("1h", 168, "Agregación horaria para análisis semanal/mensual")}
_AM = {"mean": "mean", "median": "median", "max": "max", "min": "min", "sum": "sum"}


class TemporalAggregator:
    __slots__ = ()

    @staticmethod
    def infer_freq(ts):
        if len(ts) < 2: return "unknown"
        m = np.median([(ts[i] - ts[i-1]).total_seconds() for i in range(1, min(10, len(ts)))])
        return "1min" if m <= 60 else "5min" if m <= 300 else "10min" if m <= 600 else "1hour" if m <= 3600 else "daily"

    @staticmethod
    def aggregate(pts, period, method="mean"):
        if period not in _AC: raise ValueError(f"Período no soportado: {period}. Usa 'day' o 'week'")
        w, _, d = _AC[period]; df = pd.DataFrame(pts)
        df['timestamp'] = pd.to_datetime(df['timestamp'], format='ISO8601', utc=True).dt.tz_localize(None)
        df = df.sort_values('timestamp'); of = TemporalAggregator.infer_freq(df['timestamp'].tolist())
        if period == "day" and len(pts) <= 100:
            sp = {"start": (st := df['timestamp'].min()).isoformat(), "end": (en := df['timestamp'].max()).isoformat(), "duration_hours": (en - st).total_seconds() / 3600} if len(df) > 1 else None
            return pts, {"original_points": len(pts), "aggregated_points": len(pts), "aggregation_ratio": 1, "original_frequency": of, "target_frequency": "1min", "aggregation_method": "none", "description": "Sin agregación (pocos puntos)", "time_span": sp}
        df.set_index('timestamp', inplace=True); rs = df.resample(w).agg(_AM.get(method, "mean")).dropna(); ct = df.resample(w).count().loc[rs.index]
        ag = [{"timestamp": t.isoformat(), "value": round(float(r['value']), 2), "samples_count": int(ct.loc[t]['value']), "aggregation_window": w, "aggregation_method": method} for t, r in rs.iterrows()]
        st, en = df.index.min(), df.index.max()
        mt = {"original_points": len(pts), "aggregated_points": len(ag), "aggregation_ratio": len(pts) / len(ag) if ag else 0, "original_frequency": of, "target_frequency": w, "aggregation_method": method, "description": d, "time_span": {"start": st.isoformat(), "end": en.isoformat(), "duration_hours": (en - st).total_seconds() / 3600}}
        log.debug(f"Agregación {period}: {mt['original_points']} -> {mt['aggregated_points']} puntos"); return ag, mt

    @staticmethod
    def sample_data(period, start=None):
        st = start or datetime.now() - (timedelta(days=1) if period == "day" else timedelta(weeks=1)); n = 1440 if period == "day" else 10080
        def _v(i):
            ts = st + timedelta(minutes=i); h, wd = ts.hour, ts.weekday()
            b = 10 if h < 6 else 25 + (h - 6) * 10 if h < 9 else 60 if h < 17 else 45 - (h - 17) * 5 if h < 22 else 15
            return {"timestamp": ts.isoformat(), "value": round(max(0, min(100, b * (0.6 if period == "week" and wd >= 5 else 1) + np.random.normal(0, 5))), 2)}
        return [_v(i) for i in range(n)]

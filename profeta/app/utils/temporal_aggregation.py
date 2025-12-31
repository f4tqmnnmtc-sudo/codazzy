from datetime import datetime, timedelta

import numpy as np
import pandas as pd

from app.utils.logger import log

AGGREGATION_CONFIG = {"day": ("5min", 288, "Agregación 5:1 para análisis diario"),
                      "week": ("1h", 168, "Agregación horaria para análisis semanal/mensual")}
AGGREGATION_METHODS = {"mean": "mean", "median": "median", "max": "max", "min": "min", "sum": "sum"}


class TemporalAggregator:
    __slots__ = ()
    
    @staticmethod
    def infer_freq(timestamps: list[datetime]) -> str:
        if len(timestamps) < 2: return "unknown"
        med = np.median([(timestamps[i] - timestamps[i-1]).total_seconds() 
                         for i in range(1, min(10, len(timestamps)))])
        return ("1min" if med <= 60 else "5min" if med <= 300 else "10min" if med <= 600 
                else "1hour" if med <= 3600 else "daily")
    
    @staticmethod
    def aggregate(data_points: list[dict], period: str, 
                  method: str = "mean") -> tuple[list[dict], dict]:
        if period not in AGGREGATION_CONFIG:
            raise ValueError(f"Período no soportado: {period}. Usa 'day' o 'week'")
        
        window, max_pts, desc = AGGREGATION_CONFIG[period]
        df = pd.DataFrame(data_points)
        df['timestamp'] = pd.to_datetime(df['timestamp'], format='ISO8601', utc=True).dt.tz_localize(None)
        df = df.sort_values('timestamp')
        
        orig_freq = TemporalAggregator.infer_freq(df['timestamp'].tolist())
        
        if period == "day" and len(data_points) <= 100:
            span = None
            if len(df) > 1:
                st, en = df['timestamp'].min(), df['timestamp'].max()
                span = {"start": st.isoformat(), "end": en.isoformat(), 
                        "duration_hours": (en - st).total_seconds() / 3600}
            return data_points, {"original_points": len(data_points), "aggregated_points": len(data_points),
                                 "aggregation_ratio": 1, "original_frequency": orig_freq, "target_frequency": "1min",
                                 "aggregation_method": "none", "description": "Sin agregación (pocos puntos)", "time_span": span}
        
        df.set_index('timestamp', inplace=True)
        agg_method = AGGREGATION_METHODS.get(method, "mean")
        resampled = df.resample(window).agg(agg_method).dropna()
        counts = df.resample(window).count().loc[resampled.index]
        
        aggregated = [{"timestamp": ts.isoformat(), "value": round(float(row['value']), 2),
                       "samples_count": int(counts.loc[ts]['value']), "aggregation_window": window,
                       "aggregation_method": method} for ts, row in resampled.iterrows()]
        
        st, en = df.index.min(), df.index.max()
        meta = {"original_points": len(data_points), "aggregated_points": len(aggregated),
                "aggregation_ratio": len(data_points) / len(aggregated) if aggregated else 0,
                "original_frequency": orig_freq, "target_frequency": window, "aggregation_method": method,
                "description": desc, "time_span": {"start": st.isoformat(), "end": en.isoformat(),
                                                   "duration_hours": (en - st).total_seconds() / 3600}}
        
        log.debug(f"Agregación {period}: {meta['original_points']} -> {meta['aggregated_points']} puntos")
        return aggregated, meta
    
    @staticmethod
    def sample_data(period: str, start: datetime | None = None) -> list[dict]:
        if start is None:
            start = datetime.now() - (timedelta(days=1) if period == "day" else timedelta(weeks=1))
        
        n = 1440 if period == "day" else 10080
        data = []
        
        for i in range(n):
            ts = start + timedelta(minutes=i)
            h, wd = ts.hour, ts.weekday()
            
            base = (10 if h < 6 else 25 + (h - 6) * 10 if h < 9 else 60 if h < 17 
                    else 45 - (h - 17) * 5 if h < 22 else 15)
            if period == "week" and wd >= 5: base *= 0.6
            
            val = max(0, min(100, base + np.random.normal(0, 5)))
            data.append({"timestamp": ts.isoformat(), "value": round(val, 2)})
        
        return data

from datetime import datetime

import numpy as np
import pandas as pd

from app.utils.logger import log

TIMESTAMP_FORMATS = ("%Y-%m-%d", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S", 
                     "%Y-%m-%dT%H:%M:%S.%f", "%d/%m/%Y", "%m/%d/%Y")


class DataProcessor:
    __slots__ = ()
    
    def prepare(self, values: list[float], timestamps: list[str] | None = None,
                fill_missing: bool = True, normalize: bool = False) -> dict:
        try:
            clean = self._clean(values, fill_missing)
            ts = self._parse_ts(timestamps) if timestamps else None
            norm, norm_info = (self._normalize(clean) if normalize else (clean, None))
            return {
                "values": norm, "timestamps": ts, "original_length": len(values),
                "processed_length": len(clean), "normalization_info": norm_info,
                "patterns": self._patterns(clean), "data_quality": self._quality(clean)
            }
        except Exception as e:
            log.error(f"Error preparando datos: {e}")
            raise
    
    def _clean(self, values: list[float], fill: bool = True) -> list[float]:
        if len(values) < 2: raise ValueError("Se necesitan al menos 2 valores")
        arr = np.array(values, dtype=float)
        arr[np.isinf(arr)] = np.nan
        
        if (nan_ct := np.isnan(arr).sum()) > 0:
            if not fill: raise ValueError(f"Hay {nan_ct} valores NaN y fill_missing=False")
            if np.isnan(arr).all(): raise ValueError("Todos los valores son NaN")
            s = pd.Series(arr).interpolate(method='linear', limit_direction='both').ffill().bfill()
            arr = s.values
        return arr.tolist()
    
    def _parse_ts(self, timestamps: list[str]) -> list[datetime]:
        result = []
        for ts in timestamps:
            parsed = None
            for fmt in TIMESTAMP_FORMATS:
                try:
                    parsed = datetime.strptime(ts, fmt)
                    break
                except ValueError: continue
            if parsed is None:
                try: parsed = pd.to_datetime(ts)
                except (ValueError, pd.errors.ParserError) as exc:
                    raise ValueError(f"No se pudo parsear timestamp: {ts}") from exc
            result.append(parsed)
        if result != sorted(result): log.warning("Timestamps no ordenados cronológicamente")
        return result
    
    def _normalize(self, values: list[float]) -> tuple:
        arr = np.array(values)
        mn, mx = arr.min(), arr.max()
        if mn == mx: return values, None
        return ((arr - mn) / (mx - mn)).tolist(), {"method": "min_max", "min": float(mn), 
                                                    "max": float(mx), "range": float(mx - mn)}
    
    def _patterns(self, values: list[float]) -> dict:
        arr = np.array(values)
        return {"trend": self._trend(arr), "seasonality": self._seasonality(arr),
                "volatility": self._volatility(arr), "outliers": self._outliers(arr)}
    
    def _trend(self, arr: np.ndarray) -> dict:
        x = np.arange(len(arr))
        corr = np.corrcoef(x, arr)[0, 1]
        slope = corr * (np.std(arr) / np.std(x)) if np.std(x) else 0
        thresh = np.std(arr) * 0.1
        ttype = "increasing" if slope > thresh else ("decreasing" if slope < -thresh else "neutral")
        strength = abs(slope) / np.std(arr) if np.std(arr) else 0
        return {"type": ttype, "slope": float(slope), "strength": float(strength)}
    
    def _seasonality(self, arr: np.ndarray) -> dict:
        n = len(arr)
        if n < 12: return {"detected": False, "reason": "insufficient_data"}
        
        corrs = []
        for lag in range(2, min(n // 2, 24)):
            if n <= lag: continue
            try:
                c = np.corrcoef(arr[:-lag], arr[lag:])[0, 1]
                if not np.isnan(c): corrs.append((lag, abs(c)))
            except (ValueError, FloatingPointError): continue
        
        if not corrs: return {"detected": False, "reason": "no_pattern"}
        best_lag, best_corr = max(corrs, key=lambda x: x[1])
        return {"detected": best_corr > 0.3, "period": best_lag, "strength": float(best_corr)}
    
    def _volatility(self, arr: np.ndarray) -> dict:
        if len(arr) < 2: return {"volatility": 0.0, "cv": 0.0}
        std, mean = np.std(arr), np.mean(arr)
        return {"volatility": float(std), "cv": float(std / abs(mean)) if mean else float('inf')}
    
    def _outliers(self, arr: np.ndarray) -> dict:
        if len(arr) < 4: return {"count": 0, "indices": [], "method": "insufficient_data"}
        q1, q3 = np.percentile(arr, 25), np.percentile(arr, 75)
        iqr = q3 - q1
        lo, hi = q1 - 1.5 * iqr, q3 + 1.5 * iqr
        idx = np.where((arr < lo) | (arr > hi))[0].tolist()
        return {"count": len(idx), "indices": idx, "method": "iqr", "bounds": {"lower": float(lo), "upper": float(hi)}}
    
    def _quality(self, values: list[float]) -> dict:
        arr = np.array(values)
        mean, std = np.mean(arr), np.std(arr)
        
        score = 1.0
        if len(arr) < 10: score *= 0.7
        elif len(arr) < 20: score *= 0.9
        if mean and std / abs(mean) < 0.01: score *= 0.5
        if (ur := len(np.unique(arr)) / len(arr)) < 0.5: score *= 0.8
        
        return {"length": len(values), "completeness": 1.0, "mean": float(mean), "std": float(std),
                "min": float(arr.min()), "max": float(arr.max()), "zeros_count": int((arr == 0).sum()),
                "unique_values": len(np.unique(arr)), "quality_score": max(0, min(1, score))}
    
    def denormalize(self, preds: dict[str, list[float]], norm_info: dict | None) -> dict[str, list[float]]:
        if not norm_info or norm_info.get("method") != "min_max": return preds
        mn, rng = norm_info["min"], norm_info["range"]
        return {k: [v * rng + mn for v in vals] for k, vals in preds.items()}

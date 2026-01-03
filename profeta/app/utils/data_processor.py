from datetime import datetime
import numpy as np, pandas as pd
from app.utils.logger import log

_TF = ("%Y-%m-%d", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M:%S.%f", "%d/%m/%Y", "%m/%d/%Y")


class DataProcessor:
    __slots__ = ()

    def prepare(self, vals: list[float], ts: list[str] | None = None, fill: bool = True, norm: bool = False) -> dict:
        try:
            c = self._c(vals, fill); t = self._t(ts) if ts else None; n, ni = (self._n(c) if norm else (c, None))
            return {"values": n, "timestamps": t, "original_length": len(vals), "processed_length": len(c), "normalization_info": ni, "patterns": self._p(c), "data_quality": self._q(c)}
        except Exception as e: log.error(f"Error preparando datos: {e}"); raise

    def _c(self, v, fill=True):
        if len(v) < 2: raise ValueError("Se necesitan al menos 2 valores")
        a = np.array(v, dtype=float); a[np.isinf(a)] = np.nan
        if (nc := np.isnan(a).sum()) > 0:
            if not fill: raise ValueError(f"Hay {nc} valores NaN y fill_missing=False")
            if np.isnan(a).all(): raise ValueError("Todos los valores son NaN")
            a = pd.Series(a).interpolate(method='linear', limit_direction='both').ffill().bfill().values
        return a.tolist()

    def _t(self, ts):
        def _p(s):
            for f in _TF:
                try: return datetime.strptime(s, f)
                except: pass
            try: return pd.to_datetime(s)
            except: raise ValueError(f"No se pudo parsear timestamp: {s}")
        r = [_p(x) for x in ts]; r != sorted(r) and log.warning("Timestamps no ordenados cronológicamente"); return r

    def _n(self, v):
        a = np.array(v); mn, mx = a.min(), a.max()
        return (v, None) if mn == mx else (((a - mn) / (mx - mn)).tolist(), {"method": "min_max", "min": float(mn), "max": float(mx), "range": float(mx - mn)})

    def _p(self, v):
        a = np.array(v)
        x = np.arange(len(a)); cr = np.corrcoef(x, a)[0, 1]; sl = cr * (np.std(a) / np.std(x)) if np.std(x) else 0; th = np.std(a) * 0.1
        tr = {"type": "increasing" if sl > th else ("decreasing" if sl < -th else "neutral"), "slope": float(sl), "strength": abs(sl) / np.std(a) if np.std(a) else 0}
        n = len(a); corrs = []
        if n >= 12:
            for lag in range(2, min(n // 2, 24)):
                try: c = np.corrcoef(a[:-lag], a[lag:])[0, 1]; np.isnan(c) or corrs.append((lag, abs(c)))
                except: pass
        ss = {"detected": False, "reason": "insufficient_data"} if n < 12 else ({"detected": False, "reason": "no_pattern"} if not corrs else (lambda bl, bc: {"detected": bc > 0.3, "period": bl, "strength": float(bc)})(*max(corrs, key=lambda x: x[1])))
        s, m = np.std(a), np.mean(a); vo = {"volatility": float(s), "cv": float(s / abs(m)) if m else float('inf')} if len(a) >= 2 else {"volatility": 0.0, "cv": 0.0}
        if len(a) >= 4: q1, q3 = np.percentile(a, 25), np.percentile(a, 75); iq = q3 - q1; lo, hi = q1 - 1.5 * iq, q3 + 1.5 * iq; idx = np.where((a < lo) | (a > hi))[0].tolist(); ol = {"count": len(idx), "indices": idx, "method": "iqr", "bounds": {"lower": float(lo), "upper": float(hi)}}
        else: ol = {"count": 0, "indices": [], "method": "insufficient_data"}
        return {"trend": tr, "seasonality": ss, "volatility": vo, "outliers": ol}

    def _q(self, v):
        a = np.array(v); m, s = np.mean(a), np.std(a); sc = 0.7 if len(a) < 10 else (0.9 if len(a) < 20 else 1.0)
        m and s / abs(m) < 0.01 and (sc := sc * 0.5); len(np.unique(a)) / len(a) < 0.5 and (sc := sc * 0.8)
        return {"length": len(v), "completeness": 1.0, "mean": float(m), "std": float(s), "min": float(a.min()), "max": float(a.max()), "zeros_count": int((a == 0).sum()), "unique_values": len(np.unique(a)), "quality_score": max(0, min(1, sc))}

    def denormalize(self, preds, ni): return preds if not ni or ni.get("method") != "min_max" else {k: [x * ni["range"] + ni["min"] for x in vs] for k, vs in preds.items()}

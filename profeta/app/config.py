import os
from functools import lru_cache
from random import uniform


def _env(k, fallback, cast=str):
    v = os.getenv(k, "")
    return cast(v) if v else fallback

def _env_list(k, fallback):
    v = os.getenv(k, "")
    return [float(x.strip()) for x in v.split(",")] if v else fallback


@lru_cache(maxsize=1)
def _load():
    return {
        "model_name": os.getenv("MODEL_NAME", os.getenv("CHRONOS_MODEL", "amazon/chronos-t5-tiny")),
        "use_gpu": os.getenv("USE_GPU", "true").lower() not in ("false", "0", "no"),
        
        "pred_len": _env("PROFETA_PRED_LEN", 12, int),
        "pred_len_min": _env("PROFETA_PRED_LEN_MIN", 1, int),
        "pred_len_max": _env("PROFETA_PRED_LEN_MAX", 100, int),
        "pred_len_cap": _env("PROFETA_PRED_LEN_CAP", 1024, int),
        
        "sim": _env("PROFETA_SIM", 100, int),
        "sim_min": _env("PROFETA_SIM_MIN", 10, int),
        "sim_max": _env("PROFETA_SIM_MAX", 1000, int),
        "sim_long": _env("PROFETA_SIM_LONG", 1000, int),
        "sim_floor": _env("PROFETA_SIM_FLOOR", 50, int),
        
        "conf": _env_list("PROFETA_CONF", [0.1, 0.5, 0.9]),
        "conf_extended": _env_list("PROFETA_CONF_EXT", [0.1, 0.2, 0.5, 0.8, 0.9]),
        
        "freq_fallback": _env("PROFETA_FREQ_FALLBACK", 60.0, float),
        
        "trend_tail": _env("PROFETA_TREND_TAIL", 10, int),
        "trend_thresh": _env("PROFETA_TREND_THRESH", 0.1, float),
        "peak_hour_k": _env("PROFETA_PEAK_HOUR_K", 1.2, float),
        "low_hour_k": _env("PROFETA_LOW_HOUR_K", 0.8, float),
        "peak_day_k": _env("PROFETA_PEAK_DAY_K", 1.15, float),
        "low_day_k": _env("PROFETA_LOW_DAY_K", 0.85, float),
        
        "auto_daily_days": _env("PROFETA_AUTO_DAILY_DAYS", 6.9, float),
        
        "batch_cap": _env("PROFETA_BATCH_CAP", 10, int),
        
        "daily_pts_cap": _env("PROFETA_DAILY_PTS_CAP", 1440, int),
        "daily_len": _env("PROFETA_DAILY_LEN", 240, int),
        "daily_len_cap": _env("PROFETA_DAILY_LEN_CAP", 1024, int),
        "weekly_pts_cap": _env("PROFETA_WEEKLY_PTS_CAP", 10080, int),
        "weekly_len": _env("PROFETA_WEEKLY_LEN", 168, int),
        "weekly_len_cap": _env("PROFETA_WEEKLY_LEN_CAP", 2200, int),
        "metrics_cap": _env("PROFETA_METRICS_CAP", 200, int),
        
        "gpu_base_mb": _env("PROFETA_GPU_BASE_MB", 800, float),
        "gpu_ctx_k": _env("PROFETA_GPU_CTX_K", 0.15, float),
        "gpu_sim_k": _env("PROFETA_GPU_SIM_K", 0.0144, float),
        "gpu_margin": _env("PROFETA_GPU_MARGIN", 2e9, float),
        "gpu_safety": _env("PROFETA_GPU_SAFETY", 0.8, float),
        
        "sim_reduce_k": _env_list("PROFETA_SIM_REDUCE_K", [0.5, 0.25]),
        "ctx_reduce_k": _env_list("PROFETA_CTX_REDUCE_K", [0.6, 0.4]),
        "ctx_reduce_min": _env("PROFETA_CTX_REDUCE_MIN", 100, int),
        "ctx_reduce_floor": _env("PROFETA_CTX_REDUCE_FLOOR", 50, int),
        
        "raw_cap": _env("PROFETA_RAW_CAP", 1000, int),
        
        "jitter_on": _env("PROFETA_JITTER_ON", "1", str) == "1",
        "jitter_k": _env("PROFETA_JITTER_K", 0.03, float),
    }


class Cfg:
    __slots__ = ()
    
    def __getattr__(self, k):
        return _load().get(k)
    
    def jitter(self, val, pct=None):
        c = _load()
        if not c["jitter_on"]: return val
        p = pct or c["jitter_k"]
        return val * uniform(1 - p, 1 + p)
    
    def reload(self):
        _load.cache_clear()


cfg = Cfg()

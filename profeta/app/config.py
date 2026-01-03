import os
from functools import lru_cache
from random import uniform

_e = lambda k, fb, c=str: c(v) if (v := os.getenv(k, "")) else fb
_el = lambda k, fb: [float(x.strip()) for x in v.split(",")] if (v := os.getenv(k, "")) else fb

@lru_cache(maxsize=1)
def _ld():
    g = lambda k, fb: os.getenv(k, fb)
    return {"model_name": g("MODEL_NAME", g("CHRONOS_MODEL", "amazon/chronos-t5-tiny")), "use_gpu": g("USE_GPU", "true").lower() not in ("false", "0", "no"),
        "pred_len": _e("PROFETA_PRED_LEN", 12, int), "pred_len_min": _e("PROFETA_PRED_LEN_MIN", 1, int), "pred_len_max": _e("PROFETA_PRED_LEN_MAX", 100, int), "pred_len_cap": _e("PROFETA_PRED_LEN_CAP", 1024, int),
        "sim": _e("PROFETA_SIM", 100, int), "sim_min": _e("PROFETA_SIM_MIN", 10, int), "sim_max": _e("PROFETA_SIM_MAX", 1000, int), "sim_long": _e("PROFETA_SIM_LONG", 1000, int), "sim_floor": _e("PROFETA_SIM_FLOOR", 50, int),
        "conf": _el("PROFETA_CONF", [0.1, 0.5, 0.9]), "conf_extended": _el("PROFETA_CONF_EXT", [0.1, 0.2, 0.5, 0.8, 0.9]), "freq_fallback": _e("PROFETA_FREQ_FALLBACK", 60.0, float),
        "trend_tail": _e("PROFETA_TREND_TAIL", 10, int), "trend_thresh": _e("PROFETA_TREND_THRESH", 0.1, float), "peak_hour_k": _e("PROFETA_PEAK_HOUR_K", 1.2, float), "low_hour_k": _e("PROFETA_LOW_HOUR_K", 0.8, float),
        "peak_day_k": _e("PROFETA_PEAK_DAY_K", 1.15, float), "low_day_k": _e("PROFETA_LOW_DAY_K", 0.85, float), "auto_daily_days": _e("PROFETA_AUTO_DAILY_DAYS", 6.9, float), "batch_cap": _e("PROFETA_BATCH_CAP", 10, int),
        "daily_pts_cap": _e("PROFETA_DAILY_PTS_CAP", 1440, int), "daily_len": _e("PROFETA_DAILY_LEN", 240, int), "daily_len_cap": _e("PROFETA_DAILY_LEN_CAP", 1024, int),
        "weekly_pts_cap": _e("PROFETA_WEEKLY_PTS_CAP", 10080, int), "weekly_len": _e("PROFETA_WEEKLY_LEN", 168, int), "weekly_len_cap": _e("PROFETA_WEEKLY_LEN_CAP", 2200, int), "metrics_cap": _e("PROFETA_METRICS_CAP", 200, int),
        "gpu_base_mb": _e("PROFETA_GPU_BASE_MB", 800, float), "gpu_ctx_k": _e("PROFETA_GPU_CTX_K", 0.15, float), "gpu_sim_k": _e("PROFETA_GPU_SIM_K", 0.0144, float), "gpu_margin": _e("PROFETA_GPU_MARGIN", 2e9, float), "gpu_safety": _e("PROFETA_GPU_SAFETY", 0.8, float),
        "sim_reduce_k": _el("PROFETA_SIM_REDUCE_K", [0.5, 0.25]), "ctx_reduce_k": _el("PROFETA_CTX_REDUCE_K", [0.6, 0.4]), "ctx_reduce_min": _e("PROFETA_CTX_REDUCE_MIN", 100, int), "ctx_reduce_floor": _e("PROFETA_CTX_REDUCE_FLOOR", 50, int),
        "raw_cap": _e("PROFETA_RAW_CAP", 1000, int), "jitter_on": _e("PROFETA_JITTER_ON", "1", str) == "1", "jitter_k": _e("PROFETA_JITTER_K", 0.03, float)}

class Cfg:
    __slots__ = ()
    __getattr__ = lambda s, k: _ld().get(k)
    def jitter(s, v, p=None): c = _ld(); return v if not c["jitter_on"] else v * uniform(1 - (p or c["jitter_k"]), 1 + (p or c["jitter_k"]))
    def reload(s): _ld.cache_clear()

cfg = Cfg()

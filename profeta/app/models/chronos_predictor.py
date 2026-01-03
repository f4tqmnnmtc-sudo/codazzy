import time, gc
import torch, numpy as np
from app.utils.logger import log
from app.config import cfg

_BQ = (0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9) # Bolt
_CQ = (0.01, 0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.45, 0.5, 0.55, 0.6, 0.65, 0.7, 0.75, 0.8, 0.85, 0.9, 0.95, 0.99) # Chronos 2


class _M:
    __slots__ = ('d',)
    def __init__(s, d): s.d = d
    def __enter__(s): return s
    def __exit__(s, *_): torch.cuda.is_available() and s.d == "cuda" and (torch.cuda.empty_cache(), torch.cuda.synchronize(), gc.collect())


class _Q:
    __slots__ = ('a',)
    def __init__(s, q): s.a = q
    def __call__(s, p, lv):
        o = {}
        for l in lv:
            if l in s.a: o[str(l)] = p[s.a.index(l)].tolist()
            else: u = next((i for i, q in enumerate(s.a) if q >= l), len(s.a) - 1); lo = max(0, u - 1); o[str(l)] = p[u].tolist() if lo == u else ((1 - (w := (l - s.a[lo]) / (s.a[u] - s.a[lo]))) * p[lo] + w * p[u]).tolist()
        return o


class ChronosPredictor:
    __slots__ = ('model_name', 'pipeline', 'model_type', 'device', 'torch_dtype', '_bq', '_cq')

    def __init__(s, model_name=None):
        s.model_name = model_name or cfg.model_name; s.pipeline = None; nm = s.model_name.lower()
        s.model_type = "chronos2" if "chronos-2" in nm else ("bolt" if "bolt" in nm else "t5")
        gpu = cfg.use_gpu and torch.cuda.is_available(); s.device = "cuda" if gpu else "cpu"
        s.torch_dtype = torch.bfloat16 if s.device == "cuda" else torch.float32
        s._bq, s._cq = _Q(_BQ), _Q(_CQ)
        torch.cuda.is_available() and gpu and log.info(f"GPU: {torch.cuda.get_device_name(0)}, CUDA {torch.version.cuda}")
        torch.cuda.is_available() and not cfg.use_gpu and log.info("GPU disponible pero USE_GPU=false, usando CPU")
        not torch.cuda.is_available() and log.info("GPU no disponible, usando CPU")
        s._ld()

    def _ld(s):
        log.info(f"Cargando {s.model_name} [{s.model_type}] en {s.device}")
        try:
            if s.model_type == "chronos2": from chronos import Chronos2Pipeline; s.pipeline = Chronos2Pipeline.from_pretrained(s.model_name, device_map=s.device, dtype=s.torch_dtype)
            elif s.model_type == "bolt": from chronos import ChronosBoltPipeline; s.pipeline = ChronosBoltPipeline.from_pretrained(s.model_name, device_map=s.device, torch_dtype=s.torch_dtype)
            else: from chronos import ChronosPipeline; s.pipeline = ChronosPipeline.from_pretrained(s.model_name, device_map=s.device, torch_dtype=s.torch_dtype)
            log.info(f"Modelo {s.model_type} cargado")
        except Exception as e: log.error(f"Error cargando modelo: {e}"); s.pipeline = None; raise

    @property
    def is_loaded(s): return s.pipeline is not None

    def _me(s, c, p, n): return cfg.jitter(cfg.gpu_base_mb) + c * cfg.jitter(cfg.gpu_ctx_k) + p * n * cfg.jitter(cfg.gpu_sim_k)

    def _ma(s):
        if not torch.cuda.is_available() or s.device != "cuda": return 0
        try: d = torch.cuda.current_device(); return max(0, (torch.cuda.get_device_properties(d).total_memory - torch.cuda.memory_reserved(d) - cfg.gpu_margin) / 1e6)
        except: return 0

    def adjust(s, cl, pl, ns, acr=True):
        if s.device == "cpu": return {'context_length': cl, 'prediction_length': pl, 'num_samples': ns, 'adjustments_made': [], 'memory_saved_mb': 0, 'can_fit': True}
        av, est = s._ma(), s._me(cl, pl, ns)
        if est <= av: return {'context_length': cl, 'prediction_length': pl, 'num_samples': ns, 'adjustments_made': [], 'memory_saved_mb': 0, 'can_fit': True}
        ac, ap, asim, ch = cl, pl, ns, []
        for k in cfg.sim_reduce_k:
            n = max(cfg.sim_floor, int(ns * k))
            if s._me(ac, ap, n) <= av: asim = n; ch.append(f"sim: {ns} -> {asim}"); break
        if (cr := s._me(ac, ap, asim)) <= av: return {'context_length': ac, 'prediction_length': ap, 'num_samples': asim, 'adjustments_made': ch, 'memory_saved_mb': est - cr, 'can_fit': True}
        if acr and ac > cfg.ctx_reduce_min:
            for k in cfg.ctx_reduce_k:
                nc = max(cfg.ctx_reduce_floor, int(cl * k))
                if s._me(nc, ap, asim) <= av: ac = nc; ch.append(f"context: {cl} -> {ac}"); break
        fn = s._me(ac, ap, asim); return {'context_length': ac, 'prediction_length': ap, 'num_samples': asim, 'adjustments_made': ch, 'memory_saved_mb': est - fn, 'can_fit': fn <= av}

    def predict(s, ctx, pl=None, ns=None, cl=None):
        pl, ns, lv = pl or cfg.pred_len, ns or cfg.sim, cl or cfg.conf
        if not s.is_loaded: raise RuntimeError("Modelo no cargado")
        t0 = time.time(); c = torch.tensor(ctx, dtype=torch.float32) if isinstance(ctx, list) else ctx
        with _M(s.device):
            try:
                if s.model_type == "chronos2": return s._p2(c, pl, lv, t0)
                if s.model_type == "bolt": return s._pb(c, pl, lv, t0)
                return s._p5(c, pl, ns, lv, t0)
            except RuntimeError as e: "out of memory" in str(e).lower() and log.error(f"OOM: {e}"); raise RuntimeError("Memoria GPU insuficiente") if "out of memory" in str(e).lower() else e

    # Chronos 2
    def _p2(s, c, pl, lv, t0):
        a = s.pipeline.predict(inputs=c.unsqueeze(0).unsqueeze(0), prediction_length=pl)[0].numpy()[0]
        return {"quantiles": s._cq(a, lv), "raw_samples": None, "model_info": {"model_name": s.model_name, "model_type": "chronos2", "device": s.device, "torch_dtype": str(s.torch_dtype), "context_length": len(c), "prediction_length": pl, "available_quantiles": list(_CQ)}, "processing_time": time.time() - t0}

    # Bolt
    def _pb(s, c, pl, lv, t0):
        a = s.pipeline.predict(inputs=c, prediction_length=pl)[0].numpy()
        return {"quantiles": s._bq(a, lv), "raw_samples": None, "model_info": {"model_name": s.model_name, "model_type": "bolt", "device": s.device, "torch_dtype": str(s.torch_dtype), "context_length": len(c), "prediction_length": pl, "available_quantiles": list(_BQ)}, "processing_time": time.time() - t0}

    # Chronos
    def _p5(s, c, pl, ns, lv, t0):
        o = ns
        if (av := s._ma()) > 0 and s._me(len(c), pl, ns) > av * cfg.gpu_safety: ns = max(cfg.sim_floor, ns // 2); log.warning(f"Reduciendo sim {o} -> {ns}")
        a = s.pipeline.predict(inputs=c, prediction_length=pl, num_samples=ns)[0].numpy()
        r = {"quantiles": {str(l): np.quantile(a, l, axis=0).tolist() for l in lv}, "raw_samples": a.tolist() if len(a) <= cfg.raw_cap else None,
             "model_info": {"model_name": s.model_name, "model_type": "t5", "device": s.device, "torch_dtype": str(s.torch_dtype), "context_length": len(c), "prediction_length": pl, "num_samples": ns, "num_samples_requested": o, "num_samples_adjusted": ns < o}, "processing_time": time.time() - t0}
        ns < o and r.update(warning=f"Sim reducido {o} -> {ns}"); return r

    def batch(s, ctxs, pl=None, ns=None, cl=None):
        pl, ns, lv = pl or cfg.pred_len, ns or cfg.sim, cl or cfg.conf; o = []
        for i, c in enumerate(ctxs):
            try: r = s.predict(c, pl, ns, lv); r["series_index"] = i; o.append(r)
            except Exception as e: log.error(f"Batch[{i}]: {e}"); o.append({"error": str(e), "series_index": i})
        return o

    def info(s):
        if not s.is_loaded: return {"error": "Modelo no cargado"}
        b = {"model_name": s.model_name, "model_type": s.model_type, "device": s.device, "torch_dtype": str(s.torch_dtype), "cuda_available": torch.cuda.is_available(), "model_loaded": True}
        s.model_type == "chronos2" and b.update(available_quantiles=list(_CQ), supports_covariates=True, max_context_length=8192)
        s.model_type == "bolt" and b.update(available_quantiles=list(_BQ), supports_covariates=False)
        s.model_type == "t5" and b.update(supports_num_samples=True, supports_covariates=False)
        return b

    def clear_cache(s):
        if not torch.cuda.is_available(): return {"status": "no_cuda"}
        with _M(s.device): pass
        d = torch.cuda.current_device(); return {"status": "ok", "allocated_mb": torch.cuda.memory_allocated(d) / 1e6, "reserved_mb": torch.cuda.memory_reserved(d) / 1e6}

    def cleanup(s): s.pipeline = None; torch.cuda.is_available() and s.device == "cuda" and (torch.cuda.empty_cache(), gc.collect())

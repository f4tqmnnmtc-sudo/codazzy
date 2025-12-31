import time, gc
from typing import Any

import torch
import numpy as np

from app.utils.logger import log
from app.config import cfg


BOLT_QUANTILES = (0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9)
CHRONOS2_QUANTILES = (0.01, 0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.45,
                      0.5, 0.55, 0.6, 0.65, 0.7, 0.75, 0.8, 0.85, 0.9, 0.95, 0.99)


class MemoryGuard:
    __slots__ = ('device', 'dtype')
    
    def __init__(self, device, dtype): self.device, self.dtype = device, dtype
    def __enter__(self): return self
    def __exit__(self, *_):
        if torch.cuda.is_available() and self.device == "cuda":
            torch.cuda.empty_cache()
            torch.cuda.synchronize()
            gc.collect()


class QuantileInterpolator:
    __slots__ = ('available',)
    
    def __init__(self, quantiles: tuple): self.available = quantiles
    
    def interpolate(self, preds: np.ndarray, levels: list[float]) -> dict[str, list[float]]:
        out = {}
        for lvl in levels:
            if lvl in self.available:
                out[str(lvl)] = preds[self.available.index(lvl)].tolist()
            else:
                upper = next((i for i, q in enumerate(self.available) if q >= lvl), len(self.available) - 1)
                lower = max(0, upper - 1)
                if lower == upper:
                    out[str(lvl)] = preds[upper].tolist()
                else:
                    w = (lvl - self.available[lower]) / (self.available[upper] - self.available[lower])
                    out[str(lvl)] = ((1 - w) * preds[lower] + w * preds[upper]).tolist()
        return out


class ChronosPredictor:
    __slots__ = ('model_name', 'pipeline', 'model_type', 'device', 'torch_dtype', 
                 'bolt_interpolator', 'chronos2_interpolator')
    
    def __init__(self, model_name: str = None):
        self.model_name = model_name or cfg.model_name
        self.pipeline = None
        
        nm = self.model_name.lower()
        self.model_type = "chronos2" if "chronos-2" in nm else ("bolt" if "bolt" in nm else "t5")
        
        gpu_enabled = cfg.use_gpu and torch.cuda.is_available()
        self.device = "cuda" if gpu_enabled else "cpu"
        self.torch_dtype = torch.bfloat16 if self.device == "cuda" else torch.float32
        
        self.bolt_interpolator = QuantileInterpolator(BOLT_QUANTILES)
        self.chronos2_interpolator = QuantileInterpolator(CHRONOS2_QUANTILES)
        
        if torch.cuda.is_available() and gpu_enabled:
            log.info(f"GPU: {torch.cuda.get_device_name(0)}, CUDA {torch.version.cuda}")
        elif torch.cuda.is_available() and not cfg.use_gpu:
            log.info("GPU disponible pero USE_GPU=false, usando CPU")
        else:
            log.info("GPU no disponible, usando CPU")
        
        self._load()
    
    def _load(self):
        log.info(f"Cargando {self.model_name} [{self.model_type}] en {self.device}")
        try:
            if self.model_type == "chronos2":
                from chronos import Chronos2Pipeline
                self.pipeline = Chronos2Pipeline.from_pretrained(self.model_name, device_map=self.device, dtype=self.torch_dtype)
            elif self.model_type == "bolt":
                from chronos import ChronosBoltPipeline
                self.pipeline = ChronosBoltPipeline.from_pretrained(self.model_name, device_map=self.device, torch_dtype=self.torch_dtype)
            else:
                from chronos import ChronosPipeline
                self.pipeline = ChronosPipeline.from_pretrained(self.model_name, device_map=self.device, torch_dtype=self.torch_dtype)
            log.info(f"Modelo {self.model_type} cargado")
        except Exception as e:
            log.error(f"Error cargando modelo: {e}")
            self.pipeline = None
            raise
    
    @property
    def is_loaded(self) -> bool: return self.pipeline is not None
    
    def _mem_estimate(self, ctx: int, pred: int, n_sim: int) -> float:
        base = cfg.jitter(cfg.gpu_base_mb)
        ctx_k = cfg.jitter(cfg.gpu_ctx_k)
        sim_k = cfg.jitter(cfg.gpu_sim_k)
        return base + ctx * ctx_k + pred * n_sim * sim_k
    
    def _mem_available(self) -> float:
        if not torch.cuda.is_available() or self.device != "cuda": return 0
        try:
            dev = torch.cuda.current_device()
            avail = torch.cuda.get_device_properties(dev).total_memory - torch.cuda.memory_reserved(dev)
            margin = cfg.gpu_margin
            return max(0, (avail - margin) / 1e6)
        except (RuntimeError, AssertionError): return 0
    
    def adjust(self, ctx_len: int, pred_len: int, n_sim: int, 
               allow_ctx_reduction: bool = True) -> dict:
        if self.device == "cpu":
            return {'context_length': ctx_len, 'prediction_length': pred_len, 'num_samples': n_sim,
                    'adjustments_made': [], 'memory_saved_mb': 0, 'can_fit': True}
        
        avail = self._mem_available()
        est = self._mem_estimate(ctx_len, pred_len, n_sim)
        
        if est <= avail:
            return {'context_length': ctx_len, 'prediction_length': pred_len, 'num_samples': n_sim,
                    'adjustments_made': [], 'memory_saved_mb': 0, 'can_fit': True}
        
        adj_ctx, adj_pred, adj_sim, changes = ctx_len, pred_len, n_sim, []
        floor = cfg.sim_floor
        
        for k in cfg.sim_reduce_k:
            ns = max(floor, int(n_sim * k))
            if self._mem_estimate(adj_ctx, adj_pred, ns) <= avail:
                adj_sim = ns
                changes.append(f"sim: {n_sim} -> {adj_sim}")
                break
        
        curr = self._mem_estimate(adj_ctx, adj_pred, adj_sim)
        if curr <= avail:
            return {'context_length': adj_ctx, 'prediction_length': adj_pred, 'num_samples': adj_sim,
                    'adjustments_made': changes, 'memory_saved_mb': est - curr, 'can_fit': True}
        
        ctx_min = cfg.ctx_reduce_min
        ctx_floor = cfg.ctx_reduce_floor
        if allow_ctx_reduction and adj_ctx > ctx_min:
            for k in cfg.ctx_reduce_k:
                nc = max(ctx_floor, int(ctx_len * k))
                if self._mem_estimate(nc, adj_pred, adj_sim) <= avail:
                    adj_ctx = nc
                    changes.append(f"context: {ctx_len} -> {adj_ctx}")
                    break
        
        final = self._mem_estimate(adj_ctx, adj_pred, adj_sim)
        return {'context_length': adj_ctx, 'prediction_length': adj_pred, 'num_samples': adj_sim,
                'adjustments_made': changes, 'memory_saved_mb': est - final, 'can_fit': final <= avail}
    
    def predict(self, context: list[float] | torch.Tensor, prediction_length: int = None,
                num_samples: int = None, confidence_levels: list[float] | None = None) -> dict[str, Any]:
        pred_len = prediction_length or cfg.pred_len
        n_sim = num_samples or cfg.sim
        levels = confidence_levels or cfg.conf
        
        if not self.is_loaded: raise RuntimeError("Modelo no cargado")
        
        t0 = time.time()
        ctx = torch.tensor(context, dtype=torch.float32) if isinstance(context, list) else context
        
        with MemoryGuard(self.device, self.torch_dtype):
            try:
                if self.model_type == "chronos2":
                    return self._predict_chronos2(ctx, pred_len, levels, t0)
                elif self.model_type == "bolt":
                    return self._predict_bolt(ctx, pred_len, levels, t0)
                return self._predict_t5(ctx, pred_len, n_sim, levels, t0)
            except RuntimeError as e:
                if "out of memory" in str(e).lower():
                    log.error(f"OOM: {e}")
                    raise RuntimeError("Memoria GPU insuficiente")
                raise
    
    def _predict_chronos2(self, ctx: torch.Tensor, pred_len: int, levels: list[float], t0: float) -> dict:
        preds = self.pipeline.predict(inputs=ctx.unsqueeze(0).unsqueeze(0), prediction_length=pred_len)
        arr = preds[0].numpy()[0]
        return {
            "quantiles": self.chronos2_interpolator.interpolate(arr, levels), "raw_samples": None,
            "model_info": {"model_name": self.model_name, "model_type": "chronos2", "device": self.device,
                           "torch_dtype": str(self.torch_dtype), "context_length": len(ctx),
                           "prediction_length": pred_len, "available_quantiles": list(CHRONOS2_QUANTILES)},
            "processing_time": time.time() - t0
        }
    
    def _predict_bolt(self, ctx: torch.Tensor, pred_len: int, levels: list[float], t0: float) -> dict:
        preds = self.pipeline.predict(inputs=ctx, prediction_length=pred_len)
        arr = preds[0].numpy()
        return {
            "quantiles": self.bolt_interpolator.interpolate(arr, levels), "raw_samples": None,
            "model_info": {"model_name": self.model_name, "model_type": "bolt", "device": self.device,
                           "torch_dtype": str(self.torch_dtype), "context_length": len(ctx),
                           "prediction_length": pred_len, "available_quantiles": list(BOLT_QUANTILES)},
            "processing_time": time.time() - t0
        }
    
    def _predict_t5(self, ctx: torch.Tensor, pred_len: int, n_sim: int, levels: list[float], t0: float) -> dict:
        orig = n_sim
        avail = self._mem_available()
        safety = cfg.gpu_safety
        floor = cfg.sim_floor
        
        if avail > 0 and self._mem_estimate(len(ctx), pred_len, n_sim) > avail * safety:
            n_sim = max(floor, n_sim // 2)
            log.warning(f"Reduciendo sim {orig} -> {n_sim}")
        
        preds = self.pipeline.predict(inputs=ctx, prediction_length=pred_len, num_samples=n_sim)
        arr = preds[0].numpy()
        
        raw_cap = cfg.raw_cap
        res = {
            "quantiles": {str(l): np.quantile(arr, l, axis=0).tolist() for l in levels},
            "raw_samples": arr.tolist() if len(arr) <= raw_cap else None,
            "model_info": {"model_name": self.model_name, "model_type": "t5", "device": self.device,
                           "torch_dtype": str(self.torch_dtype), "context_length": len(ctx),
                           "prediction_length": pred_len, "num_samples": n_sim,
                           "num_samples_requested": orig, "num_samples_adjusted": n_sim < orig},
            "processing_time": time.time() - t0
        }
        if n_sim < orig: res["warning"] = f"Sim reducido {orig} -> {n_sim}"
        return res
    
    def batch(self, contexts: list, prediction_length: int = None,
              num_samples: int = None, confidence_levels: list[float] | None = None) -> list[dict]:
        pred_len = prediction_length or cfg.pred_len
        n_sim = num_samples or cfg.sim
        levels = confidence_levels or cfg.conf
        
        results = []
        for i, ctx in enumerate(contexts):
            try:
                r = self.predict(ctx, pred_len, n_sim, levels)
                r["series_index"] = i
                results.append(r)
            except Exception as e:
                log.error(f"Batch[{i}]: {e}")
                results.append({"error": str(e), "series_index": i})
        return results
    
    def info(self) -> dict[str, Any]:
        if not self.is_loaded: return {"error": "Modelo no cargado"}
        base = {"model_name": self.model_name, "model_type": self.model_type, "device": self.device,
                "torch_dtype": str(self.torch_dtype), "cuda_available": torch.cuda.is_available(), "model_loaded": True}
        if self.model_type == "chronos2":
            base.update({"available_quantiles": list(CHRONOS2_QUANTILES), "supports_covariates": True, "max_context_length": 8192})
        elif self.model_type == "bolt":
            base.update({"available_quantiles": list(BOLT_QUANTILES), "supports_covariates": False})
        else:
            base.update({"supports_num_samples": True, "supports_covariates": False})
        return base
    
    def clear_cache(self) -> dict:
        if not torch.cuda.is_available(): return {"status": "no_cuda"}
        with MemoryGuard(self.device, self.torch_dtype): pass
        dev = torch.cuda.current_device()
        return {"status": "ok", "allocated_mb": torch.cuda.memory_allocated(dev) / 1e6,
                "reserved_mb": torch.cuda.memory_reserved(dev) / 1e6}
    
    def cleanup(self):
        self.pipeline = None
        with MemoryGuard(self.device, self.torch_dtype): pass

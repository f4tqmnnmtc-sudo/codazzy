import logging, sys, os

_logger = None


def _setup_logger():
    global _logger
    if _logger: return _logger
    
    _logger = logging.getLogger("profeta")
    _logger.setLevel(getattr(logging, os.getenv("LOG_LEVEL", "INFO").upper(), logging.INFO))
    
    if not _logger.handlers:
        h = logging.StreamHandler(sys.stdout)
        h.setFormatter(logging.Formatter("%(asctime)s [%(levelname)s] %(message)s", "%H:%M:%S"))
        _logger.addHandler(h)
    return _logger


class LazyLogger:
    __slots__ = ()
    def __getattr__(self, name): return getattr(_setup_logger(), name)


log = LazyLogger()


def get_logger(name: str = None, level: str = "INFO") -> logging.Logger:
    return _setup_logger()

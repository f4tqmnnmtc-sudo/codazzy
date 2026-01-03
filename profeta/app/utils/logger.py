import logging, sys, os

_l = None

def _s():
    global _l
    if _l: return _l
    _l = logging.getLogger("profeta")
    _l.setLevel(getattr(logging, os.getenv("LOG_LEVEL", "INFO").upper(), logging.INFO))
    _l.handlers or _l.addHandler((h := logging.StreamHandler(sys.stdout), h.setFormatter(logging.Formatter("%(asctime)s [%(levelname)s] %(message)s", "%H:%M:%S")))[0])
    return _l

class _L:
    __slots__ = ()
    __getattr__ = lambda s, n: getattr(_s(), n)

log = _L()
get_logger = lambda *_, **__: _s()

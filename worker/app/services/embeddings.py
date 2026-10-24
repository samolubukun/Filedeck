"""Local text embeddings.

Primary backend: fastembed (ONNX, all-MiniLM-L6-v2, 384 dims, ~87 MB downloaded lazily
into FILEDECK_EMBEDDING_CACHE_DIR). If the model cannot be loaded (offline, no download,
fastembed missing) a deterministic hashing vectorizer (same dimension) is used and the
response reports ``backend: "hash"``. Vectors of different backends are NOT comparable,
so callers must store the backend next to the vector.
"""
import hashlib
import math
import os
import re
import threading
import time
from pathlib import Path
from typing import List, Optional

DIM = 384
DEFAULT_MODEL = "sentence-transformers/all-MiniLM-L6-v2"
RETRY_AFTER_SECONDS = 300
_TOKEN = re.compile(r"[a-z0-9]{2,}")


def cache_dir() -> str:
    d = os.getenv("FILEDECK_EMBEDDING_CACHE_DIR")
    if d:
        return d
    from app.services.models import get_models_dir
    return str(get_models_dir())


def hash_embed(text: str) -> List[float]:
    """Signed feature hashing of unigrams+bigrams, L2-normalised."""
    vec = [0.0] * DIM
    toks = _TOKEN.findall(text.lower())
    feats = toks + [a + "_" + b for a, b in zip(toks, toks[1:])]
    for f in feats:
        h = int.from_bytes(hashlib.blake2b(f.encode(), digest_size=8).digest(), "little")
        vec[h % DIM] += 1.0 if (h >> 63) & 1 else -1.0
    # sublinear scaling keeps long documents from being dominated by repeated terms
    vec = [math.copysign(math.log1p(abs(x)), x) for x in vec]
    norm = math.sqrt(sum(x * x for x in vec))
    return [x / norm for x in vec] if norm else vec


class EmbeddingService:
    def __init__(self):
        self._model = None
        self._failed_at: Optional[float] = None
        self._error: Optional[str] = None
        self._lock = threading.Lock()

    @property
    def model_name(self) -> str:
        name = os.getenv("FILEDECK_EMBEDDING_MODEL", DEFAULT_MODEL)
        return name if "/" in name else f"sentence-transformers/{name}"

    def _forced(self) -> Optional[str]:
        v = os.getenv("FILEDECK_EMBEDDING_BACKEND", "auto").lower()
        return v if v in ("hash", "fastembed") else None

    def _load(self):
        if self._model is not None:
            return self._model
        if self._failed_at and time.time() - self._failed_at < RETRY_AFTER_SECONDS:
            return None
        with self._lock:
            if self._model is not None:
                return self._model
            try:
                from fastembed import TextEmbedding

                self._model = TextEmbedding(self.model_name, cache_dir=cache_dir())
                self._failed_at = None
                self._error = None
            except Exception as e:  # offline / not installed / unsupported model
                self._failed_at = time.time()
                self._error = f"{type(e).__name__}: {e}"[:300]
                self._model = None
        return self._model

    def status(self) -> dict:
        return {
            "backend": "hash" if self._forced() == "hash" or self._model is None else "fastembed",
            "model": self.model_name,
            "loaded": self._model is not None,
            "dim": DIM,
            "cache_dir": cache_dir(),
            "last_error": self._error,
        }

    def embed(self, texts: List[str]) -> dict:
        if not texts:
            return {"embeddings": [], "backend": "none", "model": None, "dim": DIM}
        if self._forced() != "hash":
            model = self._load()
            if model is not None:
                try:
                    vecs = [[float(x) for x in v] for v in model.embed(texts)]
                    return {"embeddings": vecs, "backend": "fastembed",
                            "model": self.model_name, "dim": len(vecs[0])}
                except Exception as e:
                    self._error = f"{type(e).__name__}: {e}"[:300]
                    self._failed_at = time.time()
                    self._model = None
        return {"embeddings": [hash_embed(t) for t in texts], "backend": "hash",
                "model": "hashing-384", "dim": DIM}


embedding_service = EmbeddingService()

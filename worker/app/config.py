import os
from pydantic import BaseModel


class Settings(BaseModel):
    host: str = os.getenv("FILEDECK_HOST", "127.0.0.1")
    port: int = int(os.getenv("FILEDECK_PORT", "8900"))
    worker_secret: str = os.getenv("WORKER_SECRET", "")
    ocr_language: str = os.getenv("FILEDECK_OCR_LANGUAGE", "eng")
    ocr_mode: str = os.getenv("FILEDECK_OCR_MODE", "skip_noarchive")
    ai_enabled: bool = os.getenv("FILEDECK_AI_ENABLED", "false").lower() in ("1", "true", "yes")
    llm_provider: str = os.getenv("FILEDECK_LLM_PROVIDER", "local")
    embedding_model: str = os.getenv("FILEDECK_EMBEDDING_MODEL", "all-MiniLM-L6-v2")


settings = Settings()

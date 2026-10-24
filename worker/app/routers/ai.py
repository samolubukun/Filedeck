"""AI helpers hosted by the worker: embeddings, PDF table extraction, and local GGUF LLM inference."""
import io
from typing import List, Optional, Any, Dict

from fastapi import APIRouter, File, HTTPException, UploadFile, Request
from fastapi.responses import StreamingResponse
from pydantic import BaseModel

from app.services.embeddings import embedding_service
from app.services.models import model_service

router = APIRouter()


class EmbedRequest(BaseModel):
    text: Optional[str] = None
    texts: Optional[List[str]] = None


@router.post("/embed")
async def embed(req: EmbedRequest):
    texts = req.texts if req.texts is not None else ([req.text] if req.text is not None else None)
    if texts is None:
        raise HTTPException(status_code=422, detail="Provide `text` or `texts`")
    out = embedding_service.embed(texts)
    if req.texts is None:
        out["embedding"] = out["embeddings"][0] if out["embeddings"] else []
    return out


@router.get("/embed/status")
async def embed_status():
    return embedding_service.status()


def extract_tables(data: bytes) -> list:
    import pdfplumber

    pages = []
    with pdfplumber.open(io.BytesIO(data)) as pdf:
        for i, page in enumerate(pdf.pages, start=1):
            for t_index, table in enumerate(page.extract_tables()):
                rows = [[("" if c is None else str(c).strip()) for c in row] for row in table]
                if rows:
                    pages.append({"page": i, "index": t_index, "rows": rows})
    return pages


@router.post("/extract-tables")
async def extract_tables_endpoint(file: UploadFile = File(...)):
    data = await file.read()
    if not data:
        raise HTTPException(status_code=400, detail="Empty file provided")
    try:
        tables = extract_tables(data)
    except Exception as e:
        raise HTTPException(status_code=400, detail=f"Could not extract tables: {e}")
    return {"tables": tables, "count": len(tables)}


# --- Model Management & Local LLM Endpoints ---


@router.get("/ai/models/status")
async def get_model_status():
    """Returns local model download state, disk footprint, and inference readiness."""
    return model_service.get_status()


@router.post("/ai/models/download")
async def download_model():
    """Triggers in-app streaming download of Qwen 2.5 1.5B GGUF model."""
    return model_service.start_download()


@router.post("/ai/models/cancel")
async def cancel_download():
    """Cancels active in-app model download."""
    return model_service.cancel_download()


@router.delete("/ai/models")
async def delete_model():
    """Deletes local model file to reclaim disk space."""
    return model_service.delete_model()


class ChatCompletionRequest(BaseModel):
    model: Optional[str] = None
    messages: List[Dict[str, Any]]
    stream: Optional[bool] = True
    temperature: Optional[float] = 0.2


@router.post("/v1/chat/completions")
async def local_chat_completions(req: ChatCompletionRequest):
    """Local OpenAI-compatible chat completion endpoint running in the worker."""
    if req.stream:
        return StreamingResponse(
            model_service.chat_completion_stream(
                messages=req.messages,
                temperature=req.temperature or 0.2,
            ),
            media_type="text/event-stream",
        )
    else:
        # Non-streaming collector
        full_content = []
        for chunk in model_service.chat_completion_stream(req.messages, req.temperature or 0.2):
            if chunk.startswith("data: ") and not chunk.startswith("data: [DONE]"):
                import json
                try:
                    data = json.loads(chunk[6:].strip())
                    delta = data.get("choices", [{}])[0].get("delta", {}).get("content", "")
                    if delta:
                        full_content.append(delta)
                except Exception:
                    pass
        text = "".join(full_content)
        return {
            "choices": [{"message": {"role": "assistant", "content": text}}],
            "model": "qwen2.5-1.5b",
        }

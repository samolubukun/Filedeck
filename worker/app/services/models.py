"""Local LLM model management and in-app GGUF inference runner."""
import os
import sys
import time
import threading
from pathlib import Path
from typing import Dict, Any, Optional, Iterator
import requests

MODEL_ID = "qwen2.5-1.5b"
MODEL_NAME = "Qwen 2.5 (1.5B Instruct)"
MODEL_FILENAME = "qwen2.5-1.5b-instruct-q4_k_m.gguf"
MODEL_URL = "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf"
MODEL_SIZE_BYTES = 1033282208  # ~986 MB


def get_user_data_models_dir() -> Path:
    home = Path.home()
    if sys.platform == "win32":
        app_data = os.environ.get("APPDATA")
        base = Path(app_data) if app_data else home / "AppData" / "Roaming"
        return base / "FileDeck" / "models"
    elif sys.platform == "darwin":
        return home / "Library" / "Application Support" / "FileDeck" / "models"
    else:
        return home / ".local" / "share" / "filedeck" / "models"


def get_models_dir() -> Path:
    env_dir = os.getenv("FILEDECK_MODELS_DIR")
    if env_dir:
        p = Path(env_dir)
        p.mkdir(parents=True, exist_ok=True)
        return p

    # Try local project data/models first
    project_models = Path(__file__).resolve().parent.parent.parent / "data" / "models"
    try:
        project_models.mkdir(parents=True, exist_ok=True)
        # Test write permission (fails if installed in C:\Program Files in Tauri)
        test_file = project_models / ".write_test"
        test_file.touch()
        test_file.unlink()
        return project_models
    except (PermissionError, OSError):
        # Fall back to user AppData for packaged desktop apps (Tauri)
        user_models = get_user_data_models_dir()
        user_models.mkdir(parents=True, exist_ok=True)
        return user_models


class ModelManagerService:
    def __init__(self):
        self._lock = threading.Lock()
        self._llm = None
        self._llm_loaded_path = None

        # Download state
        self._downloading = False
        self._cancel_requested = False
        self._download_error: Optional[str] = None
        self._downloaded_bytes = 0
        self._total_bytes = MODEL_SIZE_BYTES
        self._download_start_time = 0.0

    @property
    def model_path(self) -> Path:
        return get_models_dir() / MODEL_FILENAME

    def is_model_ready(self) -> bool:
        p = self.model_path
        # Must exist and be at least 90% of expected size to be considered complete
        return p.exists() and p.stat().st_size > (MODEL_SIZE_BYTES * 0.9)

    def get_status(self) -> Dict[str, Any]:
        p = self.model_path
        is_ready = self.is_model_ready()
        disk_size = p.stat().st_size if p.exists() else 0

        percent = 0.0
        speed_mbps = 0.0
        if self._downloading:
            status = "downloading"
            if self._total_bytes > 0:
                percent = round((self._downloaded_bytes / self._total_bytes) * 100.0, 1)
            elapsed = time.time() - self._download_start_time
            if elapsed > 0.5:
                speed_mbps = round((self._downloaded_bytes / (1024 * 1024)) / elapsed, 2)
        elif is_ready:
            status = "ready"
            percent = 100.0
        elif self._download_error:
            status = "error"
        else:
            status = "not_downloaded"

        return {
            "model_id": MODEL_ID,
            "name": MODEL_NAME,
            "filename": MODEL_FILENAME,
            "status": status,
            "is_ready": is_ready,
            "is_loaded": self._llm is not None,
            "size_bytes": MODEL_SIZE_BYTES,
            "size_formatted": "986 MB",
            "disk_size_bytes": disk_size,
            "downloaded_bytes": self._downloaded_bytes if self._downloading else disk_size,
            "percent": percent,
            "speed_mbps": speed_mbps,
            "error": self._download_error,
            "description": "Ultra-fast CPU inference, strong document reasoning & multilingual comprehension.",
            "models_dir": str(get_models_dir()),
        }

    def start_download(self) -> Dict[str, Any]:
        with self._lock:
            if self._downloading:
                return {"status": "already_downloading"}
            if self.is_model_ready():
                return {"status": "already_ready"}

            self._downloading = True
            self._cancel_requested = False
            self._download_error = None
            self._downloaded_bytes = 0
            self._download_start_time = time.time()

            thread = threading.Thread(target=self._download_worker, daemon=True)
            thread.start()
            return {"status": "started"}

    def cancel_download(self) -> Dict[str, Any]:
        with self._lock:
            if not self._downloading:
                return {"status": "not_downloading"}
            self._cancel_requested = True
            return {"status": "cancelling"}

    def delete_model(self) -> Dict[str, Any]:
        with self._lock:
            self._llm = None
            self._llm_loaded_path = None
            p = self.model_path
            if p.exists():
                try:
                    p.unlink()
                except Exception as e:
                    return {"status": "error", "detail": str(e)}
            return {"status": "deleted"}

    def _download_worker(self):
        dest_path = self.model_path
        part_path = dest_path.with_suffix(".part")
        try:
            headers = {"User-Agent": "FileDeck-Desktop/1.0"}
            response = requests.get(MODEL_URL, stream=True, headers=headers, timeout=30)
            response.raise_for_status()

            total = int(response.headers.get("content-length", MODEL_SIZE_BYTES))
            self._total_bytes = total

            with open(part_path, "wb") as f:
                for chunk in response.iter_content(chunk_size=1024 * 1024):  # 1MB chunks
                    if self._cancel_requested:
                        break
                    if chunk:
                        f.write(chunk)
                        self._downloaded_bytes += len(chunk)

            if self._cancel_requested:
                if part_path.exists():
                    part_path.unlink()
                with self._lock:
                    self._downloading = False
                    self._download_error = "Download cancelled by user"
                return

            # Rename to final destination
            if part_path.exists():
                if dest_path.exists():
                    dest_path.unlink()
                part_path.rename(dest_path)

            with self._lock:
                self._downloading = False
                self._download_error = None

            # Preload into memory
            self.load_model()
        except Exception as e:
            if part_path.exists():
                try:
                    part_path.unlink()
                except Exception:
                    pass
            with self._lock:
                self._downloading = False
                self._download_error = f"Download failed: {str(e)[:250]}"

    def load_model(self):
        if not self.is_model_ready():
            return None
        with self._lock:
            if self._llm is not None:
                return self._llm
            try:
                from llama_cpp import Llama

                p = str(self.model_path)
                # n_ctx=2048, n_threads automatically detects CPU cores
                self._llm = Llama(
                    model_path=p,
                    n_ctx=2048,
                    n_threads=max(1, (os.cpu_count() or 4) - 1),
                    verbose=False,
                )
                self._llm_loaded_path = p
                print(f"Loaded local GGUF model: {p}")
            except Exception as e:
                print(f"Failed to load GGUF model: {e}")
                self._llm = None
        return self._llm

    def chat_completion_stream(self, messages: list, temperature: float = 0.2) -> Iterator[str]:
        llm = self.load_model()
        if llm is None:
            # Yield error chunk
            err = "Local model is not downloaded or failed to load. Please download Qwen 2.5 in Settings or Ask AI."
            yield f'data: {{"choices": [{{"delta": {{"content": "{err}"}}}}]}}\n\n'
            yield "data: [DONE]\n\n"
            return

        try:
            for chunk in llm.create_chat_completion(
                messages=messages,
                temperature=temperature,
                stream=True,
                max_tokens=1024,
            ):
                import json

                delta = chunk.get("choices", [{}])[0].get("delta", {})
                content = delta.get("content", "")
                if content:
                    payload = {"choices": [{"delta": {"content": content}}]}
                    yield f"data: {json.dumps(payload)}\n\n"
            yield "data: [DONE]\n\n"
        except Exception as e:
            import json

            payload = {"choices": [{"delta": {"content": f"\n\n[Inference Error: {e}]"}}]}
            yield f"data: {json.dumps(payload)}\n\n"
            yield "data: [DONE]\n\n"


model_service = ModelManagerService()

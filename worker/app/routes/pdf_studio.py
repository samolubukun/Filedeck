"""
FileDeck PDF Studio FastAPI Router.

Provides upload, page thumbnail rendering, live preview calculation, tool execution,
and vault document loading endpoints for PDF Studio.
"""
from __future__ import annotations

import json
import shutil
import time
import traceback
import uuid
from pathlib import Path
from typing import Dict, List, Optional

import pymupdf
from fastapi import APIRouter, File, Form, HTTPException, Response, UploadFile
from fastapi.responses import FileResponse, JSONResponse

from app.services import pdf_env, pdf_ops

router = APIRouter(prefix="/api/pdf", tags=["pdf-studio"])

# In-memory registry of registered files: fid -> {"path": Path, "name": str}
FILES: Dict[str, dict] = {}

# Rolling buffer of temporary preview directories
_PREVIEW_DIRS: List[Path] = []

SLOW_TOOLS = {
    "ocr", "pdf_to_word", "pdf_to_pptx", "pdf_to_excel",
    "pdf_to_pdfa", "pdf_to_markdown", "compare", "pdf_to_text"
}
PREVIEW_TRIM_PAGES = 2
RENDERABLE = {".pdf", ".jpg", ".jpeg", ".png", ".tif", ".tiff", ".bmp", ".gif", ".webp"}
TEXTUAL = {".txt", ".md", ".csv"}
OFFICE_OUT = {".docx", ".xlsx", ".pptx", ".doc", ".xls", ".ppt", ".odt", ".ods", ".odp"}


def _register(path: Path) -> str:
    fid = uuid.uuid4().hex[:16]
    FILES[fid] = {"path": path, "name": path.name}
    return fid


def _file(fid: str) -> Path:
    entry = FILES.get(fid)
    if not entry or not entry["path"].exists():
        raise HTTPException(404, "Requested document file is missing or has expired.")
    return entry["path"]


def _describe_output(p: Path, workdir: Path, for_preview: bool) -> dict:
    """Format an output file into metadata readable by the frontend UI."""
    suffix = p.suffix.lower()
    info = {
        "name": p.name,
        "path": str(p),
        "size": p.stat().st_size,
        "kind": "other",
        "id": None,
        "pages": None,
    }
    try:
        if suffix == ".pdf":
            with pymupdf.open(str(p)) as d:
                if d.needs_pass:
                    info["kind"] = "encrypted"
                    return info
                info["kind"] = "pdf"
                info["pages"] = d.page_count
                head = d[0].get_text().strip()
                if head:
                    info["text_head"] = head[:400]
            info["id"] = _register(p)
        elif suffix in RENDERABLE:
            info["kind"] = "image"
            info["pages"] = 1
            info["id"] = _register(p)
        elif suffix in TEXTUAL:
            info["kind"] = "text"
            info["text"] = p.read_text("utf-8", errors="replace")[:2500]
        elif suffix == ".zip":
            import zipfile
            info["kind"] = "zip"
            with zipfile.ZipFile(p) as z:
                names = z.namelist()
            info["entries"] = names[:14]
            info["count"] = len(names)
        elif suffix in OFFICE_OUT:
            info["kind"] = "office"
            rendered = pdf_ops.render_to_pdf(p, workdir / "render") if for_preview else None
            if rendered:
                with pymupdf.open(str(rendered)) as d:
                    info["pages"] = d.page_count
                info["id"] = _register(rendered)
    except Exception:
        pass
    return info


@router.get("/deps")
def get_dependencies():
    """Return availability of external binaries and OCR models."""
    return pdf_env.system_diagnostics()


@router.post("/upload")
async def upload_files(files: List[UploadFile] = File(...)):
    """Upload one or more files into the PDF Studio workspace."""
    workdir = pdf_ops.new_workdir()
    result = []
    for uf in files:
        fid = uuid.uuid4().hex[:16]
        safe_name = Path(uf.filename or "document.pdf").name
        dest = workdir / f"{fid}_{safe_name}"
        dest.write_bytes(await uf.read())

        info = {
            "id": fid,
            "name": safe_name,
            "size": dest.stat().st_size,
            "pages": None,
            "encrypted": False,
        }
        if safe_name.lower().endswith(".pdf"):
            try:
                with pymupdf.open(str(dest)) as d:
                    if d.needs_pass:
                        info["encrypted"] = True
                    else:
                        info["pages"] = d.page_count
                        r = d[0].rect
                        info["page_w"], info["page_h"] = r.width, r.height
            except Exception:
                info["pages"] = None
        elif dest.suffix.lower() in RENDERABLE:
            info["pages"] = 1

        FILES[fid] = {"path": dest, "name": safe_name}
        result.append(info)
    return {"files": result}


@router.post("/load-vault-file")
async def load_vault_file(payload: dict):
    """
    Zero-copy register an existing FileDeck vault document from local disk.
    Avoids client re-uploading bytes over network.
    """
    path_str = payload.get("path")
    if not path_str:
        raise HTTPException(400, "Path is required.")
    p = Path(path_str)
    if not p.exists():
        raise HTTPException(404, f"Vault document not found at {path_str}")

    fid = uuid.uuid4().hex[:16]
    name = payload.get("name") or p.name
    FILES[fid] = {"path": p, "name": name}

    info = {
        "id": fid,
        "name": name,
        "size": p.stat().st_size,
        "pages": None,
        "encrypted": False,
    }
    if p.suffix.lower() == ".pdf":
        try:
            with pymupdf.open(str(p)) as d:
                if d.needs_pass:
                    info["encrypted"] = True
                else:
                    info["pages"] = d.page_count
                    r = d[0].rect
                    info["page_w"], info["page_h"] = r.width, r.height
        except Exception:
            pass
    elif p.suffix.lower() in RENDERABLE:
        info["pages"] = 1

    return info


@router.get("/thumb/{fid}")
def render_thumbnail(fid: str, page: int = 0, w: int = 200, password: str = ""):
    """Render a rapid PNG thumbnail of any page in the document."""
    path = _file(fid)
    try:
        doc = pymupdf.open(str(path))
        if doc.needs_pass and password:
            doc.authenticate(password)
        page_idx = min(max(0, page), doc.page_count - 1)
        page_obj = doc[page_idx]
        scale = max(0.05, min(w / page_obj.rect.width, 6.0))
        pix = page_obj.get_pixmap(matrix=pymupdf.Matrix(scale, scale))
        data = pix.tobytes("png")
        doc.close()
        return Response(content=data, media_type="image/png", headers={"Cache-Control": "public, max-age=60"})
    except Exception as e:
        raise HTTPException(400, f"Thumbnail rendering failed: {e}")


@router.get("/pageinfo/{fid}")
def get_pageinfo(fid: str, page: int = 0):
    """Return dimensions, rotation, and total page count."""
    path = _file(fid)
    with pymupdf.open(str(path)) as doc:
        p = doc[min(max(0, page), doc.page_count - 1)]
        return {
            "w": p.rect.width,
            "h": p.rect.height,
            "pages": doc.page_count,
            "rotation": p.rotation,
        }


@router.get("/metadata/{fid}")
def get_metadata(fid: str):
    """Return standard PDF metadata dictionary."""
    path = _file(fid)
    with pymupdf.open(str(path)) as doc:
        return {"metadata": doc.metadata or {}}


def _trim_inputs(paths: List[Path], workdir: Path) -> Tuple[List[Path], bool]:
    """Trim multi-page PDFs to the first few pages for rapid preview rendering."""
    trimmed = False
    result = []
    for p in paths:
        if p.suffix.lower() != ".pdf":
            result.append(p)
            continue
        try:
            with pymupdf.open(str(p)) as src:
                if src.page_count <= PREVIEW_TRIM_PAGES:
                    result.append(p)
                    continue
                short = pymupdf.open()
                short.insert_pdf(src, from_page=0, to_page=PREVIEW_TRIM_PAGES - 1)
                dest = workdir / f"trim_{p.name}"
                short.save(str(dest))
                short.close()
            result.append(dest)
            trimmed = True
        except Exception:
            result.append(p)
    return result, trimmed


@router.post("/preview/{tool}")
async def preview_tool(
    tool: str,
    payload: str = Form(...),
    asset: Optional[UploadFile] = File(None),
):
    """Execute tool against temporary preview directory for live side-by-side rendering."""
    if tool not in pdf_ops.TOOLS:
        raise HTTPException(404, f"Unknown tool: '{tool}'")
    data = json.loads(payload)
    options = dict(data.get("options") or {})
    paths = [_file(fid) for fid in data.get("files", [])]
    if not paths and tool != "html_to_pdf":
        raise HTTPException(400, "No files provided for tool preview.")

    workdir = pdf_ops.new_workdir()
    _PREVIEW_DIRS.append(workdir)
    while len(_PREVIEW_DIRS) > 10:
        shutil.rmtree(_PREVIEW_DIRS.pop(0), ignore_errors=True)

    if tool == "split":
        options["zip"] = False
    if tool == "pdf_to_images" and not options.get("pages"):
        options["pages"] = "1-3"

    trimmed = False
    if tool in SLOW_TOOLS:
        paths, trimmed = _trim_inputs(paths, workdir)

    extra = {}
    if asset is not None:
        apath = workdir / (Path(asset.filename or "asset.png").name)
        apath.write_bytes(await asset.read())
        extra["asset"] = apath

    t0 = time.time()
    pdf_ops.set_output_override(workdir)
    try:
        fn = pdf_ops.TOOLS[tool]
        if tool in pdf_ops.TOOLS_WITH_ASSET:
            outputs = fn(paths, options, extra_files=extra)
        else:
            outputs = fn(paths, options)
    except ValueError as e:
        return JSONResponse(status_code=422, content={"error": str(e)})
    except Exception as e:
        return JSONResponse(status_code=500, content={"error": str(e)})
    finally:
        pdf_ops.set_output_override(None)

    res = [_describe_output(p, workdir, for_preview=True) for p in outputs[:6]]
    for r in res:
        r.pop("path", None)
    return {
        "outputs": res,
        "more": max(0, len(outputs) - 6),
        "in_size": sum(x.stat().st_size for x in paths),
        "trimmed": trimmed,
        "trim_pages": PREVIEW_TRIM_PAGES,
        "duration": round(time.time() - t0, 2),
    }


@router.post("/run/{tool}")
async def run_tool(
    tool: str,
    payload: str = Form(...),
    asset: Optional[UploadFile] = File(None),
):
    """Execute tool with full file fidelity and return completed output references."""
    if tool not in pdf_ops.TOOLS:
        raise HTTPException(404, f"Unknown tool: '{tool}'")
    data = json.loads(payload)
    file_ids = data.get("files", [])
    options = data.get("options", {})
    paths = [_file(fid) for fid in file_ids]
    if not paths and tool != "html_to_pdf":
        raise HTTPException(400, "No files provided.")

    extra = {}
    if asset is not None:
        workdir = pdf_ops.new_workdir()
        apath = workdir / (Path(asset.filename or "asset.png").name)
        apath.write_bytes(await asset.read())
        extra["asset"] = apath

    t0 = time.time()
    try:
        fn = pdf_ops.TOOLS[tool]
        if tool in pdf_ops.TOOLS_WITH_ASSET:
            outputs = fn(paths, options, extra_files=extra)
        else:
            outputs = fn(paths, options)
    except ValueError as e:
        return JSONResponse(status_code=422, content={"error": str(e)})
    except Exception as e:
        traceback.print_exc()
        return JSONResponse(status_code=500, content={"error": str(e)})

    render_dir = pdf_ops.new_workdir()
    out_info = [_describe_output(p, render_dir, for_preview=False) for p in outputs]
    return {
        "outputs": out_info,
        "duration": round(time.time() - t0, 2),
        "in_size": sum(p.stat().st_size for p in paths),
    }


@router.get("/download/{fid}")
def download_file(fid: str):
    """Download an output file by its file ID."""
    p = _file(fid)
    media_type = "application/pdf"
    if p.suffix.lower() == ".zip":
        media_type = "application/zip"
    elif p.suffix.lower() in (".docx", ".xlsx", ".pptx"):
        media_type = "application/octet-stream"
    return FileResponse(p, filename=p.name, media_type=media_type)

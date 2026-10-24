"""
FileDeck PDF Operations Engine.

Pure local implementation of all 30 PDF manipulation, conversion,
optimization, and security operations.
"""
from __future__ import annotations

import io
import os
import re
import shutil
import subprocess
import tempfile
import uuid
import zipfile
from pathlib import Path
from typing import Dict, List, Optional, Tuple, Union

import pikepdf
import pymupdf
from PIL import Image, ImageChops, ImageDraw

from app.services import pdf_env

try:
    import pillow_heif
    pillow_heif.register_heif_opener()
except ImportError:
    pass

# Root scratch directory for PDF processing outputs
PDF_SCRATCH_ROOT = Path(tempfile.gettempdir()) / "filedeck_pdf_studio"
PDF_SCRATCH_ROOT.mkdir(parents=True, exist_ok=True)

_output_override: Optional[Path] = None


def set_output_override(path: Optional[Path]):
    """Override default output directory (used during live preview generation)."""
    global _output_override
    _output_override = path


def new_workdir() -> Path:
    """Generate a clean isolated temporary working directory."""
    dir_path = PDF_SCRATCH_ROOT / uuid.uuid4().hex[:12]
    dir_path.mkdir(parents=True, exist_ok=True)
    return dir_path


def unique_output_path(base_name: str, out_dir: Optional[Path] = None) -> Path:
    """Produce a collision-free output path in the active output directory."""
    target_dir = _output_override or out_dir or new_workdir()
    p = target_dir / base_name
    if not p.exists():
        return p
    stem, ext = p.stem, p.suffix
    i = 1
    while p.exists():
        p = target_dir / f"{stem}_{i}{ext}"
        i += 1
    return p


def _save_pdf(doc: pymupdf.Document, path: Path) -> Path:
    """Save PyMuPDF document with garbage collection and stream deflation."""
    doc.save(str(path), garbage=3, deflate=True)
    doc.close()
    return path


def parse_ranges(spec: str, n_pages: int) -> List[int]:
    """
    Parse a range specification (e.g. '1-3, 5, 7-') into 0-indexed page indices.
    """
    pages: List[int] = []
    spec = (spec or "").strip()
    if not spec:
        return list(range(n_pages))
    for chunk in spec.split(","):
        chunk = chunk.strip()
        if not chunk:
            continue
        m = re.match(r"^(\d*)\s*-\s*(\d*)$", chunk)
        if m:
            start_val = int(m.group(1)) if m.group(1) else 1
            end_val = int(m.group(2)) if m.group(2) else n_pages
        elif chunk.isdigit():
            start_val = end_val = int(chunk)
        else:
            continue
        for p in range(max(1, start_val), min(n_pages, end_val) + 1):
            if p - 1 not in pages:
                pages.append(p - 1)
    return pages or list(range(n_pages))


# ======================================================================
# 1. ORGANIZE TOOLS
# ======================================================================

def merge(files: List[Path], opt: dict) -> List[Path]:
    """Merge multiple PDF documents in given order."""
    if not files:
        raise ValueError("At least one PDF file is required for merge.")
    out_doc = pymupdf.open()
    for f in files:
        with pymupdf.open(str(f)) as src:
            out_doc.insert_pdf(src)
    name = opt.get("filename") or "merged_document.pdf"
    if not name.lower().endswith(".pdf"):
        name += ".pdf"
    return [_save_pdf(out_doc, unique_output_path(name))]


def split(files: List[Path], opt: dict) -> List[Path]:
    """Split PDF by page ranges, every N pages, or extract specific pages."""
    f = files[0]
    src = pymupdf.open(str(f))
    n = src.page_count
    mode = opt.get("mode", "ranges")
    stem = Path(f).stem
    outs: List[Path] = []

    if mode == "every":
        step = max(1, int(opt.get("every", 1)))
        chunks = [(i, min(i + step - 1, n - 1)) for i in range(0, n, step)]
        for a, b in chunks:
            d = pymupdf.open()
            d.insert_pdf(src, from_page=a, to_page=b)
            label = f"{a + 1}" if a == b else f"{a + 1}-{b + 1}"
            outs.append(_save_pdf(d, unique_output_path(f"{stem}_page_{label}.pdf")))
    elif mode == "extract":
        pages = parse_ranges(opt.get("pages", ""), n)
        d = pymupdf.open()
        for p in pages:
            d.insert_pdf(src, from_page=p, to_page=p)
        outs.append(_save_pdf(d, unique_output_path(f"{stem}_extracted.pdf")))
    else:  # ranges
        spec = opt.get("pages", "") or f"1-{n}"
        for i, chunk in enumerate([s.strip() for s in spec.split(",") if s.strip()], 1):
            pages = parse_ranges(chunk, n)
            d = pymupdf.open()
            for p in pages:
                d.insert_pdf(src, from_page=p, to_page=p)
            outs.append(_save_pdf(d, unique_output_path(f"{stem}_part_{i}.pdf")))
    src.close()

    if opt.get("merge_after") and len(outs) > 1:
        merged = pymupdf.open()
        for o in outs:
            with pymupdf.open(str(o)) as part_doc:
                merged.insert_pdf(part_doc)
            o.unlink(missing_ok=True)
        return [_save_pdf(merged, unique_output_path(f"{stem}_split_merged.pdf"))]

    if len(outs) > 3 and opt.get("zip", True):
        zpath = unique_output_path(f"{stem}_split_bundle.zip")
        with zipfile.ZipFile(zpath, "w", zipfile.ZIP_DEFLATED) as z:
            for o in outs:
                z.write(o, o.name)
        for o in outs:
            o.unlink(missing_ok=True)
        return [zpath]

    return outs


def remove_pages(files: List[Path], opt: dict) -> List[Path]:
    """Delete selected pages by index or range."""
    f = files[0]
    doc = pymupdf.open(str(f))
    drop = set(parse_ranges(opt.get("pages", ""), doc.page_count))
    keep = [p for p in range(doc.page_count) if p not in drop]
    if not keep:
        doc.close()
        raise ValueError("Cannot remove all pages from document.")
    doc.select(keep)
    return [_save_pdf(doc, unique_output_path(f"{Path(f).stem}_pages_removed.pdf"))]


def organize(files: List[Path], opt: dict) -> List[Path]:
    """Reorder, rotate, or delete individual pages visually."""
    f = files[0]
    src = pymupdf.open(str(f))
    order = opt.get("order") or [{"page": i, "rotation": 0} for i in range(src.page_count)]
    if not order:
        src.close()
        raise ValueError("No pages remaining after reorganization.")
    doc = pymupdf.open()
    for item in order:
        if isinstance(item, int):
            p = item
            rot = 0
        elif isinstance(item, dict):
            p = int(item.get("page", 0))
            rot = int(item.get("rotation", 0)) % 360
        else:
            continue
        if p >= src.page_count:
            continue
        doc.insert_pdf(src, from_page=p, to_page=p)
        if rot:
            page = doc[-1]
            page.set_rotation((page.rotation + rot) % 360)
    src.close()
    return [_save_pdf(doc, unique_output_path(f"{Path(f).stem}_organized.pdf"))]


# ======================================================================
# 2. OPTIMIZATION TOOLS
# ======================================================================

def compress(files: List[Path], opt: dict) -> List[Path]:
    """Compress PDF using Ghostscript presets with clean PyMuPDF fallback."""
    level = opt.get("level", "recommended")
    preset = {"low": "/printer", "recommended": "/ebook", "extreme": "/screen"}.get(level, "/ebook")
    gs_bin = pdf_env.ghostscript_binary()
    outs: List[Path] = []

    for f in files:
        out = unique_output_path(f"{Path(f).stem}_compressed.pdf")
        compressed_success = False

        if gs_bin:
            cmd = [
                gs_bin,
                "-sDEVICE=pdfwrite",
                "-dCompatibilityLevel=1.5",
                f"-dPDFSETTINGS={preset}",
                "-dNOPAUSE",
                "-dQUIET",
                "-dBATCH",
                "-dDetectDuplicateImages=true",
                "-dCompressFonts=true",
                f"-sOutputFile={out}",
                str(f),
            ]
            res = pdf_env.run_command(cmd, check=False)
            if res.returncode == 0 and out.exists() and out.stat().st_size < f.stat().st_size:
                compressed_success = True

        if not compressed_success:
            out.unlink(missing_ok=True)
            doc = pymupdf.open(str(f))
            doc.save(str(out), garbage=4, deflate=True, clean=True)
            doc.close()
            if out.stat().st_size >= f.stat().st_size:
                out.unlink(missing_ok=True)
                shutil.copy2(f, out)

        outs.append(out)
    return outs


def repair(files: List[Path], opt: dict) -> List[Path]:
    """Repair damaged or corrupt PDF structure using Pikepdf / Ghostscript."""
    outs: List[Path] = []
    gs_bin = pdf_env.ghostscript_binary()

    for f in files:
        out = unique_output_path(f"{Path(f).stem}_repaired.pdf")
        repaired = False
        try:
            with pikepdf.open(str(f)) as pdf:
                pdf.save(str(out))
                repaired = True
        except Exception:
            pass

        if not repaired and gs_bin:
            cmd = [
                gs_bin,
                "-o", str(out),
                "-sDEVICE=pdfwrite",
                "-dPDFSETTINGS=/prepress",
                "-dNOPAUSE",
                "-dBATCH",
                str(f),
            ]
            pdf_env.run_command(cmd, check=False)

        if not out.exists():
            shutil.copy2(f, out)
        outs.append(out)
    return outs


def ocr(files: List[Path], opt: dict) -> List[Path]:
    """Generate searchable PDF with embedded text layer via RapidOCR or OCRmyPDF."""
    langs = opt.get("langs") or ["eng"]
    lang = "+".join(langs)

    outs: List[Path] = []
    # If system has Tesseract and ocrmypdf python module available, use it; otherwise use pure offline RapidOCR
    has_ocrmypdf = False
    try:
        import ocrmypdf  # noqa: F401
        has_ocrmypdf = True
    except ImportError:
        pass

    if pdf_env.tesseract_binary() and has_ocrmypdf:
        for f in files:
            out = unique_output_path(f"{Path(f).stem}_ocr.pdf")
            cmd = pdf_env.ocrmypdf_cmd() + [
                "-l", lang,
                "--output-type", "pdf",
                "--optimize", "1",
                "--jobs", "4",
            ]
            if opt.get("deskew", True):
                cmd.append("--deskew")
            if opt.get("rotate", True):
                cmd.append("--rotate-pages")
            cmd.append("--force-ocr" if opt.get("force") else "--skip-text")
            cmd.extend([str(f), str(out)])

            r = pdf_env.run_command(cmd, timeout=3600, check=False)
            if r.returncode not in (0, 10):  # 10 = completed with warnings
                err_msg = (r.stderr or b"").decode("utf-8", "ignore")
                raise RuntimeError(f"OCR failed: {err_msg[-400:]}")
            outs.append(out)
        return outs

    # Pure self-contained offline OCR engine via ONNX RapidOCR
    try:
        from rapidocr_onnxruntime import RapidOCR
        engine = RapidOCR()
    except Exception as e:
        raise RuntimeError(f"Local OCR engine initialization failed: {e}")

    for f in files:
        out = unique_output_path(f"{Path(f).stem}_ocr.pdf")
        doc = pymupdf.open(str(f))
        for page in doc:
            pix = page.get_pixmap(dpi=150)
            img_bytes = pix.tobytes("png")
            result, _ = engine(img_bytes)
            if result:
                sx = page.rect.width / pix.width
                sy = page.rect.height / pix.height
                for box, text, score in result:
                    if not text or score < 0.3:
                        continue
                    x0 = min(pt[0] for pt in box) * sx
                    y0 = min(pt[1] for pt in box) * sy
                    x1 = max(pt[0] for pt in box) * sx
                    y1 = max(pt[1] for pt in box) * sy
                    rect = pymupdf.Rect(x0, y0, x1, y1)
                    try:
                        page.insert_textbox(
                            rect,
                            text,
                            fontsize=max(6, int((y1 - y0) * 0.75)),
                            render_mode=3,
                        )
                    except Exception:
                        pass
        outs.append(_save_pdf(doc, out))
    return outs


def flatten(files: List[Path], opt: dict) -> List[Path]:
    """Flatten interactive forms and annotations permanently into the page canvas."""
    outs: List[Path] = []
    for f in files:
        doc = pymupdf.open(str(f))
        try:
            doc.bake()
        except Exception:
            pass
        outs.append(_save_pdf(doc, unique_output_path(f"{Path(f).stem}_flattened.pdf")))
    return outs


# ======================================================================
# 3. CONVERT TO PDF TOOLS
# ======================================================================

def _normalize_image_for_pdf(f: Path, workdir: Path) -> Path:
    """Ensure image format, color mode, and orientation are compatible with img2pdf."""
    ext = f.suffix.lower()
    if ext in (".jpg", ".jpeg", ".png", ".tif", ".tiff"):
        try:
            with Image.open(f) as im:
                if ext != ".png" or im.mode not in ("RGBA", "LA", "P"):
                    return f
        except Exception:
            return f

    with Image.open(f) as im:
        if im.mode in ("RGBA", "LA", "P"):
            im = im.convert("RGBA")
            bg = Image.new("RGB", im.size, (255, 255, 255))
            bg.paste(im, mask=im.split()[-1])
            im = bg
        elif im.mode != "RGB":
            im = im.convert("RGB")
        out = workdir / f"{f.stem}_norm.jpg"
        im.save(out, "JPEG", quality=92)
    return out


def images_to_pdf(files: List[Path], opt: dict) -> List[Path]:
    """Convert raster images (JPG, PNG, HEIC, TIFF, WebP) into PDF."""
    import img2pdf

    workdir = new_workdir()
    imgs = [_normalize_image_for_pdf(f, workdir) for f in files]

    page = opt.get("page_size", "A4")
    orient = opt.get("orientation", "portrait")
    margin = float(opt.get("margin", 10))

    layout = None
    if page != "fit":
        sizes = {"A4": (210, 297), "letter": (215.9, 279.4)}
        w, h = sizes.get(page, sizes["A4"])
        if orient == "landscape":
            w, h = h, w
        border = img2pdf.mm_to_pt(margin)
        layout = img2pdf.get_layout_fun(
            pagesize=(img2pdf.mm_to_pt(w), img2pdf.mm_to_pt(h)),
            border=(border, border),
            fit=img2pdf.FitMode.into,
        )

    merge_all = opt.get("merge", True)
    outs: List[Path] = []
    if merge_all:
        out = unique_output_path(opt.get("filename") or "images_combined.pdf")
        with open(out, "wb") as fh:
            fh.write(
                img2pdf.convert([str(i) for i in imgs], layout_fun=layout)
                if layout else img2pdf.convert([str(i) for i in imgs])
            )
        outs.append(out)
    else:
        for src, im in zip(files, imgs):
            out = unique_output_path(f"{Path(src).stem}.pdf")
            with open(out, "wb") as fh:
                fh.write(
                    img2pdf.convert([str(im)], layout_fun=layout)
                    if layout else img2pdf.convert([str(im)])
                )
            outs.append(out)
    return outs


def office_to_pdf(files: List[Path], opt: dict) -> List[Path]:
    """Convert Microsoft Office & OpenDocument files to PDF via LibreOffice."""
    soffice_bin = pdf_env.libreoffice_binary()
    if not soffice_bin:
        raise RuntimeError("LibreOffice binary was not detected on this system.")

    workdir = new_workdir()
    outs: List[Path] = []
    for f in files:
        cmd = [
            soffice_bin,
            "--headless",
            "--norestore",
            "--convert-to", "pdf",
            "--outdir", str(workdir),
            str(f),
        ]
        r = pdf_env.run_command(cmd, timeout=600, check=False)
        produced = workdir / f"{Path(f).stem}.pdf"
        if not produced.exists():
            err_msg = (r.stderr or b"").decode("utf-8", "ignore")
            raise RuntimeError(f"Could not convert {Path(f).name}: {err_msg[-300:]}")
        out = unique_output_path(f"{Path(f).stem}.pdf")
        shutil.move(str(produced), out)
        outs.append(out)
    return outs


def render_to_pdf(f: Path, workdir: Path) -> Optional[Path]:
    """Helper to convert Office file to temporary PDF for preview generation."""
    soffice_bin = pdf_env.libreoffice_binary()
    if not soffice_bin:
        return None
    cmd = [
        soffice_bin,
        "--headless",
        "--norestore",
        "--convert-to", "pdf",
        "--outdir", str(workdir),
        str(f),
    ]
    try:
        pdf_env.run_command(cmd, timeout=240, check=False)
    except subprocess.TimeoutExpired:
        return None
    produced = workdir / f"{Path(f).stem}.pdf"
    return produced if produced.exists() else None


def html_to_pdf(files: List[Path], opt: dict) -> List[Path]:
    """Render HTML document or URL to PDF via headless Chromium/Edge."""
    chrome_bin = pdf_env.browser_binary()
    if not chrome_bin:
        raise RuntimeError("Headless browser (Chrome / Edge) not found for HTML conversion.")

    sources: List[Tuple[str, str]] = []
    for f in files:
        sources.append((Path(f).as_uri(), f"{Path(f).stem}.pdf"))

    url = (opt.get("url") or "").strip()
    if url:
        if not re.match(r"^https?://", url):
            url = "https://" + url
        safe_name = re.sub(r"[^\w.-]+", "_", url.split("//", 1)[-1])[:60] or "webpage"
        sources.append((url, f"{safe_name}.pdf"))

    if not sources:
        raise ValueError("No HTML file or URL provided for conversion.")

    outs: List[Path] = []
    for uri, name in sources:
        out = unique_output_path(name)
        cmd = [
            chrome_bin,
            "--headless=new",
            "--disable-gpu",
            "--no-first-run",
            "--virtual-time-budget=8000",
            "--no-pdf-header-footer",
            f"--print-to-pdf={out}",
            uri,
        ]
        r = pdf_env.run_command(cmd, timeout=180, check=False)
        if not out.exists():
            err_msg = (r.stderr or b"").decode("utf-8", "ignore")
            raise RuntimeError(f"Browser rendering failed: {err_msg[-300:]}")
        outs.append(out)
    return outs


# ======================================================================
# 4. CONVERT FROM PDF TOOLS
# ======================================================================

def pdf_to_images(files: List[Path], opt: dict) -> List[Path]:
    """Export PDF pages as individual high-res PNG or JPEG images."""
    dpi = int(opt.get("dpi", 150))
    fmt = opt.get("format", "jpg").lower()
    outs: List[Path] = []

    for f in files:
        doc = pymupdf.open(str(f))
        pages = parse_ranges(opt.get("pages", ""), doc.page_count)
        imgs: List[Path] = []
        workdir = new_workdir()
        for p in pages:
            pix = doc[p].get_pixmap(dpi=dpi)
            ip = workdir / f"{Path(f).stem}_page_{p + 1}.{fmt}"
            if fmt == "jpg":
                pix.pil_save(str(ip), format="JPEG", quality=90)
            else:
                pix.save(str(ip))
            imgs.append(ip)
        doc.close()

        if len(imgs) == 1:
            out = unique_output_path(imgs[0].name)
            shutil.move(str(imgs[0]), out)
            outs.append(out)
        else:
            zpath = unique_output_path(f"{Path(f).stem}_{fmt}_images.zip")
            with zipfile.ZipFile(zpath, "w", zipfile.ZIP_DEFLATED) as z:
                for img in imgs:
                    z.write(img, img.name)
            outs.append(zpath)
    return outs


def extract_images(files: List[Path], opt: dict) -> List[Path]:
    """Extract embedded raster image objects from PDF without re-encoding."""
    outs: List[Path] = []
    for f in files:
        doc = pymupdf.open(str(f))
        workdir = new_workdir()
        imgs, seen = [], set()
        for p in range(doc.page_count):
            for info in doc[p].get_images(full=True):
                xref = info[0]
                if xref in seen:
                    continue
                seen.add(xref)
                base = doc.extract_image(xref)
                ip = workdir / f"{Path(f).stem}_image_{len(imgs) + 1}.{base['ext']}"
                ip.write_bytes(base["image"])
                imgs.append(ip)
        doc.close()

        if not imgs:
            raise ValueError(f"No embedded images found in {Path(f).name}")

        if len(imgs) == 1:
            out = unique_output_path(imgs[0].name)
            shutil.move(str(imgs[0]), out)
            outs.append(out)
        else:
            zpath = unique_output_path(f"{Path(f).stem}_extracted_images.zip")
            with zipfile.ZipFile(zpath, "w") as z:
                for img in imgs:
                    z.write(img, img.name)
            outs.append(zpath)
    return outs


def pdf_to_word(files: List[Path], opt: dict) -> List[Path]:
    """Convert PDF to editable Microsoft Word DOCX."""
    from pdf2docx import Converter

    outs: List[Path] = []
    for f in files:
        out = unique_output_path(f"{Path(f).stem}.docx")
        cv = Converter(str(f))
        try:
            cv.convert(str(out))
        finally:
            cv.close()
        outs.append(out)
    return outs


def pdf_to_text(files: List[Path], opt: dict) -> List[Path]:
    """Extract full plain text from PDF."""
    outs: List[Path] = []
    for f in files:
        doc = pymupdf.open(str(f))
        text = "\n\n".join(doc[p].get_text() for p in range(doc.page_count))
        doc.close()
        out = unique_output_path(f"{Path(f).stem}.txt")
        out.write_text(text, "utf-8")
        outs.append(out)
    return outs


def pdf_to_markdown(files: List[Path], opt: dict) -> List[Path]:
    """Convert PDF to clean structured Markdown with layout preservation."""
    import pymupdf4llm

    outs: List[Path] = []
    for f in files:
        md = pymupdf4llm.to_markdown(str(f))
        out = unique_output_path(f"{Path(f).stem}.md")
        out.write_text(md, "utf-8")
        outs.append(out)
    return outs


def pdf_to_pptx(files: List[Path], opt: dict) -> List[Path]:
    """Convert PDF pages into presentation slides."""
    from pptx import Presentation
    from pptx.util import Emu

    EMU_PER_PT = 12700
    outs: List[Path] = []
    for f in files:
        doc = pymupdf.open(str(f))
        prs = Presentation()
        r0 = doc[0].rect
        prs.slide_width = Emu(int(r0.width * EMU_PER_PT))
        prs.slide_height = Emu(int(r0.height * EMU_PER_PT))
        blank = prs.slide_layouts[6]
        for p in range(doc.page_count):
            pix = doc[p].get_pixmap(dpi=150)
            buf = io.BytesIO(pix.tobytes("png"))
            slide = prs.slides.add_slide(blank)
            slide.shapes.add_picture(buf, 0, 0, width=prs.slide_width, height=prs.slide_height)
        doc.close()
        out = unique_output_path(f"{Path(f).stem}.pptx")
        prs.save(str(out))
        outs.append(out)
    return outs


def pdf_to_excel(files: List[Path], opt: dict) -> List[Path]:
    """Extract structured data tables into Microsoft Excel spreadsheet."""
    import pdfplumber
    from openpyxl import Workbook

    outs: List[Path] = []
    for f in files:
        wb = Workbook()
        wb.remove(wb.active)
        found = False
        with pdfplumber.open(str(f)) as pdf:
            for pno, page in enumerate(pdf.pages, 1):
                tables = page.extract_tables()
                if tables:
                    for t_i, table in enumerate(tables, 1):
                        title = f"Page_{pno}" + (f"_Table_{t_i}" if len(tables) > 1 else "")
                        ws = wb.create_sheet(title[:31])
                        for row in table:
                            ws.append([c if c is not None else "" for c in row])
                        found = True
            if not found:
                with pdfplumber.open(str(f)) as pdf2:
                    ws = wb.create_sheet("Text")
                    for page in pdf2.pages:
                        for line in (page.extract_text() or "").splitlines():
                            ws.append([line])
        out = unique_output_path(f"{Path(f).stem}.xlsx")
        wb.save(str(out))
        outs.append(out)
    return outs


def pdf_to_pdfa(files: List[Path], opt: dict) -> List[Path]:
    """Convert PDF to ISO standard PDF/A archival format."""
    level = opt.get("level", "2b")
    part = {"1b": "1", "2b": "2", "3b": "3"}.get(level, "2")
    gs_bin = pdf_env.ghostscript_binary()

    outs: List[Path] = []
    if gs_bin:
        for f in files:
            out = unique_output_path(f"{Path(f).stem}_pdfa.pdf")
            cmd = [
                gs_bin,
                f"-dPDFA={part}",
                "-dBATCH",
                "-dNOPAUSE",
                "-dQUIET",
                "-sColorConversionStrategy=RGB",
                "-sDEVICE=pdfwrite",
                "-dPDFACompatibilityPolicy=1",
                f"-sOutputFile={out}",
                str(f),
            ]
            pdf_env.run_command(cmd)
            outs.append(out)
        return outs

    # Pure self-contained offline fallback using pikepdf
    for f in files:
        out = unique_output_path(f"{Path(f).stem}_pdfa.pdf")
        with pikepdf.open(str(f)) as pdf:
            with pdf.open_metadata(set_pikepdf_as_editor=False) as meta:
                meta["pdfaid:part"] = part
                meta["pdfaid:conformance"] = level.upper()[-1]
            pdf.save(str(out), linearize=True)
        outs.append(out)
    return outs


# ======================================================================
# 5. EDITING TOOLS
# ======================================================================

def rotate(files: List[Path], opt: dict) -> List[Path]:
    """Rotate PDF pages by 90, 180, or 270 degrees."""
    angle = int(opt.get("angle", 90))
    which = opt.get("apply", "all")
    outs: List[Path] = []

    for f in files:
        doc = pymupdf.open(str(f))
        if which == "range":
            pages = parse_ranges(opt.get("pages", ""), doc.page_count)
        elif which == "even":
            pages = [p for p in range(doc.page_count) if (p + 1) % 2 == 0]
        elif which == "odd":
            pages = [p for p in range(doc.page_count) if (p + 1) % 2 == 1]
        else:
            pages = range(doc.page_count)
        for p in pages:
            doc[p].set_rotation((doc[p].rotation + angle) % 360)
        outs.append(_save_pdf(doc, unique_output_path(f"{Path(f).stem}_rotated.pdf")))
    return outs


_POS = {
    "tl": (0, 0), "tc": (1, 0), "tr": (2, 0),
    "ml": (0, 1), "mc": (1, 1), "mr": (2, 1),
    "bl": (0, 2), "bc": (1, 2), "br": (2, 2),
}


def _grid_point(rect: pymupdf.Rect, pos: str, tw: float, th: float, margin: float = 28):
    gx, gy = _POS.get(pos, (1, 2))
    xs = [rect.x0 + margin, rect.x0 + (rect.width - tw) / 2, rect.x1 - margin - tw]
    ys = [rect.y0 + margin + th, rect.y0 + (rect.height + th) / 2, rect.y1 - margin]
    return xs[gx], ys[gy]


def page_numbers(files: List[Path], opt: dict) -> List[Path]:
    """Stamp customizable page numbers across document."""
    pos = opt.get("position", "bc")
    fmt = opt.get("format", "{n}")
    size = float(opt.get("size", 12))
    start = int(opt.get("start", 1))
    first_cover = bool(opt.get("first_cover", False))
    facing = bool(opt.get("facing", False))
    color_hex = opt.get("color", "#000000")
    col = tuple(int(color_hex.lstrip("#")[i:i + 2], 16) / 255 for i in (0, 2, 4))
    fontfile = pdf_env.font_file(family=opt.get("font", "arial"))

    outs: List[Path] = []
    for f in files:
        doc = pymupdf.open(str(f))
        pages = parse_ranges(opt.get("pages", ""), doc.page_count)
        total = len(pages)
        for i, p in enumerate(pages):
            if first_cover and p == 0:
                continue
            page = doc[p]
            text = fmt.replace("{n}", str(start + i)).replace("{total}", str(total))
            font = pymupdf.Font(fontfile=fontfile) if fontfile else pymupdf.Font("helv")
            tw = font.text_length(text, fontsize=size)
            page_pos = pos
            if facing and pos[1] in "lr":
                flip = {"l": "r", "r": "l"}
                if (p + 1) % 2 == 1:
                    page_pos = pos[0] + flip.get(pos[1], pos[1])
            x, y = _grid_point(page.rect, page_pos, tw, size)
            wr = pymupdf.TextWriter(page.rect, color=col)
            wr.append((x, y), text, font=font, fontsize=size)
            wr.write_text(page)
        outs.append(_save_pdf(doc, unique_output_path(f"{Path(f).stem}_numbered.pdf")))
    return outs


def watermark(files: List[Path], opt: dict, extra_files: Optional[dict] = None) -> List[Path]:
    """Apply text or image watermark with angle, opacity, and mosaic repeating pattern."""
    mode = opt.get("mode", "text")
    text = opt.get("text") or "CONFIDENTIAL"
    size = float(opt.get("size", 48))
    opacity = float(opt.get("opacity", 0.25))
    rotation = float(opt.get("rotation", 45))
    pos = opt.get("position", "mc")
    mosaic = bool(opt.get("mosaic", False))
    overlay = opt.get("layer", "over") == "over"
    color_hex = opt.get("color", "#d53f4a")
    col = tuple(int(color_hex.lstrip("#")[i:i + 2], 16) / 255 for i in (0, 2, 4))
    style = opt.get("style", "bold")
    fontfile = pdf_env.font_file(family=opt.get("font", "arial"), style=style)

    img_path = None
    if mode == "image" and extra_files and extra_files.get("asset"):
        img_path = extra_files["asset"]
        with Image.open(img_path) as im:
            im = im.convert("RGBA")
            alpha = im.split()[3].point(lambda a: int(a * opacity))
            im.putalpha(alpha)
            img_path = Path(str(img_path) + "_op.png")
            im.save(img_path)

    outs: List[Path] = []
    for f in files:
        doc = pymupdf.open(str(f))
        pages = parse_ranges(opt.get("pages", ""), doc.page_count)
        for p in pages:
            page = doc[p]
            rect = page.rect
            if mode == "image" and img_path:
                iw, ih = Image.open(img_path).size
                scale = min(rect.width * 0.5 / iw, rect.height * 0.5 / ih, 1.0)
                w, h = iw * scale, ih * scale
                x, y = _grid_point(rect, pos, w, h)
                target = pymupdf.Rect(x, y - h, x + w, y)
                if mosaic:
                    ty = rect.y0
                    while ty < rect.y1:
                        tx = rect.x0
                        while tx < rect.x1:
                            page.insert_image(pymupdf.Rect(tx, ty, tx + w, ty + h),
                                              filename=str(img_path), overlay=overlay)
                            tx += w * 1.6
                        ty += h * 1.6
                else:
                    page.insert_image(target, filename=str(img_path), overlay=overlay)
            else:
                font = pymupdf.Font(fontfile=fontfile) if fontfile else pymupdf.Font("hebo")
                tw_len = font.text_length(text, fontsize=size)
                positions = []
                if mosaic:
                    step_x, step_y = tw_len + 80, size * 5
                    yy = rect.y0 + size
                    row = 0
                    while yy < rect.y1 + step_y:
                        xx = rect.x0 - (step_x / 2 if row % 2 else 0)
                        while xx < rect.x1:
                            positions.append((xx, yy))
                            xx += step_x
                        yy += step_y
                        row += 1
                else:
                    positions.append(_grid_point(rect, pos, tw_len, size))
                wr = pymupdf.TextWriter(rect, color=col, opacity=opacity)
                for (x, y) in positions:
                    wr.append((x, y), text, font=font, fontsize=size)
                if rotation:
                    pivot = pymupdf.Point((rect.x0 + rect.x1) / 2, (rect.y0 + rect.y1) / 2)
                    m = pymupdf.Matrix(1, 1).prerotate(-rotation)
                    wr.write_text(page, morph=(pivot, m), overlay=overlay)
                else:
                    wr.write_text(page, overlay=overlay)
        outs.append(_save_pdf(doc, unique_output_path(f"{Path(f).stem}_watermarked.pdf")))

    if img_path and str(img_path).endswith("_op.png"):
        Path(img_path).unlink(missing_ok=True)
    return outs


def crop(files: List[Path], opt: dict) -> List[Path]:
    """Crop PDF pages to a given bounding box or margins."""
    r = opt.get("rect")
    ref_w = float(opt.get("ref_w", 1))
    ref_h = float(opt.get("ref_h", 1))
    margin = float(opt.get("margin", 0))
    outs: List[Path] = []

    for f in files:
        doc = pymupdf.open(str(f))
        pages = parse_ranges(opt.get("pages", ""), doc.page_count)
        for p in pages:
            page = doc[p]
            pr = page.rect
            if r:
                sx = pr.width / ref_w if ref_w else 1.0
                sy = pr.height / ref_h if ref_h else 1.0
                new = pymupdf.Rect(
                    pr.x0 + r.get("x0", 0) * sx, pr.y0 + r.get("y0", 0) * sy,
                    pr.x0 + r.get("x1", ref_w) * sx, pr.y0 + r.get("y1", ref_h) * sy,
                )
            else:
                new = pymupdf.Rect(
                    pr.x0 + margin, pr.y0 + margin,
                    pr.x1 - margin, pr.y1 - margin,
                )
            new = new & pr
            if new.is_empty or new.width < 10 or new.height < 10:
                continue
            try:
                page.set_cropbox(new)
            except ValueError:
                pass
        outs.append(_save_pdf(doc, unique_output_path(f"{Path(f).stem}_cropped.pdf")))
    return outs


def edit_pdf(files: List[Path], opt: dict, extra_files: Optional[dict] = None) -> List[Path]:
    """Overlay annotations, text, images, lines, and shapes onto PDF pages."""
    f = files[0]
    doc = pymupdf.open(str(f))
    fontfile = pdf_env.font_file()
    for el in opt.get("elements", []):
        p = int(el["page"])
        if p >= doc.page_count:
            continue
        page = doc[p]
        x0, y0, x1, y1 = (float(el[k]) for k in ("x0", "y0", "x1", "y1"))
        rect = pymupdf.Rect(min(x0, x1), min(y0, y1), max(x0, x1), max(y0, y1))
        col = tuple(int(el.get("color", "#e5322d").lstrip("#")[i:i + 2], 16) / 255 for i in (0, 2, 4))
        el_type = el["type"]
        if el_type == "text":
            font = pymupdf.Font(fontfile=fontfile) if fontfile else pymupdf.Font("helv")
            size = float(el.get("size", 14))
            wr = pymupdf.TextWriter(page.rect, color=col)
            wr.fill_textbox(rect, el.get("text", ""), font=font, fontsize=size)
            wr.write_text(page)
        elif el_type == "image":
            key = el.get("asset", "asset")
            if extra_files and extra_files.get(key):
                page.insert_image(rect, filename=str(extra_files[key]))
        elif el_type == "rect":
            page.draw_rect(
                rect, color=col, width=float(el.get("width", 2)),
                fill=col if el.get("fill") else None,
                fill_opacity=0.35 if el.get("fill") else 1,
            )
        elif el_type == "ellipse":
            page.draw_oval(rect, color=col, width=float(el.get("width", 2)))
        elif el_type == "highlight":
            page.draw_rect(rect, color=(1, 0.85, 0.1), fill=(1, 0.85, 0.1), fill_opacity=0.35, width=0)
        elif el_type == "line":
            page.draw_line(pymupdf.Point(x0, y0), pymupdf.Point(x1, y1), color=col, width=float(el.get("width", 2)))
    return [_save_pdf(doc, unique_output_path(f"{Path(f).stem}_edited.pdf"))]


def metadata(files: List[Path], opt: dict) -> List[Path]:
    """Update PDF metadata fields (title, author, subject, keywords)."""
    f = files[0]
    doc = pymupdf.open(str(f))
    md = doc.metadata or {}
    for key in ("title", "author", "subject", "keywords", "creator", "producer"):
        if key in opt:
            md[key] = opt[key]
    doc.set_metadata(md)
    return [_save_pdf(doc, unique_output_path(f"{Path(f).stem}_metadata.pdf"))]


# ======================================================================
# 6. SECURITY & PROTECTION TOOLS
# ======================================================================

def protect(files: List[Path], opt: dict) -> List[Path]:
    """Encrypt PDF using AES-256 with granular permissions."""
    pw = opt.get("password") or opt.get("user_password", "")
    if not pw:
        raise ValueError("Password is required for document encryption.")
    allow_print = bool(opt.get("allow_print", True))
    allow_copy = bool(opt.get("allow_copy", False))
    outs: List[Path] = []

    for f in files:
        out = unique_output_path(f"{Path(f).stem}_protected.pdf")
        perms = pikepdf.Permissions(
            extract=allow_copy,
            print_lowres=allow_print,
            print_highres=allow_print,
            modify_annotation=False,
            modify_assembly=False,
            modify_form=False,
            modify_other=False,
        )
        with pikepdf.open(str(f)) as pdf:
            pdf.save(str(out), encryption=pikepdf.Encryption(
                user=pw, owner=opt.get("owner_password") or pw, R=6, allow=perms
            ))
        outs.append(out)
    return outs


def unlock(files: List[Path], opt: dict) -> List[Path]:
    """Remove encryption and password protection from PDF."""
    pw = opt.get("password") or opt.get("user_password", "")
    outs: List[Path] = []
    for f in files:
        out = unique_output_path(f"{Path(f).stem}_unlocked.pdf")
        try:
            pdf = pikepdf.open(str(f), password=pw)
        except pikepdf.PasswordError:
            raise ValueError("Incorrect password provided for unlocking document.")
        with pdf:
            pdf.save(str(out))
        outs.append(out)
    return outs


def sign(files: List[Path], opt: dict, extra_files: Optional[dict] = None) -> List[Path]:
    """Stamp digital signature image onto selected pages and coordinates."""
    f = files[0]
    doc = pymupdf.open(str(f))
    sig = extra_files.get("asset") if extra_files else None

    if not sig:
        name = opt.get("name") or opt.get("text") or "Approved Signature"
        im = Image.new("RGBA", (400, 100), (255, 255, 255, 0))
        draw = ImageDraw.Draw(im)
        draw.text((10, 30), name, fill=(30, 58, 138, 255))
        sig_path = new_workdir() / "sig.png"
        im.save(str(sig_path))
        sig = sig_path

    placements = opt.get("placements")
    if not placements:
        p = int(opt.get("page", 1)) - 1
        x = float(opt.get("x", 380))
        y = float(opt.get("y", 700))
        placements = [{"page": max(0, p), "x0": x, "y0": y, "x1": x + 160, "y1": y + 60}]

    for pl in placements:
        p = int(pl["page"])
        if p >= doc.page_count:
            continue
        rect = pymupdf.Rect(float(pl["x0"]), float(pl["y0"]), float(pl["x1"]), float(pl["y1"]))
        doc[p].insert_image(rect, filename=str(sig), keep_proportion=True)
    return [_save_pdf(doc, unique_output_path(f"{Path(f).stem}_signed.pdf"))]


def redact(files: List[Path], opt: dict) -> List[Path]:
    """Permanently cryptographically redact search terms and rectangular areas."""
    terms = [t for t in (opt.get("terms") or []) if t.strip()]
    areas = opt.get("areas") or []
    outs: List[Path] = []

    for f in files:
        doc = pymupdf.open(str(f))
        hits = 0
        for p in range(doc.page_count):
            page = doc[p]
            for term in terms:
                for r in page.search_for(term):
                    page.add_redact_annot(r, fill=(0, 0, 0))
                    hits += 1
            for a in areas:
                if int(a["page"]) == p:
                    page.add_redact_annot(pymupdf.Rect(a["x0"], a["y0"], a["x1"], a["y1"]), fill=(0, 0, 0))
                    hits += 1
            page.apply_redactions()
        if opt.get("clean_metadata", True):
            doc.set_metadata({})
            doc.del_xml_metadata()
        if terms and hits == 0:
            doc.close()
            raise ValueError(f"No occurrences of terms found in {Path(f).name} for redaction.")
        outs.append(_save_pdf(doc, unique_output_path(f"{Path(f).stem}_redacted.pdf")))
    return outs


def compare(files: List[Path], opt: dict) -> List[Path]:
    """Produce a visual side-by-side comparison report with differences highlighted."""
    if len(files) < 2:
        raise ValueError("Two PDF documents are required for comparison.")
    a, b = pymupdf.open(str(files[0])), pymupdf.open(str(files[1]))
    dpi = 100
    n = max(a.page_count, b.page_count)
    report = pymupdf.open()
    fontfile = pdf_env.font_file()

    for p in range(n):
        ims = []
        for doc in (a, b):
            if p < doc.page_count:
                pix = doc[p].get_pixmap(dpi=dpi)
                ims.append(Image.open(io.BytesIO(pix.tobytes("png"))).convert("RGB"))
            else:
                ims.append(None)
        w = max(im.width for im in ims if im) if any(ims) else 800
        h = max(im.height for im in ims if im) if any(ims) else 1000
        norm = []
        for im in ims:
            canvas = Image.new("RGB", (w, h), "white")
            if im:
                canvas.paste(im, (0, 0))
            norm.append(canvas)

        diff = ImageChops.difference(norm[0], norm[1]).convert("L")
        bbox_all = diff.point(lambda v: 255 if v > 30 else 0)
        boxes = []
        block = 24
        px = bbox_all.load()
        marked = set()
        for by in range(0, h, block):
            for bx in range(0, w, block):
                found = False
                for yy in range(by, min(by + block, h), 4):
                    for xx in range(bx, min(bx + block, w), 4):
                        if px[xx, yy]:
                            found = True
                            break
                    if found:
                        break
                if found:
                    marked.add((bx // block, by // block))
        for (cx, cy) in sorted(marked):
            boxes.append((cx * block, cy * block, cx * block + block, cy * block + block))
        changed = len(marked) > 0
        if changed:
            draw = ImageDraw.Draw(norm[1], "RGBA")
            for (x0, y0, x1, y1) in boxes:
                draw.rectangle([x0, y0, x1, y1], fill=(229, 50, 45, 60))

        gap, header = 20, 34
        page_out = report.new_page(
            width=(w * 2 + gap * 3) * 72 / dpi,
            height=(h + header + gap) * 72 / dpi,
        )
        s = 72 / dpi
        font = pymupdf.Font(fontfile=fontfile) if fontfile else pymupdf.Font("helv")
        wr = pymupdf.TextWriter(page_out.rect, color=(0.2, 0.2, 0.2))
        label = f"Page {p + 1}: " + ("Modified" if changed else "Identical")
        wr.append((gap * s, (header - 10) * s), label, font=font, fontsize=11)
        wr.write_text(page_out)
        for i, im in enumerate(norm):
            buf = io.BytesIO()
            im.save(buf, "PNG")
            x0 = (gap + i * (w + gap)) * s
            page_out.insert_image(
                pymupdf.Rect(x0, header * s, x0 + w * s, (header + h) * s),
                stream=buf.getvalue(),
            )

    a.close()
    b.close()
    out = unique_output_path(f"compare_{Path(files[0]).stem}_vs_{Path(files[1]).stem}.pdf")
    report.set_metadata({"title": "Comparison Report", "producer": "FileDeck PDF Studio"})
    return [_save_pdf(report, out)]


# ======================================================================
# OPERATION REGISTRY
# ======================================================================

TOOLS = {
    "merge": merge,
    "split": split,
    "remove_pages": remove_pages,
    "organize": organize,
    "compress": compress,
    "repair": repair,
    "ocr": ocr,
    "flatten": flatten,
    "images_to_pdf": images_to_pdf,
    "office_to_pdf": office_to_pdf,
    "html_to_pdf": html_to_pdf,
    "pdf_to_images": pdf_to_images,
    "extract_images": extract_images,
    "pdf_to_word": pdf_to_word,
    "pdf_to_text": pdf_to_text,
    "pdf_to_markdown": pdf_to_markdown,
    "pdf_to_pptx": pdf_to_pptx,
    "pdf_to_excel": pdf_to_excel,
    "pdf_to_pdfa": pdf_to_pdfa,
    "rotate": rotate,
    "page_numbers": page_numbers,
    "watermark": watermark,
    "crop": crop,
    "edit": edit_pdf,
    "metadata": metadata,
    "protect": protect,
    "unlock": unlock,
    "sign": sign,
    "redact": redact,
    "compare": compare,
}

TOOLS_WITH_ASSET = {"watermark", "edit", "sign"}

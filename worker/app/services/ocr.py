import base64
import io
import os
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple
from PIL import Image, ImageDraw, ImageOps

import pdfplumber
import pypdfium2

from app.config import settings
from app.services.barcodes import scan_barcodes_from_image

def clean_scanned_image(image: Image.Image) -> Image.Image:
    """Pre-process scanned page (unpaper-style cleanup):
    - Border clean: trim scanner perimeter black bars/shadows
    - Normalize contrast safely without clipping sparse text pixels
    """
    img = image.copy()
    if img.mode not in ("RGB", "L"):
        img = img.convert("RGB")

    try:
        # 1. Border clean: if outermost margins have dark scanner shadows/lines, trim slightly
        w, h = img.size
        margin_x = int(w * 0.01)
        margin_y = int(h * 0.01)
        if margin_x > 2 and margin_y > 2:
            bbox = (margin_x, margin_y, w - margin_x, h - margin_y)
            img = img.crop(bbox)

        # 2. Gentle contrast enhancement without destroying low-density text (cutoff=0)
        if img.mode == "RGB":
            # Autocontrast on RGB directly or gentle stretch
            img = ImageOps.autocontrast(img, cutoff=0)
        else:
            img = ImageOps.autocontrast(img, cutoff=0)
    except Exception as e:
        print(f"Scan pre-cleaning warning: {e}")

    return img


try:
    import pytesseract

    HAS_PYTESSERACT = True
except ImportError:  # optional dependency
    HAS_PYTESSERACT = False

OFFICE_EXTENSIONS = (
    ".doc", ".docx", ".odt", ".rtf", ".xls", ".xlsx", ".ods", ".ppt", ".pptx", ".odp",
)

# Below this many characters of embedded text a PDF page set is treated as a scan and OCR'd.
MIN_EMBEDDED_TEXT_CHARS = 20


def tesseract_available() -> bool:
    return HAS_PYTESSERACT and shutil.which("tesseract") is not None


def soffice_binary() -> Optional[str]:
    for name in ("soffice", "libreoffice"):
        found = shutil.which(name)
        if found:
            return found
    for candidate in (
        r"C:\Program Files\LibreOffice\program\soffice.exe",
        r"C:\Program Files (x86)\LibreOffice\program\soffice.exe",
    ):
        if os.path.exists(candidate):
            return candidate
    return None


_rapid_ocr_engine = None


def get_rapid_ocr():
    global _rapid_ocr_engine
    if _rapid_ocr_engine is None:
        try:
            from rapidocr_onnxruntime import RapidOCR

            _rapid_ocr_engine = RapidOCR()
        except Exception as e:
            print(f"RapidOCR initialization warning: {e}")
            _rapid_ocr_engine = False
    return _rapid_ocr_engine if _rapid_ocr_engine is not False else None


def ocr_single_image(image: Image.Image) -> Tuple[str, List[Dict[str, Any]]]:
    """Run OCR on a PIL Image. Returns (extracted_text, list_of_word_boxes).
    Uses tesseract if available; otherwise falls back to embedded RapidOCR (ONNX).
    """
    cleaned_img = clean_scanned_image(image)
    if tesseract_available():
        try:
            text = pytesseract.image_to_string(cleaned_img, lang=settings.ocr_language).strip()
            if text:
                return text, []
        except Exception as e:
            print(f"Tesseract OCR failed, falling back to embedded RapidOCR: {e}")

    engine = get_rapid_ocr()
    if engine:
        try:
            import numpy as np

            rgb = cleaned_img.convert("RGB") if cleaned_img.mode != "RGB" else cleaned_img
            res, _ = engine(np.array(rgb))
            if not res:
                raw_rgb = image.convert("RGB") if image.mode != "RGB" else image
                res, _ = engine(np.array(raw_rgb))
            if res:
                lines = [item[1] for item in res if item and len(item) > 1 and item[1]]
                boxes = [
                    {"box": item[0], "text": item[1], "score": float(item[2])}
                    for item in res
                    if item and len(item) > 2
                ]
                return "\n".join(lines).strip(), boxes
        except Exception as e:
            print(f"RapidOCR extraction error: {e}")

    return "", []


def generate_searchable_pdf_from_pdf(contents: bytes) -> Optional[bytes]:
    """Inject invisible OCR text layer into a scanned PDF in-memory (zero external dependencies)."""
    engine = get_rapid_ocr()
    if not engine:
        return None
    try:
        import numpy as np
        from pypdf import PdfReader, PdfWriter
        from reportlab.pdfgen import canvas

        reader = PdfReader(io.BytesIO(contents))
        pdf_doc = pypdfium2.PdfDocument(contents)
        writer = PdfWriter()

        for idx, orig_page in enumerate(reader.pages):
            scale = 2.0
            pil_page = pdf_doc[idx].render(scale=scale).to_pil()
            cleaned = clean_scanned_image(pil_page)
            ocr_res, _ = engine(np.array(cleaned.convert("RGB")))
            if not ocr_res:
                ocr_res, _ = engine(np.array(pil_page.convert("RGB")))

            if not ocr_res:
                writer.add_page(orig_page)
                continue

            page_w = float(orig_page.mediabox.width)
            page_h = float(orig_page.mediabox.height)

            overlay_buf = io.BytesIO()
            c = canvas.Canvas(overlay_buf, pagesize=(page_w, page_h))
            text_obj = c.beginText()
            text_obj.setTextRenderMode(3)  # Invisible text mode

            for item in ocr_res:
                box, text = item[0], item[1]
                if not text or not text.strip():
                    continue
                xs = [pt[0] / scale for pt in box]
                ys = [pt[1] / scale for pt in box]
                min_x = min(xs)
                max_y = max(ys)
                min_y = min(ys)
                font_size = max(6.0, (max_y - min_y) * 0.85)

                pdf_x = max(0.0, min_x)
                pdf_y = max(0.0, page_h - max_y)

                text_obj.setFont("Helvetica", font_size)
                text_obj.setTextOrigin(pdf_x, pdf_y)
                text_obj.textLine(text.strip())

            c.drawText(text_obj)
            c.save()

            overlay_reader = PdfReader(io.BytesIO(overlay_buf.getvalue()))
            if overlay_reader.pages:
                orig_page.merge_page(overlay_reader.pages[0])
            writer.add_page(orig_page)

        out_buf = io.BytesIO()
        writer.write(out_buf)
        return out_buf.getvalue()
    except Exception as e:
        print(f"generate_searchable_pdf_from_pdf error: {e}")
        return None


def generate_searchable_pdf_from_image(image: Image.Image) -> Optional[bytes]:
    """Convert an image into a searchable 1-page PDF archive with invisible text overlay."""
    try:
        from pypdf import PdfReader, PdfWriter
        from reportlab.lib.utils import ImageReader
        from reportlab.pdfgen import canvas

        rgb_img = image.convert("RGB")
        w_px, h_px = rgb_img.size
        # 150 DPI standard conversion to points
        w_pt, h_pt = float(w_px * 72 / 150), float(h_px * 72 / 150)
        if w_pt <= 0 or h_pt <= 0:
            w_pt, h_pt = float(w_px), float(h_px)

        base_buf = io.BytesIO()
        c_base = canvas.Canvas(base_buf, pagesize=(w_pt, h_pt))
        img_temp_buf = io.BytesIO()
        rgb_img.save(img_temp_buf, format="JPEG", quality=90)
        c_base.drawImage(ImageReader(img_temp_buf), 0, 0, width=w_pt, height=h_pt)
        c_base.save()

        # Run OCR to get bounding boxes
        _text, boxes = ocr_single_image(rgb_img)
        if not boxes:
            return base_buf.getvalue()

        sx = w_pt / float(w_px)
        sy = h_pt / float(h_px)

        overlay_buf = io.BytesIO()
        c_text = canvas.Canvas(overlay_buf, pagesize=(w_pt, h_pt))
        text_obj = c_text.beginText()
        text_obj.setTextRenderMode(3)  # Invisible text

        for b in boxes:
            box, text = b["box"], b["text"]
            if not text.strip():
                continue
            xs = [pt[0] * sx for pt in box]
            ys = [pt[1] * sy for pt in box]
            min_x = min(xs)
            max_y = max(ys)
            font_size = max(6.0, (max(ys) - min(ys)) * 0.85)

            pdf_x = max(0.0, min_x)
            pdf_y = max(0.0, h_pt - max_y)

            text_obj.setFont("Helvetica", font_size)
            text_obj.setTextOrigin(pdf_x, pdf_y)
            text_obj.textLine(text.strip())

        c_text.drawText(text_obj)
        c_text.save()

        reader_base = PdfReader(io.BytesIO(base_buf.getvalue()))
        reader_overlay = PdfReader(io.BytesIO(overlay_buf.getvalue()))
        writer = PdfWriter()
        p = reader_base.pages[0]
        if reader_overlay.pages:
            p.merge_page(reader_overlay.pages[0])
        writer.add_page(p)

        out_buf = io.BytesIO()
        writer.write(out_buf)
        return out_buf.getvalue()
    except Exception as e:
        print(f"generate_searchable_pdf_from_image error: {e}")
        return None


def ocr_pdf_pages(contents: bytes) -> str:
    """OCR every page of a PDF. Uses Tesseract if installed, otherwise embedded RapidOCR."""
    pages: List[str] = []
    try:
        pdf_doc = pypdfium2.PdfDocument(contents)
        for idx in range(len(pdf_doc)):
            raw_img = pdf_doc[idx].render(scale=2.5).to_pil()
            page_text, _ = ocr_single_image(raw_img)
            if page_text:
                pages.append(page_text.strip())
    except Exception as e:
        print(f"OCR failed: {e}")
    return "\n\n".join(p for p in pages if p)


def make_pdfa_archive(contents: bytes) -> Optional[bytes]:
    """Produce an OCR'd PDF/A archive copy.
    Tries ocrmypdf if installed; falls back to in-memory PDF invisible text overlay.
    """
    exe = shutil.which("ocrmypdf")
    if exe:
        with tempfile.TemporaryDirectory() as tmp:
            src, dst = os.path.join(tmp, "in.pdf"), os.path.join(tmp, "out.pdf")
            Path(src).write_bytes(contents)
            try:
                subprocess.run(
                    [exe, "--skip-text", "--output-type", "pdfa", "-l", settings.ocr_language, src, dst],
                    check=True, capture_output=True, timeout=900,
                )
                return Path(dst).read_bytes()
            except Exception as e:
                print(f"ocrmypdf failed: {e}")

    # Fallback to zero-install in-memory invisible text layer injection
    return generate_searchable_pdf_from_pdf(contents)


def convert_office_to_pdf(contents: bytes, filename: str) -> Optional[bytes]:
    """Convert an Office document to PDF using headless LibreOffice (graceful if missing)."""
    exe = soffice_binary()
    if not exe:
        print("LibreOffice not found; Office document support disabled")
        return None
    with tempfile.TemporaryDirectory() as tmp:
        src = os.path.join(tmp, os.path.basename(filename) or "document")
        Path(src).write_bytes(contents)
        try:
            subprocess.run(
                [exe, "--headless", "--convert-to", "pdf", "--outdir", tmp, src],
                check=True, capture_output=True, timeout=300,
            )
            out = os.path.join(tmp, Path(src).stem + ".pdf")
            return Path(out).read_bytes() if os.path.exists(out) else None
        except Exception as e:
            print(f"LibreOffice conversion failed: {e}")
            return None


def generate_thumbnail_from_image(image: Image.Image, max_width: int = 500) -> str:
    """Resize image to thumbnail dimensions and return base64-encoded WebP."""
    img = image.copy()
    if img.mode not in ("RGB", "L"):
        img = img.convert("RGB")

    width, height = img.size
    if width > max_width:
        ratio = max_width / float(width)
        new_height = int(height * ratio)
        img = img.resize((max_width, new_height), Image.Resampling.LANCZOS)

    buf = io.BytesIO()
    img.save(buf, format="WEBP", quality=85)
    return base64.b64encode(buf.getvalue()).decode("ascii")


def generate_placeholder_thumbnail(title: str = "Document") -> str:
    """Generate a placeholder thumbnail when no image can be rendered."""
    img = Image.new("RGB", (350, 480), color=(248, 250, 252))
    draw = ImageDraw.Draw(img)
    # Simple document border
    draw.rectangle([(15, 15), (335, 465)], outline=(203, 213, 225), width=2)
    # Header box
    draw.rectangle([(30, 40), (320, 90)], fill=(226, 232, 240))
    # Fake text lines
    for y in range(120, 420, 28):
        draw.line([(30, y), (300, y)], fill=(203, 213, 225), width=4)

    buf = io.BytesIO()
    img.save(buf, format="WEBP", quality=85)
    return base64.b64encode(buf.getvalue()).decode("ascii")


def process_pdf(contents: bytes) -> Tuple[str, int, Optional[str], List[Dict[str, Any]]]:
    """Process PDF document: extract text, generate thumbnail, detect barcodes."""
    text_content = ""
    page_count = 1
    thumbnail_b64 = None
    all_barcodes = []

    # 1. Page count and Thumbnail rendering with pypdfium2
    try:
        pdf_doc = pypdfium2.PdfDocument(contents)
        page_count = len(pdf_doc)

        # Render first page as high-res PIL Image for thumbnail and barcode scanning
        if page_count > 0:
            first_page = pdf_doc[0]
            # render with scale=2 for crisp preview
            first_image = first_page.render(scale=2.0).to_pil()
            thumbnail_b64 = generate_thumbnail_from_image(first_image, max_width=500)

            # Check barcodes on page 1
            bcs = scan_barcodes_from_image(first_image, page_number=1)
            all_barcodes.extend(bcs)

            # Scan every page: separator pages (PATCHT) can be anywhere in a batch scan
            for p_idx in range(1, page_count):
                page_img = pdf_doc[p_idx].render(scale=1.5).to_pil()
                bcs = scan_barcodes_from_image(page_img, page_number=p_idx + 1)
                all_barcodes.extend(bcs)
    except Exception as e:
        print(f"pypdfium2 error: {e}")

    # 2. Extract full text with pdfplumber
    try:
        with pdfplumber.open(io.BytesIO(contents)) as pdf:
            extracted_pages = []
            for p in pdf.pages:
                t = p.extract_text()
                if t:
                    extracted_pages.append(t)
            if extracted_pages:
                text_content = "\n\n".join(extracted_pages)
    except Exception as e:
        print(f"pdfplumber text extraction error: {e}")

    if len(text_content.strip()) < MIN_EMBEDDED_TEXT_CHARS:
        ocr_text = ocr_pdf_pages(contents)
        if ocr_text:
            text_content = ocr_text

    if not thumbnail_b64:
        thumbnail_b64 = generate_placeholder_thumbnail()

    return text_content.strip(), max(page_count, 1), thumbnail_b64, all_barcodes


def process_image(contents: bytes) -> Tuple[str, int, Optional[str], List[Dict[str, Any]]]:
    """Process an image file: generate thumbnail, scan barcodes, and extract OCR text."""
    raw_img = Image.open(io.BytesIO(contents))
    cleaned_img = clean_scanned_image(raw_img)
    thumbnail_b64 = generate_thumbnail_from_image(cleaned_img, max_width=500)
    barcodes = scan_barcodes_from_image(raw_img, page_number=1)
    text, _ = ocr_single_image(raw_img)
    return text, 1, thumbnail_b64, barcodes


def process_plain_text(contents: bytes) -> Tuple[str, int, Optional[str], List[Dict[str, Any]]]:
    """Process plain text / markdown / csv."""
    try:
        text = contents.decode("utf-8", errors="replace")
    except Exception:
        text = ""
    thumbnail_b64 = generate_placeholder_thumbnail("Text File")
    return text.strip(), 1, thumbnail_b64, []


def extract_text_from_office(contents: bytes, ext: str) -> str:
    """Extract text from .docx, .xlsx, or .pptx directly without LibreOffice."""
    import zipfile
    import xml.etree.ElementTree as ET

    try:
        with zipfile.ZipFile(io.BytesIO(contents)) as z:
            if ext == ".docx":
                if "word/document.xml" in z.namelist():
                    tree = ET.fromstring(z.read("word/document.xml"))
                    paragraphs = []
                    for p in tree.iter("{http://schemas.openxmlformats.org/wordprocessingml/2006/main}p"):
                        texts = [node.text for node in p.iter("{http://schemas.openxmlformats.org/wordprocessingml/2006/main}t") if node.text]
                        if texts:
                            paragraphs.append("".join(texts))
                    return "\n\n".join(paragraphs)
            elif ext in (".xlsx", ".ods"):
                if "xl/sharedStrings.xml" in z.namelist():
                    tree = ET.fromstring(z.read("xl/sharedStrings.xml"))
                    strings = [t.text.strip() for t in tree.iter("{http://schemas.openxmlformats.org/spreadsheetml/2006/main}t") if t.text]
                    return "\n".join(strings)
            elif ext in (".pptx", ".odp"):
                slides = [n for n in z.namelist() if n.startswith("ppt/slides/slide") and n.endswith(".xml")]
                texts = []
                for s in sorted(slides):
                    tree = ET.fromstring(z.read(s))
                    for t in tree.iter("{http://schemas.openxmlformats.org/drawingml/2006/main}t"):
                        if t.text:
                            texts.append(t.text.strip())
                return "\n".join(texts)
    except Exception as e:
        print(f"Direct office text extraction error: {e}")
    return ""


def collate_pdfs(odd_bytes: bytes, even_bytes: bytes) -> bytes:
    """Collate two single-sided scans of a double-sided document.
    
    The even pages are reversed (since ADF scans bottom-to-top) and interleaved
    with the odd pages.
    """
    from pypdf import PdfReader, PdfWriter

    odd_reader = PdfReader(io.BytesIO(odd_bytes))
    even_reader = PdfReader(io.BytesIO(even_bytes))

    odd_pages = list(odd_reader.pages)
    even_pages = list(even_reader.pages)
    even_pages.reverse()

    writer = PdfWriter()
    max_len = max(len(odd_pages), len(even_pages))
    for i in range(max_len):
        if i < len(odd_pages):
            writer.add_page(odd_pages[i])
        if i < len(even_pages):
            writer.add_page(even_pages[i])

    buf = io.BytesIO()
    writer.write(buf)
    return buf.getvalue()


def split_pdf(contents: bytes, separator_pages: List[int], retain_separator: bool = False) -> List[bytes]:
    """Split a PDF at 1-based separator pages. Empty parts are dropped."""
    from pypdf import PdfReader, PdfWriter

    reader = PdfReader(io.BytesIO(contents))
    seps = set(separator_pages)
    groups: List[List[int]] = [[]]
    for i in range(len(reader.pages)):
        page_no = i + 1
        if page_no in seps:
            if retain_separator:
                groups[-1].append(i)
            groups.append([])
        else:
            groups[-1].append(i)
    parts: List[bytes] = []
    for g in groups:
        if not g:
            continue
        writer = PdfWriter()
        for i in g:
            writer.add_page(reader.pages[i])
        buf = io.BytesIO()
        writer.write(buf)
        parts.append(buf.getvalue())
    return parts


def find_separator_pages(barcodes: List[Dict[str, Any]], separator: str) -> List[int]:
    return sorted({b["page"] for b in barcodes if separator and b.get("value", "").strip().upper() == separator.upper()})


def extract_document_data(contents: bytes, filename: str, separator: str = "PATCHT") -> Dict[str, Any]:
    """Main document extraction entrypoint."""
    ext = os.path.splitext(filename.lower())[1]
    archive: Optional[bytes] = None

    if ext in OFFICE_EXTENSIONS:
        pdf_bytes = convert_office_to_pdf(contents, filename)
        if pdf_bytes:
            content, page_count, thumbnail_b64, barcodes = process_pdf(pdf_bytes)
            archive = pdf_bytes
        else:
            content = extract_text_from_office(contents, ext)
            page_count, thumbnail_b64, barcodes = 1, generate_placeholder_thumbnail(os.path.basename(filename)), []
        return {
            "content": content,
            "page_count": page_count,
            "thumbnail_b64": thumbnail_b64,
            "barcodes": barcodes,
            "separator_pages": [],
            "archive_b64": base64.b64encode(archive).decode("ascii") if archive else None,
        }


    if ext in (".pdf", ""):
        # Check PDF magic bytes '%PDF'
        if contents.startswith(b"%PDF"):
            content, page_count, thumbnail_b64, barcodes = process_pdf(contents)
        else:
            # Fallback to text
            content, page_count, thumbnail_b64, barcodes = process_plain_text(contents)
    elif ext in (".png", ".jpg", ".jpeg", ".webp", ".tif", ".tiff", ".bmp"):
        content, page_count, thumbnail_b64, barcodes = process_image(contents)
        try:
            raw_img = Image.open(io.BytesIO(contents))
            archive = generate_searchable_pdf_from_image(raw_img)
        except Exception as e:
            print(f"Image searchable PDF generation failed: {e}")
    elif ext in (".txt", ".md", ".csv", ".json", ".log"):
        content, page_count, thumbnail_b64, barcodes = process_plain_text(contents)
    else:
        # Default try PDF or text
        if contents.startswith(b"%PDF"):
            content, page_count, thumbnail_b64, barcodes = process_pdf(contents)
        else:
            content, page_count, thumbnail_b64, barcodes = process_plain_text(contents)

    is_pdf = contents.startswith(b"%PDF")
    if is_pdf:
        archive = make_pdfa_archive(contents)

    return {
        "content": content,
        "page_count": page_count,
        "thumbnail_b64": thumbnail_b64,
        "barcodes": barcodes,
        "separator_pages": find_separator_pages(barcodes, separator) if is_pdf else [],
        "archive_b64": base64.b64encode(archive).decode("ascii") if archive else None,
    }

from contextlib import asynccontextmanager
from typing import Any, Dict, List, Optional

import base64
import json

from fastapi import FastAPI, File, Form, HTTPException, UploadFile
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel

from app.config import settings
from app.services.classifier import classifier_service
from app.services.ocr import extract_document_data, split_pdf, collate_pdfs, tesseract_available, soffice_binary


@asynccontextmanager
async def lifespan(app: FastAPI):
    print(f"FileDeck Worker started on {settings.host}:{settings.port}")
    yield
    print("FileDeck Worker shutting down")


app = FastAPI(title="FileDeck Worker", version="0.2.0", lifespan=lifespan)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.get("/health")
async def health_check():
    return {
        "status": "ok",
        "service": "filedeck-worker",
        "classifier_trained": classifier_service.is_trained,
        "ocr": tesseract_available(),
        "office": soffice_binary() is not None,
    }


class ProcessDocResult(BaseModel):
    content: str
    page_count: int
    thumbnail_b64: Optional[str] = None
    barcodes: List[Dict[str, Any]] = []
    separator_pages: List[int] = []
    archive_b64: Optional[str] = None


@app.post("/process-doc", response_model=ProcessDocResult)
async def process_document(
    file: UploadFile = File(...),
    separator: str = Form("PATCHT"),
):
    contents = await file.read()
    if not contents:
        raise HTTPException(status_code=400, detail="Empty file provided")

    filename = file.filename or "document.pdf"
    result = extract_document_data(contents, filename, separator=separator)

    return ProcessDocResult(**result)


@app.post("/split-pdf")
async def split_pdf_endpoint(
    file: UploadFile = File(...),
    separator_pages: str = Form("[]"),
    retain_separator: bool = Form(False),
):
    contents = await file.read()
    try:
        pages = [int(p) for p in json.loads(separator_pages)]
        parts = split_pdf(contents, pages, retain_separator)
    except Exception as e:
        raise HTTPException(status_code=400, detail=f"Could not split PDF: {e}")
    return {"parts": [base64.b64encode(p).decode("ascii") for p in parts]}


def _pdf_response(data: bytes) -> Dict[str, str]:
    return {"pdf_b64": base64.b64encode(data).decode("ascii")}


@app.post("/pdf/rotate")
async def pdf_rotate(file: UploadFile = File(...), degrees: int = Form(90), pages: str = Form("[]")):
    """Rotate all pages (or the 1-based `pages` list) clockwise by `degrees`."""
    from pypdf import PdfReader, PdfWriter
    import io

    try:
        reader = PdfReader(io.BytesIO(await file.read()))
        only = set(json.loads(pages))
        writer = PdfWriter()
        for i, page in enumerate(reader.pages, start=1):
            if not only or i in only:
                page.rotate(degrees)
            writer.add_page(page)
        buf = io.BytesIO()
        writer.write(buf)
        return _pdf_response(buf.getvalue())
    except Exception as e:
        raise HTTPException(status_code=400, detail=f"Could not rotate PDF: {e}")


@app.post("/pdf/merge")
async def pdf_merge(files: List[UploadFile] = File(...)):
    from pypdf import PdfReader, PdfWriter
    import io

    try:
        writer = PdfWriter()
        for f in files:
            for page in PdfReader(io.BytesIO(await f.read())).pages:
                writer.add_page(page)
        buf = io.BytesIO()
        writer.write(buf)
        return _pdf_response(buf.getvalue())
    except Exception as e:
        raise HTTPException(status_code=400, detail=f"Could not merge PDFs: {e}")


@app.post("/pdf/collate")
async def pdf_collate(odd_file: UploadFile = File(...), even_file: UploadFile = File(...)):
    """Collate odd and even page single-sided scans of a double-sided document."""
    try:
        odd_bytes = await odd_file.read()
        even_bytes = await even_file.read()
        collated = collate_pdfs(odd_bytes, even_bytes)
        return _pdf_response(collated)
    except Exception as e:
        raise HTTPException(status_code=400, detail=f"Could not collate PDFs: {e}")


@app.post("/pdf/edit")
async def pdf_edit(file: UploadFile = File(...), operations: str = Form(...)):
    """operations: [{"page": 1-based, "rotate": degrees, "doc": output index}, ...] in output order."""
    from pypdf import PdfReader, PdfWriter
    import io

    try:
        reader = PdfReader(io.BytesIO(await file.read()))
        ops = json.loads(operations)
        writers: Dict[int, Any] = {}
        for op in ops:
            page = reader.pages[int(op["page"]) - 1]
            if op.get("rotate"):
                page.rotate(int(op["rotate"]))
            writers.setdefault(int(op.get("doc", 0)), PdfWriter()).add_page(page)
        parts = []
        for key in sorted(writers):
            buf = io.BytesIO()
            writers[key].write(buf)
            parts.append(base64.b64encode(buf.getvalue()).decode("ascii"))
        return {"parts": parts}
    except Exception as e:
        raise HTTPException(status_code=400, detail=f"Could not edit PDF: {e}")


@app.post("/pdf/remove-password")
async def pdf_remove_password(file: UploadFile = File(...), password: str = Form("")):
    from pypdf import PdfReader, PdfWriter
    import io

    try:
        reader = PdfReader(io.BytesIO(await file.read()))
        if reader.is_encrypted and not reader.decrypt(password):
            raise HTTPException(status_code=400, detail="Incorrect password")
        writer = PdfWriter()
        for page in reader.pages:
            writer.add_page(page)
        buf = io.BytesIO()
        writer.write(buf)
        return _pdf_response(buf.getvalue())
    except HTTPException:
        raise
    except Exception as e:
        raise HTTPException(status_code=400, detail=f"Could not remove password: {e}")


class ClassifyRequest(BaseModel):
    content: str
    threshold: Optional[float] = 0.5


@app.post("/classify")
async def classify_text(req: ClassifyRequest):
    return classifier_service.predict(req.content, threshold=req.threshold or 0.5)


@app.get("/classifier/status")
async def classifier_status():
    return {
        "trained": classifier_service.is_trained,
        "models": {
            "tags": len(classifier_service.tags_classifiers),
            "has_correspondent": classifier_service.correspondent_classifier is not None,
            "has_doc_type": classifier_service.doc_type_classifier is not None,
        },
    }


class TrainDocument(BaseModel):
    content: str
    tags: List[int] = []
    correspondent_id: Optional[int] = None
    document_type_id: Optional[int] = None


class RetrainRequest(BaseModel):
    documents: List[TrainDocument]


@app.post("/retrain")
async def retrain_classifier(req: RetrainRequest):
    docs = [d.model_dump() for d in req.documents]
    return classifier_service.train(docs)


class MailTestRequest(BaseModel):
    imap_server: str
    imap_port: Optional[int] = None
    imap_security: int = 1
    username: str
    password: str


@app.post("/mail/test")
async def test_mail(req: MailTestRequest):
    import imaplib
    import ssl

    port = req.imap_port or (993 if req.imap_security == 1 else 143)
    try:
        if req.imap_security == 1:
            context = ssl.create_default_context()
            server = imaplib.IMAP4_SSL(req.imap_server, port, ssl_context=context)
        else:
            server = imaplib.IMAP4(req.imap_server, port)
            if req.imap_security == 2:
                server.starttls()
        server.login(req.username, req.password)
        server.logout()
        return {"success": True, "message": "Successfully authenticated with IMAP server"}
    except Exception as e:
        return {"success": False, "detail": str(e)}


class MailFetchRequest(BaseModel):
    imap_server: str
    imap_port: Optional[int] = None
    imap_security: int = 1
    username: str
    password: str
    folder: str = "INBOX"
    filter_from: Optional[str] = None
    filter_subject: Optional[str] = None
    limit: int = 20


@app.post("/mail/fetch")
async def fetch_mail(req: MailFetchRequest):
    import email
    from email.header import decode_header
    import imaplib
    import ssl

    def decode_mime(header_val):
        if not header_val:
            return ""
        parts = decode_header(header_val)
        res = []
        for text, enc in parts:
            if isinstance(text, bytes):
                res.append(text.decode(enc or "utf-8", errors="replace"))
            else:
                res.append(str(text))
        return " ".join(res)

    port = req.imap_port or (993 if req.imap_security == 1 else 143)
    try:
        if req.imap_security == 1:
            context = ssl.create_default_context()
            server = imaplib.IMAP4_SSL(req.imap_server, port, ssl_context=context)
        else:
            server = imaplib.IMAP4(req.imap_server, port)
            if req.imap_security == 2:
                server.starttls()
        server.login(req.username, req.password)
        server.select(req.folder or "INBOX")

        status, msg_ids = server.search(None, "ALL")
        if status != "OK" or not msg_ids[0]:
            server.logout()
            return {"messages_processed": 0, "attachments": []}

        id_list = msg_ids[0].split()
        selected_ids = id_list[-req.limit :]

        attachments = []
        for mid in selected_ids:
            res, data = server.fetch(mid, "(RFC822)")
            if res != "OK":
                continue
            raw_email = data[0][1]
            msg = email.message_from_bytes(raw_email)

            subject = decode_mime(msg.get("Subject", ""))
            from_addr = decode_mime(msg.get("From", ""))
            message_id = msg.get("Message-ID", "")

            if req.filter_from and req.filter_from.lower() not in from_addr.lower():
                continue
            if req.filter_subject and req.filter_subject.lower() not in subject.lower():
                continue

            for part in msg.walk():
                if part.get_content_maintype() == "multipart":
                    continue
                filename = part.get_filename()
                if filename:
                    filename = decode_mime(filename)
                    payload = part.get_payload(decode=True)
                    if payload:
                        attachments.append({
                            "message_id": message_id,
                            "from": from_addr,
                            "subject": subject,
                            "filename": filename,
                            "data_b64": base64.b64encode(payload).decode("ascii"),
                        })

        server.logout()
        return {"messages_processed": len(selected_ids), "attachments": attachments}
    except Exception as e:
        raise HTTPException(status_code=400, detail=f"Mail fetch error: {e}")


from app.routers.ai import router as ai_router  # noqa: E402
from app.routes.pdf_studio import router as pdf_studio_router
from app.routers.extraction import router as extraction_router

app.include_router(ai_router)
app.include_router(pdf_studio_router)
app.include_router(extraction_router)


if __name__ == "__main__":
    import uvicorn
    uvicorn.run("app.main:app", host=settings.host, port=settings.port, reload=False)

import os
import sys
from pathlib import Path

os.environ.setdefault("FILEDECK_EMBEDDING_BACKEND", "hash")  # deterministic & offline for CI
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import math  # noqa: E402

from fastapi.testclient import TestClient  # noqa: E402

from app.main import app  # noqa: E402

client = TestClient(app)


def cos(a, b):
    return sum(x * y for x, y in zip(a, b))


def test_embed_single_and_batch():
    r = client.post("/embed", json={"text": "invoice for widgets"}).json()
    assert r["backend"] == "hash" and r["dim"] == 384
    assert len(r["embedding"]) == 384
    assert math.isclose(sum(x * x for x in r["embedding"]), 1.0, rel_tol=1e-6)
    b = client.post("/embed", json={"texts": ["a b c", "invoice for widgets"]}).json()
    assert len(b["embeddings"]) == 2
    assert b["embeddings"][1] == r["embedding"]  # deterministic


def test_hash_similarity_orders_related_text():
    q, near, far = (client.post("/embed", json={"text": t}).json()["embedding"] for t in (
        "quarterly invoice widgets", "invoice from Acme about widgets", "holiday photos Lisbon trip"))
    assert cos(q, near) > cos(q, far)


def test_embed_requires_input():
    assert client.post("/embed", json={}).status_code == 422


def _table_pdf():
    cells = [["Item", "Qty"], ["Widget", "4"], ["Gadget", "7"]]
    ops = []
    x0, y0, cw, rh = 72, 700, 100, 24
    for r in range(len(cells) + 1):
        ops.append(f"{x0} {y0 - r * rh} m {x0 + 2 * cw} {y0 - r * rh} l S")
    for c in range(3):
        ops.append(f"{x0 + c * cw} {y0} m {x0 + c * cw} {y0 - len(cells) * rh} l S")
    for r, row in enumerate(cells):
        for c, t in enumerate(row):
            ops.append(f"BT /F1 12 Tf {x0 + c * cw + 6} {y0 - r * rh - 16} Td ({t}) Tj ET")
    stream = "\n".join(ops).encode()
    objs = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
        b"<< /Length %d >>\nstream\n" % len(stream) + stream + b"\nendstream",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    ]
    out, offs = b"%PDF-1.4\n", []
    for i, o in enumerate(objs, 1):
        offs.append(len(out))
        out += b"%d 0 obj\n" % i + o + b"\nendobj\n"
    x = len(out)
    out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1)
    out += b"".join(b"%010d 00000 n \n" % o for o in offs)
    out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, x)
    return out


def test_extract_tables():
    r = client.post("/extract-tables", files={"file": ("t.pdf", _table_pdf(), "application/pdf")})
    assert r.status_code == 200, r.text
    body = r.json()
    assert body["count"] == 1
    assert body["tables"][0]["rows"] == [["Item", "Qty"], ["Widget", "4"], ["Gadget", "7"]]


def test_extract_tables_rejects_garbage():
    r = client.post("/extract-tables", files={"file": ("t.pdf", b"nope", "application/pdf")})
    assert r.status_code == 400

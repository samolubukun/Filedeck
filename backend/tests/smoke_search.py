"""End-to-end smoke test: upload -> index -> search / filter / order / autocomplete.

Usage (from backend/):  python tests/smoke_search.py
Starts the compiled debug server against a throw-away data dir on port 18080.
"""
import json
import os
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid

PORT = 18080
BASE = f"http://127.0.0.1:{PORT}"
EXE = os.path.join("target", "debug", "filedeck-backend.exe" if os.name == "nt" else "filedeck-backend")


def pdf(text: str) -> bytes:
    stream = f"BT /F1 18 Tf 72 720 Td ({text}) Tj ET".encode()
    objs = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
        b"<< /Length %d >>\nstream\n" % len(stream) + stream + b"\nendstream",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    ]
    out = b"%PDF-1.4\n"
    offsets = []
    for i, o in enumerate(objs, 1):
        offsets.append(len(out))
        out += b"%d 0 obj\n" % i + o + b"\nendobj\n"
    xref = len(out)
    out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1)
    for off in offsets:
        out += b"%010d 00000 n \n" % off
    out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, xref)
    return out


def req(method, path, body=None, headers=None, raw=False):
    data = None
    h = dict(headers or {})
    if body is not None and not raw:
        data = json.dumps(body).encode()
        h["Content-Type"] = "application/json"
    elif raw:
        data = body
    r = urllib.request.Request(BASE + path, data=data, method=method, headers=h)
    try:
        with urllib.request.urlopen(r, timeout=120) as resp:
            txt = resp.read().decode()
            return resp.status, (json.loads(txt) if txt else None)
    except urllib.error.HTTPError as e:
        txt = e.read().decode()
        try:
            return e.code, json.loads(txt)
        except Exception:
            return e.code, txt


def upload(name, text, **fields):
    boundary = uuid.uuid4().hex
    parts = []
    for k, v in fields.items():
        parts.append(f'--{boundary}\r\nContent-Disposition: form-data; name="{k}"\r\n\r\n{v}\r\n'.encode())
    parts.append(
        f'--{boundary}\r\nContent-Disposition: form-data; name="document"; filename="{name}"\r\n'
        f"Content-Type: application/pdf\r\n\r\n".encode() + pdf(text) + b"\r\n"
    )
    parts.append(f"--{boundary}--\r\n".encode())
    return req("POST", "/api/documents/post_document/", b"".join(parts),
               {"Content-Type": f"multipart/form-data; boundary={boundary}"}, raw=True)


def wait_task(task_id, timeout=90):
    end = time.time() + timeout
    while time.time() < end:
        _, tasks = req("GET", f"/api/tasks/?task_id={task_id}")
        if tasks and tasks[0]["status"] in ("success", "failure"):
            return tasks[0]
        time.sleep(0.4)
    return None


def upload_wait(name, text, **fields):
    st, tid = upload(name, text, **fields)
    return st, (wait_task(tid) if st == 200 else None)


def image_pdf(*pages):
    """PDF made of PIL images; each page is a QR code (str) or a blank page (None)."""
    import io
    import zxingcpp
    from PIL import Image

    imgs = []
    for p in pages:
        page = Image.new("RGB", (612, 792), "white")
        if p:
            import numpy as np

            qr = np.array(zxingcpp.write_barcode(zxingcpp.BarcodeFormat.QRCode, p, width=400, height=400))
            qr_img = Image.fromarray(qr).convert("RGB").resize((400, 400))
            page.paste(qr_img, (100, 200))
        imgs.append(page)
    buf = io.BytesIO()
    imgs[0].save(buf, "PDF", save_all=True, append_images=imgs[1:], resolution=72)
    return buf.getvalue()


def upload_bytes(name, data, **fields):
    boundary = uuid.uuid4().hex
    parts = []
    for k, v in fields.items():
        parts.append(f'--{boundary}\r\nContent-Disposition: form-data; name="{k}"\r\n\r\n{v}\r\n'.encode())
    parts.append(
        f'--{boundary}\r\nContent-Disposition: form-data; name="document"; filename="{name}"\r\n'
        f"Content-Type: application/octet-stream\r\n\r\n".encode() + data + b"\r\n"
    )
    parts.append(f"--{boundary}--\r\n".encode())
    return req("POST", "/api/documents/post_document/", b"".join(parts),
               {"Content-Type": f"multipart/form-data; boundary={boundary}"}, raw=True)


failures = []


def check(label, cond, detail=""):
    print(("PASS " if cond else "FAIL ") + label + ("" if cond else f"  -> {detail}"))
    if not cond:
        failures.append(label)


def main():
    tmp = tempfile.mkdtemp(prefix="filedeck-smoke-")
    env = dict(os.environ)
    env.update({
        "FILEDECK_PORT": str(PORT),
        "FILEDECK_MEDIA_ROOT": os.path.join(tmp, "media"),
        "FILEDECK_CONSUME_DIR": os.path.join(tmp, "consume"),
        "FILEDECK_INDEX_DIR": os.path.join(tmp, "index"),
        "FILEDECK_DATABASE_URL": "sqlite://" + os.path.join(tmp, "db.sqlite3").replace("\\", "/"),
    })
    log_path = os.path.join(tmp, "server.log")
    log = open(log_path, "w")
    srv = subprocess.Popen([EXE], env=env, stdout=log, stderr=subprocess.STDOUT)
    try:
        for _ in range(60):
            try:
                if req("GET", "/api/status/")[0] == 200:
                    break
            except Exception:
                pass
            time.sleep(0.5)
        else:
            print("server did not start")
            return 1

        _, tag = req("POST", "/api/tags/", {"name": "invoice", "color": "#ff0000"})
        _, corr = req("POST", "/api/correspondents/", {"name": "Acme Corp"})
        tag_id, corr_id = tag["id"], corr["id"]

        s1, t1 = upload_wait("a.pdf", "Quarterly invoice from Acme about widgets", title="Widget invoice",
                             correspondent=corr_id, tags=json.dumps([tag_id]), created="2024-03-05")
        s2, t2 = upload_wait("b.pdf", "Holiday photos itinerary Lisbon", title="Lisbon trip", created="2023-07-01")
        check("uploads ingested", s1 == 200 and s2 == 200 and t1 and t2 and t1["status"] == "success"
              and t2["status"] == "success", (t1, t2))

        # ---- Phase 4: pipeline behaviour ----
        st, tid = upload("a-again.pdf", "Quarterly invoice from Acme about widgets", title="dup")
        st_dup = wait_task(tid)
        check("duplicate fails the task", st_dup and st_dup["status"] == "failure"
              and "duplicate" in json.dumps(st_dup["result_data"]).lower(), st_dup)
        st, r = upload_bytes("malware.exe", b"MZ....")
        check("unsupported type rejected (400)", st == 400, (st, r))
        st, r = upload_bytes("empty.pdf", b"")
        check("empty upload rejected (400)", st == 400, (st, r))

        st, doc_a = req("GET", "/api/documents/?title_search=widget")
        check("mime_type stored as pdf", doc_a["results"][0]["mime_type"] == "application/pdf", doc_a)

        _, mtag = req("POST", "/api/tags/", {"name": "auto-gadget", "match": "gadgets", "matching_algorithm": 1})
        _, inbox = req("POST", "/api/tags/", {"name": "Inbox", "is_inbox_tag": True})
        _, mcorr = req("POST", "/api/correspondents/", {"name": "Globex", "match": "Globex Corporation", "matching_algorithm": 3})
        s, t = upload_wait("c.pdf", "Globex Corporation sells gadgets. Invoice date 14.02.2022", title="Gadget bill")
        _, r = req("GET", "/api/documents/?title_search=gadget")
        d = r["results"][0] if r["count"] else {}
        check("matching: tag auto-assigned", mtag["id"] in d.get("tags", []), d)
        check("matching: inbox tag added", inbox["id"] in d.get("tags", []), d)
        check("matching: correspondent auto-assigned", d.get("correspondent") == mcorr["id"], d)
        check("created date parsed from content", d.get("created") == "2022-02-14", d.get("created"))
        req("DELETE", f"/api/documents/{d['id']}/")
        req("DELETE", f"/api/tags/{inbox['id']}/")

        try:
            import zxingcpp  # noqa: F401
            have_zxing = True
        except ImportError:
            have_zxing = False
        if have_zxing:
            st, tid = upload_bytes("asn.pdf", image_pdf("ASN4711"), title="asn doc")
            t = wait_task(tid)
            _, r = req("GET", "/api/documents/?title_search=asn")
            check("ASN barcode sets archive_serial_number",
                  t and t["status"] == "success" and r["results"][0]["archive_serial_number"] == 4711, (t, r))
            st, tid = upload_bytes("asn2.pdf", image_pdf("ASN4711", None), title="asn clash")
            t = wait_task(tid)
            check("clashing ASN fails the task", t and t["status"] == "failure", t)

            before = req("GET", "/api/documents/")[1]["count"]
            st, tid = upload_bytes("batch.pdf", image_pdf("DOC1", "PATCHT", "DOC2", "DOC2B", "PATCHT", "DOC3"))
            t = wait_task(tid)
            after = req("GET", "/api/documents/")[1]["count"]
            check("PATCHT barcode splits into 3 documents", t and t["status"] == "success" and after - before == 3,
                  (t, before, after))
        else:
            print("SKIP barcode tests (run with the worker venv python for zxingcpp)")

        consume_dir = env["FILEDECK_CONSUME_DIR"]
        os.makedirs(consume_dir, exist_ok=True)
        dropped = os.path.join(consume_dir, "dropped.txt")
        with open(dropped, "w") as fh:
            fh.write("Plain text dropped into the consume folder about zeppelins")
        end = time.time() + 30
        found = 0
        while time.time() < end and not found:
            time.sleep(1)
            found = req("GET", "/api/documents/?text=zeppelins")[1]["count"]
        check("consume folder ingests dropped file", found == 1)
        time.sleep(1)
        check("consume folder removes source after success", not os.path.exists(dropped))
        ignored = os.path.join(consume_dir, "~$lock.docx")
        with open(ignored, "w") as fh:
            fh.write("x")
        time.sleep(4)
        check("consume folder ignores lock files", os.path.exists(ignored))
        os.remove(ignored)
        _, r = req("GET", "/api/documents/?text=zeppelins")
        req("DELETE", f"/api/documents/{r['results'][0]['id']}/")
        # leave only the two fixture docs for the remaining assertions
        for q in ("title_search=asn", "title_search=batch", "query=title:batch_01", "query=title:batch_02", "query=title:batch_03"):
            for item in req("GET", f"/api/documents/?{q}")[1]["results"]:
                req("DELETE", f"/api/documents/{item['id']}/")
        _, allr = req("GET", "/api/documents/?page_size=100")
        for item in allr["results"]:
            if item["title"] not in ("Widget invoice", "Lisbon trip"):
                req("DELETE", f"/api/documents/{item['id']}/")

        st, r = req("GET", "/api/documents/")
        check("list returns 2", st == 200 and r["count"] == 2, r)
        check("list has next/previous/all", all(k in r for k in ("next", "previous", "all")), r)

        st, r = req("GET", "/api/documents/?title_search=widget")
        check("title_search finds widget doc", st == 200 and r["count"] == 1 and r["results"][0]["title"] == "Widget invoice", r)
        check("search hit metadata present", "__search_hit__" in (r["results"][0] if r.get("results") else {}), r)

        st, r = req("GET", "/api/documents/?query=" + urllib.parse.quote("title:lisbon"))
        check("advanced query title:lisbon", st == 200 and r["count"] == 1, r)

        st, r = req("GET", "/api/documents/?text=acme")
        check("text search matches correspondent name", st == 200 and r["count"] == 1, r)

        st, r = req("GET", f"/api/documents/?tags__id__all={tag_id}")
        check("tags__id__all", st == 200 and r["count"] == 1, r)
        st, r = req("GET", f"/api/documents/?tags__id__none={tag_id}")
        check("tags__id__none", st == 200 and r["count"] == 1 and r["results"][0]["title"] == "Lisbon trip", r)
        st, r = req("GET", f"/api/documents/?correspondent__id__in={corr_id}")
        check("correspondent__id__in", st == 200 and r["count"] == 1, r)
        st, r = req("GET", "/api/documents/?correspondent__isnull=true")
        check("correspondent__isnull", st == 200 and r["count"] == 1, r)
        st, r = req("GET", "/api/documents/?created__date__gt=2024-01-01")
        check("created__date__gt", st == 200 and r["count"] == 1, r)
        st, r = req("GET", "/api/documents/?created__year=2023")
        check("created__year", st == 200 and r["count"] == 1, r)
        st, r = req("GET", "/api/documents/?title__icontains=TRIP")
        check("title__icontains", st == 200 and r["count"] == 1, r)
        st, r = req("GET", "/api/documents/?ordering=title")
        check("ordering=title", st == 200 and r["results"][0]["title"] == "Lisbon trip", r)
        st, r = req("GET", "/api/documents/?ordering=-created")
        check("ordering=-created", st == 200 and r["results"][0]["title"] == "Widget invoice", r)
        st, r = req("GET", "/api/documents/?page_size=1&page=2")
        check("pagination", st == 200 and len(r["results"]) == 1 and r["count"] == 2 and r["next"] is None and r["previous"], r)
        st, r = req("GET", "/api/documents/?bogus__filter=1")
        check("unknown filter rejected (400)", st == 400, (st, r))

        st, r = req("GET", "/api/search/autocomplete/?term=wid")
        check("autocomplete", st == 200 and "widget" in r, r)
        st, r = req("GET", "/api/search/?query=widget")
        check("global search", st == 200 and r["total"] >= 1 and r["documents"], r)
        st, r = req("GET", "/api/search/?query=ab")
        check("global search min length", st == 400, (st, r))

        doc_id = req("GET", "/api/documents/?title_search=lisbon")[1]["results"][0]["id"]
        st, _ = req("PATCH", f"/api/documents/{doc_id}/", {"title": "Porto journey"})
        st, r = req("GET", "/api/documents/?title_search=porto")
        check("index updated on edit", r["count"] == 1, r)
        st, r = req("GET", "/api/documents/?title_search=lisbon")
        check("old title gone from index", r["count"] == 0, r)

        req("POST", "/api/documents/bulk_edit/", {"documents": [doc_id], "method": "modify_tags",
                                                   "parameters": {"add_tags": [tag_id], "remove_tags": []}})
        st, r = req("GET", "/api/documents/?text=invoice")
        check("tag name searchable after bulk edit", r["count"] == 2, r)

        # ---- Phase 3: document operations ----
        lis = req("GET", "/api/documents/?title_search=lisbon")[1]
        wid = req("GET", "/api/documents/?title_search=widget")[1]
        lis_id = lis["results"][0]["id"] if lis["count"] else doc_id
        wid_id = wid["results"][0]["id"]

        st, r = req("GET", f"/api/documents/{wid_id}/metadata/")
        check("metadata endpoint", st == 200 and r["original_mime_type"] == "application/pdf" and r["original_size"], r)
        st, r = req("GET", "/api/documents/next_asn/")
        check("next_asn", st == 200 and isinstance(r, int), r)
        st, r = req("GET", f"/api/documents/{wid_id}/suggestions/")
        check("suggestions endpoint", st == 200 and "tags" in r and "dates" in r, r)

        before_sum = req("GET", f"/api/documents/{wid_id}/")[1]["checksum"]
        st, r = req("POST", "/api/documents/rotate/", {"documents": [wid_id], "degrees": 90})
        after = req("GET", f"/api/documents/{wid_id}/")[1]
        check("rotate replaces file (checksum changes, text kept)",
              st == 200 and after["checksum"] != before_sum and "widgets" in after["content"], (st, r, after))
        st, r = req("POST", "/api/documents/reprocess/", {"documents": [wid_id]})
        check("reprocess", st == 200, (st, r))
        st, r = req("POST", "/api/documents/bulk_edit/", {"documents": [wid_id], "method": "rotate",
                                                           "parameters": {"degrees": 90}})
        check("bulk_edit rotate", st == 200, (st, r))
        # back to 0 degrees: embedded-text extraction of upside-down pages is reversed without OCR
        req("POST", "/api/documents/rotate/", {"documents": [wid_id], "degrees": 180})

        n0 = req("GET", "/api/documents/")[1]["count"]
        st, r = req("POST", "/api/documents/merge/", {"documents": [wid_id, lis_id]})
        mt = wait_task(r["task_id"]) if st == 200 else None
        check("merge creates a new document", st == 200 and mt and mt["status"] == "success"
              and req("GET", "/api/documents/")[1]["count"] == n0 + 1, (st, r, mt))
        merged = req("GET", "/api/documents/?title_search=merged")[1]
        check("merged doc has text from both sources",
              merged["count"] == 1 and "widgets" in merged["results"][0]["content"]
              and "Lisbon" in merged["results"][0]["content"], merged)
        if merged["count"]:
            mid = merged["results"][0]["id"]
            check("merged doc has 2 pages", merged["results"][0]["page_count"] == 2, merged["results"][0])
            st, r = req("POST", "/api/documents/bulk_edit/", {"documents": [mid], "method": "delete_pages",
                                                               "parameters": {"pages": [2]}})
            d = req("GET", f"/api/documents/{mid}/")[1]
            check("delete_pages removes a page", st == 200 and d["page_count"] == 1 and "Lisbon" not in d["content"], (st, r, d))
            st, r = req("POST", "/api/documents/edit_pdf/", {"documents": [mid], "update_document": True,
                                                              "operations": [{"page": 1, "rotate": 90}]})
            check("edit_pdf update in place", st == 200, (st, r))
            req("DELETE", f"/api/documents/{mid}/")

        try:
            from pypdf import PdfReader, PdfWriter
            import io as _io
            w = PdfWriter()
            for p in PdfReader(_io.BytesIO(pdf("Secret vault contents"))).pages:
                w.add_page(p)
            w.encrypt("hunter2")
            buf = _io.BytesIO()
            w.write(buf)
            enc = buf.getvalue()
            st, tid = upload_bytes("locked.pdf", enc, title="Locked file")
            lt = wait_task(tid)
            lk = req("GET", "/api/documents/?title_search=locked")[1]
            if lk["count"]:
                lid = lk["results"][0]["id"]
                st, r = req("POST", "/api/documents/remove_password/", {"documents": [lid], "password": "nope",
                                                                         "update_document": True})
                check("remove_password rejects wrong password", st == 400, (st, r))
                st, r = req("POST", "/api/documents/remove_password/", {"documents": [lid], "password": "hunter2",
                                                                         "update_document": True})
                d = req("GET", f"/api/documents/{lid}/")[1]
                check("remove_password unlocks and extracts text", st == 200 and "vault" in d["content"], (st, r, d))
                req("DELETE", f"/api/documents/{lid}/")
            else:
                print("SKIP remove_password (encrypted upload not ingested):", lt)
        except ImportError:
            print("SKIP remove_password (pypdf unavailable)")

        # trash
        st, r = req("GET", "/api/trash/")
        trashed_before = r["count"]
        check("trash lists deleted docs", st == 200 and trashed_before >= 1, r)
        tid_ = r["results"][0]["id"]
        st, _ = req("POST", "/api/trash/", {"documents": [tid_], "action": "restore"})
        check("trash restore", st == 200 and req("GET", f"/api/documents/{tid_}/")[0] == 200)
        req("DELETE", f"/api/documents/{tid_}/")
        st, _ = req("POST", "/api/trash/", {"action": "empty"})
        check("trash empty", st == 200 and req("GET", "/api/trash/")[1]["count"] == 0)
        st, r = req("POST", "/api/trash/", {"action": "bogus"})
        check("trash rejects bad action", st == 400, (st, r))

        _, t_a = req("POST", "/api/tags/", {"name": "bulk-a"})
        _, t_b = req("POST", "/api/tags/", {"name": "bulk-b"})
        st, _ = req("POST", "/api/bulk_edit_objects/", {"objects": [t_a["id"], t_b["id"]], "object_type": "tags",
                                                         "operation": "delete"})
        left = [t["name"] for t in req("GET", "/api/tags/")[1]["results"]]
        check("bulk_edit_objects delete", st == 200 and "bulk-a" not in left and "bulk-b" not in left, left)

        # ---- Phase 3: extra CRUD (routes must be registered without clashing) ----
        st, wf = req("POST", "/api/workflows/", {"name": "wf1", "order": 1, "enabled": True})
        if st in (200, 201):
            st, r = req("PATCH", f"/api/workflows/{wf['id']}/", {"enabled": False})
            check("workflow partial PATCH keeps name", st == 200 and r.get("name") == "wf1" and r.get("enabled") is False, (st, r))
        st, r = req("GET", "/api/tasks/run/")
        check("tasks/run is POST-only", st == 405, (st, r))
        st, r = req("POST", "/api/tasks/run/", {"task_name": "nonsense"})
        check("tasks/run rejects unknown task", st == 400, (st, r))

        req("POST", "/api/tags/", {"name": "x"})
        req("PATCH", f"/api/tags/{tag_id}/", {"name": "receipt"})
        for _ in range(40):  # background reindex is asynchronous
            r = req("GET", "/api/documents/?query=tag:receipt")[1]
            if r["count"] == 2:
                break
            time.sleep(0.25)
        st, r = req("GET", "/api/documents/?query=tag:receipt")
        check("taxonomy rename reindexes", r["count"] == 2, r)

        req("DELETE", f"/api/documents/{doc_id}/")
        st, r = req("GET", "/api/documents/?title_search=porto")
        check("deleted doc leaves index", r["count"] == 0, r)

        st, r = req("POST", "/api/search/reindex/")
        check("reindex endpoint", st == 200 and r["indexed"] == 1, (st, r))
    finally:
        srv.terminate()
        try:
            srv.wait(timeout=10)
        except Exception:
            srv.kill()

    log.close()
    if failures:
        print("\n--- server log (tail) ---")
        with open(log_path, errors="replace") as fh:
            print("".join(fh.readlines()[-40:]))
    print(f"\n{len(failures)} failure(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())

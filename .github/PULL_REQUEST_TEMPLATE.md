## 📝 Summary of Changes

A concise description of the problem solved, feature implemented, or optimization made.

### 🔗 Related Issues
- Closes #(issue_number)

---

## 🛠️ Components Touched
- [ ] **Backend** (`backend/`) — Rust core, SQLite migrations, Tantivy search, IDP rules
- [ ] **Worker** (`worker/`) — Python FastAPI sidecar, OCR pipeline, OpenPyXL export
- [ ] **Frontend** (`frontend/`) — React UI, DataEntry grid, PDF Studio, State store
- [ ] **Documentation** (`docs/` / `README.md` / `CHANGELOG.md`)

---

## 🧪 Testing Checklist

- [ ] `cargo check` runs without errors in `backend/`
- [ ] `cargo test` passes cleanly in `backend/`
- [ ] `npx tsc --noEmit` passes with 0 type errors in `frontend/`
- [ ] Python worker imports and endpoints verified
- [ ] Verified manually in the local browser / desktop app

---

## 📸 Screenshots / Demos (If Applicable)
Please attach before/after screenshots or terminal output demonstrating the changes.

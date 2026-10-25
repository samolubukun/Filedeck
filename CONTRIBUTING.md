# Contributing to FileDeck

Thank you for your interest in contributing to **FileDeck**! We welcome contributions from developers of all experience levels. Whether you are fixing a bug, adding a new PDF tool, optimizing search indexing, or improving documentation, your help makes FileDeck better for everyone.

---

## 🏛️ Architecture Overview

FileDeck is organized as a local-first, multi-service workspace:

- **`backend/` (Rust Axum Core)**: High-performance HTTP/REST API, SQLite WAL database management, Tantivy full-text BM25 search index, file watcher, and the IDP validation/export engine.
- **`worker/` (Python 3.12 + FastAPI)**: Background processing daemon handling OCR (RapidOCR / ONNX), PyMuPDF document operations, FastEmbed vector embeddings, and OpenPyXL spreadsheet exports.
- **`frontend/` (React 18 + Vite + TypeScript)**: Responsive desktop web UI, virtualized data entry review grids, PDF viewer drawer, and unified in-app confirmation modals.

---

## 🚀 Quickstart Local Development

### Prerequisites

- **Rust**: 1.80+ (`rustup default stable`)
- **Python**: 3.11 or 3.12 (`python --version`)
- **Node.js**: 18+ or 20+ (`node --version`)
- **Package Managers**: `cargo`, `npm`, and `pip`

### 1. Clone the Repository

```bash
git clone https://github.com/samolubukun/Filedeck.git
cd Filedeck
```

### 2. Setup the Python Worker

```bash
cd worker
python -m venv .venv

# On Windows (PowerShell):
.\.venv\Scripts\Activate.ps1
# On macOS / Linux:
source .venv/bin/activate

pip install -r requirements.txt
uvicorn app.main:app --host 127.0.0.1 --port 8900 --reload
```

### 3. Setup the Frontend

In a separate terminal:

```bash
cd frontend
npm install
npm run dev
# The frontend will run on http://localhost:5173
```

### 4. Run the Backend Core

In a separate terminal:

```bash
cd backend
cargo run
# The backend will start on http://localhost:8080
```

---

## 🧪 Testing Guidelines

Before opening a pull request, ensure all test suites pass cleanly across all three layers:

### Backend (Rust)

```bash
cd backend
cargo check
cargo test
```

### Frontend (TypeScript / React)

```bash
cd frontend
npx tsc --noEmit
npm run build
```

### End-to-End Test Suite

To run the automated IDP and ingestion end-to-end verification suite:

```bash
python backend/tests/test_idp_end_to_end.py
```

---

## 🌿 Git Branching & Commit Conventions

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

- `feat:` A new user-facing feature or API endpoint
- `fix:` A bug fix
- `docs:` Documentation updates or README revisions
- `style:` Formatting, missing semi-colons, etc. (no code logic changes)
- `refactor:` Code restructuring without changing behavior
- `test:` Adding missing tests or correcting existing tests
- `chore:` Tooling, dependency bumps, or build scripts

### Branch Naming

- `feature/short-description` (e.g. `feature/ocr-batch-export`)
- `fix/issue-description` (e.g. `fix/pdf-page-rotation`)
- `docs/section-name` (e.g. `docs/api-guide`)

---

## 📋 Pull Request Workflow

1. Fork the repository and create your branch from `main`.
2. Commit your changes with clear, descriptive commit messages.
3. Ensure all tests and linters pass without errors or warnings.
4. Open a Pull Request against `main`.
5. Fill out the PR template with a summary of changes and testing instructions.
6. A maintainer will review your contribution and provide constructive feedback!

---

## 💬 Community & Help

If you have questions, run into setup issues, or want to discuss a major feature before writing code, please [open an issue](https://github.com/samolubukun/Filedeck/issues) or start a discussion.

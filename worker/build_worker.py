#!/usr/bin/env python3
"""
FileDeck — Python Processing Worker Standalone Freezing Recipe
==============================================================
Freezes the FastAPI Python worker into a self-contained single-file executable
using PyInstaller. The output binary is placed in:
    src-tauri/resources/worker/filedeck-worker[.exe]

Usage:
    python worker/build_worker.py
"""

import os
import sys
import shutil
import subprocess
from pathlib import Path

WORKER_DIR = Path(__file__).resolve().parent
PROJECT_ROOT = WORKER_DIR.parent
TAURI_RESOURCES_DIR = PROJECT_ROOT / "src-tauri" / "resources" / "worker"

def build():
    print("=" * 65)
    print("  FileDeck — Freezing Standalone Python Worker")
    print("=" * 65)

    # Ensure output resource folder exists
    TAURI_RESOURCES_DIR.mkdir(parents=True, exist_ok=True)

    # Check for pyinstaller
    try:
        import PyInstaller
        print(f"[OK] Found PyInstaller {PyInstaller.__version__}")
    except ImportError:
        print("[INFO] PyInstaller not found. Installing via pip...")
        subprocess.check_call([sys.executable, "-m", "pip", "install", "pyinstaller"])

    binary_name = "filedeck-worker.exe" if sys.platform == "win32" else "filedeck-worker"
    entrypoint = WORKER_DIR / "entrypoint.py"

    cmd = [
        sys.executable,
        "-m",
        "PyInstaller",
        "--name", "filedeck-worker",
        "--onefile",
        "--clean",
        "--noconfirm",
        f"--distpath={TAURI_RESOURCES_DIR}",
        f"--workpath={WORKER_DIR / 'build' / 'pyinstaller'}",
        f"--specpath={WORKER_DIR / 'build' / 'spec'}",
        # Hidden imports for FastAPI, uvicorn, and dynamically imported packages
        "--hidden-import=uvicorn.logging",
        "--hidden-import=uvicorn.loops",
        "--hidden-import=uvicorn.loops.auto",
        "--hidden-import=uvicorn.protocols",
        "--hidden-import=uvicorn.protocols.http",
        "--hidden-import=uvicorn.protocols.http.auto",
        "--hidden-import=uvicorn.protocols.websockets",
        "--hidden-import=uvicorn.protocols.websockets.auto",
        "--hidden-import=uvicorn.lifespans",
        "--hidden-import=uvicorn.lifespans.on",
        "--hidden-import=fastapi",
        "--hidden-import=pydantic",
        "--hidden-import=pydantic_settings",
        "--hidden-import=fitz",
        "--hidden-import=pymupdf",
        "--hidden-import=pikepdf",
        "--hidden-import=PIL",
        "--hidden-import=openpyxl",
        "--hidden-import=docx",
        "--hidden-import=pptx",
        "--hidden-import=httpx",
        "--hidden-import=requests",
        "--collect-submodules=app",
        "--collect-all=fitz",
        str(entrypoint),
    ]

    print(f"\n[EXEC] Running PyInstaller build command...")
    result = subprocess.run(cmd, cwd=str(WORKER_DIR))
    if result.returncode != 0:
        print(f"\n[ERROR] PyInstaller failed with code {result.returncode}")
        sys.exit(result.returncode)

    output_binary = TAURI_RESOURCES_DIR / binary_name
    if output_binary.exists():
        size_mb = output_binary.stat().st_size / (1024 * 1024)
        print(f"\n[SUCCESS] Standalone worker built successfully!")
        print(f"          Target : {output_binary}")
        print(f"          Size   : {size_mb:.2f} MB\n")
    else:
        print(f"\n[WARN] Expected output binary not found at {output_binary}")

if __name__ == "__main__":
    build()

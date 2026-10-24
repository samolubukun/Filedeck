"""
FileDeck PDF Tool Engine Environment & Subprocess Management.

Provides cross-platform discovery of system binaries (Ghostscript, LibreOffice,
Tesseract, Chromium/Edge) and fonts without external dependencies.
"""
from __future__ import annotations

import functools
import os
import platform
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Dict, List, Optional, Tuple

IS_MAC = sys.platform == "darwin"
IS_WIN = sys.platform.startswith("win")
IS_LINUX = not IS_MAC and not IS_WIN

# Suppress console window flashing when launching child processes on Windows
CREATE_NO_WINDOW = 0x08000000 if IS_WIN else 0


def run_flags() -> dict:
    """Return platform-specific subprocess creation flags."""
    return {"creationflags": CREATE_NO_WINDOW} if IS_WIN else {}


def _first_existing(paths: List[Optional[str]]) -> Optional[str]:
    """Return the first path that exists on disk."""
    for p in paths:
        if p and Path(p).exists():
            return str(p)
    return None


def _glob_first(pattern_root: str, pattern: str) -> Optional[str]:
    """Find the first matching glob entry in root directory."""
    root = Path(pattern_root)
    if not root.exists():
        return None
    matches = sorted(root.glob(pattern), reverse=True)
    return str(matches[0]) if matches else None


# ----------------------------------------------------------------------
# External Engine Discovery
# ----------------------------------------------------------------------

@functools.lru_cache(maxsize=None)
def ghostscript_binary() -> Optional[str]:
    """Locate Ghostscript binary for PDF compression and PDF/A compilation."""
    if IS_WIN:
        return (
            shutil.which("gswin64c")
            or shutil.which("gswin32c")
            or _glob_first(r"C:\Program Files\gs", r"*\bin\gswin64c.exe")
            or _glob_first(r"C:\Program Files (x86)\gs", r"*\bin\gswin32c.exe")
        )
    return shutil.which("gs") or _first_existing([
        "/opt/homebrew/bin/gs",
        "/usr/local/bin/gs",
        "/usr/bin/gs",
    ])


@functools.lru_cache(maxsize=None)
def libreoffice_binary() -> Optional[str]:
    """Locate LibreOffice binary for Office-to-PDF conversion."""
    if IS_WIN:
        return shutil.which("soffice") or _first_existing([
            r"C:\Program Files\LibreOffice\program\soffice.exe",
            r"C:\Program Files (x86)\LibreOffice\program\soffice.exe",
        ])
    if IS_MAC:
        return _first_existing([
            "/Applications/LibreOffice.app/Contents/MacOS/soffice",
            str(Path.home() / "Applications/LibreOffice.app/Contents/MacOS/soffice"),
        ]) or shutil.which("soffice")
    return shutil.which("soffice") or shutil.which("libreoffice")


@functools.lru_cache(maxsize=None)
def browser_binary() -> Optional[str]:
    """Locate headless Chromium or Edge binary for HTML-to-PDF rendering."""
    if IS_WIN:
        return _first_existing([
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        ]) or shutil.which("chrome") or shutil.which("msedge")
    if IS_MAC:
        return _first_existing([
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
        ])
    return (
        shutil.which("google-chrome")
        or shutil.which("chromium")
        or shutil.which("chromium-browser")
        or shutil.which("microsoft-edge")
    )


@functools.lru_cache(maxsize=None)
def tesseract_binary() -> Optional[str]:
    """Locate Tesseract OCR binary."""
    if IS_WIN:
        return shutil.which("tesseract") or _first_existing([
            r"C:\Program Files\Tesseract-OCR\tesseract.exe",
            r"C:\Program Files (x86)\Tesseract-OCR\tesseract.exe",
        ])
    return shutil.which("tesseract") or _first_existing([
        "/opt/homebrew/bin/tesseract",
        "/usr/local/bin/tesseract",
        "/usr/bin/tesseract",
    ])


def ocrmypdf_cmd() -> List[str]:
    """Return the Python invocation command for OCRmyPDF."""
    return [sys.executable, "-m", "ocrmypdf"]


def subprocess_env() -> Dict[str, str]:
    """Build an execution environment with discovered engine directories prepended to PATH."""
    env = os.environ.copy()
    extra_dirs = [
        str(Path(p).parent)
        for p in (ghostscript_binary(), tesseract_binary(), libreoffice_binary())
        if p
    ]
    if extra_dirs:
        env["PATH"] = os.pathsep.join(extra_dirs + [env.get("PATH", "")])
    return env


def run_command(cmd: List[str], timeout: int = 1800, check: bool = True) -> subprocess.CompletedProcess:
    """Execute an external command safely with unified PATH and no flashing Windows consoles."""
    return subprocess.run(
        cmd,
        capture_output=True,
        check=check,
        timeout=timeout,
        env=subprocess_env(),
        **run_flags(),
    )


def ocr_languages() -> List[str]:
    """Query installed Tesseract language models."""
    exe = tesseract_binary()
    if not exe:
        return []
    try:
        r = subprocess.run(
            [exe, "--list-langs"],
            capture_output=True,
            text=True,
            timeout=20,
            env=subprocess_env(),
            **run_flags(),
        )
        langs = [line.strip() for line in r.stdout.splitlines()[1:] if line.strip()]
        return sorted(langs)
    except Exception:
        return []


# ----------------------------------------------------------------------
# System Font Discovery
# ----------------------------------------------------------------------

FONT_MAP = {
    "win": {
        "arial": {"": "arial.ttf", "bold": "arialbd.ttf", "italic": "ariali.ttf"},
        "times": {"": "times.ttf", "bold": "timesbd.ttf", "italic": "timesi.ttf"},
        "courier": {"": "cour.ttf", "bold": "courbd.ttf", "italic": "couri.ttf"},
        "verdana": {"": "verdana.ttf", "bold": "verdanab.ttf", "italic": "verdanai.ttf"},
        "georgia": {"": "georgia.ttf", "bold": "georgiab.ttf", "italic": "georgiai.ttf"},
    },
    "mac": {
        "arial": {"": "Arial.ttf", "bold": "Arial Bold.ttf", "italic": "Arial Italic.ttf"},
        "times": {"": "Times New Roman.ttf", "bold": "Times New Roman Bold.ttf"},
        "courier": {"": "Courier New.ttf", "bold": "Courier New Bold.ttf"},
    },
    "linux": {
        "arial": {"": "LiberationSans-Regular.ttf", "bold": "LiberationSans-Bold.ttf"},
        "times": {"": "LiberationSerif-Regular.ttf", "bold": "LiberationSerif-Bold.ttf"},
        "courier": {"": "LiberationMono-Regular.ttf", "bold": "LiberationMono-Bold.ttf"},
    },
}


def font_file(family: str = "arial", style: str = "") -> Optional[str]:
    """Locate a matching TrueType font on the host operating system."""
    fam = family.lower()
    sty = style.lower()
    os_key = "win" if IS_WIN else ("mac" if IS_MAC else "linux")

    font_dir = None
    if IS_WIN:
        font_dir = Path(os.environ.get("WINDIR", r"C:\Windows")) / "Fonts"
    elif IS_MAC:
        font_dir = Path("/System/Library/Fonts")
    else:
        font_dir = Path("/usr/share/fonts")

    fam_dict = FONT_MAP.get(os_key, {}).get(fam, {})
    target_name = fam_dict.get(sty) or fam_dict.get("")

    if target_name and font_dir and font_dir.exists():
        direct = font_dir / target_name
        if direct.exists():
            return str(direct)
        for hit in font_dir.rglob(target_name):
            return str(hit)

    return None


def system_diagnostics() -> Dict[str, any]:
    """Return diagnostic availability of all external processing dependencies."""
    return {
        "ghostscript": ghostscript_binary() is not None,
        "libreoffice": libreoffice_binary() is not None,
        "tesseract": tesseract_binary() is not None,
        "browser": browser_binary() is not None,
        "ocr_languages": ocr_languages(),
        "platform": platform.platform(),
        "python": sys.version.split()[0],
    }

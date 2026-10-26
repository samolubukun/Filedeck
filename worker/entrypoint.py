import os
import sys
import multiprocessing

# Required for PyInstaller frozen applications on Windows
multiprocessing.freeze_support()

# Ensure worker root directory is in sys.path
BASE_DIR = os.path.dirname(os.path.abspath(__file__))
if BASE_DIR not in sys.path:
    sys.path.insert(0, BASE_DIR)

import uvicorn
from app.main import app
from app.config import settings

def main():
    host = os.environ.get("FILEDECK_WORKER_HOST", settings.host)
    port = int(os.environ.get("FILEDECK_WORKER_PORT", settings.port))
    print(f"[FileDeck Worker Standalone] Starting on {host}:{port}...")
    uvicorn.run(app, host=host, port=port, log_level="info")

if __name__ == "__main__":
    main()

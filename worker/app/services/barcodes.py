from typing import Any, Dict, List
from PIL import Image

try:
    import zxingcpp
    HAS_ZXING = True
except ImportError:
    HAS_ZXING = False


def scan_barcodes_from_image(image: Image.Image, page_number: int = 1) -> List[Dict[str, Any]]:
    """Scan barcodes from a PIL image using zxing-cpp."""
    if not HAS_ZXING:
        return []

    try:
        # zxing-cpp supports PIL.Image directly
        results = zxingcpp.read_barcodes(image)
        barcodes = []
        for result in results:
            barcodes.append({
                "page": page_number,
                "value": result.text,
                "format": str(result.format).replace("BarcodeFormat.", ""),
            })
        return barcodes
    except Exception as e:
        print(f"Barcode scanning failed on page {page_number}: {e}")
        return []

"""Decode rendered native PDFs with an independent decoder; authoring QA only."""
import hashlib
import importlib.metadata
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / ".tools" / "barcode-qa"))
import zxingcpp
from PIL import Image

folder = Path(sys.argv[1]).resolve(strict=True)
results = []
for prefix, expected in [
    ("etiquetas-numericas", "0012345678901"),
    ("etiquetas-alfanumericas", "CABLE-12-ROJO"),
    ("etiquetas-Windows-GDI", "0012345678901"),
]:
    for page, count in [(1, 16), (2, 1)]:
        path = folder / f"{prefix}-{page}.png"
        image = Image.open(path).convert("RGB")
        # Repeated identical symbols in aligned rows can be merged by a full-sheet
        # detector. Scan each physical label independently, as at the counter.
        scale_x, scale_y = image.width / 210, image.height / 297
        decoded = 0
        for slot in range(count):
            x, y = 12 + (slot % 2) * 95, 12 + (slot // 2) * 34
            region = image.crop((round(x * scale_x), round(y * scale_y),
                                 round((x + 91) * scale_x), round((y + 33) * scale_y)))
            found = zxingcpp.read_barcodes(region, formats=zxingcpp.BarcodeFormat.Code128)
            assert len(found) == 1, f"{path.name}, label {slot + 1}: decoded {len(found)}"
            assert found[0].text == expected, f"{path.name}, label {slot + 1}: data changed"
            decoded += 1
        results.append({
            "file": path.name,
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "decoded": decoded,
            "value": expected,
            "format": "Code128",
        })
report = {
    "decoder": "zxing-cpp",
    "version": importlib.metadata.version("zxing-cpp"),
    "source": "Native Rust PDF and Windows GDI / Microsoft Print to PDF, rendered at 300 dpi",
    "physical_printer_tested": False,
    "method": "Each 91 x 33 mm label is decoded independently; full-sheet detection may merge repeated, aligned symbols.",
    "results": results,
}
with (folder / "barcode-verification.json").open("x", encoding="utf-8") as target:
    json.dump(report, target, ensure_ascii=False, indent=2)
print(json.dumps({"decoded": sum(r["decoded"] for r in results), "pages": len(results), "version": report["version"]}))

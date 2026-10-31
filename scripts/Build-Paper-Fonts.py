"""Build the embedded paper fonts from the pinned, OFL-licensed Manrope source.

Authoring only. Neither Python nor FontTools is needed on the customer's computer.
"""
from pathlib import Path
import hashlib
import json
import shutil
import sys

root = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(root / '.tools/font-authoring'))
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

source = root / '.tools/font-source/Manrope-Variable.ttf'
metadata = json.loads((source.parent / 'source.json').read_text(encoding='utf-8-sig'))
assert hashlib.sha256(source.read_bytes()).hexdigest().upper() == metadata['sha256']
destination = root / 'assets/fonts'
destination.mkdir(parents=True, exist_ok=True)
for weight, style in [(400, 'Regular'), (700, 'Bold')]:
    font = instantiateVariableFont(TTFont(source), {'wght': weight}, inplace=True)
    font.flavor = None
    font['OS/2'].usWeightClass = weight
    font['OS/2'].fsSelection = (font['OS/2'].fsSelection & ~((1 << 0) | (1 << 5) | (1 << 6))) | ((1 << 5) if weight == 700 else (1 << 6))
    font['head'].macStyle = (font['head'].macStyle & ~3) | (1 if weight == 700 else 0)
    font['name'].names = [record for record in font['name'].names if record.nameID not in (1, 2, 3, 4, 6, 16, 17)]
    names = {1: 'Donovan Paper', 2: style, 3: f'DonovanPaper-{style}-1.0', 4: f'Donovan Paper {style}', 6: f'DonovanPaper-{style}', 16: 'Donovan Paper', 17: style}
    for name_id, value in names.items():
        font['name'].setName(value, name_id, 3, 1, 0x409)
        font['name'].setName(value, name_id, 1, 0, 0)
    font.save(destination / f'DonovanPaper-{style}.ttf')
shutil.copyfile(source.parent / 'OFL.txt', destination / 'OFL.txt')
(destination / 'SOURCE.json').write_text(json.dumps({**metadata, 'modifications': 'Static 400 and 700 weight instances, family renamed to Donovan Paper.', 'tool': 'fontTools 4.60.1', 'license': 'SIL Open Font License 1.1'}, indent=2) + '\n', encoding='utf-8')
print('Created embedded Regular and Bold paper fonts with OFL license and provenance.')

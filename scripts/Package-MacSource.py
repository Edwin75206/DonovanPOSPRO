"""Package reproducible Apple Silicon source without local data or credentials."""
from pathlib import Path
import hashlib
import json
import os
import zipfile

root = Path(__file__).resolve().parents[1]
version = json.loads((root / "desktop/package.json").read_text(encoding="utf-8"))["version"]
output = root / "output" / "macos" / "traslado"
output.mkdir(parents=True, exist_ok=True)
archive = output / f"DonovanPOS-{version}-Mac-M4-codigo-y-contexto.zip"
excluded = {"node_modules", "target", "dist", ".runtime", "gen", "test-results", "playwright-report", ".git", "__pycache__"}
files = []
for folder in ("desktop", "assets", "docs/v2", "docs/releases", "packaging"):
    for current, folders, names in os.walk(root / folder):
        folders[:] = [name for name in folders if name not in excluded]
        for name in names:
            file = Path(current) / name
            if file.suffix.lower() in {".secret", ".db", ".sqlite", ".sqlite3", ".donovanbackup", ".p12", ".pfx", ".key", ".pem", ".tsbuildinfo", ".log"} or name.endswith((".db-wal", ".db-shm", ".sqlite-wal", ".sqlite-shm")):
                continue
            if name.startswith(".env") or file.is_symlink():
                continue
            files.append(file)
for name in ("Build-MacOS.command", "Build-MySqlWindows.ps1", "Prepare-MySqlWindows.ps1",
             "Generate-MySqlSchema.py", "Enter-NativeEnvironment.ps1", "Test-NativeInstaller.ps1",
             "Package-MacSource.py", "Check-Repository.py"):
    files.append(root / "scripts" / name)
files += [root / name for name in (".gitignore", "AGENTS.md", "CONTINUAR-EN-MAC.md", "PROMPT-PARA-MAC.md", "README.md", "docs/VERSIONES-Y-GITHUB.md")]
readme = """Donovan POS Pro 2.3.0 · Construrama Chicontla
FUENTES PARA COMPILACIÓN NATIVA EN MAC M4 / APPLE SILICON

Este ZIP NO es el instalador para el cliente. No incluye bases ni contraseñas.
La compilación y el instalador Mac todavía requieren validación en una Mac.
Incluye el código actualizado y CONTINUAR-EN-MAC.md con decisiones y pendientes.
Abre la carpeta DonovanPOS como proyecto en Codex y pide leer ese archivo.
AGENTS.md señala ese contexto para las próximas conversaciones.
No es una exportación del chat ni incluye el historial Git.

1. Extrae la carpeta en tu Mac con macOS 15 o posterior.
2. En la Mac de desarrollo necesitas Xcode Command Line Tools, Rust y Node.
3. Desde la carpeta extraída ejecuta:
   bash scripts/Build-MacOS.command --verify
4. El script descarga versiones oficiales fijadas por SHA-256, ejecuta pruebas
   y crea un paquete .pkg para validación local. Consulta docs/v2/MYSQL-Y-MACOS.md.
5. Para distribuir: firma Developer ID y notarización, modo --release, y pruebas
   del instalador, impresión, Keychain y restauración en dos instalaciones.

El cliente final recibe el .pkg completo: no instala herramientas de desarrollo.
MySQL Server almacena el negocio; Workbench permite administrarlo.
La operación comercial es local y no necesita internet.
"""
with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=6) as z:
    z.writestr("DonovanPOS/LEEME-MAC.txt", readme)
    manifest = []
    for file in sorted(set(files)):
        relative = file.relative_to(root)
        archive_name = str(Path("DonovanPOS") / relative).replace("\\", "/")
        z.write(file, archive_name)
        manifest.append(f"{hashlib.sha256(file.read_bytes()).hexdigest()}  {relative.as_posix()}")
    z.writestr("DonovanPOS/MANIFIESTO-SHA256.txt", "\n".join(manifest) + "\n")
with zipfile.ZipFile(archive) as z:
    assert z.testzip() is None
    for line in z.read("DonovanPOS/MANIFIESTO-SHA256.txt").decode().splitlines():
        expected, relative = line.split("  ", 1)
        assert hashlib.sha256(z.read("DonovanPOS/" + relative)).hexdigest() == expected
with archive.open("rb") as stream:
    digest = hashlib.file_digest(stream, "sha256").hexdigest()
archive.with_suffix(".sha256.txt").write_text(f"{digest}  {archive.name}\n", encoding="ascii")
print(f"{archive}\nSHA256 {digest}\n{archive.stat().st_size} bytes")

"""Collect notices from the resolved local packages, without downloading code."""
import json
import hashlib
import shutil
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
metadata = json.loads((ROOT / "artifacts/v2/cargo-license-metadata.json").read_text(encoding="utf-8-sig"))
packages = {p["id"]: p for p in metadata["packages"]}
nodes = {n["id"]: n for n in metadata["resolve"]["nodes"]}
pending, included = [metadata["resolve"]["root"]], set()
while pending:
    identifier = pending.pop()
    if identifier in included:
        continue
    included.add(identifier)
    for dependency in nodes[identifier]["deps"]:
        if any(k["kind"] != "dev" for k in dependency["dep_kinds"]):
            pending.append(dependency["pkg"])

entries, sections, missing = [], [], []
supplements=json.loads((ROOT/"assets/licenses/supplements/sources.json").read_text(encoding="utf-8-sig"))


def add(ecosystem, name, version, license_id, repository, directory):
    sources = []
    for file in directory.rglob("*"):
        relative = file.relative_to(directory)
        if len(relative.parts) > 4 or "node_modules" in relative.parts or not file.is_file():
            continue
        if (file.name.lower().startswith(("license", "licence", "copying", "copyright", "notice")) or file.name=="OFL.txt") and file.suffix.lower() not in (".rs", ".js", ".ts", ".py", ".json"):
            if file.stat().st_size > 1_000_000:
                raise ValueError(f"Review unexpectedly large license: {name}/{relative}")
            sources.append((str(relative).replace("\\", "/"), file.read_text(encoding="utf-8", errors="replace")))
    for supplement in supplements:
        if (supplement["ecosystem"],supplement["name"],supplement["version"])==(ecosystem,name,version):
            sources.append((supplement["url"],(ROOT/"assets/licenses/supplements"/supplement["file"]).read_text(encoding="utf-8")))
    entry = {"ecosystem": ecosystem, "name": name, "version": version, "license": license_id, "repository": repository, "notices": [n for n, _ in sources]}
    entries.append(entry)
    if not sources:
        missing.append(f"{ecosystem}:{name}@{version} ({license_id})")
    parts = [f"{ecosystem}: {name} {version}", f"License metadata: {license_id}", f"Source: {repository or ''}"]
    for filename, content in sorted(sources):
        parts.extend([f"\n--- {filename} ---\n", content])
    sections.append("\n".join(parts))


for identifier in sorted(included):
    p = packages[identifier]
    if p["source"] is None:
        continue
    add("Rust", p["name"], p["version"], p["license"], p["repository"], Path(p["manifest_path"]).parent)

lock = json.loads((ROOT / "desktop/package-lock.json").read_text(encoding="utf-8"))
for location, entry in sorted(lock["packages"].items()):
    if not location or entry.get("dev"):
        continue
    directory = ROOT / "desktop" / location
    if not directory.exists():
        continue  # Platform-specific optional package not installed on Windows.
    p = json.loads((directory / "package.json").read_text(encoding="utf-8"))
    repository = p.get("repository", p.get("homepage", ""))
    if isinstance(repository, dict):
        repository = repository.get("url", "")
    add("npm", p.get("name", location), p["version"], p.get("license", entry.get("license")), repository, directory)

add("Font", "DonovanPaper (derivative of Manrope)", "1", "OFL-1.1", "https://github.com/sharanda/manrope", ROOT / "assets/fonts")
folder = ROOT / "assets/licenses"
folder.mkdir(parents=True, exist_ok=True)
lock_packages=tomllib.loads((ROOT/"desktop/src-tauri/Cargo.lock").read_text(encoding="utf-8"))["package"]
source_archives=[]
for identifier in sorted(included):
    p=packages[identifier]
    if "MPL-2.0" not in (p["license"] or ""):
        continue
    package_root=Path(p["manifest_path"]).parent
    archive_name=f"{p['name']}-{p['version']}.crate"
    archive=package_root.parent.parent.parent/"cache"/package_root.parent.name/archive_name
    expected=next(item["checksum"] for item in lock_packages if item["name"]==p["name"] and item["version"]==p["version"])
    assert hashlib.sha256(archive.read_bytes()).hexdigest()==expected, f"Source checksum mismatch: {archive_name}"
    (folder/"sources").mkdir(exist_ok=True)
    shutil.copyfile(archive,folder/"sources"/archive_name)
    source_archives.append(archive_name)
header = "Donovan POS Pro — Third-party notices\n\nCollected from locally resolved package versions. Includes native build dependencies as well as application dependencies. These packages are not separate client installation requirements. Original copyright and license text follows.\n\nOriginal source archives for MPL-2.0 components are included in the sources subfolder, under their original licenses: "+", ".join(source_archives)+". These .crate files are gzip-compressed tar archives.\n"
(folder / "THIRD_PARTY_NOTICES.txt").write_text(header + "\n\n" + ("\n\n" + "=" * 80 + "\n\n").join(sections), encoding="utf-8")
(folder / "dependencies.json").write_text(json.dumps(entries, ensure_ascii=False, indent=2), encoding="utf-8")
(ROOT / "artifacts/v2/licenses-needing-review.json").write_text(json.dumps(missing, ensure_ascii=False, indent=2), encoding="utf-8")
print(json.dumps({"packages": len(entries), "missing_notices": missing, "notice_bytes": (folder / "THIRD_PARTY_NOTICES.txt").stat().st_size}))

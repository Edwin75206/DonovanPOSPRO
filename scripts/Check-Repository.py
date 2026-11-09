"""Audit versioned files and optional history without printing secret values.

This checks the Git index, so run git add for intended files before executing it.
It is a publication guard, not a guarantee that every possible secret is detected.
"""
from pathlib import Path, PurePosixPath
import argparse
import hashlib
import json
import re
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
FORBIDDEN_DIRS = {"node_modules", "target", "dist", ".runtime", ".tools", "artifacts",
                  "output", "tmp", "research", "test-results", "playwright-report",
                  "__pycache__", ".codex", "bin", "obj"}
FORBIDDEN_SUFFIXES = {".db", ".sqlite", ".sqlite3", ".donovanbackup", ".secret", ".pem",
                      ".key", ".p12", ".pfx", ".keystore", ".mobileprovision", ".log",
                      ".exe", ".dll", ".pdb", ".msi", ".msix", ".pkg", ".dmg", ".apk",
                      ".zip", ".7z", ".pyc", ".tsbuildinfo"}
SECRET_PATTERNS = {
    "GitHub token": re.compile(rb"(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{60,})"),
    "OpenAI key": re.compile(rb"sk-(?:proj-|svcacct-)[A-Za-z0-9_-]{40,}"),
    "AWS access key": re.compile(rb"(?:AKIA|ASIA)[A-Z0-9]{16}"),
    "private key": re.compile(rb"(?m)^-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----"),
    "database URL with password": re.compile(rb"(?:mysql|postgres(?:ql)?)://[^\s:/]+:[^\s@]{6,}@"),
}


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT)


def forbidden(path):
    p = PurePosixPath(path)
    name = p.name.lower()
    if any(part.lower() in FORBIDDEN_DIRS for part in p.parts):
        return "generated or local directory"
    if p.suffix.lower() in FORBIDDEN_SUFFIXES or name.endswith((".db-wal", ".db-shm", ".sqlite-wal", ".sqlite-shm")):
        return "data, secret, installer or generated file"
    if (name == ".env" or name.startswith(".env.")) and name not in {".env.example", ".env.template"}:
        return "environment secrets file"
    if name in {"auth.json", "credentials.json", "instance.json", ".instance.lock", ".ds_store", "thumbs.db"}:
        return "machine-specific file"
    return None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--history", action="store_true", help="also audit all reachable Git objects")
    options = parser.parse_args()
    entries = []
    for entry in git("ls-files", "--stage", "-z").split(b"\0"):
        if not entry:
            continue
        meta, path = entry.split(b"\t", 1)
        _, oid, stage = meta.split()
        if stage != b"0":
            raise SystemExit("Resolve merge conflicts before publishing")
        entries.append((oid.decode(), path.decode()))
    tracked_count = len(entries)
    if options.history:
        for line in git("rev-list", "--objects", "--all").decode().splitlines():
            if " " in line:
                entries.append(tuple(line.split(" ", 1)))
    issues = set()
    seen = set()
    process = subprocess.Popen(["git", "cat-file", "--batch"], cwd=ROOT, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    try:
        for oid, path in entries:
            if (oid, path) in seen:
                continue
            seen.add((oid, path))
            process.stdin.write((oid + "\n").encode())
            process.stdin.flush()
            header = process.stdout.readline().split()
            if len(header) != 3:
                raise RuntimeError("Cannot inspect a Git object")
            kind, size = header[1], int(header[2])
            data = process.stdout.read(size)
            process.stdout.read(1)
            if kind != b"blob":
                continue
            reason = forbidden(path)
            if reason:
                issues.add((path, reason))
            if size > 10 * 1024 * 1024:
                issues.add((path, "file larger than 10 MiB; use Releases"))
            if b"\0" not in data:
                for label, pattern in SECRET_PATTERNS.items():
                    if pattern.search(data):
                        issues.add((path, label))
    finally:
        process.stdin.close()
        process.stdout.close()
        process.wait()
    # Versions are checked against the index, not unstaged local changes.
    def doc(path):
        return git("show", ":" + path).decode("utf-8-sig")
    versions = {
        "package.json": json.loads(doc("desktop/package.json"))["version"],
        "package-lock.json": json.loads(doc("desktop/package-lock.json"))["packages"][""]["version"],
        "Cargo.toml": tomllib.loads(doc("desktop/src-tauri/Cargo.toml"))["package"]["version"],
        "tauri.conf.json": json.loads(doc("desktop/src-tauri/tauri.conf.json"))["version"],
    }
    cargo_lock = tomllib.loads(doc("desktop/src-tauri/Cargo.lock"))
    versions["Cargo.lock"] = next(p["version"] for p in cargo_lock["package"] if p["name"] == "donovan-pos")
    if len(set(versions.values())) != 1:
        issues.add(("versions", json.dumps(versions)))
    for path, expected in json.loads(doc("packaging/migration-checksums.json")).items():
        if hashlib.sha256(git("show", ":" + path)).hexdigest() != expected:
            issues.add((path, "published migration bytes changed, including line endings"))
        if hashlib.sha256((ROOT / path).read_bytes()).hexdigest() != expected:
            issues.add((path, "checkout differs from published migration checksum"))
    if issues:
        for path, reason in sorted(issues):
            print(f"BLOCK: {path}: {reason}")
        return 1
    print(f"OK: {tracked_count} tracked files; versions {next(iter(versions.values()))}; "
          f"{'history and index' if options.history else 'index'} audited. No blocked files or recognized secrets.")
    return 0


if __name__ == "__main__":
    sys.exit(main())

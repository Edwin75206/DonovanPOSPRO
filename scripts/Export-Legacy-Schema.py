"""Capture the version-1 schema, never its business data, as a native migration baseline."""
import pathlib
import sqlite3

root = pathlib.Path(__file__).resolve().parents[1]
source = root / 'artifacts/install-test-data/donovan.db'
db = sqlite3.connect(source.as_uri() + '?mode=ro', uri=True)
rows = db.execute("SELECT name,sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY CASE type WHEN 'table' THEN 0 WHEN 'index' THEN 1 ELSE 2 END,rowid").fetchall()
parts = ['-- Version 1.0.0 schema baseline. No business or user records.']
for name, sql in rows:
    if name.startswith('sqlite_') or name.startswith('ProductSearch_'):
        continue
    parts.append(sql.rstrip(';') + ';')
for migration, version in db.execute('SELECT MigrationId,ProductVersion FROM __EFMigrationsHistory'):
    parts.append(f"INSERT INTO __EFMigrationsHistory VALUES ('{migration}','{version}');")
destination = root / 'desktop/src-tauri/migrations/001_legacy.sql'
destination.parent.mkdir(parents=True, exist_ok=True)
destination.write_text('\n\n'.join(parts) + '\n', encoding='utf-8')
print(f'Exported schema: {len(parts)-1} statements, no business records.')

"""Developer-only generator. Committed SQL/contract are bundled, Python is not required by customers."""
import json, re, sqlite3
from pathlib import Path

root = Path(__file__).resolve().parents[1] / 'desktop/src-tauri/migrations'
db = sqlite3.connect(':memory:')
for file in sorted(root.glob('00*.sql')):
    db.executescript(file.read_text(encoding='utf-8-sig'))

def quote(name):
    assert re.fullmatch(r'[A-Za-z_][A-Za-z_0-9]*', name)
    return '`' + name + '`'

def checks(sql):
    result = []
    for match in re.finditer(r'\bCHECK\s*\(', sql, re.I):
        start = match.end(); end = start; depth = 1; quoted = False
        while depth:
            char = sql[end]; end += 1
            if char == "'": quoted = not quoted
            if not quoted:
                depth += (char == '(') - (char == ')')
        result.append(re.sub(r'\blength\(', 'CHAR_LENGTH(', sql[start:end-1], flags=re.I).replace('"', '`'))
    return result

tables = {}
for name, sql in db.execute("SELECT name,sql FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE 'ProductSearch%' ORDER BY name"):
    columns = list(db.execute(f'PRAGMA table_info({quote(name)})'))
    fks = list(db.execute(f'PRAGMA foreign_key_list({quote(name)})'))
    indexes = []
    indexed = {c[1] for c in columns if c[5]}
    for _, idx, unique, origin, partial in db.execute(f'PRAGMA index_list({quote(name)})'):
        if partial: continue
        cols = [x[2] for x in db.execute(f'PRAGMA index_info({quote(idx)})')]
        indexed.update(cols)
        if origin != 'pk': indexes.append({'name':idx,'unique':bool(unique),'columns':cols})
    out = []; spec = []
    for _, col, typ, notnull, default, pk in columns:
        kind = {'INTEGER':'BIGINT','BLOB':'LONGBLOB','TEXT':'VARCHAR(512)' if col in indexed else 'LONGTEXT'}[typ]
        auto = bool(pk and re.search(rf'"{re.escape(col)}"[^,]+AUTOINCREMENT', sql, re.I))
        # All SQLite primary keys have a logical non-null identity in this product.
        nullable = not (notnull or pk)
        line = f'  {quote(col)} {kind}' + (' NULL' if nullable else ' NOT NULL')
        if auto: line += ' AUTO_INCREMENT'
        if default is not None: line += f' DEFAULT ({default})'
        out.append(line)
        spec.append({'name':col,'type':kind.lower(),'nullable':nullable,'primary':bool(pk),'auto':auto})
    pks = [c[1] for c in sorted(columns, key=lambda c:c[5]) if c[5]]
    if pks: out.append('  PRIMARY KEY (' + ','.join(map(quote,pks)) + ')')
    for fk in fks:
        out.append(f'  CONSTRAINT {quote("FK_"+name+"_"+fk[3])} FOREIGN KEY ({quote(fk[3])}) REFERENCES {quote(fk[2])} ({quote(fk[4])}) ON DELETE {fk[6]} ON UPDATE {fk[5]}')
    for i, expr in enumerate(checks(sql)):
        out.append(f'  CONSTRAINT {quote("CK_"+name+"_"+str(i+1))} CHECK ({expr})')
    for idx in indexes:
        out.append('  ' + ('UNIQUE ' if idx['unique'] else '') + f'KEY {quote(idx["name"])} (' + ','.join(map(quote,idx['columns'])) + ')')
    if name == 'CashSession':
        out.append('  UNIQUE KEY `IX_CashSession_OneOpen` ((CASE WHEN `ClosedAt` IS NULL THEN 1 ELSE NULL END))')
    tables[name] = {'name':name,'columns':spec,'foreignKeys':[{'table':f[2],'from':f[3],'to':f[4],'delete':f[6],'update':f[5]} for f in fks], 'indexes':indexes,'checks':checks(sql),'sql':'CREATE TABLE '+quote(name)+' (\n'+',\n'.join(out)+'\n) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;'}

ordered = []
while tables:
    ready = [n for n,t in tables.items() if all(f['table'] not in tables for f in t['foreignKeys'])]
    if not ready: raise RuntimeError('Cyclic foreign keys')
    for n in ready: ordered.append(tables.pop(n))
target = root / 'mysql'; target.mkdir(exist_ok=True)
# Version 001 shipped with CRLF; its exact bytes are an installed schema checksum.
# Future schema changes require a new migration, not reformatting the existing one.
(target/'001_mysql.sql').write_text('-- MySQL 8.4 / InnoDB. Business schema equivalent to native SQLite version 5.\n-- Never apply to an existing or foreign schema.\n\n'+'\n\n'.join(t['sql'] for t in ordered)+'\n',encoding='utf-8',newline='\r\n')
(target/'contract.json').write_text(json.dumps([{k:v for k,v in t.items() if k!='sql'} for t in ordered],ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(f'Generated {len(ordered)} tables, explicit foreign keys, checks and indexes.')

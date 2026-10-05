import json
import sqlite3
from pathlib import Path

root = Path(__file__).resolve().parent.parent
directory = root / '.fixtures'
directory.mkdir(exist_ok=True)
database = directory / 'demo.sqlite'
if database.exists():
    raise SystemExit('Fixture already exists; remove only .fixtures/demo.sqlite to regenerate.')
connection = sqlite3.connect(database)
connection.executescript('''
CREATE TABLE customers(id INTEGER PRIMARY KEY, name TEXT NOT NULL, email TEXT, region TEXT);
CREATE TABLE orders(id INTEGER PRIMARY KEY, customer_id INTEGER REFERENCES customers(id), status TEXT, total REAL, created_at TEXT, metadata TEXT);
CREATE VIEW paid_orders AS SELECT * FROM orders WHERE status = 'paid';
''')
connection.executemany('INSERT INTO customers VALUES (?, ?, ?, ?)', [
    (i, f'Demo customer {i:03}', f'customer{i}@example.test', ['NZ', 'AU', 'UK'][i % 3])
    for i in range(1, 61)
])
connection.executemany('INSERT INTO orders VALUES (?, ?, ?, ?, ?, ?)', [
    (i, (i % 60) + 1, ['paid', 'pending', 'refunded'][i % 3], round(i * 12.75, 2),
     f'2026-09-{(i % 28) + 1:02}', json.dumps({'source': 'synthetic', 'itemCount': i % 5 + 1}))
    for i in range(1, 351)
])
connection.commit()
connection.close()
config = [{
    'id': 'demo-sqlite', 'name': 'Cellar demo · SQLite', 'engine': 'sqlite',
    'host': '', 'port': 0, 'database': str(database), 'user': '', 'ssl_mode': 'disable',
    'env_tag': 'local', 'application_name': None, 'color': '#a78bfa'
}]
(directory / 'connections.json').write_text(json.dumps(config, indent=2))
print('Created disposable SQLite fixture with 60 customers and 350 orders.')
print(f'CELLAR_EXTENSION_CONFIG={directory / "connections.json"}')

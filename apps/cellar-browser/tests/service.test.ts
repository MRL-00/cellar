import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { dispatch } from '../server/tools.js';
import type { Result } from '../ui/types.js';

process.env.CELLAR_EXTENSION_CONFIG ??= fileURLToPath(new URL('../.fixtures/connections.json', import.meta.url));
const data = async (name: string, args: Record<string, unknown>) => await dispatch(name, args) as unknown as Result;

test('aliases disclose no file paths, hosts or credentials', async () => {
  const aliases = await dispatch('cellar_open', {});
  const text = JSON.stringify(aliases);
  assert.ok(text.includes('demo-sqlite'));
  assert.ok(!/database|password|host|\.sqlite|\/Users\//.test(text));
});
test('SQLite schema, bound pages, sorting, filters and quoted SQL work', async () => {
  const schema = await dispatch('cellar_schema', { connection: 'demo-sqlite' });
  assert.equal((schema.tables as unknown[]).length, 3);
  const base = { connection: 'demo-sqlite', schema: 'main', table: 'orders', sort: 'id' };
  const first = await data('cellar_browse', base);
  assert.equal(first.rows.length, 100);
  assert.equal(first.rows[0][0], 1);
  assert.equal(first.truncated, true);
  const second = await data('cellar_browse', { ...base, offset: 100 });
  assert.equal(second.rows[0][0], 101);
  const last = await data('cellar_browse', { ...base, offset: 300 });
  assert.equal(last.rows.length, 50);
  assert.equal(last.truncated, false);
  const filtered = await data('cellar_browse', { ...base, filters: [{ column: 'status', operator: 'equals', value: 'paid' }], descending: true });
  assert.ok(filtered.rows.every(row => row[2] === 'paid'));
  const injection = await data('cellar_browse', { ...base, filters: [{ column: 'status', operator: 'contains', value: "%' OR 1=1 --" }] });
  assert.equal(injection.rows.length, 0);
  const cte = await data('cellar_query', { connection: 'demo-sqlite', sql: "WITH x AS (SELECT ';' AS value) SELECT value FROM x" });
  assert.equal(cte.rows[0][0], ';');
  const empty = await data('cellar_query', { connection: 'demo-sqlite', sql: 'SELECT id, status FROM orders WHERE id < 0' });
  assert.equal(empty.rows.length, 0);
  assert.equal(empty.columns.length, 2);
});
test('writes and unsupported operations leave the SQLite fixture byte-identical', async () => {
  const file = new URL('../.fixtures/demo.sqlite', import.meta.url);
  const hash = async () => createHash('sha256').update(await readFile(file)).digest('hex');
  const before = await hash();
  for (const sql of ['DELETE FROM orders', 'SELECT 1; DROP TABLE orders', "SELECT load_extension('/tmp/a')", "ATTACH '/tmp/a' AS x", 'PRAGMA query_only = OFF', 'BEGIN', 'SELECT * INTO TEMP x FROM orders']) {
    await assert.rejects(dispatch('cellar_query', { connection: 'demo-sqlite', sql }));
  }
  assert.equal(await hash(), before);
  await assert.rejects(dispatch('cellar_query', { connection: 'demo-sqlite', sql: 'SELECT 1', password: 'forbidden' }), /Invalid tool/);
  await assert.rejects(dispatch('cellar_browse', { connection: 'demo-sqlite', schema: 'main', table: 'orders', sort: 'id; DELETE FROM orders' }), /Unknown sort/);
});
test('row and byte caps remain honest', async () => {
  const result = await data('cellar_query', { connection: 'demo-sqlite', sql: `SELECT '${'x'.repeat(16300)}' AS payload FROM orders`, limit: 500 });
  assert.ok(result.rows.length < 100);
  assert.equal(result.truncated, true);
  assert.ok(Buffer.byteLength(JSON.stringify(result)) < 530000);
  await assert.rejects(dispatch('cellar_query', { connection: 'demo-sqlite', sql: 'SELECT 1', limit: 501 }));
});
test('cancellation stops a running SQLite computation', async () => {
  const requestId = crypto.randomUUID();
  const promise = dispatch('cellar_query', { connection: 'demo-sqlite', requestId, sql: 'WITH RECURSIVE x(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM x WHERE n<1000000000) SELECT sum(n) FROM x' });
  const rejection = assert.rejects(promise, /cancelled/);
  await new Promise(resolve => setTimeout(resolve, 150));
  assert.equal((await dispatch('cellar_cancel', { requestId })).cancelled, true);
  await rejection;
});
test('SQLite expensive computation times out', async () => {
  const started = Date.now();
  await assert.rejects(dispatch('cellar_query', { connection: 'demo-sqlite', sql: 'WITH RECURSIVE x(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM x WHERE n<1000000000) SELECT sum(n) FROM x' }), /timed out/);
  assert.ok(Date.now() - started < 6500);
});
test('Postgres real driver schema, pages, numeric/JSON types and read-only guard', { skip: process.env.CELLAR_TEST_POSTGRES !== '1' }, async () => {
  const schema = await dispatch('cellar_schema', { connection: 'demo-postgres' });
  assert.equal((schema.tables as unknown[]).length, 3);
  const page = await data('cellar_browse', { connection: 'demo-postgres', schema: 'public', table: 'orders', sort: 'id', offset: 100, filters: [{ column: 'status', operator: 'equals', value: 'paid' }] });
  assert.equal(page.rows.length, 16);
  assert.equal(page.rows[0][2], 'paid');
  const result = await data('cellar_query', { connection: 'demo-postgres', sql: 'SELECT count(*) AS count, sum(total) AS total FROM orders' });
  assert.equal(result.rows[0][0], 350);
  for (const sql of ['DELETE FROM orders', 'WITH x AS (DELETE FROM orders RETURNING *) SELECT * FROM x', "SELECT nextval('foo')", 'SELECT * INTO TEMP copied FROM orders', 'SELECT * FROM orders FOR UPDATE', 'COMMIT', 'SELECT 1; SET transaction_read_only=off', "SELECT pg_read_file('/etc/passwd')", 'SELECT 1 AS x, 2 AS x']) {
    await assert.rejects(dispatch('cellar_query', { connection: 'demo-postgres', sql }));
  }
  const unchanged = await data('cellar_query', { connection: 'demo-postgres', sql: 'SELECT count(*) AS count FROM orders' });
  assert.equal(unchanged.rows[0][0], 350);
  const started = Date.now();
  await assert.rejects(dispatch('cellar_query', { connection: 'demo-postgres', sql: 'SELECT count(*) AS count FROM orders a CROSS JOIN orders b CROSS JOIN orders c CROSS JOIN orders d' }), /interrupted|timed out/);
  assert.ok(Date.now() - started < 5500);
});

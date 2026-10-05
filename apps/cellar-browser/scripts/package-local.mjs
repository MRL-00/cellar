import { cp, mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const source = fileURLToPath(new URL('..', import.meta.url));
const destination = resolve(process.argv[2] ?? '../../../cellar-local-marketplace');
const plugin = resolve(destination, 'plugins/cellar-browser');
try { await mkdir(destination); }
catch (error) {
  if (error.code !== 'EEXIST' || process.argv[3] !== '--refresh') throw error;
  const existing = JSON.parse(await readFile(resolve(destination, '.agents/plugins/marketplace.json'), 'utf8'));
  if (existing.name !== 'cellar-local-dev') throw new Error('Refusing to overwrite an unrelated marketplace');
}
await mkdir(plugin, { recursive: true });
for (const entry of ['.codex-plugin', '.mcp.json', 'dist', 'bin', 'skills', 'assets']) {
  await cp(resolve(source, entry), resolve(plugin, entry), { recursive: true });
}
await mkdir(resolve(plugin, 'scripts'), { recursive: true });
await cp(resolve(source, 'scripts/start.sh'), resolve(plugin, 'scripts/start.sh'));
await mkdir(resolve(plugin, '.fixtures'), { recursive: true });
await cp(resolve(source, '.fixtures/demo.sqlite'), resolve(plugin, '.fixtures/demo.sqlite'));
const configs = JSON.parse(await readFile(resolve(source, '.fixtures/connections.json'), 'utf8'));
const sqlite = configs.find(config => config.id === 'demo-sqlite');
sqlite.database = resolve(plugin, '.fixtures/demo.sqlite');
await writeFile(resolve(plugin, '.fixtures/connections.json'), JSON.stringify([sqlite], null, 2));
await mkdir(resolve(destination, '.agents/plugins'), { recursive: true });
await writeFile(resolve(destination, '.agents/plugins/marketplace.json'), JSON.stringify({
  name: 'cellar-local-dev', interface: { displayName: 'Cellar local development' },
  plugins: [{ name: 'cellar-browser', source: { source: 'local', path: './plugins/cellar-browser' }, policy: { installation: 'AVAILABLE', authentication: 'ON_INSTALL' }, category: 'Developer Tools' }],
}, null, 2));
console.log(destination);

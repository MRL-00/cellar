import assert from 'node:assert/strict';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';

test('packaged MCP configuration launches with plugin-relative cwd and Codex Node runtime', async () => {
  const root = fileURLToPath(new URL('..', import.meta.url));
  const config = JSON.parse(await readFile(resolve(root, '.mcp.json'), 'utf8')).mcpServers['cellar-browser'];
  assert.equal(config.cwd, '.');
  assert.ok(!JSON.stringify(config).includes('${PLUGIN_ROOT}'));
  const transport = new StdioClientTransport({
    command: config.command, args: config.args, cwd: resolve(root, config.cwd),
    env: { PATH: '/usr/bin:/bin', CODEX_MCP_NODE_PATH: process.execPath,CELLAR_EXTENSION_STATE:resolve(root,'.fixtures/mcp-test-state') },
  });
  const client = new Client({ name: 'cellar-installed-wiring-test', version: '1.0.0' });
  try {
    await client.connect(transport);
    assert.equal((await client.listTools()).tools.length, 17);
    const opened = await client.callTool({ name: 'cellar_open', arguments: {} });
    assert.equal(opened.isError, undefined);
  } finally { await client.close(); }
});

test('stdio MCP advertises sidebar/panel entrypoints and private bounded results', async () => {
  const transport = new StdioClientTransport({ command: process.execPath, args: ['--import', 'tsx', fileURLToPath(new URL('../server/mcp.ts', import.meta.url))], cwd: fileURLToPath(new URL('..', import.meta.url)), env: { PATH: process.env.PATH!, CELLAR_EXTENSION_CONFIG: process.env.CELLAR_EXTENSION_CONFIG ?? fileURLToPath(new URL('../.fixtures/connections.json', import.meta.url)) } });
  const client = new Client({ name: 'cellar-contract-test', version: '1.0.0' });
  try {
    await client.connect(transport);
    const tools = await client.listTools();
    assert.equal(tools.tools.length, 17);
    const open = tools.tools.find(tool => tool.name === 'cellar_open')!;
    assert.equal(open.annotations?.readOnlyHint, true);
    assert.deepEqual(open._meta?.['openai/ui'], { entrypoints: [{ type: 'global' }, { type: 'thread' }] });
    assert.equal(tools.tools.find(tool=>tool.name==='cellar_commit')?.annotations?.readOnlyHint,false);
    assert.equal(tools.tools.find(tool=>tool.name==='cellar_commit')?.annotations?.destructiveHint,true);
    assert.deepEqual(tools.tools.find(tool=>tool.name==='cellar_connection')?._meta?.ui,{visibility:['app']});
    assert.deepEqual(tools.tools.find(tool=>tool.name==='cellar_project_connections')?._meta?.ui,{visibility:['app']});
    assert.ok(tools.tools.find(tool=>tool.name==='cellar_project')?.description?.includes('Never infer'));
    assert.deepEqual(tools.tools.find(tool=>tool.name==='cellar_file')?._meta?.['openai/ui'],{entrypoints:[{type:'file',extensions:['.sqlite','.sqlite3','.db']}]});
    const resource = await client.readResource({ uri: 'ui://cellar-browser/v5.html' });
    assert.equal(resource.contents[0].mimeType, 'text/html;profile=mcp-app');
    assert.ok('text' in resource.contents[0] && resource.contents[0].text.includes('Cellar'));
    const result = await client.callTool({ name: 'cellar_browse', arguments: { connection: 'demo-sqlite', schema: 'main', table: 'customers', limit: 5 } });
    assert.equal(result.isError, undefined);
    assert.ok(!JSON.stringify(result.content).includes('customer1@example.test'));
    const metadata = result._meta as { data: { rows: unknown[] } };
    assert.equal(metadata.data.rows.length, 5);
    const rejected = await client.callTool({ name: 'cellar_query', arguments: { connection: 'demo-sqlite', sql: 'DELETE FROM customers' } });
    assert.equal(rejected.isError, true);
  } finally { await client.close(); }
});

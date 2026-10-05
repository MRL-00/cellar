import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import {readFile} from 'node:fs/promises';
import {resolve} from 'node:path';

const proxy = process.argv.includes('--proxy');
const child = spawn('codex', ['app-server', ...(proxy ? ['proxy'] : ['--stdio'])], { stdio: ['pipe', 'pipe', 'pipe'] });
const waiting = new Map();
let next = 1;
const timer = setTimeout(() => { console.error('Codex host check timed out'); child.kill(); process.exitCode = 1; }, 45000);
const lines = createInterface({ input: child.stdout });
createInterface({ input: child.stderr }).on('line', line => {
  if (/cellar-browser/.test(line) && /ERROR|failed|invalid/i.test(line)) console.error(line);
});
lines.on('line', line => {
  try {
    const message = JSON.parse(line);
    if (message.id !== undefined && waiting.has(message.id)) {
      const pending = waiting.get(message.id);
      waiting.delete(message.id);
      if (message.error) pending.reject(new Error(JSON.stringify(message.error)));
      else pending.resolve(message.result);
    }
  } catch { return; }
});
child.on('error', error => { console.error(error.message); });
child.on('close', () => { for (const pending of waiting.values()) pending.reject(new Error('Codex app-server proxy closed')); });
function request(method, params) {
  const id = next++;
  return new Promise((resolve, reject) => {
    waiting.set(id, { resolve, reject });
    child.stdin.write(JSON.stringify({ id, method, params }) + '\n');
  });
}
try {
  await request('initialize', { clientInfo: { name: 'cellar-host-check', title: 'Cellar host check', version: '0.5.2' }, capabilities: { experimentalApi: true, requestAttestation: false } });
  child.stdin.write(JSON.stringify({ method: 'initialized' }) + '\n');
  if (proxy) {
    const reload = await request('config/mcpServer/reload', {});
    console.log(JSON.stringify({ reloaded: reload }));
  } else {
    const started = await request('thread/start', { cwd: process.cwd(), ephemeral: true });
    const threadId = started.thread.id;
    const result = await request('mcpServerStatus/list', { threadId, detail: 'full' });
    const cellar = result.data.find(server => server.name === 'cellar-browser');
    console.log(JSON.stringify({ cellar: cellar && { name: cellar.name, runtimeStatus: cellar.runtimeStatus, pluginId: cellar.pluginId, tools: Object.keys(cellar.tools), toolsError: cellar.toolsError, resources: cellar.resources } }));
    if (!cellar?.tools?.cellar_open || cellar.toolsError) throw new Error('Codex did not discover cellar_open');
    const opened = await request('mcpServer/tool/call', { threadId, server: 'cellar-browser', tool: 'cellar_open', arguments: {} });
    console.log(JSON.stringify({ opened }));
    if (opened.isError || opened.result?.isError) throw new Error('Codex cellar_open failed');
    const resource = await request('mcpServer/resource/read', { threadId, server: 'cellar-browser', uri: 'ui://cellar-browser/v5.html' });
    const html = resource.contents[0];
    if (html.mimeType !== 'text/html;profile=mcp-app' || !html.text?.includes('Cellar')) throw new Error('Codex UI resource unavailable');
    console.log(JSON.stringify({ resource: { mimeType: html.mimeType, bytes: Buffer.byteLength(html.text) } }));
    const browsed = await request('mcpServer/tool/call', { threadId, server: 'cellar-browser', tool: 'cellar_browse', arguments: { connection: 'demo-sqlite', schema: 'main', table: 'customers', limit: 3 } });
    if (browsed.isError || browsed._meta?.data?.rows?.length !== 3) throw new Error('Codex synthetic table read failed');
    console.log(JSON.stringify({ browse: { rowCount: browsed._meta.data.rows.length } }));
    if(process.argv.includes('--attachment')){
      const path=resolve('.fixtures/demo.sqlite');
      const original=await readFile(path);
      const attached=await request('mcpServer/tool/call',{threadId,server:'cellar-browser',tool:'cellar_attach',arguments:{path,name:'Synthetic host verification.sqlite'}});
      if(attached.isError||!attached._meta?.data?.importedCopy||!attached._meta?.data?.preferredConnection)throw new Error('Codex synthetic attachment open failed');
      const imported=await request('mcpServer/tool/call',{threadId,server:'cellar-browser',tool:'cellar_browse',arguments:{connection:attached._meta.data.preferredConnection,schema:'main',table:'customers',limit:3}});
      if(imported.isError||imported._meta?.data?.rows?.length!==3)throw new Error('Codex imported attachment read failed');
      if(!(await readFile(path)).equals(original))throw new Error('Synthetic source attachment was modified');
      console.log(JSON.stringify({attachment:{privateCopy:true,rowCount:3,originalPreserved:true},visualHostVerification:false}));
    }
  }
} catch (error) { console.error(error.message); process.exitCode = 1; }
finally { clearTimeout(timer); child.kill(); lines.close(); }

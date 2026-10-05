import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { randomBytes } from 'node:crypto';
import { dispatch } from './tools.js';

const port = Number(process.env.CELLAR_PREVIEW_PORT ?? 4317);
process.env.CELLAR_EXTENSION_CONFIG ??= new URL('../.fixtures/connections.json',import.meta.url).pathname;
process.env.CELLAR_EXTENSION_STATE ??= new URL('../.fixtures/private-state',import.meta.url).pathname;
const token = randomBytes(32).toString('hex');
const origin = `http://127.0.0.1:${port}`;
const server = createServer(async (request, response) => {
  response.setHeader('Cache-Control', 'no-store');
  response.setHeader('X-Content-Type-Options', 'nosniff');
  response.setHeader('Content-Security-Policy', "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; font-src data:; connect-src 'self'; img-src data:; frame-ancestors 'none'; base-uri 'none'; form-action 'none'");
  if (request.headers.host !== `127.0.0.1:${port}`) { response.writeHead(403).end(); return; }
  if (request.method === 'GET' && request.url === '/') {
    try {
      const html = await readFile(new URL('../dist/index.html', import.meta.url), 'utf8');
      response.setHeader('Content-Type', 'text/html');
      response.end(html.replace('<head>', `<head><meta name="cellar-preview" content="${token}">`));
    } catch { response.writeHead(503).end('Run npm run build first'); }
    return;
  }
  if (request.method !== 'POST' || request.url !== '/api/tool' || request.headers.origin !== origin
      || request.headers['x-cellar-token'] !== token || request.headers['content-type'] !== 'application/json') {
    response.writeHead(403).end(); return;
  }
  const abort = new AbortController();
  response.on('close', () => { if (!response.writableEnded) abort.abort(); });
  let body = '';
  try {
    for await (const chunk of request) {
      body += chunk.toString();
      if (Buffer.byteLength(body) > 65536) throw new Error('Request too large');
    }
    const { name, args } = JSON.parse(body);
    const data = await dispatch(name, args, abort.signal);
    response.setHeader('Content-Type', 'application/json');
    response.end(JSON.stringify(data));
  } catch (error) {
    response.writeHead(400, { 'Content-Type': 'application/json' }).end(JSON.stringify({ error: error instanceof Error ? error.message : 'Request failed' }));
  }
});
server.requestTimeout = 10000;
server.listen(port, '127.0.0.1', () => console.error(`Cellar local preview: ${origin}`));

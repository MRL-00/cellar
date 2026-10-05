import { readFile } from 'node:fs/promises';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { registerAppResource, registerAppTool, RESOURCE_MIME_TYPE } from '@modelcontextprotocol/ext-apps/server';
import { dispatch, schemas, type ToolName } from './tools.js';

const uri = 'ui://cellar-browser/v5.html';
export function createServer() {
  const server = new McpServer({ name: 'cellar-browser', version: '0.5.2' });
  registerAppResource(server, 'Cellar database workspace', uri, {}, async () => ({ contents: [{
    uri, mimeType: RESOURCE_MIME_TYPE,
    text: await readFile(new URL('../dist/index.html', import.meta.url), 'utf8'),
    _meta: { ui: { prefersBorder: false, csp: { connectDomains: [], resourceDomains: [] } } },
  }] }));
  const descriptions: Record<ToolName, string> = {
    cellar_project:'Prepare Connect from this project using ONLY the active task project root explicitly supplied in the current Codex environment context or selected by the user. Never infer it from the plugin/server cwd, search directories, read config or send credentials. This selects a folder only; the workspace asks the user before discovery or connection. If the project root is unavailable, ask the user to choose it.',
    cellar_project_connections:'Review and explicitly confirm bounded project connection discovery or a session-only read-only connection. UI-only. Credentials stay inside the local server.',
    cellar_open: 'Open the Cellar database workspace to connect, browse, query and review row edits. Lists aliases without paths, hosts or credentials.',
    cellar_schema: 'Read a bounded schema for a configured SQLite or Postgres alias. Metadata is delivered privately to the workspace UI.',
    cellar_browse: 'Browse a bounded read-only table page with column filters and sorting. Rows are delivered to the UI, not model context.',
    cellar_query: 'Execute one read-only SELECT or WITH query against a configured alias. Functions are restricted. Results go to the UI, not model context. Do not include secrets in SQL.',
    cellar_cancel: 'Cancel an in-flight workspace query by its caller-generated request ID.',
    cellar_connection: 'Save or remove a user-created database profile. Passwords go to the OS keychain. This tool is for the workspace UI only.',
    cellar_preview: 'Build the transactional SQL review for staged row inserts, updates and deletes without executing changes.',
    cellar_commit: 'Commit an explicitly confirmed, reviewed row change plan atomically with optimistic conflict checks. Only enabled editable connections allow writes.',
    cellar_attach: 'Open a user-selected SQLite database from this conversation. For an attachment, materialize the exact user-selected file locally first and pass its path. Never search unrelated files. Creates a private editable copy; the original stays unchanged.',
    cellar_file: 'Open a SQLite database file in the Cellar workspace.',
    cellar_upload: 'Import a user-selected SQLite file in bounded chunks to a private editable working copy. UI-only.',
    cellar_export: 'Download a consistent snapshot of a selected SQLite database in bounded chunks. UI-only.',
    cellar_sql: 'Select the SQL statement at the cursor or preflight a bounded batch using Cellar statement parsing. Does not execute SQL.',
    cellar_details: 'Read bounded index, foreign key, default and view-definition metadata for the workspace UI.',
    cellar_plan: 'Explain a read-only query without ANALYZE; does not execute the SELECT query.',
  };
  for (const name of Object.keys(schemas) as ToolName[]) {
    const config = {
      title: name === 'cellar_open' ? 'Cellar' : name.replace('cellar_', 'Cellar '),
      description: descriptions[name], inputSchema: schemas[name].shape,
      annotations: { readOnlyHint: !['cellar_project','cellar_project_connections','cellar_connection','cellar_commit','cellar_attach','cellar_upload'].includes(name), destructiveHint: name === 'cellar_commit', idempotentHint: !['cellar_project','cellar_project_connections','cellar_cancel','cellar_connection','cellar_commit','cellar_attach','cellar_upload'].includes(name), openWorldHint: name === 'cellar_connection'||name==='cellar_project_connections' },
      _meta: name === 'cellar_file' ? {
        ui:{resourceUri:uri},'openai/ui':{entrypoints:[{type:'file',extensions:['.sqlite','.sqlite3','.db']}]},
      } : name === 'cellar_open' ? {
        ui: { resourceUri: uri }, 'openai/ui': { entrypoints: [{ type: 'global' }, { type: 'thread' }] },
      } : name === 'cellar_attach'||name==='cellar_project' ? {ui:{resourceUri:uri}} : { ui: { visibility: ['app'] } },
    };
    const handler = async (args: Record<string, unknown>, extra: { signal: AbortSignal; _meta?: Record<string,unknown> }) => {
      try {
        const resource = extra._meta?.['openai/resource'] as {path?:string} | undefined;
        const owner=typeof extra._meta?.threadId==='string'?extra._meta.threadId:'host-session';
        const data = await dispatch(name, args, extra.signal,resource?.path,owner);
        return {
          content: [{ type: 'text' as const, text: name === 'cellar_open' ? 'Cellar workspace opened.' : 'Cellar workspace operation completed.' }],
          structuredContent: { readOnly: !data.committed, rowCount: Array.isArray(data.rows) ? data.rows.length : 0, truncated: data.truncated ?? false },
          _meta: { data },
        };
      } catch (error) {
        return { isError: true, content: [{ type: 'text' as const, text: error instanceof Error ? error.message : 'Operation failed' }] };
      }
    };
    if (name === 'cellar_open' || name === 'cellar_file' || name === 'cellar_attach'||name==='cellar_project') registerAppTool(server, name, config, handler);
    else server.registerTool(name, config, handler);
  }
  return server;
}
if (process.argv[1]?.endsWith('/mcp.ts') || process.argv[1]?.endsWith('/mcp.mjs')) await createServer().connect(new StdioServerTransport());

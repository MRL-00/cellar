import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { z } from 'zod';
import {projectAliases,projectContext,projectOperation,selectProject,sessionProfile} from './project-connections.js';

const alias = z.string().min(1).max(80);
const requestId = z.string().uuid();
const change = z.object({ kind: z.enum(['update','insert','delete']), original: z.array(z.union([z.string(),z.number(),z.boolean(),z.null()])).max(128).optional(), revision: z.string().length(64).optional(), values: z.record(z.string().max(16384).nullable()) }).strict();
const changeArguments = { connection: alias, schema: z.string().min(1).max(256), table: z.string().min(1).max(256), changes: z.array(change).min(1).max(50) };
const profile = z.object({ id:z.string().default(''), name:z.string().min(1).max(80), engine:z.enum(['sqlite','postgres','supabase','neon']), host:z.string().max(255).default(''), port:z.number().int().min(0).max(65535).default(5432), database:z.string().min(1).max(4096), user:z.string().max(255).default(''), ssl_mode:z.enum(['disable','prefer','require','verify-ca','verify-full']).default('verify-full'), ssl_ca_pem:z.string().max(16384).optional(),env_tag:z.enum(['local','dev','staging','prod']).default('dev'), application_name:z.string().nullable().default(null), color:z.string().nullable().default(null), allow_edits:z.boolean().default(false) }).strict();
export const schemas = {
  cellar_project:z.object({root:z.string().min(1).max(4096)}).strict(),
  cellar_project_connections:z.object({stage:z.enum(['status','scope','discover','connect','disconnect']),root:z.string().max(4096).optional(),scope:z.string().uuid().optional(),candidate:z.string().uuid().optional(),confirmation:z.string().max(80).optional(),productionAcknowledged:z.boolean().optional()}).strict(),
  cellar_open: z.object({}).strict(),
  cellar_schema: z.object({ connection: alias }).strict(),
  cellar_browse: z.object({
    connection: alias, schema: z.string().min(1).max(256), table: z.string().min(1).max(256),
    filters: z.array(z.object({ column: z.string().max(256), operator: z.enum(['equals', 'notEquals', 'contains', 'notContains','startsWith','endsWith','like','greaterThan','greaterThanOrEqual','lessThan','lessThanOrEqual','isNull', 'isNotNull']), value: z.string().max(1024).optional() }).strict()).max(12).default([]),
    sort: z.string().max(256).optional(), descending: z.boolean().default(false),
    offset: z.number().int().min(0).max(100000).default(0), limit: z.number().int().min(1).max(500).default(100),
    requestId: requestId.optional(),
  }).strict(),
  cellar_query: z.object({ connection: alias, sql: z.string().min(1).max(32768), limit: z.number().int().min(1).max(500).default(100), requestId: requestId.optional() }).strict(),
  cellar_cancel: z.object({ requestId }).strict(),
  cellar_connection: z.object({operation:z.enum(['save','remove','details']), connection:alias.optional(),profile:profile.optional(),password:z.string().max(4096).optional()}).strict(),
  cellar_preview: z.object(changeArguments).strict(),
  cellar_commit: z.object({...changeArguments,preview_id:z.string().length(64),confirmation:z.string().max(80)}).strict(),
  cellar_attach: z.object({path:z.string().max(4096).optional(),name:z.string().max(255).optional()}).strict(),
  cellar_file: z.object({file:z.object({name:z.string(),resourceUri:z.string()})}).strict(),
  cellar_upload: z.object({stage:z.enum(['begin','chunk','finish','cancel']),id:z.string().uuid().optional(),name:z.string().max(80).optional(),chunk:z.string().max(44000).optional()}).strict(),
  cellar_export: z.object({connection:alias,stage:z.enum(['prepare','chunk','finish']),id:z.string().uuid().optional(),offset:z.number().int().min(0).max(67108864).optional()}).strict(),
  cellar_sql:z.object({connection:alias,sql:z.string().max(32768),operation:z.enum(['statement','all']),cursor:z.number().int().min(0).max(131072).optional()}).strict(),
  cellar_details:z.object({connection:alias,schema:z.string().min(1).max(256),table:z.string().min(1).max(256)}).strict(),
  cellar_plan:z.object({connection:alias,sql:z.string().min(1).max(32768)}).strict(),
};
export type ToolName = keyof typeof schemas;
export type Data = Record<string, unknown>;
const active = new Map<string, () => void>();
const binary = fileURLToPath(new URL('../bin/cellar-extension-service', import.meta.url));

export async function dispatch(name: string, raw: unknown, signal?: AbortSignal, resourcePath?: string,owner='preview'): Promise<Data> {
  if (!(name in schemas)) throw new Error('Unknown tool');
  const tool = name as ToolName;
  const parsed = schemas[tool].safeParse(raw);
  if (!parsed.success) throw new Error('Invalid tool arguments');
  const args = parsed.data as Data;
  if(tool==='cellar_export'&&typeof args.connection==='string'&&args.connection.startsWith('project-'))throw new Error('Project connections are session-only and read-only; database download is unavailable');
  if(tool==='cellar_project')return {...await serviceRequest('connections',{},signal),...await selectProject(owner,args.root as string),projectRequested:true};
  if(tool==='cellar_project_connections'){
    const data=await projectOperation(owner,args as {stage:string;root?:string;scope?:string;candidate?:string;confirmation?:string;productionAcknowledged?:boolean});
    if(args.stage==='connect'||args.stage==='disconnect'){
      const existing=await serviceRequest('connections',{},signal);
      return {...data,connections:[...(existing.connections as unknown[]),...projectAliases(owner)]};
    }
    return data;
  }
  if (tool === 'cellar_file') return { file:args.file };
  if (tool === 'cellar_attach') {
    const { attach } = await import('./uploads.js');
    return attach((resourcePath ?? args.path) as string | undefined, args.name as string | undefined, signal);
  }
  if (tool === 'cellar_upload') {
    const { upload } = await import('./uploads.js');
    return upload(args as {stage:string;id?:string;name?:string;chunk?:string},signal);
  }
  if (tool === 'cellar_cancel') {
    const cancel = active.get(args.requestId as string);
    cancel?.();
    return { cancelled: Boolean(cancel) };
  }
  if (tool === 'cellar_export') {
    const { exportDatabase } = await import('./uploads.js');
    return exportDatabase(args as {connection:string;stage:string;id?:string;offset?:number},signal);
  }
  const action = { cellar_open: 'connections', cellar_schema: 'schema', cellar_browse: 'browse', cellar_query: 'query',cellar_connection:'connection',cellar_preview:'preview',cellar_commit:'commit',cellar_sql:'sql',cellar_details:'details',cellar_plan:'plan' }[tool];
  const session=typeof args.connection==='string'?await sessionProfile(owner,args.connection):undefined;
  if(session&&!['schema','browse','query','sql','details','plan'].includes(action))throw new Error('Project connections are session-only and read-only');
  const data=await serviceRequest(action,{...args,...session},signal);
  if(tool==='cellar_open')return {...data,connections:[...(data.connections as unknown[]),...projectAliases(owner)],projectScope:projectContext(owner)};
  return data;
}

export function serviceRequest(action:string,args:Data,signal?:AbortSignal):Promise<Data> {
  if (active.size >= 4) throw new Error('Four requests are already running; retry after one completes');
  const id = (args.requestId as string | undefined) ?? crypto.randomUUID();
  if (active.has(id)) throw new Error('Request ID is already running');
  delete args.requestId;
  return new Promise((resolve, reject) => {
    const child = spawn(binary, [], { stdio: ['pipe', 'pipe', 'ignore'], env: { PATH: process.env.PATH, HOME:process.env.HOME, CELLAR_EXTENSION_CONFIG: process.env.CELLAR_EXTENSION_CONFIG, CELLAR_EXTENSION_STATE:process.env.CELLAR_EXTENSION_STATE } });
    let output = '';
    let finished = false;
    const finish = (error?: Error, data?: Data) => {
      if (finished) return;
      finished = true;
      clearTimeout(timer);
      active.delete(id);
      signal?.removeEventListener('abort', cancel);
      child.kill('SIGKILL');
      if (error) reject(error); else resolve(data!);
    };
    const cancel = () => finish(new Error('Query cancelled'));
    const timer = setTimeout(() => finish(new Error('Request timed out after 6 seconds')), 6000);
    active.set(id, cancel);
    signal?.addEventListener('abort', cancel, { once: true });
    if (signal?.aborted) { cancel(); return; }
    child.on('error', () => finish(new Error('Build the Rust service before starting the extension')));
    child.stdout.on('data', (chunk: Buffer) => {
      output += chunk.toString();
      if (Buffer.byteLength(output) > 1_048_576) finish(new Error('Response exceeds the 1 MiB transport limit'));
    });
    child.stdin.on('error', () => finish(new Error('Database service unavailable')));
    child.on('close', (code) => {
      if (finished) return;
      if (code !== 0) { finish(new Error('Database service failed')); return; }
      try {
        const result = JSON.parse(output) as Data;
        if (typeof result.error === 'string') finish(new Error(result.error));
        else finish(undefined, result);
      } catch { finish(new Error('Invalid database service response')); }
    });
    child.stdin.end(JSON.stringify({ action, ...args }));
  });
}

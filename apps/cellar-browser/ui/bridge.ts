import { App } from '@modelcontextprotocol/ext-apps';

const previewToken = document.querySelector<HTMLMetaElement>('meta[name="cellar-preview"]')?.content;
const app = new App({ name: 'Cellar', version: '0.5.2' }, {});
let connected: Promise<void> | undefined;
const fileListeners = new Set<(file:{name:string;resourceUri:string})=>void>();
const projectListeners=new Set<()=>void>();
let openedFile: {name:string;resourceUri:string} | undefined;
let initialData:Record<string,unknown>|undefined;
app.ontoolresult=result=>{if(result._meta?.data){initialData=result._meta.data as Record<string,unknown>;if(initialData.projectRequested)for(const listener of projectListeners)listener();}};
export function onProject(listener:()=>void){projectListeners.add(listener);return ()=>{projectListeners.delete(listener);};}
export async function initialise<T>():Promise<T> {
  if(previewToken)return call<T>('cellar_open',{});
  connected??=app.connect();await connected;
  return (initialData?.connections?initialData:await call<T>('cellar_open',{})) as T;
}
app.ontoolinput = input => {
  const file = input.arguments?.file as {name?:string;resourceUri?:string} | undefined;
  if (file?.name && file.resourceUri) {
    openedFile = {name:file.name,resourceUri:file.resourceUri};
    for (const listener of fileListeners) listener(openedFile);
  }
};
export function onFile(listener:(file:{name:string;resourceUri:string})=>void) {
  fileListeners.add(listener);
  if (openedFile) listener(openedFile);
  return ()=> { fileListeners.delete(listener); };
}
export async function call<T>(name: string, args: Record<string, unknown>): Promise<T> {
  if (previewToken) {
    const response = await fetch('/api/tool', { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Cellar-Token': previewToken }, body: JSON.stringify({ name, args }) });
    const data = await response.json();
    if (!response.ok || data.error) throw new Error(data.error ?? 'Operation failed');
    return data as T;
  }
  connected ??= app.connect();
  await connected;
  const result = await app.callServerTool({ name, arguments: args });
  if (result.isError) throw new Error(result.content?.find(item => item.type === 'text')?.text ?? 'Operation failed');
  return result._meta?.data as T;
}
export async function fullscreen() {
  if (!previewToken) await app.requestDisplayMode({ mode: 'fullscreen' });
}
export function hostIntegrationAvailable(){return !previewToken&&Boolean(app.getHostCapabilities()?.message?.text);}
export async function requestProjectContext(){
 if(previewToken)throw new Error('Choose a project folder in the preview');
 connected??=app.connect();await connected;
 if(!hostIntegrationAvailable())throw new Error('This host does not support requests to the Codex chat. Choose a project folder in the workspace.');
 const result=await app.sendMessage({role:'user',content:[{type:'text',text:'Prepare Connect from this project in Cellar. Use ONLY the active task project root supplied in the current environment/task context or explicitly selected by me. Call cellar_project with that absolute folder path. Do not infer it from the plugin/server cwd, scan directories, read .env/config files, send connection strings, passwords or usernames to tools/chat, or connect to a database. The workspace will ask me to confirm discovery and a read-only connection. If the active project folder is unavailable or ambiguous, ask me to choose it.'}]});
 if(result.isError)throw new Error('Codex declined the project-context request. Choose the project folder in the workspace.');
}
export async function findChatDatabases() {
  if(previewToken)throw new Error('Chat attachment discovery is available inside Codex. Use Add connection to import a file in this preview.');
  connected??=app.connect();await connected;
  if(!hostIntegrationAvailable())throw new Error('This host does not support requests to the Codex chat. Ask Codex to open the attached database directly.');
  const result=await app.sendMessage({role:'user',content:[{type:'text',text:'Find the SQLite database files attached or explicitly referenced in this current chat. Use only this chat’s authorized files; do not scan unrelated files, sessions or stored connections. Materialize the selected SQLite file locally using the available file/Library tools and open it with cellar_attach. If multiple databases are present, let me choose. Do not extract or configure credentials from chat.'}]});
  if(result.isError)throw new Error('Codex declined the attachment request. Ask Codex to open the attached database directly.');
}
app.onhostcontextchanged = context => {
  if (context.theme) document.documentElement.dataset.theme = context.theme;
};

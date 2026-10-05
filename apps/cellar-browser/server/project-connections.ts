import {createHash,randomUUID} from 'node:crypto';
import {basename,join,relative,resolve} from 'node:path';
import {checkedPath,projectRoot,readProjectFile,sqliteFiles,sqliteHeader} from './project-files.js';
import {parseAppsettings,parseEnv,type Definition} from './project-formats.js';

const TTL=15*60*1000;
const configs=['.env','.env.local','.env.development','.env.test','.env.staging','.env.production','appsettings.json','appsettings.Development.json','appsettings.Staging.json','appsettings.Production.json'];
type Profile={id:string;name:string;engine:'sqlite'|'postgres';database:string;host:string;port:number;user:string;ssl_mode:string;env_tag:string;application_name:null;color:null;allow_edits:false;managed:true;has_password:false};
type Candidate={id:string;engine:'sqlite'|'postgres';source:string;target:'Local file'|'Loopback PostgreSQL'|'Remote PostgreSQL';production:boolean;ready:boolean};
type PrivateCandidate={public:Candidate;profile:Profile;password?:string;sourcePath?:string;digest?:string};
type Scope={id:string;root:string;expires:number;candidates:Map<string,PrivateCandidate>;active:Map<string,PrivateCandidate>};
const scopes=new Map<string,Scope>();
setInterval(()=>{for(const [owner,scope] of scopes)if(scope.expires<=Date.now())scopes.delete(owner);},30000).unref();
function current(owner:string){const scope=scopes.get(owner);if(scope&&scope.expires>Date.now())return scope;if(scope)scopes.delete(owner);}
function expose(scope:Scope){return {id:scope.id,name:basename(scope.root),path:scope.root,expiresAt:new Date(scope.expires).toISOString()};}
function aliases(scope:Scope){return [...scope.active.values()].map(({profile})=>({id:profile.id,name:profile.name,engine:profile.engine,environment:profile.env_tag,readOnly:true,managed:false,project:true}));}
export function projectAliases(owner:string){const scope=current(owner);return scope?aliases(scope):[];}
export function projectContext(owner:string){const scope=current(owner);return scope?expose(scope):undefined;}
export async function selectProject(owner:string,raw:string){
 const root=await projectRoot(raw);
 if(scopes.size>=16&&!scopes.has(owner))throw new Error('Too many project sessions are open');
 const scope:Scope={id:randomUUID(),root,expires:Date.now()+TTL,candidates:new Map(),active:new Map()};
 scopes.set(owner,scope);return {projectScope:expose(scope)};
}
async function validate(scope:Scope,candidate:PrivateCandidate){
 await checkedPath(scope.root,scope.root,true);
 if(candidate.sourcePath){const bytes=await readProjectFile(scope.root,candidate.sourcePath);if(createHash('sha256').update(bytes).digest('hex')!==candidate.digest)throw new Error('Configuration changed; discover connections again');}
 if(candidate.profile.engine==='sqlite'&&!await sqliteHeader(scope.root,candidate.profile.database))throw new Error('Select an existing SQLite file inside this project');
}
export async function projectOperation(owner:string,args:{stage:string;root?:string;scope?:string;candidate?:string;confirmation?:string;productionAcknowledged?:boolean}){
 if(args.stage==='scope'){if(!args.root)throw new Error('Choose the project folder');return selectProject(owner,args.root);}
 const scope=current(owner);
 if(args.stage==='status')return {projectScope:scope?expose(scope):undefined};
 if(!scope||args.scope!==scope.id)throw new Error('Select this project again; the local session expired');
 if(args.stage==='disconnect'){scope.active.clear();scope.candidates.clear();return {projectConnections:[]};}
 if(args.stage==='connect'){
  if(args.confirmation!=='CONNECT READ ONLY')throw new Error('Confirm the selected read-only connection');
  const selected=scope.candidates.get(args.candidate??'');
  if(!selected?.public.ready)throw new Error('Choose an available project connection');
  if(selected.public.production&&!args.productionAcknowledged)throw new Error('Acknowledge the production-like target before connecting');
  try{await validate(scope,selected);}catch{throw new Error('Project files changed or became unavailable; discover again');}
  scope.active.set(selected.profile.id,selected);return {projectConnections:aliases(scope),preferredConnection:selected.profile.id};
 }
 if(args.stage!=='discover'||args.confirmation!=='DISCOVER PROJECT CONNECTIONS')throw new Error('Confirm reading configuration in the selected project');
 const checkedScope:Scope=scope;
 scope.candidates.clear();const warnings:string[]=[];
 async function add(definition:Definition,source:string,sourcePath?:string,digest?:string){
  if(checkedScope.candidates.size>=32)return;
  const id=randomUUID();let ready=true;
  const profile:Profile={id:'project-'+id,name:`Project · ${definition.engine==='sqlite'?'SQLite':'PostgreSQL'} · ${source}`,engine:definition.engine,database:definition.database,host:definition.host??'',port:definition.port??5432,user:definition.user??'',ssl_mode:definition.ssl_mode??'disable',env_tag:definition.production?'prod':'dev',application_name:null,color:null,allow_edits:false,managed:true,has_password:false};
  if(definition.engine==='sqlite'){
   profile.database=resolve(checkedScope.root,definition.database);
   try{ready=!definition.database.split(/[\\/]/).includes('..')&&await sqliteHeader(checkedScope.root,profile.database);}catch{ready=false;}
  }else if(!['disable','prefer','require','verify-ca','verify-full'].includes(profile.ssl_mode)||profile.host.startsWith('/')||/[\r\n@]/.test(profile.host)||profile.host.length>255||profile.database.length>256||profile.user.length>255)ready=false;
  const candidate:PrivateCandidate={public:{id,engine:definition.engine,source,target:definition.engine==='sqlite'?'Local file':['localhost','127.0.0.1','::1'].includes(profile.host)?'Loopback PostgreSQL':'Remote PostgreSQL',production:definition.production,ready},profile,password:definition.password,sourcePath,digest};
  checkedScope.candidates.set(id,candidate);
 }
 for(const file of configs){
  const path=join(scope.root,file);let bytes:Buffer;
  try{bytes=await readProjectFile(scope.root,path);}catch(error){if((error as NodeJS.ErrnoException).code!=='ENOENT')warnings.push(`${file}: skipped; unavailable, unsafe or larger than 64 KiB.`);continue;}
  try{
   const parsed=file.startsWith('.env')?parseEnv(bytes.toString('utf8'),file):parseAppsettings(bytes.toString('utf8'),file);
   const digest=createHash('sha256').update(bytes).digest('hex');
   for(const definition of parsed.definitions)await add(definition,file,path,digest);
   if(parsed.unresolved)warnings.push(`${file}: unresolved variables were skipped; values are never inferred from the server environment.`);
   if(parsed.unsupported)warnings.push(`${file}: unsupported connection syntax was skipped.`);
  }catch{warnings.push(`${file}: could not parse supported connection definitions.`);}
 }
 const files=await sqliteFiles(scope.root);
 for(const path of files.found){if(await sqliteHeader(scope.root,path).catch(()=>false))await add({engine:'sqlite',database:path,production:/(?:^|[/_.-])prod(?:uction)?(?:[/_.-]|$)/i.test(relative(scope.root,path))},relative(scope.root,path));}
 if(files.bounded||scope.candidates.size>=32)warnings.push('Discovery reached its bounded scan limit; deeper or additional files were omitted.');
 return {projectScope:expose(scope),candidates:[...scope.candidates.values()].map(item=>item.public),warnings};
}
export async function sessionProfile(owner:string,id:string){
 if(!id.startsWith('project-'))return;
 const scope=current(owner);const candidate=scope?.active.get(id);
 if(!scope||!candidate)throw new Error('Project connection expired; select it again');
 try{await validate(scope,candidate);}catch{scope.active.delete(id);throw new Error('Project files changed or became unavailable; discover again');}
 return {session_profile:candidate.profile,session_password:candidate.password};
}

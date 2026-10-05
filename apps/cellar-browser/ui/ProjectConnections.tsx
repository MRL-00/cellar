import {Modal} from './Modal.js';
import {useEffect,useRef,useState} from 'react';
import {call,hostIntegrationAvailable,requestProjectContext} from './bridge.js';
import type {Connection} from './types.js';

type Scope={id:string;name:string;path:string;expiresAt:string};
type Candidate={id:string;engine:string;source:string;target:string;production:boolean;ready:boolean};
export function ProjectConnections({onClose,onConnected}:{onClose:()=>void;onConnected:(connections:Connection[],preferred?:string)=>void}){
 const [scope,setScope]=useState<Scope>();const [root,setRoot]=useState('');
 const [consent,setConsent]=useState(false);const [confirmed,setConfirmed]=useState(false);const [production,setProduction]=useState(false);
 const [candidates,setCandidates]=useState<Candidate[]>([]);const [selected,setSelected]=useState('');const [warnings,setWarnings]=useState<string[]>([]);
 const [busy,setBusy]=useState(false);const [error,setError]=useState('');const [discovered,setDiscovered]=useState(false);
 const touched=useRef(false);
 useEffect(()=>{void call<{projectScope?:Scope}>('cellar_project_connections',{stage:'status'}).then(data=>{if(touched.current)return;setScope(data.projectScope);setRoot(data.projectScope?.path??'');}).catch(()=>setError('Project context is unavailable. Choose the folder explicitly.'));},[]);
 const candidate=candidates.find(item=>item.id===selected);
 async function run(work:()=>Promise<void>){setBusy(true);setError('');try{await work();}catch{setError('Project operation could not finish. Files may have changed, the session may have expired, or the selected path is unsupported. Choose the folder and try again.');}finally{setBusy(false);}}
 async function choose(){await run(async()=>{const data=await call<{projectScope:Scope}>('cellar_project_connections',{stage:'scope',root});setScope(data.projectScope);setCandidates([]);setDiscovered(false);setConsent(false);setConfirmed(false);});}
 async function discover(){if(!scope)return;await run(async()=>{const data=await call<{candidates:Candidate[];warnings:string[]}>('cellar_project_connections',{stage:'discover',scope:scope.id,confirmation:'DISCOVER PROJECT CONNECTIONS'});setCandidates(data.candidates);setWarnings(data.warnings);setDiscovered(true);setSelected('');setConfirmed(false);setProduction(false);});}
 async function connect(){if(!scope||!candidate)return;await run(async()=>{const data=await call<{connections:Connection[];preferredConnection:string}>('cellar_project_connections',{stage:'connect',scope:scope.id,candidate:candidate.id,confirmation:'CONNECT READ ONLY',productionAcknowledged:production});onConnected(data.connections,data.preferredConnection);onClose();});}
 return <Modal title="Connect from this project" className="connection-dialog project-dialog" busy={busy} onClose={onClose}>
  <p>Use database settings already in your codebase. Choose the project, review the discoveries, then connect read-only.</p>
  {!discovered&&<><div className="project-context-actions"><button disabled={busy||!hostIntegrationAvailable()} onClick={()=>void run(async()=>{await requestProjectContext();onClose();})}>Use current Codex project</button><small>{hostIntegrationAvailable()?'Codex supplies only the project folder; it must not read connection values.':'Choose a folder for this local preview. Codex project context is available in the host chat.'}</small></div>
  <label>Project folder<input aria-label="Project folder" value={root} placeholder="/absolute/path/to/project" onChange={event=>{touched.current=true;setRoot(event.target.value);setScope(undefined);setCandidates([]);setConsent(false);setDiscovered(false);}}/><button disabled={busy||!root} onClick={()=>void choose()}>Use this folder</button></label></>}
  {scope&&<><div className="project-scope"><strong>{scope.name}</strong><code>{scope.path}</code><small>{discovered?'Configuration read locally for this review.':'Folder explicitly selected. No project configuration read yet.'}</small></div>
   {!discovered&&<><p>Reads supported root-level .env and appsettings JSON files, plus SQLite headers up to three folder levels deep. Skips symlinks, hidden folders, dependency/build folders and configuration files larger than 64 KiB. No scripts run.</p>
   <label className="checkbox"><input type="checkbox" aria-label="Allow local project configuration discovery" checked={consent} onChange={event=>setConsent(event.target.checked)}/>Read database configuration in this selected project locally</label>
   <button disabled={busy||!consent} onClick={()=>void discover()}>{busy?'Working…':'Discover connections'}</button></>}
  </>}
  {discovered&&<><h3>Discovered connections</h3><p>Passwords, connection strings and usernames stay in the local server. This screen shows source and target category only. Discovery has not connected to any database.</p>
   <fieldset className="project-candidates"><legend>Choose a connection</legend>{candidates.map((item,index)=><label key={item.id} className={`project-candidate ${selected===item.id?'selected':''}`}><input type="radio" name="project-candidate" aria-label={`Select ${item.engine} from ${item.source} ${index+1}`} disabled={!item.ready||busy} checked={selected===item.id} onChange={()=>{setSelected(item.id);setConfirmed(false);setProduction(false);}}/><span><strong>{item.engine==='sqlite'?'SQLite':'PostgreSQL'} <code>{item.source}</code></strong><small>{item.target} · {item.production?'Production-like target':'Environment unverified'} · {item.ready?'Read-only':'Unavailable or unsupported path/settings'}</small></span>{item.production&&<b className="danger">PROD?</b>}</label>)}</fieldset>
   {!candidates.length&&<p>No supported connections found in this bounded scan. Use Add connection for other formats, deeper files or missing settings.</p>}
   {warnings.length>0&&<div className="project-warnings" role="status">{warnings.map((warning,index)=><p key={index}>{warning}</p>)}</div>}
   <button disabled={busy} onClick={()=>{setDiscovered(false);setCandidates([]);setSelected('');setConfirmed(false);setConsent(false);}}>Change project or rediscover</button>
   {candidate&&<div className="project-confirm"><p><strong>Connect read-only</strong> uses the original database. The local server keeps this connection and any credential in memory for up to 15 minutes. Nothing is saved to the keychain or a connection file. Database results may be shown in the workspace.</p><p>Localhost and development filenames do not establish safety. Remote PostgreSQL requires verified TLS; unresolved variables are never guessed.</p>
    {candidate.production&&<label className="checkbox"><input type="checkbox" aria-label="Acknowledge production-like target" checked={production} onChange={event=>setProduction(event.target.checked)}/>I understand this may be a production database</label>}
    <label className="checkbox"><input type="checkbox" aria-label="Confirm read-only database access" checked={confirmed} onChange={event=>setConfirmed(event.target.checked)}/>Connect to this selected database read-only</label>
   </div>}
  </>}
  {error&&<p role="alert" className="error">{error}</p>}
  <div className="dialog-actions">{scope&&<button disabled={busy} onClick={()=>void run(async()=>{const result=await call<{connections:Connection[]}>('cellar_project_connections',{stage:'disconnect',scope:scope.id});onConnected(result.connections);onClose();})}>Forget project connections</button>}<button disabled={busy} onClick={onClose}>Cancel</button><button className="primary" disabled={busy||!candidate?.ready||!confirmed||(candidate.production&&!production)} onClick={()=>void connect()}>Connect read-only</button></div>
 </Modal>;
}

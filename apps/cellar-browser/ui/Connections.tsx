import { useEffect,useState,type CSSProperties } from 'react';
import { call } from './bridge.js';
import type { Connection } from './types.js';
import {Icon} from './Shell.js';
import {useModalFocus} from './Modal.js';

const engines=[['postgres','PostgreSQL'],['mysql','MySQL'],['sqlite','SQLite'],['supabase','Supabase'],['neon','Neon'],['planetscale','PlanetScale'],['firestore','Firestore'],['convex','Convex'],['cosmos','Cosmos DB'],['mssql','SQL Server'],['azure','Azure SQL']];
const supported=new Set(['postgres','sqlite','supabase','neon']);
const accents=['#4f8ff7','#f6a44a','#d97a5a','#5bb8e0','#a78bfa','#4ade80','#f87171'];
const engineColors:Record<string,string>={postgres:'#4f8ff7',mysql:'#f6a44a',mssql:'#d97a5a',azure:'#5bb8e0',sqlite:'#a78bfa',firestore:'#f4c542',convex:'#f25c4d',cosmos:'#6b5ce7',supabase:'#3ecf8e',neon:'#00e599',planetscale:'#c8ccd4'};

export function Connections({onSaved,onClose,existing}:{onSaved:(connections:Connection[])=>void;onClose:()=>void;existing?:string}) {
  const [engine,setEngine]=useState<Connection['engine']>('postgres');
  const [tab,setTab]=useState('General');
  const [color,setColor]=useState<string|null>(null);
  const [applicationName,setApplicationName]=useState('');
  const [name,setName]=useState('');
  const [host,setHost]=useState('localhost');
  const [port,setPort]=useState(5432);
  const [database,setDatabase]=useState('');
  const [user,setUser]=useState('postgres');
  const [password,setPassword]=useState('');
  const [environment,setEnvironment]=useState('dev');
  const [allowEdits,setAllowEdits]=useState(false);
  const [ssl,setSsl]=useState('verify-full');
  const [certificate,setCertificate]=useState('');
  const [busy,setBusy]=useState(false);
  const [error,setError]=useState('');
  const [uri,setUri]=useState('');
  const [removeConfirmation,setRemoveConfirmation]=useState('');
  const modalRef=useModalFocus(onClose,busy);
  useEffect(()=>{
    if(!existing)return;setBusy(true);
    void call<{profile:{name:string;engine:Connection['engine'];host:string;port:number;database:string;user:string;ssl_mode:string;ssl_ca_pem?:string;env_tag:string;allow_edits:boolean;color?:string;application_name?:string}}>('cellar_connection',{operation:'details',connection:existing}).then(({profile})=>{
      setName(profile.name);setEngine(profile.engine);setHost(profile.host);setPort(profile.port);setDatabase(profile.database);setUser(profile.user);setSsl(profile.ssl_mode);setCertificate(profile.ssl_ca_pem??'');setEnvironment(profile.env_tag??'dev');setAllowEdits(profile.allow_edits);
      setColor(profile.color??null);setApplicationName(profile.application_name??'');
    }).catch(error=>setError(error.message)).finally(()=>setBusy(false));
  },[existing]);
  function parseUri() {
    try {
      const parsed=new URL(uri);
      if(!['postgres:','postgresql:'].includes(parsed.protocol))throw new Error('Use a PostgreSQL connection string');
      setHost(parsed.hostname);setPort(Number(parsed.port||5432));setDatabase(decodeURIComponent(parsed.pathname.slice(1)));setUser(decodeURIComponent(parsed.username));setPassword(decodeURIComponent(parsed.password));setUri('');
    }catch {setError('Invalid PostgreSQL connection string');}
  }
  async function save() {
    setBusy(true);setError('');
    try {
      const result=await call<{connections:Connection[]}>('cellar_connection',{operation:'save',profile:{id:existing??'',name,engine,host,port,database,user,ssl_mode:ssl,ssl_ca_pem:certificate||undefined,env_tag:environment,allow_edits:allowEdits,color,application_name:applicationName||null},password:engine==='sqlite'?undefined:password});
      setPassword('');setUri('');onSaved(result.connections);onClose();
    }catch(error){setError(error instanceof Error?error.message:'Cannot save connection');}
    finally{setBusy(false);}
  }
  async function upload(file:File) {
    if(file.size>67108864){setError('Upload limit is 64 MiB. Larger local SQLite files can be connected by path.');return;}
    setBusy(true);setError('');let id:string|undefined;
    try {
      const begun=await call<{id:string}>('cellar_upload',{stage:'begin',name:file.name});id=begun.id;
      for(let offset=0;offset<file.size;offset+=32768){
        const bytes=new Uint8Array(await file.slice(offset,offset+32768).arrayBuffer());
        await call('cellar_upload',{stage:'chunk',id,chunk:btoa(Array.from(bytes,b=>String.fromCharCode(b)).join(''))});
      }
      const result=await call<{connections:Connection[]}>('cellar_upload',{stage:'finish',id});id=undefined;
      onSaved(result.connections);onClose();
    }catch(error){setError(error instanceof Error?error.message:'Cannot import database');if(id)void call('cellar_upload',{stage:'cancel',id}).catch(()=>undefined);}
    finally{setBusy(false);}
  }
  return <div className="modal-backdrop"><section ref={modalRef} role="dialog" aria-modal="true" aria-label={existing?'Edit connection':'New connection'} className="connection-dialog native-connection">
    <div className="modal-heading"><Icon name="database"/><strong>{existing?'Edit connection':'New connection'}</strong><button aria-label="Close connection editor" disabled={busy} onClick={onClose}><Icon name="close"/></button></div>
    <div className="connection-body">
      <div className="engine-picker" aria-label="Database engine">{engines.map(([id,label])=><button key={id} style={{'--engine-color':engineColors[id]} as CSSProperties} className={engine===id?'selected':''} aria-pressed={engine===id} disabled={busy||!supported.has(id)} title={supported.has(id)?label:'This engine is not supported in the Codex extension'} onClick={()=>setEngine(id as Connection['engine'])}><span className="engine-glyph"><Icon name={'engine-'+id}/></span><span>{label}</span></button>)}</div>
      <div className="connection-tabs" role="tablist" aria-label="Connection settings">{[['General','database'],['SSH tunnel','ssh'],['SSL / TLS','lock'],['Options','settings']].map(([label,icon])=><button role="tab" aria-selected={tab===label} key={label} onClick={()=>setTab(label)}><Icon name={icon}/>{label}</button>)}</div>
      <div role="tabpanel" aria-label={tab}>
      {tab==='General'&&<div className="native-form">
        <label><span>Name<small>Shown in the sidebar</small></span><input aria-label="Connection name" value={name} onChange={e=>setName(e.target.value)} maxLength={80} autoFocus/></label>
        {engine!=='sqlite'&&<label><span>Host</span><div className="host-port"><input aria-label="Database host" value={host} onChange={e=>setHost(e.target.value)}/><span>:</span><input aria-label="Database port" type="number" value={port} onChange={e=>setPort(Number(e.target.value))}/></div></label>}
        <label><span>{engine==='sqlite'?'Database file':'Database'}</span><input aria-label={engine==='sqlite'?'SQLite path':'Database name'} placeholder={engine==='sqlite'?'/absolute/path/database.sqlite':''} value={database} onChange={e=>setDatabase(e.target.value)}/></label>
        {engine!=='sqlite'&&<><label><span>User</span><input aria-label="Database user" value={user} onChange={e=>setUser(e.target.value)}/></label><label><span>Password</span><input aria-label="Database password" type="password" autoComplete="new-password" placeholder={existing?'Leave blank to keep stored password':''} value={password} onChange={e=>setPassword(e.target.value)}/></label><p>Passwords are saved in the Mac OS keychain. A blank password keeps the stored password when editing.</p></>}
        {engine==='supabase'&&<p>Use the PostgreSQL direct or session-pooler connection details from Supabase. An API key or project URL is not a database connection.</p>}
        <hr/><div className="native-form-row"><span>Accent</span><div className="accent-swatches">{accents.map(accent=><button key={accent} aria-label={'Accent '+accent} aria-pressed={color===accent} style={{background:accent}} onClick={()=>setColor(accent)}/>)}<button aria-label="Clear accent" onClick={()=>setColor(null)}>×</button></div></div><p className="form-hint">Visual marker — protects against running on prod by mistake</p>
        <div className="native-form-row"><span>Environment</span><div className="segments" aria-label="Connection environment">{[['prod','prod'],['staging','staging'],['dev','dev'],['local','local']].map(([value,label])=><button key={value} aria-pressed={environment===value} onClick={()=>setEnvironment(value)}>{label}</button>)}</div></div>
      </div>}
      {tab==='SSH tunnel'&&<div className="native-form"><label className="checkbox"><input type="checkbox" disabled/>Use SSH tunnel</label><p>SSH tunneling lands in a follow-up slice. Connect directly for now.</p></div>}
      {tab==='SSL / TLS'&&<div className="native-form"><div className="native-form-row"><span>Use SSL / TLS</span><button role="switch" aria-label="Use SSL / TLS" aria-checked={ssl!=='disable'} className="native-toggle" disabled={engine==='sqlite'} title="Disabling TLS is permitted only for local databases" onClick={()=>setSsl(ssl==='disable'?'verify-full':'disable')}/></div>{ssl!=='disable'&&<div className="native-form-row"><span>SSL mode</span><div className="segments" aria-label="TLS mode">{[['disable','Disable'],['prefer','Prefer'],['require','Require'],['verify-ca','Verify CA'],['verify-full','Verify Full']].map(([value,label])=><button key={value} aria-pressed={ssl===value} disabled={engine==='sqlite'||['prefer','verify-ca'].includes(value)} title={['disable','require'].includes(value)?'Local databases only':undefined} onClick={()=>setSsl(value)}>{label}</button>)}</div></div>}<p>{engine==='sqlite'?'SQLite uses a local file; TLS does not apply.':'Remote connections always verify TLS certificates and hostnames.'}</p>{engine!=='sqlite'&&<><label><span>Root certificate<small>Optional PEM</small></span><textarea aria-label="Root certificate" value={certificate} onChange={e=>setCertificate(e.target.value)} maxLength={16384}/></label><p>For Supabase, use the root certificate supplied in Database settings when its certificate is not trusted by the system.</p></>}</div>}
      {tab==='Options'&&<div className="native-form">
        <label><span>Application name</span><input aria-label="Application name" value={applicationName} onChange={e=>setApplicationName(e.target.value)} maxLength={80}/></label>
        <label className="checkbox"><input aria-label="Enable editing" type="checkbox" checked={allowEdits} onChange={e=>setAllowEdits(e.target.checked)}/>Enable row editing and reviewed commits</label>
        {environment==='prod'&&allowEdits&&<p className="danger">Production editing is enabled. Every commit requires SQL review and typed confirmation.</p>}
        {engine==='sqlite'?<><label><span>Import file</span><input aria-label="Import SQLite file" type="file" accept=".db,.sqlite,.sqlite3" disabled={busy} onChange={e=>{const file=e.target.files?.[0];if(file)void upload(file);}}/></label><p>Imported files become private editable copies. Connecting by path uses the original database; reviewed commits write there when editing is enabled.</p></>:<label><span>Connection string</span><div className="uri-field"><input aria-label="Connection string" type="password" autoComplete="off" value={uri} onChange={e=>setUri(e.target.value)}/><button onClick={parseUri}>Fill fields</button></div></label>}
        {existing&&<label><span>Remove connection<small>Keeps the database</small></span><div className="uri-field"><input aria-label="Remove confirmation" placeholder="Type REMOVE" value={removeConfirmation} onChange={e=>setRemoveConfirmation(e.target.value)}/><button className="danger" disabled={busy||removeConfirmation!=='REMOVE'} onClick={()=>{setBusy(true);void call<{connections:Connection[]}>('cellar_connection',{operation:'remove',connection:existing}).then(result=>{onSaved(result.connections);onClose();}).catch(error=>setError(error.message)).finally(()=>setBusy(false));}}>Remove connection</button></div></label>}
      </div>}
      </div>{error&&<p role="alert" className="error">{error}</p>}
    </div>
    <div className="connection-footer"><button disabled title="Connection testing is not implemented in the extension"><Icon name="power"/>Test connection</button><div><button disabled={busy} onClick={onClose}>Cancel</button><button className="primary" disabled={busy||!name||!database} onClick={()=>void save()}><Icon name="plus"/>{busy?'Saving…':existing?'Save changes':'Save'}</button></div></div>
  </section></div>;
}

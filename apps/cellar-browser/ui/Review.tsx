import {Modal} from './Modal.js';
import { useEffect, useState } from 'react';
import { call } from './bridge.js';
import type { Change, Table } from './types.js';

export function Review({connection,table,changes,production,onDone,onClose}:{connection:string;table:Table;changes:Change[];production:boolean;onDone:()=>void;onClose:()=>void}) {
  const [preview,setPreview]=useState<{sql:string;previewId:string;changeCount:number}>();
  const [confirmation,setConfirmation]=useState('');
  const [error,setError]=useState('');
  const [busy,setBusy]=useState(false);
  const argumentsForPlan={connection,schema:table.schema,table:table.name,changes:changes.map(({kind,original,revision,values})=>({kind,original,revision,values}))};
  useEffect(()=>{let active=true;void call<typeof preview>('cellar_preview',argumentsForPlan).then(data=>{if(active)setPreview(data);}).catch(error=>{if(active)setError(error.message);});return()=>{active=false;};},[]);
  const required=`COMMIT ${changes.length} CHANGES`;
  async function commit(){
    if(!preview)return;setBusy(true);setError('');
    try{await call('cellar_commit',{...argumentsForPlan,preview_id:preview.previewId,confirmation});onDone();}
    catch(error){const message=error instanceof Error?error.message:'Commit failed';setError(/timed out|service failed|cancelled|unavailable/i.test(message)?`${message}. Commit status may be unknown. Close this review and inspect fresh database rows before retrying; do not automatically repeat the commit.`:message);}
    finally{setBusy(false);}
  }
  return <Modal title={`Review & Commit · ${table.name}`} label="Review and commit" className="review-dialog" busy={busy} onClose={onClose}>{production&&<p className="danger">PRODUCTION DATABASE · changes will affect the connected database.</p>}<p>{changes.length} pending row changes. All changes commit together or roll back together. Changed or deleted original rows cause a conflict.</p><pre>{preview?.sql??'Building SQL preview…'}</pre>{error&&<p role="alert" className="error">{error}</p>}<label>Type {required}<input aria-label="Commit confirmation" value={confirmation} onChange={e=>setConfirmation(e.target.value)} autoComplete="off"/></label><div className="dialog-actions"><button disabled={busy} onClick={onClose}>Back to edits</button><button className="primary" disabled={busy||!preview||confirmation!==required} onClick={()=>void commit()}>{busy?'Committing…':'Commit transaction'}</button></div></Modal>;
}

export function InsertRow({table,onStage,onClose}:{table:Table;onStage:(values:Record<string,string|null>)=>void;onClose:()=>void}) {
  const [values,setValues]=useState<Record<string,string|null>>({});
  return <Modal title={`Insert row · ${table.name}`} label="Insert row" className="connection-dialog" onClose={onClose}><p>Unchecked columns use database defaults. Checked empty values are empty strings. Choose NULL explicitly.</p>{table.columns.map(column=><div className="insert-field" key={column.name}><label className="checkbox"><input type="checkbox" aria-label={`Include ${column.name}`} checked={column.name in values} onChange={e=>setValues(previous=>{const next={...previous};if(e.target.checked)next[column.name]='';else delete next[column.name];return next;})}/>{column.name} <small>{column.type}</small></label>{column.name in values&&<div className="form-row"><input aria-label={`Insert ${column.name}`} value={values[column.name]??''} disabled={values[column.name]===null} onChange={e=>setValues({...values,[column.name]:e.target.value})}/><label className="checkbox"><input type="checkbox" checked={values[column.name]===null} onChange={e=>setValues({...values,[column.name]:e.target.checked?null:''})}/>NULL</label></div>}</div>)}<div className="dialog-actions"><button onClick={onClose}>Cancel</button><button className="primary" disabled={!Object.keys(values).length} onClick={()=>{onStage(values);onClose();}}>Stage insert</button></div></Modal>;
}

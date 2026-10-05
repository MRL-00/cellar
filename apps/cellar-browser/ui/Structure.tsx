import {Modal} from './Modal.js';
import {useEffect,useState} from 'react';
import {call} from './bridge.js';
import type {Table,Result} from './types.js';

type Details={ddl?:string;indexes:{name:string;definition:string}[];foreignKeys:{name:string;column?:string;targetSchema:string;targetTable:string;targetColumn?:string;definition:string}[];defaults:{column:string;default:string|null;generated:boolean}[]};
export function Structure({connection,table,onOpen}:{connection:string;table:Table;onOpen:(schema:string,table:string)=>void}) {
  const [details,setDetails]=useState<Details>();const [error,setError]=useState('');
  useEffect(()=>{let active=true;void call<Details>('cellar_details',{connection,schema:table.schema,table:table.name}).then(data=>{if(active)setDetails(data);}).catch(error=>{if(active)setError(error.message);});return()=>{active=false;};},[connection,table]);
  return <div className="structure"><h3>{table.schema}.{table.name} · {table.kind}</h3>{error&&<p role="alert" className="error">{error}</p>}<table><thead><tr><th>Column</th><th>Type</th><th>Nullable</th><th>Default</th></tr></thead><tbody>{table.columns.map(column=><tr key={column.name}><td>{column.primaryKey?'⚿ ':''}{column.name}</td><td>{column.type}</td><td>{column.nullable?'YES':'NO'}</td><td>{details?.defaults.find(item=>item.column===column.name)?.default??'—'}</td></tr>)}</tbody></table>
    <h3>Indexes</h3>{details?.indexes.map(index=><div key={index.name}><strong>{index.name}</strong><pre>{index.definition}</pre></div>)}{details&&!details.indexes.length&&<p>No indexes.</p>}
    <h3>Foreign keys / relationships</h3>{details?.foreignKeys.map(key=><div className="relationship" key={key.name}><strong>{key.name}</strong><span>{key.column??table.name} → </span><button onClick={()=>onOpen(key.targetSchema,key.targetTable)}>{key.targetSchema}.{key.targetTable}{key.targetColumn?`.${key.targetColumn}`:''}</button><pre>{key.definition}</pre></div>)}{details&&!details.foreignKeys.length&&<p>No foreign keys.</p>}
    {details?.ddl&&<><h3>{table.kind==='view'?'View definition':'DDL'}</h3><pre>{details.ddl}</pre></>}
  </div>;
}

export function QueryPlan({connection,sql,onClose}:{connection:string;sql:string;onClose:()=>void}) {
  const [plan,setPlan]=useState<{plan?:unknown}&Partial<Result>>();const [error,setError]=useState('');
  useEffect(()=>{let active=true;void call<typeof plan>('cellar_plan',{connection,sql}).then(data=>{if(active)setPlan(data);}).catch(error=>{if(active)setError(error.message);});return()=>{active=false;};},[connection,sql]);
  return <Modal title="Query plan" className="review-dialog" onClose={onClose}><p>EXPLAIN only. ANALYZE is off; this does not execute the SELECT query.</p>{error&&<p role="alert" className="error">{error}</p>}<pre>{plan?JSON.stringify(plan.plan??plan.rows,null,2):'Reading plan…'}</pre><button onClick={onClose}>Close</button></Modal>;
}

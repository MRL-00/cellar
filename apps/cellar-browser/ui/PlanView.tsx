import {useEffect,useRef,useState} from 'react';
import {call} from './bridge.js';
import {Icon} from './Shell.js';

type PlanResponse={plan?:unknown;rows?:unknown[][];durationMs?:number};
type PlanNode={title:string;detail?:string;children:PlanNode[]};
function nodes(data:PlanResponse):PlanNode[] {
  if(data.rows)return data.rows.map(row=>({title:String(row[3]??row.join(' · ')),detail:`node ${row[0]} · parent ${row[1]}`,children:[]}));
  function convert(value:unknown):PlanNode|undefined {
    if(!value||typeof value!=='object')return;
    const item=value as Record<string,unknown>;
    if(item.Plan)return convert(item.Plan);
    if(!item['Node Type'])return;
    const details=[item['Relation Name'],item['Index Name'],item['Total Cost']!==undefined?`cost ${item['Total Cost']}`:undefined,item['Plan Rows']!==undefined?`rows ${item['Plan Rows']}`:undefined].filter(value=>value!==undefined).join(' · ');
    return {title:String(item['Node Type']),detail:details,children:Array.isArray(item.Plans)?item.Plans.flatMap(child=>{const next=convert(child);return next?[next]:[];}):[]};
  }
  return (Array.isArray(data.plan)?data.plan:[data.plan]).flatMap(value=>{const node=convert(value);return node?[node]:[];});
}
function Node({node}:{node:PlanNode}){return <div className="plan-node"><div><Icon name="tree"/><strong>{node.title}</strong>{node.detail&&<span>{node.detail}</span>}</div>{node.children.length>0&&<section>{node.children.map((child,index)=><Node node={child} key={index}/>)}</section>}</div>;}
export function PlanView({connection,sql}:{connection:string;sql:string}) {
  const [data,setData]=useState<PlanResponse>();
  const [snapshot,setSnapshot]=useState('');
  const [busy,setBusy]=useState(false);
  const [error,setError]=useState('');
  const generation=useRef(0);
  async function run(){const stamp=++generation.current;setBusy(true);setError('');try{const response=await call<PlanResponse>('cellar_plan',{connection,sql});if(stamp===generation.current){setData(response);setSnapshot(sql);}}catch(error){if(stamp===generation.current)setError(error instanceof Error?error.message:'Plan failed');}finally{if(stamp===generation.current)setBusy(false);}}
  useEffect(()=>{void run();return()=>{generation.current++;};},[connection]);
  const tree=data?nodes(data):[];
  return <div className="inline-plan"><div className="panel-toolbar"><Icon name="tree"/><span>Execution plan</span>{data&&snapshot!==sql&&<span className="plan-stale">stale</span>}<div className="plan-modes"><button className="chosen">Estimate</button><button disabled title="ANALYZE is unavailable in this read-only extension">Analyze</button></div><button disabled={!data} onClick={()=>void navigator.clipboard?.writeText(JSON.stringify(data?.plan??data?.rows,null,2))}><Icon name="copy"/>JSON</button><button disabled={busy} onClick={()=>void run()}><Icon name="bolt"/>{busy?'Explaining…':'Run'}</button></div>{error?<p role="alert" className="error">{error}</p>:data?<><div className="plan-metrics"><span><small>MODE</small>estimate · ANALYZE off</span><span><small>ROUND TRIP</small>{data.durationMs??'—'} ms</span></div><div className="plan-tree">{tree.length?tree.map((node,index)=><Node node={node} key={index}/>):<pre>{JSON.stringify(data?.plan??data?.rows,null,2)}</pre>}</div></>:<div className="panel-empty">Loading execution plan…</div>}</div>;
}

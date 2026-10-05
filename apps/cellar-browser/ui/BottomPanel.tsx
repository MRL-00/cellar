import {useState} from 'react';
import {PlanView} from './PlanView.js';
import {Icon} from './Shell.js';
import {Grid} from './Grid.js';
import {exportResult} from './export.js';
import type {Tab} from './types.js';

type Panel='results'|'messages'|'plan'|'history'|'notices';
type Level='all'|'info'|'success'|'warning'|'error';
type Message={level:Exclude<Level,'all'>;text:string;metrics:string};
export function BottomPanel({tab,connection,busy,error,notice,history,onHistory,onResults,onStructure,onDownload,onCancel}:{tab:Tab;connection:string;busy:boolean;error:string;notice:string;history:string[];onHistory:(sql:string)=>void;onResults:()=>void;onStructure?:()=>void;onDownload?:()=>void;onCancel:()=>void}) {
  const [selected,setSelected]=useState<Panel>('results');
  const [visible,setVisible]=useState(true);
  const [level,setLevel]=useState<Level>('all');
  const [exporting,setExporting]=useState(false);
  const [height,setHeight]=useState(280);
  const messages:Message[]=[];
  const failure=tab.executionError??error;
  if(failure)messages.push({level:'error',text:failure,metrics:'—'});
  else if(busy)messages.push({level:'info',text:'Running request with a 100-row response limit.',metrics:'—'});
  else if(tab.result){messages.push({level:'success',text:tab.table?`Loaded ${tab.table.schema}.${tab.table.name} rows ${tab.offset+1}–${tab.offset+tab.result.rows.length}.`:`Statement completed: ${tab.result.rows.length} rows returned.`,metrics:`${tab.result.durationMs} ms | ${tab.result.rows.length} rows`});if(tab.result.truncated)messages.push({level:'warning',text:'Response hit the 100-row or byte limit; more rows may exist.',metrics:'100 row limit'});}
  const warnings=messages.filter(message=>message.level==='warning');
  const counts:Record<Panel,number|undefined>={results:tab.result?.rows.length,messages:messages.length,plan:undefined,history:history.length,notices:warnings.length+(notice?1:0)};
  const items:[Panel,string,string][]=[['results','Results','table'],['messages','Messages','info'],['plan','Plan','tree'],['history','History','history'],['notices','Notices','triangle-alert']];
  function choose(panel:Panel){setSelected(panel);setVisible(true);if(panel==='results')onResults();}
  function resize(event:React.PointerEvent<HTMLDivElement>){const y=event.clientY;const start=height;const node=event.currentTarget;node.setPointerCapture(event.pointerId);const move=(next:PointerEvent)=>setHeight(Math.max(130,Math.min(window.innerHeight*.65,start+y-next.clientY)));const end=()=>{node.removeEventListener('pointermove',move);node.removeEventListener('pointerup',end);};node.addEventListener('pointermove',move);node.addEventListener('pointerup',end);}
  return <section className={`bottom-panel ${visible?'':'collapsed'}`} style={{height:visible?height:29}} aria-label="Execution panel"><div className="panel-resizer" role="separator" aria-label="Resize execution panel" aria-orientation="horizontal" onPointerDown={resize}/><div className="result-tabs bottom-panel-header"><div className="bottom-tabs" role="tablist" aria-label="Execution views">{items.map(([id,label,icon])=><button key={id} role="tab" aria-selected={selected===id} className={selected===id?'chosen':''} onClick={()=>choose(id)}><Icon name={icon}/>{label}{counts[id]!==undefined&&<span className="panel-count">{counts[id]}</span>}</button>)}{onStructure&&<button className="structure-trigger" onClick={onStructure}>Structure</button>}<span className="panel-meta">{tab.table?`${tab.table.schema}.${tab.table.name} · table rows shown above`:`${tab.title} · query tab`}</span></div><div className="panel-actions">{busy&&<button className="cancel" aria-label="Cancel query" onClick={onCancel}>■</button>}{onDownload&&<button aria-label="Download database" title="Download database" disabled={busy||Boolean(tab.changes?.length)} onClick={onDownload}><Icon name="download"/></button>}<div className="export-menu"><button aria-label="Export results" title="Export results" disabled={!tab.result} onClick={()=>setExporting(!exporting)}><Icon name="file-text"/></button>{exporting&&tab.result&&<div className="native-menu"><button onClick={()=>{exportResult(tab.result!,'csv',tab.title);setExporting(false);}}>Export CSV page</button><button onClick={()=>{exportResult(tab.result!,'json',tab.title);setExporting(false);}}>Export JSON page</button></div>}</div><button aria-label={visible?'Hide execution panel':'Show execution panel'} title={visible?'Hide execution panel':'Show execution panel'} onClick={()=>setVisible(!visible)}><Icon name="chevrons-down"/></button></div></div>
    {visible&&<div className="bottom-panel-body" role="tabpanel" aria-label={`${items.find(([id])=>id===selected)?.[1]} panel`}>
      {selected==='results'&&(tab.table?<div className="panel-empty"><strong>Table rows are already shown</strong><p>{tab.table.schema}.{tab.table.name} is shown above. Open a query tab to see SQL results here.</p></div>:tab.result?<Grid result={tab.result}/>:<div className="panel-empty"><strong>Run a query to see results</strong></div>)}
      {selected==='messages'&&<><div className="message-filters">{(['all','success','warning','error','info'] as Level[]).map(filter=><button key={filter} className={level===filter?'chosen':''} onClick={()=>setLevel(filter)}>{filter} {filter==='all'?messages.length:messages.filter(message=>message.level===filter).length}</button>)}<span>{messages.length} total</span></div><div className="message-scroll"><table className="execution-messages"><thead><tr><th>time</th><th>level</th><th>source</th><th>message</th><th>metrics</th></tr></thead><tbody>{messages.filter(message=>level==='all'||message.level===level).map((message,index)=><tr key={index}><td>{tab.executedAt?new Date(tab.executedAt).toLocaleTimeString('en-GB',{hour12:false}):'—'}</td><td className={`level-${message.level}`}>{message.level}</td><td>execution</td><td>{message.text}</td><td>{message.metrics}</td></tr>)}</tbody></table>{messages.length===0&&<div className="panel-empty">No execution messages. Run or refresh the active tab.</div>}</div></>}
      {selected==='plan'&&(tab.table?<div className="panel-empty"><strong>No SQL statement selected</strong><p>Open a query tab to inspect its EXPLAIN plan. ANALYZE is off.</p></div>:<PlanView connection={connection} sql={tab.sql}/>)}
      {selected==='history'&&<div className="history panel-history"><div className="panel-toolbar">Successful queries · this connection · current session</div>{history.length?history.map((sql,index)=><button key={index} disabled={busy} onClick={()=>onHistory(sql)}><Icon name="terminal"/><code>{sql}</code></button>):<div className="panel-empty">No successful SQL queries yet.</div>}</div>}
      {selected==='notices'&&<div className="panel-notices">{notice&&<div><Icon name="info"/><span>{notice}</span></div>}{warnings.map((warning,index)=><div key={index}><Icon name="triangle-alert"/><span>{warning.text}</span></div>)}{!notice&&!warnings.length&&<div className="panel-empty">No notices for the active execution.</div>}</div>}
    </div>}
  </section>;
}

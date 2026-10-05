import {ContextMenu,type MenuAction} from './ContextMenu.js';
import { useEffect, useRef, useState } from 'react';
import { call, fullscreen, onFile, onProject, initialise } from './bridge.js';
import {ProjectConnections} from './ProjectConnections.js';
import { downloadDatabase } from './export.js';
import { Connections } from './Connections.js';
import { Titlebar, Sidebar, Icon } from './Shell.js';
import {Structure,QueryPlan} from './Structure.js';
import { Review, InsertRow } from './Review.js';
import {TableFilters} from './TableFilters.js';
import {BottomPanel} from './BottomPanel.js';
import { Editor } from './Editor.js';
import { Grid } from './Grid.js';
import {WorkspaceSearch} from './WorkspaceSearch.js';
import {GridFooter} from './GridFooter.js';
import {readPreference,writePreference} from './preferences.js';
import type { Connection, Filter, Result, Tab, Table, Change } from './types.js';

const fresh = (table?: Table): Tab => ({ id: crypto.randomUUID(), title: table?.name ?? 'Query', table, sql: 'SELECT 1 AS ready', filters: [], offset: 0, descending: false });
export function App() {
  const [tabMenu,setTabMenu]=useState<{x:number;y:number;items:MenuAction[]}>();
  const [connections, setConnections] = useState<Connection[]>([]);
  const [connection, setConnection] = useState('');
  const [profileRevision,setProfileRevision]=useState(0);
  const [tables, setTables] = useState<Table[]>([]);
  const [palette,setPalette]=useState(false);
  useEffect(()=>{const handle=(event:KeyboardEvent)=>{if((event.metaKey||event.ctrlKey)&&event.key.toLowerCase()==='k'){event.preventDefault();setPalette(previous=>!previous);}};window.addEventListener('keydown',handle);return()=>window.removeEventListener('keydown',handle);},[]);
  const [search, setSearch] = useState('');
  const [tabs, setTabs] = useState<Tab[]>([]);
  const [active, setActive] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [column, setColumn] = useState('');
  const [operator, setOperator] = useState<Filter['operator']>('contains');
  const [value, setValue] = useState('');
  const [panel, setPanel] = useState<'results' | 'structure'>('results');
  const [history, setHistory] = useState<string[]>([]);
  const [connectionEditor,setConnectionEditor]=useState(false);
  const [editingConnection,setEditingConnection]=useState<string>();
  const [projectDialog,setProjectDialog]=useState(false);
  const [review,setReview]=useState(false);
  const [inserting,setInserting]=useState(false);
  const [showPlan,setShowPlan]=useState(false);
  const [notice,setNotice]=useState('');
  const [sidebarWidth,setSidebarWidth]=useState(()=>{const value=readPreference('sidebar-width',256);return typeof value==='number'&&Number.isFinite(value)?Math.max(200,Math.min(600,value)):256;});
  function resizeSidebar(event:React.PointerEvent<HTMLDivElement>){const start=event.clientX;const width=sidebarWidth;const node=event.currentTarget;node.setPointerCapture(event.pointerId);const move=(next:PointerEvent)=>{const value=Math.max(200,Math.min(600,width+next.clientX-start));setSidebarWidth(value);writePreference('sidebar-width',value);};const end=()=>{node.removeEventListener('pointermove',move);node.removeEventListener('pointerup',end);node.removeEventListener('pointercancel',end);};node.addEventListener('pointermove',move);node.addEventListener('pointerup',end);node.addEventListener('pointercancel',end);}
  const [sidebarVisible,setSidebarVisible]=useState(true);
  const [bottomVisible,setBottomVisible]=useState(true);
  const pending = useRef<string>();
  const generation = useRef(0);
  const attempted = useRef(new Set<string>());
  const hasPending=useRef(false);
  hasPending.current=tabs.some(tab=>Boolean(tab.changes?.length));
  const tab = tabs.find(tab => tab.id === active);
  const current = connections.find(item => item.id === connection);
  const editable=Boolean(tab?.table && !current?.readOnly && tab.table.kind!=='view');
  const pendingChanges=tab?.changes??[];
  const update = (id: string, patch: Partial<Tab>) => setTabs(previous => previous.map(tab => tab.id === id ? { ...tab, ...patch } : tab));
  useEffect(() => {
    void initialise<{ connections: Connection[];preferredConnection?:string;importedCopy?:boolean;projectRequested?:boolean }>().then(data => { setConnections(data.connections); setConnection(data.preferredConnection??data.connections[0]?.id??'');if(data.projectRequested)setProjectDialog(true);if(data.importedCopy)setNotice('Opened an editable working copy of the chat database. Download the updated database after committing.'); }).catch(error => setError(error.message));
  }, []);
  useEffect(()=>onProject(()=>{if(hasPending.current){setError('Commit or discard pending changes before connecting from a project.');return;}setProjectDialog(true);}),[]);
  useEffect(()=>onFile(file=>{
    if(hasPending.current){setError('Commit or discard pending changes before opening a database from chat.');return;}
    void call<{connections:Connection[]}>('cellar_attach',{name:file.name}).then(data=>{
      setConnections(data.connections);setConnection(data.connections.at(-1)?.id??'');setNotice('Opened an editable working copy of the chat database. The original attachment is preserved.');
    }).catch(error=>setError(error.message));
  }),[]);
  useEffect(() => {
    const stamp = ++generation.current;
    setTables([]); setTabs([]); setActive(''); setHistory([]); setError('');
    if (!connection) return;
    setBusy(true);
    void call<{ tables: Table[] }>('cellar_schema', { connection }).then(data => {
      if (stamp !== generation.current) return;
      setTables(data.tables);
      if (data.tables[0]) { const tab = fresh(data.tables[0]); setTabs([tab]); setActive(tab.id); }
    }).catch(error => { if (stamp === generation.current) setError(error.message); }).finally(() => { if (stamp === generation.current) setBusy(false); });
  }, [connection,profileRevision]);
  useEffect(() => {
    if (tab?.table && !tab.result && !busy && !attempted.current.has(tab.id)) {
      attempted.current.add(tab.id);
      void execute(tab);
    }
  }, [active, busy]);
  useEffect(() => { setColumn(tab?.table?.columns[0]?.name ?? ''); setValue(''); }, [active]);

  async function execute(target: Tab, selection?: string,cursor?:number,batch=false) {
    if (busy) return;
    if(target.changes?.length){setError('Commit or discard pending changes before refreshing, sorting or paging.');return;}
    const id = crypto.randomUUID();
    pending.current = id;
    const stamp = generation.current;
    setBusy(true); setError(''); setPanel('results');
    update(target.id, { result: undefined,executionError:undefined });
    try {
      if(!target.table&&(cursor!==undefined||batch)){
        const parsed=await call<{statements:string[]}>('cellar_sql',{connection,sql:selection??target.sql,operation:batch||selection?'all':'statement',cursor:selection?0:cursor});
        const results:Result[]=[];
        for(const sql of parsed.statements){
          const result=await call<Result>('cellar_query',{connection,sql,limit:100,requestId:id});
          if(stamp!==generation.current||pending.current!==id)return;
          results.push(result);update(target.id,{result,batch:[...results],executedAt:new Date().toISOString(),executionError:undefined});
          setHistory(previous=>[sql,...previous.filter(item=>item!==sql)].slice(0,100));
        }
        return;
      }
      const result = await call<Result>(target.table ? 'cellar_browse' : 'cellar_query', target.table ? {
        connection, schema: target.table.schema, table: target.table.name, filters: target.filters,
        sort: target.sort, descending: target.descending, offset: target.offset, limit: 100, requestId: id,
      } : { connection, sql: selection ?? target.sql, limit: 100, requestId: id });
      if (stamp === generation.current && pending.current === id) {
        update(target.id, { result,batch:undefined,executedAt:new Date().toISOString(),executionError:undefined });
        if (!target.table) setHistory(previous => [selection ?? target.sql, ...previous.filter(sql => sql !== (selection ?? target.sql))].slice(0, 20));
      }
    } catch (error) { if (stamp === generation.current && pending.current === id) {const message=error instanceof Error?error.message:'Query failed';setError(message);update(target.id,{executionError:message,executedAt:new Date().toISOString()});} }
    finally { if (pending.current === id) { pending.current = undefined; setBusy(false); } }
  }
  function reload(patch: Partial<Tab>) {
    if (!tab || busy) return;
    if(tab.changes?.length){setError('Commit or discard pending changes before refreshing, sorting or paging.');return;}
    const target = { ...tab, ...patch };
    update(tab.id, patch);
    void execute(target);
  }
  function open(table?: Table) {
    if (busy) return;
    const existing = table && tabs.find(tab => tab.table?.schema === table.schema && tab.table.name === table.name);
    const next = existing || fresh(table);
    if (!existing) setTabs(previous => [...previous, next]);
    setActive(next.id); setPanel('results');
  }
  async function refreshSchemas(){
    if(busy||hasPending.current||!connection)return;
    const stamp=generation.current;setBusy(true);
    try{const result=await call<{tables:Table[]}>('cellar_schema',{connection});if(stamp===generation.current)setTables(result.tables);}
    catch(error){if(stamp===generation.current)setError(error instanceof Error?error.message:'Cannot refresh schemas');}
    finally{if(stamp===generation.current)setBusy(false);}
  }
  function closeTabs(ids:string[]) {
    if(tabs.some(tab=>ids.includes(tab.id)&&tab.changes?.length)){setError('Commit or discard pending changes before closing tabs.');return;}
    const remaining=tabs.filter(tab=>!ids.includes(tab.id));setTabs(remaining);if(ids.includes(active))setActive(remaining[0]?.id??'');
  }
  function showTabMenu(event:React.MouseEvent,item:Tab) {
    event.preventDefault();const others=tabs.filter(tab=>tab.id!==item.id).map(tab=>tab.id);const right=tabs.slice(tabs.findIndex(tab=>tab.id===item.id)+1).map(tab=>tab.id);
    setTabMenu({x:event.clientX,y:event.clientY,items:[
      {label:'New SQL query',icon:'terminal',run:()=>open(),disabled:busy},
      ...(item.table?[{label:'Refresh',icon:'book-open',run:()=>void execute(item),disabled:busy||Boolean(item.changes?.length)},{label:'Query SELECT *',icon:'terminal'}]:[]),
      {label:item.table?'Copy qualified name':'Copy title',icon:'copy',run:()=>{void navigator.clipboard.writeText(item.table?`${item.table.schema}.${item.table.name}`:item.title).catch(error=>setError(error.message));}},
      {label:'Close',icon:'close',run:()=>closeTabs([item.id]),disabled:busy},
      {label:'Close Others',icon:'close',run:()=>closeTabs(others),disabled:busy||!others.length},
      {label:'Close Tabs to the Right',icon:'close',run:()=>closeTabs(right),disabled:busy||!right.length}
    ]});
  }
  function filter() {
    if (!column || !tab) return;
    reload({ filters: [...tab.filters, { column, operator, value }], offset: 0 });
    setValue('');
  }
  function stage(rowIndex:number,column?:string,value?:string|null) {
    if(!tab?.result||!editable)return;
    const previous=pendingChanges.find(change=>change.rowIndex===rowIndex);
    if(previous?.kind==='delete'&&column){setError('Discard the staged row deletion before editing that row.');return;}
    const change:Change={kind:column?'update':'delete',rowIndex,original:previous?.original??tab.result.rows[rowIndex],revision:previous?.revision??tab.result.revisions?.[rowIndex],values:column?{...previous?.values,[column]:value??null}:{}};
    update(tab.id,{changes:[...pendingChanges.filter(change=>change.rowIndex!==rowIndex),change]});
  }
  function savedConnections(next:Connection[]){setConnections(next);setConnection(next.find(item=>item.id===editingConnection)?.id??next.at(-1)?.id??'');setProfileRevision(previous=>previous+1);setNotice('Connection saved. Select a table or open SQL to connect.');}
  const displayResult=tab?.result?{...tab.result,columns:tab.result.columns.map(column=>({...column,primaryKey:tab.table?.columns.find(item=>item.name===column.name)?.primaryKey??column.primaryKey})),rows:tab.result.rows.map((row,index)=>row.map((value,column)=>{const change=pendingChanges.find(change=>change.rowIndex===index&&change.kind==='update');const name=tab.result!.columns[column].name;return change&&name in change.values?change.values[name]:value;}))}:undefined;
  return <div className="workspace">
    <Titlebar current={current} table={tab?.table} onPalette={()=>setPalette(true)} onSidebar={()=>setSidebarVisible(!sidebarVisible)} onBottom={()=>setBottomVisible(!bottomVisible)} onExpand={()=>void fullscreen().catch(error=>setError(error.message))}/>
    <div className="body" style={{'--sidebar-width':`${sidebarWidth}px`} as React.CSSProperties}>
      {sidebarVisible&&<Sidebar connections={connections} connection={connection} tables={tables} tab={tab} busy={busy} pending={hasPending.current} search={search} onSearch={setSearch} onConnection={id=>{if(hasPending.current){setError('Commit or discard pending changes before switching connections.');return;}setConnection(id);}} onOpen={open} onAdd={()=>setConnectionEditor(true)} onProject={()=>setProjectDialog(true)} onEdit={id=>{setEditingConnection(id??connection);setConnectionEditor(true);}} onRefresh={()=>void refreshSchemas()} onQuery={()=>open()} onError={setError}/>}
      {sidebarVisible&&<div className="sidebar-resizer" role="separator" aria-label="Resize sidebar" aria-orientation="vertical" onPointerDown={resizeSidebar}/>}
      <main className={tab&&!tab.table?'query-main':'table-main'}><nav className="tabs">{tabs.map(item => <div className={`tab ${item.id === active ? 'active' : ''}`} key={item.id} onContextMenu={event=>showTabMenu(event,item)}><button disabled={busy} onClick={() => { setActive(item.id); setPanel('results'); }}><Icon name={item.table?'table':'terminal'}/>{item.title}{item.changes?.length?' ●':''}</button><button aria-label={`Close ${item.title}`} disabled={busy} onClick={() => {if(item.changes?.length){setError('Commit or discard this tab’s pending changes before closing.');return;}const remaining = tabs.filter(tab => tab.id !== item.id); setTabs(remaining); if (active === item.id) setActive(remaining[0]?.id ?? ''); }}>×</button></div>)}<button disabled={busy || !connection} onClick={() => open()} aria-label="New query">＋</button></nav>
        {notice&&<div className="notice">{notice}<button onClick={()=>setNotice('')}>×</button></div>}
        {!tab && <div className="empty">{busy ? 'Reading schema…' : 'Select a table or open a SQL query.'}</div>}
        {tab && <>
          {!tab.table && <><div className="query-toolbar"><button className="primary" disabled={busy} onClick={() => void execute(tab)}>▶ Run query</button><button disabled={busy} onClick={()=>void execute(tab,undefined,undefined,true)}>Run all (max 10)</button><button disabled={busy} onClick={()=>setShowPlan(true)}>Explain</button><span>⌘ / Ctrl + Enter · run selection or statement at cursor</span></div><Editor value={tab.sql} engine={current?.engine ?? 'sqlite'} tables={tables} onChange={sql => update(tab.id, { sql })} onRun={(selection,cursor) => void execute(tab, selection,cursor)} /></>}
          {tab.table && <><TableFilters connection={connection} tab={tab} busy={busy} column={column} operator={operator} value={value} onColumn={setColumn} onOperator={setOperator} onValue={setValue} onFilter={filter} onReload={reload}/>{tab.filters.length > 0 && <div className="chips">{tab.filters.map((filter, index) => <button disabled={busy} key={index} onClick={() => reload({ filters: tab.filters.filter((_, i) => index !== i), offset: 0 })}>{filter.column} {filter.operator} {filter.value} ×</button>)}<button disabled={busy} onClick={() => reload({ filters: [], offset: 0 })}>Clear all</button></div>}</>}

          {error && <div role="alert" className="error">{error}</div>}
          {tab.batch&&tab.batch.length>1&&<div className="chips">{tab.batch.map((result,index)=><button key={index} onClick={()=>update(tab.id,{result})}>Statement {index+1} · {result.rows.length} rows · {result.durationMs} ms</button>)}</div>}
          {pendingChanges.some(change=>change.kind==='insert')&&<div className="chips">{pendingChanges.filter(change=>change.kind==='insert').map((change,index)=><span key={index}>＋ Pending insert: {JSON.stringify(change.values)}</span>)}</div>}
          {tab.table && panel !== 'structure' && (displayResult ? <Grid key={`grid:${tab.id}`} result={displayResult} changed={pendingChanges.flatMap(change=>change.rowIndex===undefined?[]:[change.rowIndex])} onEdit={editable?(row,column,value)=>stage(row,column,value):undefined} onDelete={editable?row=>stage(row):undefined} sort={tab.sort} descending={tab.descending} onSort={tab.table ? column => reload({ sort: column, descending: tab.sort === column ? !tab.descending : false, offset: 0 }) : undefined} /> : <div className="empty">{busy ? 'Loading…' : 'Run a query to inspect results.'}</div>)}
          {panel === 'structure' && tab.table && <Structure connection={connection} table={tab.table} onOpen={(schema,name)=>{const target=tables.find(table=>table.schema===schema&&table.name===name);if(target)open(target);else setError('Referenced table is outside the visible schema.');}}/>}

          {tab.table&&<GridFooter tab={tab} busy={busy} editable={editable} onInsert={()=>setInserting(true)} onDiscard={()=>update(tab.id,{changes:[]})} onReview={()=>setReview(true)} onPage={offset=>reload({offset})}/>}
          {bottomVisible&&<BottomPanel key={`panel:${tab.id}`} tab={tab} connection={connection} busy={busy} error={error} notice={notice} history={history} onResults={()=>setPanel('results')} onHistory={sql=>{const next={...fresh(),sql};setTabs(previous=>[...previous,next]);setActive(next.id);setPanel('results');}} onStructure={tab.table?()=>setPanel(panel==='structure'?'results':'structure'):undefined} onDownload={current?.engine==='sqlite'&&!current.project?()=>{setBusy(true);void downloadDatabase(connection,current.name).catch(error=>setError(error.message)).finally(()=>setBusy(false));}:undefined} onCancel={()=>{if(pending.current)void call('cellar_cancel',{requestId:pending.current}).catch(error=>setError(error.message));}}/>}
        </>}
        {!tab && error && <div role="alert" className="error">{error}</div>}
      </main>
    </div><footer><span className="status-connection"><span className="connection-state">●</span>{current?.name??'Disconnected'}<span className="status-engine">{current?.engine.toUpperCase()}</span><span className="readonly">{current?.readOnly?'READ ONLY':'REVIEWED WRITES'}</span></span><span>✓ {tab?.result?.rows.length??0} rows · {tab?.result?.durationMs??0} ms · UTF-8 · LF</span></footer>
    {palette&&<WorkspaceSearch tables={tables} tabs={tabs} busy={busy} onTable={open} onTab={id=>{setActive(id);setPanel('results');}} onClose={()=>setPalette(false)}/>}
    {tabMenu&&<ContextMenu {...tabMenu} label="Tab menu" onClose={()=>setTabMenu(undefined)}/>}
    {connectionEditor&&<Connections existing={editingConnection} onSaved={savedConnections} onClose={()=>{setConnectionEditor(false);setEditingConnection(undefined);}}/>}
    {projectDialog&&<ProjectConnections onClose={()=>setProjectDialog(false)} onConnected={(next,preferred)=>{setConnections(next);setConnection(preferred??next[0]?.id??'');setProfileRevision(previous=>previous+1);setNotice(preferred?'Project connection opened read-only for this local session. Credentials are not saved.':'Project connections forgotten. Session credentials were cleared.');}}/>}
    {review&&tab?.table&&<Review connection={connection} table={tab.table} changes={pendingChanges} production={current?.environment==='prod'} onClose={()=>setReview(false)} onDone={()=>{setReview(false);update(tab.id,{changes:[]});setNotice('Transaction committed successfully.');void execute({...tab,changes:[]});}}/>}
    {inserting&&tab?.table&&<InsertRow table={tab.table} onClose={()=>setInserting(false)} onStage={values=>update(tab.id,{changes:[...pendingChanges,{kind:'insert',values}]})}/>}
    {showPlan&&tab&&!tab.table&&<QueryPlan connection={connection} sql={tab.sql} onClose={()=>setShowPlan(false)}/>}
  </div>;
}

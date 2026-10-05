import {ContextMenu,type MenuAction} from './ContextMenu.js';
import {useState} from 'react';
import {icons} from 'cellar-native-icons';
import {ConnectionGroups} from './ConnectionGroups.js';
import {ChatFiles} from './ChatFiles.js';
import type {Connection,Table,Tab} from './types.js';

export function Icon({name}:{name:string}) {
  return <span className="native-icon" aria-hidden="true" dangerouslySetInnerHTML={{__html:icons[name]??icons.table}}/>;
}
export function Titlebar({current,table,onPalette,onSidebar,onExpand,onBottom}:{current?:Connection;table?:Table;onPalette:()=>void;onSidebar:()=>void;onExpand:()=>void;onBottom:()=>void}) {
  return <header className="titlebar"><div className="title-crumbs"><Icon name="database"/><span>{current?.name??'Cellar'}</span><span className="separator">›</span><span className="engine-dot"/><span>{current?.engine??'workspace'}</span><span className="separator">›</span><Icon name="folder"/><span>{table?.schema??'SQL'}</span></div><button className="title-search" aria-label="Search workspace" onClick={onPalette}><Icon name="search"/><span>Search tables, columns, queries…</span><kbd>⌘</kbd><kbd>K</kbd></button><div className="title-actions"><button title="Toggle sidebar" aria-label="Toggle sidebar" onClick={onSidebar}><Icon name="panel-left"/></button><button title="Toggle execution panel" aria-label="Toggle execution panel" onClick={onBottom}><Icon name="panel-bottom"/></button><button title="Expand" aria-label="Expand" onClick={onExpand}><Icon name="expand"/></button></div></header>;
}
export function Sidebar({connections,connection,tables,tab,busy,pending,search,onSearch,onConnection,onOpen,onAdd,onProject,onEdit,onQuery,onRefresh,onError}:{connections:Connection[];connection:string;tables:Table[];tab?:Tab;busy:boolean;pending:boolean;search:string;onSearch:(value:string)=>void;onConnection:(id:string)=>void;onOpen:(table:Table)=>void;onAdd:()=>void;onProject:()=>void;onEdit:(id?:string)=>void;onQuery:()=>void;onRefresh:()=>void;onError:(error:string)=>void}) {
  const [closed,setClosed]=useState(new Set<string>());
  const [menu,setMenu]=useState<{x:number;y:number;label:string;items:MenuAction[]}>();
  const showMenu=(event:React.MouseEvent,label:string,items:MenuAction[])=>{event.preventDefault();setMenu({x:event.clientX,y:event.clientY,label,items});};
  const copy=(value:string)=>()=>{void navigator.clipboard.writeText(value).catch(error=>onError(error.message));};
  const [organizing,setOrganizing]=useState(false);
  const toggle=(id:string)=>setClosed(previous=>{const next=new Set(previous);if(next.has(id))next.delete(id);else next.add(id);return next;});
  const current=connections.find(item=>item.id===connection);
  return <aside className="native-sidebar"><div className="sidebar-heading"><strong>CONNECTIONS</strong><span className="count-badge">{connections.length}</span><button aria-label="＋ Add connection" title="New connection" disabled={busy||pending} onClick={onAdd}><Icon name="plus"/></button><button aria-label="Connection actions" onClick={event=>showMenu(event,'Connection actions',[
    {label:'New connection',icon:'plus',run:onAdd,disabled:busy||pending},
    {label:'New folder',icon:'folder-plus',run:()=>setOrganizing(true)},
    {label:'Import from DataGrip',reason:'Desktop profile import is unavailable; it would require separate credential access'},
    {label:'Import from TablePlus',reason:'Desktop profile import is unavailable; it would require separate credential access'},
    {label:'Refresh connected schemas',icon:'history',run:onRefresh,disabled:busy||pending||!connection},
    {label:'Organize connections…',icon:'folder',run:()=>setOrganizing(true)}
  ])}>⋯</button></div>{menu&&<ContextMenu {...menu} onClose={()=>setMenu(undefined)}/>}<label className="sidebar-search"><Icon name="search"/><input aria-label="Search tables" placeholder="Filter…" value={search} onChange={event=>onSearch(event.target.value)}/><kbd>⌘F</kbd></label><select className="screen-reader-only" aria-label="Connection" value={connection} disabled={busy} onChange={event=>onConnection(event.target.value)}>{connections.map(item=><option key={item.id} value={item.id}>{item.name}</option>)}</select><div className="tree"><ConnectionGroups connections={connections} organizing={organizing} onClose={()=>setOrganizing(false)} renderItem={(item,removeFromFolder)=><div key={item.id}><button className="connection-row" onContextMenu={event=>showMenu(event,'Connection menu',[
    {label:'New SQL query',icon:'terminal',run:onQuery,disabled:busy||pending||item.id!==connection},
    {label:'Edit…',icon:'edit',run:()=>onEdit(item.id),disabled:busy||pending||!item.managed},
    {label:'Duplicate',icon:'copy',reason:'Duplicating credentials is not supported in this extension'},
    {label:'Move to folder…',icon:'folder',run:()=>setOrganizing(true)},
    {label:'Move to new folder',icon:'folder-plus',run:()=>setOrganizing(true)},
    ...(removeFromFolder?[{label:'Remove from folder',icon:'folder',run:removeFromFolder}]:[]),
    {label:'Reconnect',icon:'history',run:onRefresh,disabled:busy||pending||item.id!==connection},
    {label:item.id===connection?'Disconnect':'Connect',icon:'power',run:item.id!==connection?()=>onConnection(item.id):undefined,disabled:busy||pending},
    {label:'Remove',icon:'trash',run:()=>onEdit(item.id),disabled:busy||pending||!item.managed,reason:'Opens connection options for typed removal confirmation'}
  ])} disabled={busy} onClick={()=>{if(item.id!==connection)onConnection(item.id);else toggle(item.id);}}><span className={`tree-chevron ${item.id===connection&&!closed.has(item.id)?'expanded':''}`}>›</span><Icon name="database"/><span>{item.name}</span><span className={`connection-state ${item.environment==='prod'?'production':''}`}>●</span></button>{item.id===connection&&!closed.has(item.id)&&[...new Set(tables.map(table=>table.schema))].map(schema=><div key={schema}><button className="schema-name" onContextMenu={event=>showMenu(event,'Schema menu',[
    {label:'New SQL query',icon:'terminal',run:onQuery,disabled:busy||pending},
    {label:'Compare schema…',icon:'diff'},
    {label:'Open ER diagram',icon:'diagram'},
    {label:'Hide from sidebar',icon:'eye-off'},
    {label:'Copy qualified name',icon:'copy',run:copy(schema)},
    {label:'Copy name',icon:'copy',run:copy(schema)}
  ])} onClick={()=>toggle(schema)}><span className={`tree-chevron ${closed.has(schema)?'':'expanded'}`}>›</span><Icon name="folder"/>{schema}<small>{tables.filter(table=>table.schema===schema).length}</small></button>{!closed.has(schema)&&['table','view'].map(kind=><div key={kind}><button className="tree-group" onClick={()=>toggle(`${schema}:${kind}`)}><span className={`tree-chevron ${closed.has(`${schema}:${kind}`)?'':'expanded'}`}>›</span><Icon name="folder"/>{kind==='table'?'tables':'views'}<small>{tables.filter(table=>table.schema===schema&&table.kind===kind).length}</small></button>{!closed.has(`${schema}:${kind}`)&&tables.filter(table=>table.schema===schema&&table.kind===kind&&table.name.toLowerCase().includes(search.toLowerCase())).map(table=><button aria-label={`▦ ${table.name}`} disabled={busy} className={`tree-table ${tab?.table===table?'active':''}`} key={table.name} onContextMenu={event=>showMenu(event,'Table menu',[
    {label:'Open',icon:'table',run:()=>onOpen(table),disabled:busy},
    {label:'Query SELECT *',icon:'terminal'},
    {label:'Import data…',icon:'upload'},
    {label:'Find Usages',icon:'search'},
    {label:'Copy qualified name',icon:'copy',run:copy(`${table.schema}.${table.name}`)},
    {label:'Copy name',icon:'copy',run:copy(table.name)}
  ])} onClick={()=>onOpen(table)}><Icon name={kind==='view'?'view':'table'}/><span>{table.name}</span></button>)}</div>)}</div>)}</div>}/></div><div className="sidebar-bottom"><button disabled={busy||pending} onClick={onProject}><Icon name="folder"/>Connect from this project</button><button disabled={busy||!connection} onClick={onQuery}><Icon name="terminal"/>＋ New SQL query</button><ChatFiles disabled={busy||pending} onError={onError}/>{current?.managed&&<button disabled={busy||pending} onClick={()=>onEdit()}>Edit connection</button>}<button onClick={()=>{document.documentElement.dataset.theme=document.documentElement.dataset.theme==='light'?'dark':'light';}}><Icon name="settings"/>Theme</button></div></aside>;
}

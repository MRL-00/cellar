import {useState} from 'react';
import {Icon} from './Shell.js';
import type {Table,Tab} from './types.js';
import {useModalFocus} from './Modal.js';

export function WorkspaceSearch({tables,tabs,busy,onTable,onTab,onClose}:{tables:Table[];tabs:Tab[];busy:boolean;onTable:(table:Table)=>void;onTab:(id:string)=>void;onClose:()=>void}) {
  const [query,setQuery]=useState('');
  const modalRef=useModalFocus(onClose);
  const needle=query.toLowerCase().trim();
  const found=tables.filter(table=>`${table.schema}.${table.name} ${table.columns.map(column=>column.name).join(' ')}`.toLowerCase().includes(needle)).slice(0,40);
  const queries=tabs.filter(tab=>!tab.table&&`${tab.title} ${tab.sql}`.toLowerCase().includes(needle)).slice(0,20);
  function chooseTable(table:Table){onTable(table);onClose();}
  function chooseTab(id:string){onTab(id);onClose();}
  return <div className="modal-backdrop" onKeyDown={event=>{if(event.key==='Escape')onClose();}}><section ref={modalRef} role="dialog" aria-modal="true" aria-label="Search workspace" className="workspace-search"><label><Icon name="search"/><input autoFocus aria-label="Search tables, columns, queries" placeholder="Search tables, columns, queries…" value={query} onChange={event=>setQuery(event.target.value)} onKeyDown={event=>{if(event.key==='Enter'&&!busy){if(found[0])chooseTable(found[0]);else if(queries[0])chooseTab(queries[0].id);}}}/><button aria-label="Close workspace search" onClick={onClose}>Esc</button></label><div className="workspace-search-results">{found.map(table=><button key={`${table.schema}.${table.name}`} disabled={busy} onClick={()=>chooseTable(table)}><Icon name="table"/><span>{table.schema}.{table.name}<small>{table.columns.filter(column=>column.name.toLowerCase().includes(needle)).map(column=>column.name).join(', ')}</small></span></button>)}{queries.map(tab=><button key={tab.id} disabled={busy} onClick={()=>chooseTab(tab.id)}><Icon name="terminal"/>{tab.title}</button>)}{!found.length&&!queries.length&&<p>No matches in the connected schema or open queries.</p>}</div></section></div>;
}

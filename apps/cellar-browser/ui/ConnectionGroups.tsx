import {ContextMenu,type MenuAction} from './ContextMenu.js';
import {Modal} from './Modal.js';
import {useState,type ReactNode} from 'react';
import {Icon} from './Shell.js';
import {readPreference,writePreference} from './preferences.js';
import type {Connection} from './types.js';

type Group={id:string;name:string;collapsed:boolean;color?:string};
type Layout={groups:Group[];assignments:Record<string,string>};
export function ConnectionGroups({connections,renderItem,organizing,onClose}:{connections:Connection[];renderItem:(connection:Connection,removeFromFolder?:()=>void)=>ReactNode;organizing:boolean;onClose:()=>void}) {
  const [layout,setLayout]=useState<Layout>(()=>readPreference('sidebar.v1',{groups:[],assignments:{}}));
  const [menu,setMenu]=useState<{x:number;y:number;items:MenuAction[]}>();
  const [editing,setEditing]=useState(false);
  const [name,setName]=useState('');
  const [renaming,setRenaming]=useState('');
  const [warning,setWarning]=useState('');
  function save(next:Layout){setLayout(next);if(!writePreference('sidebar.v1',next))setWarning('Folder changes are held for this session only. Browser storage is unavailable.');}
  function assign(id:string,group:string){save({...layout,assignments:{...layout.assignments,[id]:group}});}
  function create(){const title=name.trim();if(!title||layout.groups.length>=32)return;save({...layout,groups:renaming?layout.groups.map(group=>group.id===renaming?{...group,name:title}:group):[...layout.groups,{id:crypto.randomUUID(),name:title,collapsed:false}]});setName('');setRenaming('');}
  function remove(id:string){save({...layout,groups:layout.groups.filter(group=>group.id!==id),assignments:Object.fromEntries(Object.entries(layout.assignments).filter(([,group])=>group!==id))});}
  function setColor(id:string,color?:string){save({...layout,groups:layout.groups.map(group=>group.id===id?{...group,color}:group)});}
  function showMenu(event:React.MouseEvent,group:Group){event.preventDefault();const x=event.clientX,y=event.clientY;setMenu({x,y,items:[
    {label:'Rename folder',icon:'edit',run:()=>{setName(group.name);setRenaming(group.id);setEditing(true);}},
    {label:'Set color…',icon:'folder',run:()=>setMenu({x,y,items:[['#4f8ff7','Blue'],['#f6a44a','Orange'],['#d97a5a','Coral'],['#5bb8e0','Cyan'],['#a78bfa','Purple'],['#4ade80','Green'],['#f87171','Red']].map(([color,label])=>({label,icon:'folder',run:()=>setColor(group.id,color)})).concat([{label:'Clear color',icon:'close',run:()=>setColor(group.id)}])})},
    {label:connections.some(connection=>layout.assignments[connection.id]===group.id)?'Remove folder (keep connections)':'Remove folder',icon:'trash',run:()=>remove(group.id)}
  ]});}
  const known=new Set(layout.groups.map(group=>group.id));
  function item(connection:Connection){return <div key={connection.id} draggable onDragStart={event=>event.dataTransfer.setData('application/x-cellar-connection',connection.id)}>{renderItem(connection,known.has(layout.assignments[connection.id])?()=>assign(connection.id,''):undefined)}</div>;}
  return <>
    {menu&&<ContextMenu {...menu} label="Folder menu" onClose={()=>setMenu(undefined)}/>}
    {layout.groups.map(group=>{const members=connections.filter(connection=>layout.assignments[connection.id]===group.id);return <div className="connection-folder" key={group.id} onDragOver={event=>event.preventDefault()} onDrop={event=>{const id=event.dataTransfer.getData('application/x-cellar-connection');if(connections.some(connection=>connection.id===id))assign(id,group.id);}}><button className="folder-row" style={{color:group.color}} onContextMenu={event=>showMenu(event,group)} aria-expanded={!group.collapsed} onClick={()=>save({...layout,groups:layout.groups.map(item=>item.id===group.id?{...item,collapsed:!item.collapsed}:item)})}><span className={`tree-chevron ${group.collapsed?'':'expanded'}`}>›</span><Icon name={group.collapsed?'folder':'folder-open'}/><span>{group.name}</span><small>{members.length}</small></button>{!group.collapsed&&<div className="folder-children">{members.map(item)}</div>}</div>;})}
    <div onDragOver={event=>event.preventDefault()} onDrop={event=>{const id=event.dataTransfer.getData('application/x-cellar-connection');if(connections.some(connection=>connection.id===id))assign(id,'');}}>{connections.filter(connection=>!known.has(layout.assignments[connection.id])).map(item)}</div>
    {(organizing||editing)&&<Modal title="Connection folders" label="Organize connections" className="connection-dialog" onClose={()=>{setEditing(false);onClose();}}><p>Organize this workspace. Folder names and membership are saved in this browser.</p>{warning&&<p role="status">{warning}</p>}<form onSubmit={event=>{event.preventDefault();create();}}><label>Folder name<input aria-label="Folder name" value={name} maxLength={80} onChange={event=>setName(event.target.value)} autoFocus/></label><button disabled={!name.trim()} type="submit">{renaming?'Rename folder':'New folder'}</button>{renaming&&<button type="button" onClick={()=>{setRenaming('');setName('');}}>Cancel rename</button>}</form><div className="folder-editor">{layout.groups.map(group=><div key={group.id}><Icon name="folder"/><span>{group.name}</span><button onClick={()=>{setRenaming(group.id);setName(group.name);}}>Rename {group.name}</button><button onClick={()=>remove(group.id)}>Remove {group.name}</button></div>)}</div>{connections.map(connection=><label key={connection.id} className="folder-assignment"><span>{connection.name}</span><select aria-label={`Folder for ${connection.name}`} value={known.has(layout.assignments[connection.id])?layout.assignments[connection.id]:''} onChange={event=>assign(connection.id,event.target.value)}><option value="">Ungrouped</option>{layout.groups.map(group=><option value={group.id} key={group.id}>{group.name}</option>)}</select></label>)}<button onClick={()=>{setEditing(false);onClose();}}>Done</button></Modal>}
  </>;
}

import {useEffect,useRef} from 'react';
import {Icon} from './Shell.js';

export type MenuAction={label:string;icon?:string;run?:()=>void;disabled?:boolean;reason?:string};
export function ContextMenu({x,y,label,items,onClose}:{x:number;y:number;label:string;items:MenuAction[];onClose:()=>void}) {
  const ref=useRef<HTMLDivElement>(null);
  useEffect(()=>{
    const previous=document.activeElement as HTMLElement|null;
    const node=ref.current;if(!node)return;
    const bounds=node.getBoundingClientRect();
    node.style.left=`${Math.max(4,Math.min(x,innerWidth-bounds.width-4))}px`;
    node.style.top=`${Math.max(4,Math.min(y,innerHeight-bounds.height-4))}px`;
    node.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus();
    return()=>{if(previous?.isConnected)previous.focus();};
  },[]);
  return <><button className="menu-dismiss" aria-label={`Close ${label}`} onClick={onClose}/><div ref={ref} className="native-menu context-menu" role="menu" aria-label={label} style={{left:x,top:y}} onKeyDown={event=>{
    if(event.key==='Escape'){event.preventDefault();onClose();}
    if(['ArrowDown','ArrowUp','Home','End'].includes(event.key)){event.preventDefault();const buttons=Array.from(ref.current?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')??[]);const index=buttons.indexOf(document.activeElement as HTMLButtonElement);const next=event.key==='Home'?0:event.key==='End'?buttons.length-1:(index+(event.key==='ArrowDown'?1:-1)+buttons.length)%buttons.length;buttons[next]?.focus();}
  }}>{items.map(item=><button role="menuitem" key={item.label} disabled={item.disabled||!item.run} title={item.reason??(!item.run?'Not available in the Codex extension':undefined)} onClick={()=>{onClose();item.run?.();}}><Icon name={item.icon??'database'}/>{item.label}</button>)}</div></>;
}

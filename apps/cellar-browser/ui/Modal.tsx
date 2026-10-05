import {useEffect,useRef,type ReactNode} from 'react';
import {Icon} from './Shell.js';

export function useModalFocus(onClose:()=>void,busy=false) {
  const ref=useRef<HTMLElement>(null);
  useEffect(()=>{
    const previous=document.activeElement as HTMLElement|null;
    const node=ref.current;if(!node)return;
    const focusable=()=>Array.from(node.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),select:not(:disabled),textarea:not(:disabled),[tabindex="0"]')).filter(item=>item.getClientRects().length>0);
    if(!node.contains(document.activeElement))focusable()[0]?.focus();
    const key=(event:KeyboardEvent)=>{
      if(event.key==='Escape'){event.preventDefault();event.stopPropagation();if(!busy)onClose();}
      if(event.key==='Tab'){
        const items=focusable();const first=items[0];const last=items.at(-1);
        if(event.shiftKey&&document.activeElement===first){event.preventDefault();last?.focus();}
        else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first?.focus();}
      }
    };
    node.addEventListener('keydown',key);
    return()=>{node.removeEventListener('keydown',key);if(previous?.isConnected)previous.focus();};
  },[busy]);
  return ref;
}

export function Modal({title,label=title,className='',busy=false,onClose,children}:{title:string;label?:string;className?:string;busy?:boolean;onClose:()=>void;children:ReactNode}) {
  const ref=useModalFocus(onClose,busy);
  return <div className="modal-backdrop"><section ref={ref} role="dialog" aria-modal="true" aria-label={label} className={`native-modal ${className}`}><div className="modal-heading"><strong>{title}</strong><button aria-label={`Close ${label}`} disabled={busy} onClick={onClose}><Icon name="close"/></button></div><div className="modal-content">{children}</div></section></div>;
}

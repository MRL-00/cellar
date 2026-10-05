import {Modal} from './Modal.js';
import {useState} from 'react';
import {Icon} from './Shell.js';
import {readPreference,writePreference} from './preferences.js';
import type {Filter,Tab} from './types.js';

type Preset={name:string;filters:Filter[];sort?:string;descending:boolean};
export function Presets({connection,tab,busy,onReload}:{connection:string;tab:Tab;busy:boolean;onReload:(patch:Partial<Tab>)=>void}) {
  const storage=`presets.v1.${connection}.${tab.table?.schema}.${tab.table?.name}`;
  const [presets,setPresets]=useState<Preset[]>(()=>readPreference(storage,[]));
  const [open,setOpen]=useState(false);
  const [draft,setDraft]=useState(false);
  const [name,setName]=useState('');
  const [warning,setWarning]=useState('');
  const active=presets.find(preset=>JSON.stringify(preset.filters)===JSON.stringify(tab.filters)&&preset.sort===tab.sort&&preset.descending===tab.descending);
  function save(next:Preset[]){setPresets(next);if(!writePreference(storage,next))setWarning('Presets are held for this session only. Browser storage is unavailable.');}
  function commit(){const title=name.trim();if(!title)return;const next={name:title,filters:tab.filters,sort:tab.sort,descending:tab.descending};save([...presets.filter(preset=>preset.name!==title),next].slice(-50));setDraft(false);setName('');}
  return <div className="presets"><button className={`preset-trigger ${active?'selected':''}`} disabled={busy||Boolean(tab.changes?.length)} aria-expanded={open} aria-label="Presets" onClick={()=>setOpen(!open)}><Icon name="bookmark"/>{active?.name??'Presets'}<span>⌄</span></button>{open&&<><button className="menu-dismiss" aria-label="Close presets" onClick={()=>setOpen(false)}/><div className="native-menu preset-menu" role="menu" aria-label="Filter presets">{presets.map(preset=><div className="preset-option" key={preset.name}><button role="menuitem" onClick={()=>{onReload(active?.name===preset.name?{filters:[],sort:undefined,descending:false,offset:0}:{...preset,offset:0});setOpen(false);}}><span>{active?.name===preset.name?'✓':''}</span>{preset.name}</button><button aria-label={`Delete preset ${preset.name}`} onClick={()=>save(presets.filter(item=>item.name!==preset.name))}>×</button></div>)}{presets.length>0&&<hr/>}<button role="menuitem" onClick={()=>{setOpen(false);setDraft(true);}}><Icon name="bookmark"/>Save current as preset…</button><button role="menuitem" disabled={busy} onClick={()=>{onReload({filters:[],sort:undefined,descending:false,offset:0});setOpen(false);}}>Clear current filters</button></div></>}{draft&&<Modal title="Save current as preset" label="Save filter preset" className="preset-dialog" onClose={()=>setDraft(false)}><form onSubmit={event=>{event.preventDefault();commit();}}><label>Preset name<input aria-label="Preset name" placeholder="preset name" maxLength={80} autoFocus value={name} onChange={event=>setName(event.target.value)}/></label><p>Filters and sorting for this table are saved in this browser.</p><button type="submit" disabled={!name.trim()}>Save preset</button><button type="button" onClick={()=>setDraft(false)}>Cancel</button></form></Modal>}{warning&&<span role="status" className="preference-warning">{warning}</span>}</div>;
}

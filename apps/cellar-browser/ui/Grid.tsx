import {Modal} from './Modal.js';
import { useEffect, useMemo, useRef, useState } from 'react';
import {Icon} from './Shell.js';
import type { Cell, Result } from './types.js';

import {columnLayout,gutter,headerHeight,rowHeight as height} from './grid-layout.js';
export function Grid({ result, sort, descending, onSort, onEdit, onDelete, changed }: { result: Result; sort?: string; descending?: boolean; onSort?: (column: string) => void; onEdit?:(row:number,column:string,value:string|null)=>void;onDelete?:(row:number)=>void;changed?:number[] }) {
  const layout=useMemo(()=>columnLayout(result),[result]);
  const element = useRef<HTMLDivElement>(null);
  const [viewport,setViewport]=useState({width:1200,height:600});
  useEffect(()=>{const node=element.current;if(!node)return;const measure=()=>setViewport({width:node.clientWidth,height:node.clientHeight});const observer=new ResizeObserver(measure);observer.observe(node);measure();return()=>observer.disconnect();},[]);
  const [scroll, setScroll] = useState({ top: 0, left: 0 });
  const [selected, select] = useState<[number, number] | null>(null);
  const [detail, showDetail] = useState<{ name: string; value: Cell;row:number } | null>(null);
  const [edited,setEdited]=useState('');
  const [isNull,setNull]=useState(false);
  function inspect(row:number,column:number){const value=result.rows[row][column];showDetail({name:result.columns[column].name,value,row});setEdited(value===null?'':String(value));setNull(value===null);}
  const firstRow = Math.max(0, Math.floor(Math.max(0,scroll.top-headerHeight) / height) - 3);
  const lastRow = Math.min(result.rows.length, firstRow + Math.ceil(viewport.height / height) + 7);
  const firstColumn = Math.max(0,layout.findIndex(column=>column.left+column.width>scroll.left+gutter)-1);
  const edge=scroll.left+viewport.width;
  const end=layout.findIndex(column=>column.left>edge);
  const lastColumn=end<0?layout.length:Math.min(layout.length,end+1);
  const columns = result.columns.slice(firstColumn, lastColumn);
  function key(event: React.KeyboardEvent) {
    if (!selected) return;
    const [row, column] = selected;
    const next: [number, number] = [...selected];
    if (event.key === 'ArrowDown') next[0] = Math.min(result.rows.length - 1, row + 1);
    else if (event.key === 'ArrowUp') next[0] = Math.max(0, row - 1);
    else if (event.key === 'ArrowRight') next[1] = Math.min(result.columns.length - 1, column + 1);
    else if (event.key === 'ArrowLeft') next[1] = Math.max(0, column - 1);
    else if (event.key === 'Enter') { inspect(row,column); return; }
    else if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'c') { void navigator.clipboard?.writeText(String(result.rows[row][column] ?? 'NULL')); return; }
    else return;
    event.preventDefault();
    select(next);
    const box = element.current!;
    const top = headerHeight+next[0] * height;
    const {left,width} = layout[next[1]];
    if (top < box.scrollTop+headerHeight) box.scrollTop = top-headerHeight;
    else if (top + 2 * height > box.scrollTop + box.clientHeight) box.scrollTop = top + 2 * height - box.clientHeight;
    if (left < box.scrollLeft+gutter) box.scrollLeft = left-gutter;
    else if (left + width > box.scrollLeft + box.clientWidth) box.scrollLeft = left + width - box.clientWidth;
  }
  return <>
    <div className="grid-scroll" ref={element} role="grid" aria-label="Query results" aria-rowcount={result.rows.length + 1} aria-colcount={result.columns.length} tabIndex={0} onKeyDown={key} onScroll={event => setScroll({ top: event.currentTarget.scrollTop, left: event.currentTarget.scrollLeft })}>
      <div className="grid-canvas" style={{ width: layout.length?layout.at(-1)!.left+layout.at(-1)!.width:gutter, height: headerHeight+result.rows.length*height }}>
        <div className="grid-head" style={{ height:headerHeight }} role="row">
          <span className="row-number head-number">#</span>
          {columns.map((column, index) => <button key={firstColumn + index} role="columnheader" className="column-head" style={layout[firstColumn+index]} onClick={() => onSort?.(column.name)} disabled={!onSort} title={`${column.name} · ${column.type}`}>
            <span className={`column-title ${column.primaryKey?'primary-key':''}`}><Icon name={column.primaryKey?'type-key':/int|real|numeric|decimal|float|double/i.test(column.type)?'type-hash':/date|time/i.test(column.type)?'type-calendar':'type-text'}/><span>{column.name}{sort === column.name ? descending ? ' ↓' : ' ↑' : ''}</span></span><small>{column.type}</small>
          </button>)}
        </div>
        {result.rows.slice(firstRow, lastRow).map((row, index) => {
          const rowIndex = firstRow + index;
          return <div key={rowIndex} role="row" className={`grid-row ${changed?.includes(rowIndex)?'pending-row':''}`} style={{ top: headerHeight+rowIndex*height, height }} aria-rowindex={rowIndex + 2}>
            <span className="row-number">{rowIndex + 1}</span>
            {columns.map((column, columnIndex) => {
              const col = firstColumn + columnIndex;
              const value = row[col];
              return <div key={col} role="gridcell" aria-colindex={col + 1} aria-selected={selected?.[0] === rowIndex && selected[1] === col} className={`grid-cell ${value === null ? 'null' : ''}`} style={layout[col]} onClick={() => { select([rowIndex, col]); element.current?.focus(); }} onDoubleClick={() => inspect(rowIndex,col)}>
                {value === null ? 'NULL' : String(value)}
              </div>;
            })}
          </div>;
        })}
      </div>
      {result.rows.length === 0 && <div className="grid-empty">No rows matched.</div>}
    </div>
    {detail && <Modal title={detail.name} label="Cell editor" className="cell-dialog" onClose={()=>showDetail(null)}>{onEdit?<><textarea aria-label="Cell value" value={edited} disabled={isNull} onChange={e=>setEdited(e.target.value)} maxLength={16384}/><label className="checkbox"><input type="checkbox" aria-label="Set NULL" checked={isNull} onChange={e=>setNull(e.target.checked)}/>NULL</label><p>Staged locally until Review & Commit.</p><button className="primary" onClick={()=>{onEdit(detail.row,detail.name,isNull?null:edited);showDetail(null);}}>Stage cell edit</button>{onDelete&&<button className="danger" onClick={()=>{onDelete(detail.row);showDetail(null);}}>Stage row deletion</button>}</>:<pre>{detail.value === null ? 'NULL' : String(detail.value)}</pre>}<button onClick={() => showDetail(null)}>Close</button></Modal>}
  </>;
}

import type {Result} from './types.js';

// Cellar 1.0.10: grid/widths.rs, grid.rs and grid/view.rs.
export const gutter = 36;
export const headerHeight = 26;
export const rowHeight = 25;
export function columnLayout(result:Result) {
  let left=gutter;
  return result.columns.map((column,index)=>{
    const header=(Array.from(column.name).length+Array.from(column.type).length)*7.8+58;
    const longest=result.rows.slice(0,200).reduce((max,row)=>Math.max(max,Array.from(row[index]===null?'NULL':String(row[index])).length),0);
    const width=Math.max(64,Math.min(600,Math.max(header,longest*7.8+32)));
    const layout={left,width};left+=width;return layout;
  });
}

import { call } from './bridge.js';
import type { Result } from './types.js';

export function download(name:string,bytes:BlobPart[],type:string){
  const url=URL.createObjectURL(new Blob(bytes,{type}));
  const link=document.createElement('a');link.href=url;link.download=name;document.body.append(link);link.click();link.remove();setTimeout(()=>URL.revokeObjectURL(url),30000);
}

export function exportResult(result:Result,format:'csv'|'tsv'|'json',name:string){
  if(format==='json')download(`${name}.json`,[JSON.stringify(result.rows.map(row=>Object.fromEntries(result.columns.map((column,index)=>[column.name,row[index]]))),null,2)],'application/json');
  else {
    const quote=(value:unknown)=>`"${String(value??'').replaceAll('"','""')}"`;
    const rows=[result.columns.map(column=>column.name),...result.rows];
    download(`${name}.${format}`,['\uFEFF'+rows.map(row=>row.map(quote).join(format==='tsv'?'\t':',')).join('\r\n')],format==='tsv'?'text/tab-separated-values':'text/csv');
  }
}

export async function downloadDatabase(connection:string,name:string){
  const prepared=await call<{id:string;bytes:number}>('cellar_export',{connection,stage:'prepare'});
  const chunks:Uint8Array[]=[];
  try {
    for(let offset=0;offset<prepared.bytes;offset+=32768){
      const result=await call<{chunk:string}>('cellar_export',{connection,stage:'chunk',id:prepared.id,offset});
      chunks.push(Uint8Array.from(atob(result.chunk),c=>c.charCodeAt(0)));
    }
    download(`${name.replace(/\.(db|sqlite3?|sqlite)$/i,'')}.sqlite`,chunks.map(chunk=>chunk.buffer as ArrayBuffer),'application/vnd.sqlite3');
  }finally{await call('cellar_export',{connection,stage:'finish',id:prepared.id});}
}

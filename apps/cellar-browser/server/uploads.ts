import { open, realpath, stat, mkdir, unlink, appendFile, writeFile } from 'node:fs/promises';
import { join, basename } from 'node:path';
import { dispatch, serviceRequest, type Data } from './tools.js';
const uploads=new Map<string,{path:string;name:string;bytes:number;timer:ReturnType<typeof setTimeout>}>();

export async function upload(args:{stage:string;id?:string;name?:string;chunk?:string},signal?:AbortSignal):Promise<Data> {
  const directory=process.env.CELLAR_EXTENSION_STATE;
  if (!directory) throw new Error('Private database storage is not configured');
  if(args.stage==='begin') {
    if(uploads.size>=2) throw new Error('Two uploads are already in progress');
    await mkdir(join(directory,'uploads'),{recursive:true,mode:0o700});
    const id=crypto.randomUUID(),path=join(directory,'uploads',id);
    await writeFile(path,Buffer.alloc(0),{mode:0o600});
    const timer=setTimeout(()=>{uploads.delete(id);void unlink(path).catch(()=>undefined);},120000);
    timer.unref();uploads.set(id,{path,name:args.name??'Uploaded SQLite',bytes:0,timer});
    return {id};
  }
  const current=args.id&&uploads.get(args.id);
  if(!current) throw new Error('Upload expired; select the file again');
  if(args.stage==='chunk') {
    if(!args.chunk || !/^[A-Za-z0-9+/]*={0,2}$/.test(args.chunk)) throw new Error('Invalid file chunk');
    const bytes=Buffer.from(args.chunk,'base64');
    if(current.bytes+bytes.length>67108864) throw new Error('Uploaded databases must be under 64 MiB');
    current.bytes+=bytes.length;await appendFile(current.path,bytes);return {bytes:current.bytes};
  }
  uploads.delete(args.id!);clearTimeout(current.timer);
  try { return args.stage==='finish'?await attach(current.path,current.name,signal):{cancelled:true}; }
  finally { await unlink(current.path).catch(()=>undefined); }
}

export async function attach(path?: string, name?: string, signal?: AbortSignal): Promise<Data> {
  if (!path) throw new Error('Open a database file from this chat with Cellar, or select an existing SQLite path in Add connection.');
  const directory = process.env.CELLAR_EXTENSION_STATE;
  if (!directory) throw new Error('Private database storage is not configured');
  const source = await realpath(path);
  const metadata = await stat(source);
  if (!metadata.isFile() || metadata.size > 67_108_864) throw new Error('Select a SQLite file under 64 MiB; larger local databases can be connected by path.');
  const file = await open(source,'r');
  const header = Buffer.alloc(16);
  try { await file.read(header,0,16,0); } finally { await file.close(); }
  if (!header.equals(Buffer.from('SQLite format 3\0'))) throw new Error('Selected file is not a SQLite database');
  await mkdir(join(directory,'databases'),{recursive:true,mode:0o700});
  const destination = join(directory,'databases',`${crypto.randomUUID()}.sqlite`);
  await serviceRequest('import',{source,destination},signal);
  try {
    if((await stat(destination)).size>67108864)throw new Error('Imported database snapshot exceeds 64 MiB; connect the local file by path instead.');
    const saved = await dispatch('cellar_connection',{operation:'save',profile:{name:(name??basename(source)).slice(0,80),engine:'sqlite',database:destination,env_tag:'local',allow_edits:true}},signal);
    const connections=saved.connections as {id:string}[];
    return {...saved, importedCopy:true,preferredConnection:connections.at(-1)?.id};
  } catch (error) { await unlink(destination); throw error; }
}

const exports=new Map<string,{connection:string;path:string;bytes:number;timer:ReturnType<typeof setTimeout>}>();
export async function exportDatabase(args:{connection:string;stage:string;id?:string;offset?:number},signal?:AbortSignal):Promise<Data> {
  const directory=process.env.CELLAR_EXTENSION_STATE;
  if(!directory)throw new Error('Private database storage is not configured');
  if(args.stage==='prepare') {
    if(exports.size>=2)throw new Error('Two downloads are already in progress');
    await mkdir(join(directory,'exports'),{recursive:true,mode:0o700});
    const id=crypto.randomUUID(),path=join(directory,'exports',`${id}.sqlite`);
    try {
      await serviceRequest('snapshot',{connection:args.connection,destination:path},signal);
      const metadata=await stat(path);
      if(metadata.size>67108864)throw new Error('Database download limit is 64 MiB');
      const timer=setTimeout(()=>{exports.delete(id);void unlink(path).catch(()=>undefined);},120000);timer.unref();
      exports.set(id,{connection:args.connection,path,bytes:metadata.size,timer});
      return {id,bytes:metadata.size};
    }catch(error){await unlink(path).catch(()=>undefined);throw error;}
  }
  const current=args.id&&exports.get(args.id);
  if(!current||current.connection!==args.connection)throw new Error('Download expired; try again');
  if(args.stage==='finish'){exports.delete(args.id!);clearTimeout(current.timer);await unlink(current.path);return {finished:true};}
  const file=await open(current.path,'r');const bytes=Buffer.alloc(32768);
  try{const result=await file.read(bytes,0,bytes.length,args.offset??0);return {chunk:bytes.subarray(0,result.bytesRead).toString('base64'),bytes:result.bytesRead};}
  finally{await file.close();}
}

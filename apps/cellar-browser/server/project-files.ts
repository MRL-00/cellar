import {constants} from 'node:fs';
import {lstat,open,opendir,realpath} from 'node:fs/promises';
import {isAbsolute,join,parse,relative,resolve,sep} from 'node:path';

export const FILE_LIMIT=65536;
const ignored=new Set(['.git','.codex','.agents','.aws','.ssh','node_modules','target','dist','build','vendor','.venv','venv','.next','.cellar']);
export function within(root:string,path:string){const rel=relative(root,path);return rel===''||(!rel.startsWith('..'+sep)&&rel!=='..'&&!isAbsolute(rel));}
export async function checkedPath(root:string,path:string,directory=false){
 if(!within(root,path))throw new Error('Path is outside the selected project');
 const parts=resolve(path).slice(parse(path).root.length).split(sep).filter(Boolean);
 let current=parse(path).root;
 for(const part of parts){current=join(current,part);const info=await lstat(current);if(info.isSymbolicLink())throw new Error('Symlink paths are not supported');}
 const info=await lstat(path);
 if(directory?!info.isDirectory():!info.isFile())throw new Error('Select a regular project file');
 if(await realpath(path)!==resolve(path))throw new Error('Project path changed');
 return info;
}
export async function projectRoot(raw:string){
 if(!isAbsolute(raw)||raw.includes('\0')||raw.split(sep).includes('..'))throw new Error('Select an absolute project folder without traversal');
 const root=resolve(raw);
 if(root===parse(root).root)throw new Error('Select a project folder, not the filesystem root');
 await checkedPath(root,root,true);
 return root;
}
export async function readProjectFile(root:string,path:string,limit=FILE_LIMIT){
 const info=await checkedPath(root,path);
 if(info.size>limit)throw new Error('Project configuration exceeds the size limit');
 const file=await open(path,constants.O_RDONLY|constants.O_NOFOLLOW);
 try{
  const stat=await file.stat();
  if(!stat.isFile()||stat.size>limit||stat.ino!==info.ino||stat.dev!==info.dev)throw new Error('Project file changed');
  const buffer=Buffer.alloc(limit+1);
  const {bytesRead}=await file.read(buffer,0,buffer.length,0);
  if(bytesRead>limit)throw new Error('Project configuration exceeds the size limit');
  await checkedPath(root,path);
  return buffer.subarray(0,bytesRead);
 }finally{await file.close();}
}
export async function sqliteHeader(root:string,path:string){
 const info=await checkedPath(root,path);
 const file=await open(path,constants.O_RDONLY|constants.O_NOFOLLOW);
 try{const stat=await file.stat();if(stat.ino!==info.ino||stat.dev!==info.dev)throw new Error('Project file changed');const header=Buffer.alloc(16);await file.read(header,0,16,0);await checkedPath(root,path);return header.toString('binary')==='SQLite format 3\0';}finally{await file.close();}
}
export async function sqliteFiles(root:string){
 const found:string[]=[];let examined=0;let bounded=false;
 async function walk(dir:string,depth:number){
  await checkedPath(root,dir,true);
  const entries=await opendir(dir);
  for await(const entry of entries){
   if(++examined>400){bounded=true;return;}
   if(entry.isSymbolicLink()||ignored.has(entry.name)||entry.name.startsWith('.'))continue;
   const path=join(dir,entry.name);
   if(entry.isDirectory()&&depth<2)await walk(path,depth+1);
   else if(entry.isFile()&&/\.(?:db|sqlite|sqlite3)$/i.test(entry.name))found.push(path);
   if(examined>400)return;
  }
 }
 await walk(root,0);return {found:found.slice(0,32),bounded:bounded||found.length>32};
}

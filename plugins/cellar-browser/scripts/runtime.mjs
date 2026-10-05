import {createHash} from 'node:crypto';
import {readFile,mkdir,mkdtemp,writeFile,rename,rm,access,lstat} from 'node:fs/promises';
import {get} from 'node:https';
import {execFile,spawn} from 'node:child_process';
import {promisify} from 'node:util';
import {homedir} from 'node:os';
import {resolve,join} from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';

const execute=promisify(execFile);
const root=fileURLToPath(new URL('..',import.meta.url));
const files=['dist/index.html','dist/mcp.mjs','bin/cellar-extension-service','.fixtures/demo.sqlite','LICENSE','THIRD-PARTY-NOTICES.txt'];
const digest=bytes=>createHash('sha256').update(bytes).digest('hex');

export function platformKey(platform=process.platform,arch=process.arch){
  if(platform!=='darwin'||arch!=='arm64')throw new Error('This Cellar preview supports Apple Silicon macOS only. Intel Mac, Windows and Linux runtimes are not available yet.');
  return 'macos-arm64';
}

export function download(url,redirects=0){
  if(new URL(url).protocol!=='https:')return Promise.reject(new Error('Runtime download requires HTTPS'));
  return new Promise((resolveDownload,reject)=>{
    const request=get(url,{headers:{'User-Agent':'Cellar-Codex-Plugin'}},response=>{
      if([301,302,303,307,308].includes(response.statusCode)){
        response.resume();
        if(!response.headers.location||redirects>=5){reject(new Error('Runtime download redirect limit exceeded'));return;}
        download(new URL(response.headers.location,url).href,redirects+1).then(resolveDownload,reject);return;
      }
      if(response.statusCode!==200){response.resume();reject(new Error(`Runtime download failed (${response.statusCode}). Check your network and retry.`));return;}
      const chunks=[];let size=0;
      response.on('data',chunk=>{size+=chunk.length;if(size>64*1024*1024){response.destroy(new Error('Runtime download exceeds 64 MiB'));return;}chunks.push(chunk);});
      response.on('end',()=>resolveDownload(Buffer.concat(chunks)));
      response.on('error',reject);
    });
    request.setTimeout(30000,()=>request.destroy(new Error('Runtime download timed out. Check your network and retry.')));
    request.on('error',reject);
  });
}

async function cached(directory,sha256){
  try{
    const receipt=JSON.parse(await readFile(join(directory,'verified.json'),'utf8'));
    if(receipt.sha256!==sha256)return false;
    for(const path of files){
      if(!(await lstat(join(directory,path))).isFile())return false;
      if(digest(await readFile(join(directory,path)))!==receipt.files[path])return false;
    }
    return true;
  }catch{return false;}
}

export async function ensureRuntime(metadata,{state,fetchBytes=download,key=platformKey()}={}){
  if(!/^\d+\.\d+\.\d+$/.test(metadata.version))throw new Error('Invalid Cellar runtime version');
  const artifact=metadata.artifacts[key];
  if(!artifact||!/^[a-f0-9]{64}$/.test(artifact.sha256)||!/^cellar-plugin-\d+\.\d+\.\d+-macos-arm64\.tar\.gz$/.test(artifact.file))throw new Error('Invalid Cellar runtime manifest');
  const storage=resolve(state??process.env.CELLAR_EXTENSION_STATE??join(homedir(),'.cellar/extensions/cellar-browser'));
  const parent=join(storage,'runtimes');
  const destination=join(parent,metadata.version+'-'+key);
  if(await cached(destination,artifact.sha256))return destination;
  await mkdir(parent,{recursive:true,mode:0o700});
  const temporary=await mkdtemp(join(parent,'.install-'));
  try{
    process.stderr.write(`Cellar: downloading verified ${metadata.version} runtime…\n`);
    const bytes=await fetchBytes(`https://github.com/MRL-00/cellar/releases/download/cellar-plugin-v${metadata.version}/${artifact.file}`);
    if(digest(bytes)!==artifact.sha256)throw new Error('Cellar runtime checksum mismatch. Download was not installed.');
    const archive=join(temporary,'runtime.tar.gz');await writeFile(archive,bytes,{mode:0o600});
    const {stdout}=await execute('/usr/bin/tar',['-tzf',archive]);
    const entries=stdout.trim().split('\n').map(path=>path.replace(/^\.\//,''));
    if(entries.length!==files.length||new Set(entries).size!==files.length||entries.some(path=>!files.includes(path)))throw new Error('Unexpected files in Cellar runtime archive');
    const extracted=join(temporary,'runtime');await mkdir(extracted,{mode:0o700});
    await execute('/usr/bin/tar',['-xzf',archive,'-C',extracted]);
    const hashes={};
    for(const path of files){
      if(!(await lstat(join(extracted,path))).isFile())throw new Error('Unexpected runtime file type');
      hashes[path]=digest(await readFile(join(extracted,path)));
    }
    await writeFile(join(extracted,'verified.json'),JSON.stringify({sha256:artifact.sha256,files:hashes}),{mode:0o600});
    if(await cached(destination,artifact.sha256))return destination;
    try{await access(destination);throw new Error('Existing Cellar runtime failed verification. Remove only its versioned runtime cache and retry.');}catch(error){if(error.code!=='ENOENT')throw error;}
    try{await rename(extracted,destination);}catch(error){if(!await cached(destination,artifact.sha256))throw error;}
    return destination;
  }finally{await rm(temporary,{recursive:true,force:true});}
}

async function main(){
  const metadata=JSON.parse(await readFile(join(root,'runtime.json'),'utf8'));
  const storage=resolve(process.env.CELLAR_EXTENSION_STATE??join(homedir(),'.cellar/extensions/cellar-browser'));
  const runtime=await ensureRuntime(metadata,{state:storage});
  const config=join(runtime,'demo-connections.json');
  await writeFile(config,JSON.stringify([{id:'demo-sqlite',name:'Cellar demo · SQLite',engine:'sqlite',database:join(runtime,'.fixtures/demo.sqlite'),host:'',port:0,user:'',ssl_mode:'disable',env_tag:'local',allow_edits:false}]),{mode:0o600});
  const child=spawn(process.execPath,[join(runtime,'dist/mcp.mjs')],{stdio:'inherit',cwd:runtime,env:{...process.env,CELLAR_EXTENSION_STATE:storage,CELLAR_EXTENSION_CONFIG:process.env.CELLAR_EXTENSION_CONFIG??config}});
  for(const signal of ['SIGTERM','SIGINT'])process.on(signal,()=>child.kill(signal));
  child.on('error',error=>{process.stderr.write(`Cellar could not start: ${error.message}\n`);process.exitCode=1;});
  child.on('exit',(code,signal)=>{process.exitCode=code??(signal?1:0);});
}

if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href){
  main().catch(error=>{process.stderr.write(`Cellar: ${error.message}\n`);process.exitCode=1;});
}

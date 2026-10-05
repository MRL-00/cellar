import {cp,mkdir,mkdtemp,readFile,writeFile,rm,readdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {tmpdir,homedir} from 'node:os';
import {resolve,join} from 'node:path';
import {fileURLToPath} from 'node:url';

if(process.platform!=='darwin'||process.arch!=='arm64')throw new Error('Build this release on Apple Silicon macOS');
const execute=promisify(execFile);
const app=fileURLToPath(new URL('..',import.meta.url));
const repo=resolve(app,'../..');
const plugin=join(repo,'plugins/cellar-browser');
const version=JSON.parse(await readFile(join(app,'package.json'),'utf8')).version;
const output=resolve(process.argv[2]??join(repo,'release-artifacts'));
await mkdir(output,{recursive:true});
const staging=await mkdtemp(join(tmpdir(),'cellar-release-'));
try{
  for(const entry of ['dist/index.html','dist/mcp.mjs','bin/cellar-extension-service','.fixtures/demo.sqlite']){
    await mkdir(resolve(staging,entry,'..'),{recursive:true});
    await cp(join(app,entry),join(staging,entry));
  }
  await cp(join(repo,'LICENSE'),join(staging,'LICENSE'));
  const notices=[];
  for(const font of ['inter','jetbrains-mono'])notices.push(await readFile(join(repo,'apps/desktop-gpui/assets/fonts/licenses',font+'-OFL.txt'),'utf8'));
  // Preserve license text for every npm dependency included by the bundler.
  const {stdout}=await execute(process.execPath,['--input-type=module','-e',"import {build} from 'esbuild';const r=await build({entryPoints:['server/mcp.ts','ui/main.tsx'],bundle:true,platform:'node',format:'esm',write:false,metafile:true,external:['cellar-native-icons'],outdir:'unused'});console.log(JSON.stringify(Object.keys(r.metafile.inputs)));"],{cwd:app,maxBuffer:8*1024*1024});
  const packages=new Set(JSON.parse(stdout).filter(path=>path.startsWith('node_modules/')).map(path=>path.split('/').slice(1,path.split('/')[1].startsWith('@')?3:2).join('/')));
  for(const name of [...packages].sort()){
    for(const filename of ['LICENSE','LICENSE.md','LICENSE.txt','LICENSE-MIT']){
      try{notices.push(name+'\n'+await readFile(join(app,'node_modules',name,filename),'utf8'));break;}
      catch(error){if(error.code!=='ENOENT')throw error;}
    }
  }
  const registry=join(process.env.CARGO_HOME??join(homedir(),'.cargo'),'registry/src');
  const sources=await readdir(registry);
  const tree=await execute('cargo',['tree','--offline','--locked','-p','cellar-extension-service','--prefix','none','--format','{p}'],{cwd:repo});
  for(const line of new Set(tree.stdout.split('\n'))){
    const match=/^([\w-]+) v([\w.+-]+)/.exec(line);if(!match||match[1].startsWith('cellar-'))continue;
    const name=match[1]+'-'+match[2];
    for(const source of sources){
      const directory=join(registry,source,name);
      let entries;try{entries=await readdir(directory);}catch(error){if(error.code==='ENOENT')continue;throw error;}
      for(const file of entries.filter(file=>/^(LICENSE|LICENCE|COPYING|COPYRIGHT|NOTICE)([.-]|$)/i.test(file))){
        try{notices.push(name+' / '+file+'\n'+await readFile(join(directory,file),'utf8'));}catch(error){if(error.code!=='EISDIR')throw error;}
      }
      break;
    }
  }
  await writeFile(join(staging,'THIRD-PARTY-NOTICES.txt'),notices.join('\n\n--------------------\n\n'));
  const file=`cellar-plugin-${version}-macos-arm64.tar.gz`;
  await execute('/usr/bin/tar',['-czf',join(output,file),'-C',staging,'dist/index.html','dist/mcp.mjs','bin/cellar-extension-service','.fixtures/demo.sqlite','LICENSE','THIRD-PARTY-NOTICES.txt'],{env:{...process.env,COPYFILE_DISABLE:'1'}});
  const sha256=createHash('sha256').update(await readFile(join(output,file))).digest('hex');
  await writeFile(join(output,'SHA256SUMS'),`${sha256}  ${file}\n`);
  await writeFile(join(plugin,'runtime.json'),JSON.stringify({version,artifacts:{'macos-arm64':{file,sha256}}},null,2)+'\n');
  for(const entry of ['.codex-plugin','assets','skills'])await cp(join(app,entry),join(plugin,entry),{recursive:true});
  console.log(JSON.stringify({version,file,sha256,output}));
}finally{await rm(staging,{recursive:true,force:true});}

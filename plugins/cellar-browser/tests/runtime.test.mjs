import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,mkdir,writeFile,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {ensureRuntime,platformKey} from '../scripts/runtime.mjs';

const execute=promisify(execFile);
const paths=['dist/index.html','dist/mcp.mjs','bin/cellar-extension-service','.fixtures/demo.sqlite','LICENSE','THIRD-PARTY-NOTICES.txt'];
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
async function fixture(extra=[]){
  const root=await mkdtemp(join(tmpdir(),'cellar-installer-test-'));
  for(const path of [...paths,...extra]){await mkdir(join(root,path,'..'),{recursive:true});await writeFile(join(root,path),'synthetic '+path);}
  const archive=join(root,'test.tar.gz');
  await execute('/usr/bin/tar',['-czf',archive,'-C',root,...paths,...extra],{env:{...process.env,COPYFILE_DISABLE:'1'}});
  const bytes=await readFile(archive);
  return {root,bytes,metadata:{version:'0.5.2',artifacts:{'macos-arm64':{file:'cellar-plugin-0.5.2-macos-arm64.tar.gz',sha256:sha(bytes)}}}};
}

test('unsupported platforms fail before attempting installation',()=>{
  assert.equal(platformKey('darwin','arm64'),'macos-arm64');
  assert.throws(()=>platformKey('darwin','x64'),/Apple Silicon/);
  assert.throws(()=>platformKey('linux','arm64'),/Apple Silicon/);
});
test('verified cold install, warm cache and tamper detection work in an unrelated directory',async()=>{
  const f=await fixture();let downloads=0;
  const options={state:join(f.root,'private-state'),key:'macos-arm64',fetchBytes:async url=>{assert.ok(url.endsWith('/cellar-plugin-v0.5.2/cellar-plugin-0.5.2-macos-arm64.tar.gz'));downloads++;return f.bytes;}};
  try{
    const runtime=await ensureRuntime(f.metadata,options);
    assert.equal(await readFile(join(runtime,'dist/mcp.mjs'),'utf8'),'synthetic dist/mcp.mjs');
    assert.equal(await ensureRuntime(f.metadata,options),runtime);assert.equal(downloads,1);
    await writeFile(join(runtime,'dist/mcp.mjs'),'tampered');
    await assert.rejects(ensureRuntime(f.metadata,options),/failed verification/);
    assert.equal(await readFile(join(runtime,'dist/mcp.mjs'),'utf8'),'tampered');
  }finally{await rm(f.root,{recursive:true,force:true});}
});
test('checksum mismatch and extra archive files are rejected',async()=>{
  const f=await fixture(['unexpected.txt']);
  const options={state:join(f.root,'private-state'),key:'macos-arm64',fetchBytes:async()=>f.bytes};
  try{
    await assert.rejects(ensureRuntime({...f.metadata,artifacts:{'macos-arm64':{...f.metadata.artifacts['macos-arm64'],sha256:'0'.repeat(64)}}},options),/checksum mismatch/);
    await assert.rejects(ensureRuntime(f.metadata,options),/Unexpected files/);
  }finally{await rm(f.root,{recursive:true,force:true});}
});

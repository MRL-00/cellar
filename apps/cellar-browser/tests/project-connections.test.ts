import assert from 'node:assert/strict';
import {test} from 'node:test';
import {copyFile,mkdir,mkdtemp,readFile,realpath,rm,symlink,writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {tmpdir} from 'node:os';
import {randomUUID} from 'node:crypto';
import {parseAppsettings,parseEnv} from '../server/project-formats.js';
import {projectOperation,selectProject,sessionProfile,projectAliases} from '../server/project-connections.js';
import {dispatch} from '../server/tools.js';

const secret='SYNTHETIC-PASSWORD-DO-NOT-EXPOSE';
async function fixture(){const root=await realpath(await mkdtemp(join(tmpdir(),'cellar-project-')));await mkdir(join(root,'data'));await copyFile(new URL('../.fixtures/demo.sqlite',import.meta.url),join(root,'data/demo.sqlite'));return root;}
type Discovery={candidates:{id:string;source:string;engine:string;ready:boolean;production:boolean}[];warnings:string[]};
async function discover(owner:string,id:string){return await projectOperation(owner,{stage:'discover',scope:id,confirmation:'DISCOVER PROJECT CONNECTIONS'}) as Discovery;}

test('static project formats support literal URL, split env and appsettings without evaluating variables',()=>{
 const env=parseEnv(`DATABASE_URL=postgresql://reader:${secret}@localhost:55440/postgres\nSQLITE_PATH=data/demo.sqlite\nPGHOST=localhost\nPGDATABASE=postgres\nPGUSER=reader\nPGPASSWORD=${secret}`,'synthetic.env');
 assert.equal(env.definitions.length,3);assert.equal(env.definitions[0].password,secret);
 assert.equal(parseEnv('DATABASE_URL=postgresql://${USER}:${PASSWORD}@localhost/db','synthetic.env').definitions.length,0);
 assert.equal(parseEnv('DATABASE_URL=$(touch /tmp/never)','synthetic.env').unresolved,true);
 const settings=parseAppsettings(JSON.stringify({ConnectionStrings:{Pg:`Host=localhost;Database=postgres;Username=reader;Password=${secret}`,Sqlite:'Data Source=data/demo.sqlite'}}),'appsettings.json');
 assert.equal(settings.definitions.length,2);
 assert.equal(parseAppsettings(JSON.stringify({ConnectionStrings:{Pg:'Host=prod.example.test;Database=app;Username=reader;Password=abc'}}),'appsettings.json').definitions[0].ssl_mode,'verify-full');
 assert.equal(parseEnv('DATABASE_URL=postgresql://reader:password@localhost/app?sslmode=disable&options=bad','synthetic.env').unsupported,true);
});
test('scope selection and discovery never connect; only safe metadata leaves local memory',async()=>{
 const root=await fixture(),owner=randomUUID();
 try{
  await writeFile(join(root,'.env'),`DATABASE_URL=postgresql://reader:${secret}@127.0.0.1:1/app`);
  const scope=(await selectProject(owner,root)).projectScope;
  assert.equal(projectAliases(owner).length,0);
  await assert.rejects(projectOperation(owner,{stage:'discover',scope:scope.id}),/Confirm reading/);
  const found=await discover(owner,scope.id);
  assert.equal(found.candidates.length,2);assert.equal(projectAliases(owner).length,0);
  assert.ok(!JSON.stringify(found).includes(secret));assert.ok(!JSON.stringify(found).includes('reader'));assert.ok(!JSON.stringify(found).includes('postgresql://'));
  await assert.rejects(projectOperation(owner,{stage:'connect',scope:scope.id,candidate:found.candidates[0].id}),/Confirm/);
 }finally{await rm(root,{recursive:true,force:true});}
});
test('project paths reject traversal, root folders, symlinks and out-of-root databases',async()=>{
 const root=await fixture(),outside=await fixture(),owner=randomUUID();
 try{
  await assert.rejects(selectProject(owner,'/'),/project folder/);await assert.rejects(selectProject(owner,root+'/../'+root.split('/').at(-1)),/traversal/);
  await symlink(outside,join(root,'escape'));await assert.rejects(selectProject(owner,join(root,'escape')),/Symlink/);
  await symlink(join(outside,'data/demo.sqlite'),join(root,'linked.sqlite'));
  await writeFile(join(root,'.env'),`SQLITE_PATH=${join(outside,'data/demo.sqlite')}`);
  const scope=(await selectProject(owner,root)).projectScope;const found=await discover(owner,scope.id);
  assert.equal(found.candidates.find(item=>item.source==='.env')?.ready,false);
  assert.ok(!found.candidates.some(item=>item.source==='linked.sqlite'||item.source.startsWith('escape')));
  await writeFile(join(root,'.env'),'SQLITE_PATH=data/../data/demo.sqlite');
  assert.equal((await discover(owner,scope.id)).candidates.find(item=>item.source==='.env')?.ready,false);
 }finally{await rm(root,{recursive:true,force:true});await rm(outside,{recursive:true,force:true});}
});
test('symlink config, oversized config, unresolved variables and deep/dependency folders are skipped',async()=>{
 const root=await fixture(),outside=await fixture(),owner=randomUUID();
 try{
  await writeFile(join(outside,'config'),`DATABASE_URL=postgresql://reader:${secret}@localhost/postgres`);
  await symlink(join(outside,'config'),join(root,'.env'));
  await writeFile(join(root,'.env.local'),'X'.repeat(65537));await writeFile(join(root,'.env.development'),'DATABASE_URL=postgresql://${USER}:${PASSWORD}@localhost/db');
  await mkdir(join(root,'node_modules/nested'),{recursive:true});await copyFile(join(root,'data/demo.sqlite'),join(root,'node_modules/nested/ignored.db'));
  await mkdir(join(root,'one/two/three'),{recursive:true});await copyFile(join(root,'data/demo.sqlite'),join(root,'one/two/three/deep.db'));
  const scope=(await selectProject(owner,root)).projectScope,found=await discover(owner,scope.id);
  assert.equal(found.candidates.length,1);assert.equal(found.warnings.length,3);assert.ok(!JSON.stringify(found).includes(secret));
 }finally{await rm(root,{recursive:true,force:true});await rm(outside,{recursive:true,force:true});}
});
test('production-like target requires acknowledgement and config changes invalidate selection',async()=>{
 const root=await fixture(),owner=randomUUID();
 try{
  await writeFile(join(root,'.env'),'DATABASE_URL=postgresql://reader:synthetic@prod.example.test/app');
  const scope=(await selectProject(owner,root)).projectScope,found=await discover(owner,scope.id),candidate=found.candidates.find(item=>item.engine==='postgres')!;
  assert.equal(candidate.production,true);
  await assert.rejects(projectOperation(owner,{stage:'connect',scope:scope.id,candidate:candidate.id,confirmation:'CONNECT READ ONLY'}),/production-like/);
  await writeFile(join(root,'.env'),'DATABASE_URL=postgresql://reader:changed@prod.example.test/app');
  await assert.rejects(projectOperation(owner,{stage:'connect',scope:scope.id,candidate:candidate.id,confirmation:'CONNECT READ ONLY',productionAcknowledged:true}),/files changed/);
 }finally{await rm(root,{recursive:true,force:true});}
});
test('directory enumeration and candidate count stay bounded',async()=>{
 const root=await fixture(),owner=randomUUID();
 try{await Promise.all(Array.from({length:405},(_,index)=>writeFile(join(root,`ignored-${index}.txt`),'synthetic')));const scope=(await selectProject(owner,root)).projectScope,found=await discover(owner,scope.id);assert.ok(found.candidates.length<=32);assert.ok(found.warnings.some(warning=>warning.includes('bounded scan limit')));}finally{await rm(root,{recursive:true,force:true});}
});
test('selected SQLite session is isolated, read-only, nonpersistent and rejects swapped paths',async()=>{
 const root=await fixture(),owner=randomUUID(),previous=process.env.CELLAR_EXTENSION_STATE;
 try{
  process.env.CELLAR_EXTENSION_STATE=join(root,'state');
  const scope=(await selectProject(owner,root)).projectScope,found=await discover(owner,scope.id);
  const connected=await dispatch('cellar_project_connections',{stage:'connect',scope:scope.id,candidate:found.candidates[0].id,confirmation:'CONNECT READ ONLY'},undefined,undefined,owner);
  const id=connected.preferredConnection as string;
  const before=await readFile(join(root,'data/demo.sqlite'));
  const result=await dispatch('cellar_query',{connection:id,sql:'SELECT count(*) AS count FROM customers'},undefined,undefined,owner);
  assert.ok(Array.isArray(result.rows));assert.equal(result.readOnly,true);
  await assert.rejects(dispatch('cellar_query',{connection:id,sql:'DELETE FROM customers'},undefined,undefined,owner),/read-only/);
  await assert.rejects(dispatch('cellar_commit',{connection:id,schema:'main',table:'customers',changes:[{kind:'insert',values:{}}],preview_id:'a'.repeat(64),confirmation:'COMMIT 1 CHANGE'},undefined,undefined,owner),/read-only/);
  await assert.rejects(dispatch('cellar_export',{connection:id,stage:'prepare'},undefined,undefined,owner),/read-only/);
  await assert.rejects(sessionProfile('another-owner',id),/expired/);assert.deepEqual(await readFile(join(root,'data/demo.sqlite')),before);
  await assert.rejects(readFile(join(root,'state/profiles.json')));
  await rm(join(root,'data/demo.sqlite'));await symlink(new URL('../.fixtures/demo.sqlite',import.meta.url),join(root,'data/demo.sqlite'));
  await assert.rejects(sessionProfile(owner,id),/files changed/);
 }finally{if(previous)process.env.CELLAR_EXTENSION_STATE=previous;else delete process.env.CELLAR_EXTENSION_STATE;await rm(root,{recursive:true,force:true});}
});
test('project session expires and is disconnected without writing credentials',async()=>{
 const root=await fixture(),owner=randomUUID(),now=Date.now;
 try{const scope=(await selectProject(owner,root)).projectScope,found=await discover(owner,scope.id);const connected=await projectOperation(owner,{stage:'connect',scope:scope.id,candidate:found.candidates[0].id,confirmation:'CONNECT READ ONLY'});Date.now=()=>now()+16*60*1000;await assert.rejects(sessionProfile(owner,connected.preferredConnection as string),/expired/);assert.equal(projectAliases(owner).length,0);}finally{Date.now=now;await rm(root,{recursive:true,force:true});}
});
test('project PostgreSQL uses a local in-memory credential without exposing or persisting it',{skip:process.env.CELLAR_TEST_POSTGRES!=='1'},async()=>{
 const root=await fixture(),owner=randomUUID(),previous=process.env.CELLAR_EXTENSION_STATE;
 try{
  process.env.CELLAR_EXTENSION_STATE=join(root,'state');
  await writeFile(join(root,'.env'),`DATABASE_URL=postgresql://cellar_project_reader:${secret}@127.0.0.1:55440/postgres`);
  const scope=(await selectProject(owner,root)).projectScope,found=await discover(owner,scope.id);
  const chosen=found.candidates.find(item=>item.engine==='postgres')!;
  const connected=await projectOperation(owner,{stage:'connect',scope:scope.id,candidate:chosen.id,confirmation:'CONNECT READ ONLY'});
  const id=connected.preferredConnection as string;
  assert.ok(!JSON.stringify(connected).includes(secret));assert.ok(!JSON.stringify(connected).includes('cellar_project_reader'));
  const result=await dispatch('cellar_query',{connection:id,sql:'SELECT count(*) AS count FROM orders'},undefined,undefined,owner);
  assert.equal(result.readOnly,true);assert.ok(!JSON.stringify(result).includes(secret));
  await assert.rejects(readFile(join(root,'state/profiles.json')));
  await projectOperation(owner,{stage:'disconnect',scope:scope.id});await assert.rejects(sessionProfile(owner,id),/expired/);
  await writeFile(join(root,'.env'),'DATABASE_URL=postgresql://cellar_project_reader:wrong-synthetic-password@127.0.0.1:55440/postgres');
  const wrong=await discover(owner,scope.id),invalid=await projectOperation(owner,{stage:'connect',scope:scope.id,candidate:wrong.candidates.find(item=>item.engine==='postgres')!.id,confirmation:'CONNECT READ ONLY'});
  await assert.rejects(dispatch('cellar_query',{connection:invalid.preferredConnection,sql:'SELECT 1'},undefined,undefined,owner),/Postgres connection failed/);
 }finally{if(previous)process.env.CELLAR_EXTENSION_STATE=previous;else delete process.env.CELLAR_EXTENSION_STATE;await rm(root,{recursive:true,force:true});}
});

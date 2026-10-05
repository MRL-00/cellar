import assert from 'node:assert/strict';
import {test} from 'node:test';
import {mkdtemp,writeFile,rm,readFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {randomUUID} from 'node:crypto';
import {dispatch} from '../server/tools.js';
import type {Result} from '../ui/types.js';

test('Postgres transactional updates/inserts/deletes preserve JSON, detect stale rows and roll back constraints',{skip:process.env.CELLAR_TEST_POSTGRES!=='1'},async()=>{
  const directory=await mkdtemp(join(tmpdir(),'cellar-pg-edits-'));
  const previous=process.env.CELLAR_EXTENSION_CONFIG;
  const config=join(directory,'connections.json');
  await writeFile(config,JSON.stringify([{id:'edit-fixture',name:'Disposable edits',engine:'postgres',host:'127.0.0.1',port:Number(process.env.CELLAR_TEST_POSTGRES_PORT??55440),database:'cellar_edit_fixture',user:'cellar_editor',ssl_mode:'disable',allow_edits:true}]));
  process.env.CELLAR_EXTENSION_CONFIG=config;
  const args={connection:'edit-fixture',schema:'public',table:'sample'};
  const read=async()=>await dispatch('cellar_browse',args) as unknown as Result;
  let original:Result|undefined;
  try{
    original=await read();assert.equal(original.rows.length,2,'The disposable sample fixture must contain two rows');
    const insertedId=String(Math.max(...original.rows.map(row=>Number(row[0])))+1);
    const changedName=`Changed O'Brien ${randomUUID()}`;
    const changes=[{kind:'update',original:original.rows[0],revision:original.revisions![0],values:{name:changedName,payload:'{"safe":false}'}},{kind:'insert',values:{id:insertedId,name:'Inserted'}},{kind:'delete',original:original.rows[1],revision:original.revisions![1],values:{}}];
    const preview=await dispatch('cellar_preview',{...args,changes});
    await dispatch('cellar_commit',{...args,changes,preview_id:preview.previewId,confirmation:'COMMIT 3 CHANGES'});
    const after=await read();assert.equal(after.rows.length,2);assert.equal(after.rows[0][1],changedName);assert.equal(JSON.parse(after.rows[0][2] as string).safe,false);
    await assert.rejects(dispatch('cellar_commit',{...args,changes,preview_id:preview.previewId,confirmation:'COMMIT 3 CHANGES'}),/changed/);
    const rollback=[{kind:'update',original:after.rows[0],revision:after.revisions![0],values:{name:'Must roll back'}},{kind:'insert',values:{id:insertedId,name:'Duplicate'}}];
    const failed=await dispatch('cellar_preview',{...args,changes:rollback});
    await assert.rejects(dispatch('cellar_commit',{...args,changes:rollback,preview_id:failed.previewId,confirmation:'COMMIT 2 CHANGES'}),/constraints/);
    assert.equal((await read()).rows[0][1],changedName);
    const readOnly=JSON.parse(await readFile(config,'utf8'));readOnly[0].allow_edits=false;await writeFile(config,JSON.stringify(readOnly));
    await assert.rejects(dispatch('cellar_commit',{...args,changes:rollback,preview_id:failed.previewId,confirmation:'COMMIT 2 CHANGES'}),/Enable editing/);
  }finally{
    try{
      if(original){
        const profile=JSON.parse(await readFile(config,'utf8'));profile[0].allow_edits=true;await writeFile(config,JSON.stringify(profile));
        const current=await read();
        const values=(row:Result['rows'][number])=>Object.fromEntries(original!.columns.map((column,index)=>[column.name,row[index]===null?null:String(row[index])]));
        const restore:({kind:string;original?:Result['rows'][number];revision?:string;values:Record<string,string|null>})[]=[];
        for(const [index,row] of current.rows.entries()){
          const before=original.rows.find(item=>item[0]===row[0]);
          if(!before)restore.push({kind:'delete',original:row,revision:current.revisions![index],values:{}});
          else if(JSON.stringify(before)!==JSON.stringify(row))restore.push({kind:'update',original:row,revision:current.revisions![index],values:values(before)});
        }
        for(const row of original.rows)if(!current.rows.some(item=>item[0]===row[0]))restore.push({kind:'insert',values:values(row)});
        if(restore.length){const preview=await dispatch('cellar_preview',{...args,changes:restore});await dispatch('cellar_commit',{...args,changes:restore,preview_id:preview.previewId,confirmation:`COMMIT ${restore.length} CHANGES`});}
        assert.deepEqual((await read()).rows,original.rows,'Restore the preserved synthetic fixture');
      }
    }finally{if(previous)process.env.CELLAR_EXTENSION_CONFIG=previous;else delete process.env.CELLAR_EXTENSION_CONFIG;await rm(directory,{recursive:true,force:true});}
  }
});

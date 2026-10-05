import assert from 'node:assert/strict';
import {test} from 'node:test';
import {fileURLToPath} from 'node:url';
import {dispatch} from '../server/tools.js';
import type {Result,Table} from '../ui/types.js';
process.env.CELLAR_EXTENSION_CONFIG??=fileURLToPath(new URL('../.fixtures/connections.json',import.meta.url));

for(const connection of ['demo-sqlite','demo-postgres'])test(`${connection} range filters, primary keys, schema relationships, plans and cursor/batch SQL`,{skip:connection==='demo-postgres'&&process.env.CELLAR_TEST_POSTGRES!=='1'},async()=>{
  const schema=connection==='demo-sqlite'?'main':'public';
  const metadata=await dispatch('cellar_schema',{connection}) as {tables:Table[]};
  assert.equal(metadata.tables.find(table=>table.name==='customers')!.columns[0].primaryKey,true);
  assert.equal(metadata.tables.find(table=>table.name==='paid_orders')!.kind,'view');
  const filtered=await dispatch('cellar_browse',{connection,schema,table:'orders',filters:[{column:'total',operator:'greaterThan',value:'1000'},{column:'total',operator:'lessThanOrEqual',value:'1020'}]}) as unknown as Result;
  assert.ok(filtered.rows.length>0);assert.ok(filtered.rows.every(row=>Number(row[3])>1000&&Number(row[3])<=1020));
  const details=await dispatch('cellar_details',{connection,schema,table:'orders'});
  assert.equal((details.foreignKeys as {targetTable:string}[])[0].targetTable,'customers');
  const plan=await dispatch('cellar_plan',{connection,sql:'SELECT id FROM orders WHERE id=1'});
  assert.ok(plan.plan||plan.rows);
  const batch=await dispatch('cellar_sql',{connection,operation:'all',sql:"SELECT ';' AS text; SELECT 2 AS number;"});
  assert.equal((batch.statements as string[]).length,2);
  const cursor=await dispatch('cellar_sql',{connection,operation:'statement',cursor:27,sql:"SELECT '☕;' AS first; SELECT 2 AS second;"});
  assert.ok((cursor.statements as string[])[0].includes('second'));
  await assert.rejects(dispatch('cellar_sql',{connection,operation:'all',sql:'SELECT 1; DELETE FROM orders;'}),/read-only/);
  await assert.rejects(dispatch('cellar_sql',{connection,operation:'all',sql:Array(11).fill('SELECT 1;').join('')}),/10 statements/);
});

import assert from 'node:assert/strict';
import {readFile,mkdtemp,rm,cp} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {Client} from '@modelcontextprotocol/sdk/client/index.js';
import {StdioClientTransport} from '@modelcontextprotocol/sdk/client/stdio.js';
import {ensureRuntime} from '../../../plugins/cellar-browser/scripts/runtime.mjs';

const repo=fileURLToPath(new URL('../../..',import.meta.url));
const source=resolve(process.argv.slice(2).find(arg=>!arg.startsWith('--'))??join(repo,'plugins/cellar-browser'));
const directory=await mkdtemp(join(tmpdir(),'cellar-public-install-'));
const plugin=join(directory,'plugin');
const state=join(directory,'private-state');
await cp(source,plugin,{recursive:true});
const metadata=JSON.parse(await readFile(join(plugin,'runtime.json'),'utf8'));
if(process.argv.includes('--local'))await ensureRuntime(metadata,{state,fetchBytes:async()=>readFile(join(repo,'release-artifacts',metadata.artifacts['macos-arm64'].file))});
const config=JSON.parse(await readFile(join(plugin,'.mcp.json'),'utf8')).mcpServers['cellar-browser'];
const transport=new StdioClientTransport({command:config.command,args:config.args,cwd:plugin,env:{PATH:'/usr/bin:/bin',CODEX_MCP_NODE_PATH:process.execPath,CELLAR_EXTENSION_STATE:state,HOME:directory},stderr:'pipe'});
transport.stderr?.on('data',chunk=>process.stderr.write(chunk));
const client=new Client({name:'cellar-public-install-check',version:'1.0.0'});
try{
  await client.connect(transport);
  assert.equal((await client.listTools()).tools.length,17);
  const opened=await client.callTool({name:'cellar_open',arguments:{}});
  assert.ok(!opened.isError);
  assert.equal(opened._meta.data.connections[0].readOnly,true);
  const browsed=await client.callTool({name:'cellar_browse',arguments:{connection:'demo-sqlite',schema:'main',table:'customers',limit:3}});
  assert.ok(!browsed.isError);assert.equal(browsed._meta.data.rows.length,3);
  assert.ok(!JSON.stringify(browsed.content).includes('customer1@example.test'));
  const rejected=await client.callTool({name:'cellar_query',arguments:{connection:'demo-sqlite',sql:'DELETE FROM customers'}});
  assert.equal(rejected.isError,true);
  const resource=await client.readResource({uri:'ui://cellar-browser/v5.html'});
  assert.equal(resource.contents[0].mimeType,'text/html;profile=mcp-app');
  assert.ok(resource.contents[0].text.includes('<title>Cellar</title>'));
  console.log(JSON.stringify({version:metadata.version,tools:17,syntheticRows:3,readOnlyDemo:true,writeRejected:true,uiResource:true,download:process.argv.includes('--local')?'local-artifact':'public-GitHub',visualNativeHostVerified:false}));
}finally{await client.close();await rm(directory,{recursive:true,force:true});}

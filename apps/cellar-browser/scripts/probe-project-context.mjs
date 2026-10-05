import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
const server=new McpServer({name:'cellar-context-probe',version:'1'});
server.registerTool('context_probe',{inputSchema:{}},async(_args,extra)=>{
 const capabilities=server.server.getClientCapabilities();
 let roots;
 if(capabilities?.roots)try{roots=(await server.server.listRoots({}, {timeout:3000})).roots;}catch{roots='unavailable';}
 return {content:[{type:'text',text:JSON.stringify({capabilities,roots,metaKeys:Object.keys(extra._meta??{}),serverCwd:process.cwd()})}]};
});
await server.connect(new StdioServerTransport());

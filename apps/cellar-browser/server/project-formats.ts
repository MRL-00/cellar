export type Definition={engine:'sqlite'|'postgres';database:string;host?:string;port?:number;user?:string;password?:string;ssl_mode?:string;production:boolean};
export type Parsed={definitions:Definition[];unresolved:boolean;unsupported:boolean};
const template=/\$|\%[^%]+\%|\{\{|\$\(|`/;
const production=(text:string)=>/(?:^|[^a-z])prod(?:uction)?(?:[^a-z]|$)/i.test(text);
const local=(host:string)=>['localhost','127.0.0.1','::1'].includes(host);
export function envValues(text:string){
 const values:Record<string,string>={};
 for(const line of text.split(/\r?\n/).slice(0,1024)){
  const match=/^\s*(?:export\s+)?([A-Z][A-Z0-9_]*)\s*=\s*(.*?)\s*$/.exec(line);
  if(!match)continue;let value=match[2];
  if((value.startsWith('"')&&value.endsWith('"'))||(value.startsWith("'")&&value.endsWith("'")))value=value.slice(1,-1);
  else value=value.replace(/\s+#.*$/,'');
  if(value.length<=4096)values[match[1]]=value;
 }
 return values;
}
function urlDefinition(value:string,source:string):Definition|undefined{
 if(/^postgres(?:ql)?:\/\//i.test(value)){
  const url=new URL(value);const host=url.hostname.replace(/^\[|\]$/g,'');
  if(url.searchParams.size&&[...url.searchParams.keys()].some(key=>!['sslmode','application_name'].includes(key)))return;
  const database=decodeURIComponent(url.pathname.slice(1));const user=decodeURIComponent(url.username);
  if(!host||!database||!user||host.startsWith('/')||/[\r\n]/.test(host)||!Number(url.port||5432))return;
  return {engine:'postgres',host,port:Number(url.port||5432),database,user,password:decodeURIComponent(url.password),ssl_mode:local(host)?url.searchParams.get('sslmode')??'disable':'verify-full',production:production(source+' '+host+' '+database)};
 }
 if(/^file:/i.test(value)){
  const database=value.slice(5);
  if(database.startsWith('//')||database.includes('?')||!database)return;
  return {engine:'sqlite',database,production:production(source+' '+database)};
 }
}
function connectionString(value:string,source:string):Definition|undefined{
 const fields:Record<string,string>={};
 for(const part of value.split(';')){if(!part.trim())continue;const index=part.indexOf('=');if(index<1)return;const key=part.slice(0,index).trim().toLowerCase().replace(/\s/g,'');if(key in fields)return;fields[key]=part.slice(index+1).trim();}
 const database=fields.database??fields.initialcatalog;
 const host=fields.host??fields.server;
 const user=fields.username??fields.userid??fields.user;
 if(host&&database&&user){
  if(Object.keys(fields).some(key=>!['host','server','database','initialcatalog','username','userid','user','password','port','sslmode','pooling','timeout','commandtimeout'].includes(key)))return;
  const port=Number(fields.port??5432);if(!Number.isInteger(port)||port<1||port>65535)return;
  return {engine:'postgres',host,port,database,user,password:fields.password,ssl_mode:local(host)?'disable':'verify-full',production:production(source+' '+host+' '+database)};
 }
 if(fields.datasource&&Object.keys(fields).every(key=>['datasource','mode','cache'].includes(key)))return {engine:'sqlite',database:fields.datasource,production:production(source+' '+fields.datasource)};
}
export function parseEnv(text:string,source:string):Parsed{
 const values=envValues(text),definitions:Definition[]=[];let unresolved=false,unsupported=false;
 function add(value:string,parser=(input:string)=>urlDefinition(input,source)){
  if(template.test(value)){unresolved=true;return;}
  try{const definition=parser(value);if(definition)definitions.push(definition);else unsupported=true;}catch{unsupported=true;}
 }
 for(const key of ['DATABASE_URL','POSTGRES_URL','POSTGRESQL_URL','SQLITE_URL'])if(values[key])add(values[key]);
 for(const key of ['SQLITE_PATH','DATABASE_PATH'])if(values[key])add(values[key],database=>({engine:'sqlite',database,production:production(source+' '+database)}));
 const pg=values.PGHOST?['PGHOST','PGPORT','PGDATABASE','PGUSER','PGPASSWORD','PGSSLMODE']:values.DB_HOST&&['pgsql','postgres','postgresql'].includes(values.DB_CONNECTION)?['DB_HOST','DB_PORT','DB_DATABASE','DB_USERNAME','DB_PASSWORD','DB_SSLMODE']:undefined;
 if(pg){const [h,p,d,u,pw,ssl]=pg;const relevant=pg.map(key=>values[key]??'');
  if(relevant.some(value=>template.test(value)))unresolved=true;
  else if(values[h]&&values[d]&&values[u]){const port=Number(values[p]??5432);if(!Number.isInteger(port)||port<1||port>65535)unsupported=true;else definitions.push({engine:'postgres',host:values[h],port,database:values[d],user:values[u],password:values[pw],ssl_mode:local(values[h])?values[ssl]??'disable':'verify-full',production:production(source+' '+values[h]+' '+values[d])});}
  else unresolved=true;
 }
 if(values.DB_CONNECTION==='sqlite'&&values.DB_DATABASE)add(values.DB_DATABASE,database=>({engine:'sqlite',database,production:production(source+' '+database)}));
 return {definitions,unresolved,unsupported};
}
export function parseAppsettings(text:string,source:string):Parsed{
 const object=JSON.parse(text) as {ConnectionStrings?:Record<string,unknown>};
 const definitions:Definition[]=[];let unresolved=false,unsupported=false;
 for(const value of Object.values(object.ConnectionStrings??{}).slice(0,32)){
  if(typeof value!=='string'||value.length>4096){unsupported=true;continue;}
  if(template.test(value)){unresolved=true;continue;}
  try{const definition=urlDefinition(value,source)??connectionString(value,source);if(definition)definitions.push(definition);else unsupported=true;}catch{unsupported=true;}
 }
 return {definitions,unresolved,unsupported};
}

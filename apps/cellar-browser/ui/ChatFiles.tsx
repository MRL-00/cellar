import {findChatDatabases,hostIntegrationAvailable} from './bridge.js';
export function ChatFiles({disabled,onError}:{disabled:boolean;onError:(message:string)=>void}){
  const available=hostIntegrationAvailable();
  return <button disabled={disabled||!available} title={available?'Ask Codex to open a database attached to this chat':'Requires the Codex plugin host and chat messaging; unavailable in the browser preview'} onClick={()=>void findChatDatabases().catch(error=>onError(error.message))}>Find databases in this chat</button>;
}

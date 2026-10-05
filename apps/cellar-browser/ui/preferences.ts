export function readPreference<T>(key:string,fallback:T):T {
  try {const value=localStorage.getItem(`cellar-browser.${key}`);return value?JSON.parse(value) as T:fallback;}catch{return fallback;}
}
export function writePreference(key:string,value:unknown):boolean {
  try {localStorage.setItem(`cellar-browser.${key}`,JSON.stringify(value));return true;}catch{return false;}
}

export type Connection = { id: string; name: string; engine: 'sqlite' | 'postgres' | 'supabase' | 'neon'; environment: string | null; readOnly: boolean; managed?:boolean;project?:boolean };
export type Column = { name: string; type: string; nullable?: boolean; primaryKey?: boolean };
export type Table = { schema: string; name: string; kind: string; columns: Column[] };
export type Cell = string | number | boolean | null;
export type Result = { columns: Column[]; rows: Cell[][]; truncated: boolean; durationMs: number; readOnly: boolean; revisions?:string[] };
export type Filter = { column: string; operator: 'equals' | 'notEquals' | 'contains' | 'notContains' | 'startsWith' | 'endsWith' | 'like' | 'greaterThan' | 'greaterThanOrEqual' | 'lessThan' | 'lessThanOrEqual' | 'isNull' | 'isNotNull'; value?: string };
export type Change = {kind:'update'|'insert'|'delete';original?:Cell[];revision?:string;values:Record<string,string|null>;rowIndex?:number};
export type Tab = { id: string; title: string; table?: Table; sql: string; filters: Filter[]; offset: number; sort?: string; descending: boolean; result?: Result; changes?:Change[];batch?:Result[];executedAt?:string;executionError?:string };

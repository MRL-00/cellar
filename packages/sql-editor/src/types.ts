export type SqlEngine = "postgres" | "mysql" | "sqlite" | "mssql" | "azure";

export interface SqlColumnMeta {
  name: string;
  data_type: string;
  nullable?: boolean;
  is_primary_key?: boolean;
}

export interface SqlRelationMeta {
  name: string;
  schema: string;
  columns: SqlColumnMeta[];
}

export interface SqlSchemaMeta {
  name: string;
  tables: SqlRelationMeta[];
  views: SqlRelationMeta[];
}

export interface SqlDatabaseMeta {
  name: string;
  is_default?: boolean;
  schemas: SqlSchemaMeta[];
}

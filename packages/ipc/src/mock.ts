// Mock implementations of the generated `commands` for `pnpm dev:web` mode,
// where the Tauri IPC bridge is not present. They return empty datasets so
// the UI can render without throwing. Anything destructive (save, delete) is
// a no-op that resolves with the input — enough to drive UI flow but never
// pretend to persist.

import type {
  CellarError,
  ConnectionConfig,
  Database,
  DetectedParameter,
  DriverInfo,
  Engine,
  ErGraph,
  QueryParam,
  QueryPlan,
  QueryHistoryRecord,
  QueryResult,
  QueryTemplate,
  Result,
  TableBrowseRequest,
  TableChangeRequest,
  TableCommitPreview,
  TableCommitResult,
  SchemaSource,
  SchemaComparison,
  MigrationStatement,
  MigrationApplyResult,
  SchemaSnapshotMeta,
  Dialect,
} from "./generated";

function sourceLabel(source: SchemaSource): string {
  return source.label ?? source.schema;
}

function ok<T>(data: T): Result<T, CellarError> {
  return { status: "ok", data };
}

const mockDriverInfo: DriverInfo = {
  engine: "postgres",
  version: "PostgreSQL 16.2 (mock — vite web mode)",
};

export const mockCommands = {
  listConnections: async (): Promise<Result<ConnectionConfig[], CellarError>> => ok([]),

  saveConnection: async (
    config: ConnectionConfig,
    _password: string | null,
  ): Promise<Result<ConnectionConfig, CellarError>> => ok(config),

  deleteConnection: async (_id: string): Promise<Result<null, CellarError>> => ok(null),

  testConnection: async (
    _config: ConnectionConfig,
    _password: string | null,
  ): Promise<Result<DriverInfo, CellarError>> => ok(mockDriverInfo),

  connect: async (_id: string): Promise<Result<DriverInfo, CellarError>> => ok(mockDriverInfo),

  reconnect: async (_id: string): Promise<Result<DriverInfo, CellarError>> => ok(mockDriverInfo),

  disconnect: async (_id: string): Promise<Result<null, CellarError>> => ok(null),

  introspect: async (
    _connectionId: string,
    _refresh: boolean | null,
  ): Promise<Result<Database[], CellarError>> => ok([]),

  erGraph: async (
    _connectionId: string,
    database: string,
    _schemas: string[] | null,
  ): Promise<Result<ErGraph, CellarError>> =>
    ok({ database, schemas: [], nodes: [], edges: [] }),

  runQuery: async (
    _connectionId: string,
    _sql: string,
    _maxRows: number | null,
    _offset: number | null,
    _database: string | null,
    _tabId: string | null,
    _queryId: string | null,
    _params: QueryParam[] | null,
  ): Promise<Result<QueryResult, CellarError>> =>
    ok({
      columns: [],
      rows: [],
      notices: [],
      notice_capture: {
        supported: false,
        reason: "Mock IPC mode does not connect to a database server.",
      },
      rows_affected: null,
      duration_ms: 0,
      truncated: false,
      total_rows: null,
    }),

  // Web mode has no Rust tokenizer; report no parameters so the editor runs
  // queries directly. Real parameter detection only runs in the Tauri build.
  detectQueryParameters: async (
    _sql: string,
    _engine: Engine,
  ): Promise<Result<DetectedParameter[], CellarError>> => ok([]),

  cancelQuery: async (
    _connectionId: string,
    _queryId: string,
  ): Promise<Result<boolean, CellarError>> => ok(false),

  explainQuery: async (
    _connectionId: string,
    sql: string,
    mode: "estimate" | "analyze",
    database: string | null,
  ): Promise<Result<QueryPlan, CellarError>> =>
    ok({
      mode,
      engine: "postgres",
      database,
      sql,
      root: {
        node_type: "Result",
        relation_name: null,
        schema_name: null,
        alias: null,
        index_name: null,
        join_type: null,
        startup_cost: 0,
        total_cost: 0.01,
        plan_rows: 1,
        plan_width: 4,
        actual_startup_time_ms: null,
        actual_total_time_ms: null,
        actual_rows: null,
        actual_loops: null,
        details: [],
        children: [],
      },
      planning_time_ms: 0,
      execution_time_ms: null,
      duration_ms: 0,
      raw_json: [{ Plan: { "Node Type": "Result" } }],
    }),

  browseTable: async (
    _request: TableBrowseRequest,
  ): Promise<Result<QueryResult, CellarError>> =>
    ok({
      columns: [],
      rows: [],
      notices: [],
      notice_capture: {
        supported: false,
        reason: "Mock IPC mode does not connect to a database server.",
      },
      rows_affected: null,
      duration_ms: 0,
      truncated: false,
      total_rows: null,
    }),

  previewTableChanges: async (
    _request: TableChangeRequest,
  ): Promise<Result<TableCommitPreview, CellarError>> =>
    ok({
      sql: "BEGIN;\n\n-- preview unavailable in web mode\n\nCOMMIT;",
      expected_rows: 0,
      statement_count: 0,
    }),

  commitTableChanges: async (
    _connectionId: string,
    _request: TableChangeRequest,
    _tabId: string | null,
  ): Promise<Result<TableCommitResult, CellarError>> =>
    ok({
      sql: "BEGIN;\n\nCOMMIT;",
      rows_affected: 0,
      duration_ms: 0,
    }),

  listQueryHistory: async (
    _connectionId: string | null,
    _database: string | null,
    _tabId: string | null,
    _search: string | null,
    _limit: number | null,
  ): Promise<Result<QueryHistoryRecord[], CellarError>> => ok([]),

  // Schema comparison / migration: web mode has no live DB to introspect, so
  // return an empty comparison and no-op snapshot operations.
  compareSchemas: async (
    source: SchemaSource,
    target: SchemaSource,
  ): Promise<Result<SchemaComparison, CellarError>> =>
    ok({
      diff: {
        source_label: sourceLabel(source),
        target_label: sourceLabel(target),
        source_schema: source.schema,
        target_schema: target.schema,
        tables: [],
        views: [],
        summary: {
          tables_added: 0,
          tables_removed: 0,
          tables_modified: 0,
          tables_unchanged: 0,
          views_added: 0,
          views_removed: 0,
          views_modified: 0,
          views_unchanged: 0,
        },
      },
      statements: [],
      dialect: "postgres",
    }),

  buildMigrationScript: async (
    _statements: MigrationStatement[],
    _dialect: Dialect,
    _wrapInTransaction: boolean,
  ): Promise<string> => "-- Cellar schema migration\n-- No changes selected.\n",

  applyMigration: async (
    _connectionId: string,
    _database: string,
    _sql: string,
    _tabId: string | null,
  ): Promise<Result<MigrationApplyResult, CellarError>> => ok({ duration_ms: 0 }),

  saveSchemaSnapshot: async (
    connectionId: string,
    database: string,
  ): Promise<Result<SchemaSnapshotMeta, CellarError>> =>
    ok({
      id: `mock-${database}`,
      label: `${connectionId} · ${database}`,
      engine: "postgres",
      connection_id: connectionId,
      connection_name: connectionId,
      database,
      schemas: [],
      table_count: 0,
      created_at_ms: 0,
    }),

  listSchemaSnapshots: async (): Promise<
    Result<SchemaSnapshotMeta[], CellarError>
  > => ok([]),

  deleteSchemaSnapshot: async (_id: string): Promise<Result<null, CellarError>> =>
    ok(null),

  // AI keys: in web mode there is no keychain, so hold them in memory for the
  // session. Enough to drive the settings/panel flow without persistence.
  aiStoreKey: async (provider: string, key: string): Promise<Result<null, CellarError>> => {
    mockAiKeys.set(provider, key);
    return ok(null);
  },

  aiLoadKey: async (provider: string): Promise<Result<string | null, CellarError>> =>
    ok(mockAiKeys.get(provider) ?? null),

  aiDeleteKey: async (provider: string): Promise<Result<null, CellarError>> => {
    mockAiKeys.delete(provider);
    return ok(null);
  },

  aiHasKey: async (provider: string): Promise<Result<boolean, CellarError>> =>
    ok(mockAiKeys.has(provider)),

  // Query templates: in web mode there is no `~/.cellar/queries`, so hold them
  // in memory for the session — enough to drive the save/list/delete flow.
  listQueryTemplates: async (): Promise<Result<QueryTemplate[], CellarError>> =>
    ok(
      [...mockTemplates.values()].sort((a, b) =>
        a.name.toLowerCase().localeCompare(b.name.toLowerCase()),
      ),
    ),

  saveQueryTemplate: async (
    template: QueryTemplate,
  ): Promise<Result<QueryTemplate, CellarError>> => {
    mockTemplates.set(template.name, template);
    return ok(template);
  },

  deleteQueryTemplate: async (name: string): Promise<Result<null, CellarError>> => {
    mockTemplates.delete(name);
    return ok(null);
  },
};

const mockAiKeys = new Map<string, string>();
const mockTemplates = new Map<string, QueryTemplate>();

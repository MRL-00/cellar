// Mock implementations of the generated `commands` for `pnpm dev:web` mode,
// where the Tauri IPC bridge is not present. They return empty datasets so
// the UI can render without throwing. Anything destructive (save, delete) is
// a no-op that resolves with the input — enough to drive UI flow but never
// pretend to persist.

import type {
  CellarError,
  CellValue,
  ConnectionConfig,
  Database,
  DatagripImport,
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
  UsageReference,
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

const screenshotFixtures = (() => {
  const env = (import.meta as { env?: Record<string, string | undefined> }).env;
  return env?.["VITE_CELLAR_SCREENSHOT_FIXTURES"] === "1";
})();

const mockDriverInfo: DriverInfo = {
  engine: "postgres",
  version: "PostgreSQL 16.2 (mock — vite web mode)",
};

const fixtureConnection: ConnectionConfig = {
  id: "shop-eu-prod",
  name: "shop-eu (prod)",
  engine: "postgres",
  host: "prod-pg.internal.shop.eu",
  port: 5432,
  database: "shop_eu",
  user: "analytics_ro",
  ssl_mode: "require",
  env_tag: "prod",
  application_name: "Cellar",
  color: "#a78bfa",
};

const fixtureTables = [
  ["orders", 1840165],
  ["order_items", 7200000],
  ["customers", 184000],
  ["products", 12000],
  ["product_variants", 38000],
  ["carts", 221000],
  ["shipments", 1700000],
  ["addresses", 312000],
  ["payments", 1800000],
  ["refunds", 18000],
  ["promotions", 412],
  ["categories", 184],
  ["suppliers", 92],
  ["inventory", 50000],
] as const;

const orderColumns = [
  column("order_id", "uuid", false, true, 1),
  column("order_ref", "text", false, false, 2),
  column("customer_id", "uuid", false, false, 3),
  column("order_status", "text", false, false, 4),
  column("gross_eur", "numeric(10,2)", false, false, 5),
  column("currency", "char(3)", false, false, 6),
  column("channel", "text", false, false, 7),
  column("created_at", "timestamptz", false, false, 8),
];

const fixtureDatabases: Database[] = [
  {
    name: "shop_eu",
    is_default: true,
    schemas: [
      {
        name: "public",
        tables: fixtureTables.map(([name, rowCount]) => ({
          name,
          schema: "public",
          row_count: rowCount,
          columns: name === "orders" ? orderColumns : [
            column("id", "uuid", false, true, 1),
            column("name", "text", false, false, 2),
            column("updated_at", "timestamptz", false, false, 3),
          ],
          primary_key: name === "orders" ? ["order_id"] : ["id"],
          foreign_keys:
            name === "orders"
              ? [
                  {
                    name: "orders_customer_id_fkey",
                    columns: ["customer_id"],
                    referenced_schema: "public",
                    referenced_table: "customers",
                    referenced_columns: ["id"],
                  },
                ]
              : [],
          indexes: [
            {
              name: `${name}_pkey`,
              columns: name === "orders" ? ["order_id"] : ["id"],
              unique: true,
              primary: true,
            },
          ],
        })),
        views: [
          {
            name: "revenue_by_country",
            schema: "public",
            columns: [
              column("country", "char(2)", false, false, 1),
              column("orders", "bigint", false, false, 2),
              column("gross_eur", "numeric", false, false, 3),
            ],
            definition: "select country, count(*) orders from orders group by 1",
          },
        ],
      },
      {
        name: "analytics",
        tables: [
          {
            name: "daily_revenue",
            schema: "analytics",
            row_count: 1420,
            columns: [
              column("day", "date", false, true, 1),
              column("gross_eur", "numeric", false, false, 2),
            ],
            primary_key: ["day"],
            foreign_keys: [],
            indexes: [{ name: "daily_revenue_pkey", columns: ["day"], unique: true, primary: true }],
          },
        ],
        views: [],
      },
    ],
  },
];

function column(
  name: string,
  dataType: string,
  nullable: boolean,
  isPrimaryKey: boolean,
  ordinal: number,
) {
  return {
    name,
    data_type: dataType,
    nullable,
    default: null,
    is_primary_key: isPrimaryKey,
    ordinal,
    comment: null,
  };
}

function cell(value: string | number | boolean | null): CellValue {
  if (value === null) return { type: "Null" };
  if (typeof value === "boolean") return { type: "Bool", value };
  if (typeof value === "number") return Number.isInteger(value)
    ? { type: "Int", value }
    : { type: "Numeric", value: value.toFixed(2) };
  if (/^[0-9a-f]{8}-[0-9a-f-]{27}$/i.test(value)) return { type: "Uuid", value };
  if (/^\d{4}-\d{2}-\d{2}T/.test(value)) return { type: "TimestampTz", value };
  return { type: "Text", value };
}

const fixtureRows = [
  ["40000000-0000-0000-0000-000000000001", "EU-0184220", "10000000-0000-0000-0000-000000000101", "shipped", 154.92, "EUR", "web", "2026-06-29T08:42:10Z"],
  ["e0000000-0000-0000-0000-000000000002", "EU-0184230", "b0000000-0000-0000-0000-000000000102", "fulfilled", 197.87, "EUR", "android", "2026-06-29T09:18:43Z"],
  ["80000000-0000-0000-0000-000000000003", "EU-0184240", "50000000-0000-0000-0000-000000000103", "pending", 122.90, "EUR", "web", "2026-06-29T10:03:21Z"],
  ["20000000-0000-0000-0000-000000000004", "EU-0184250", "f0000000-0000-0000-0000-000000000104", "fulfilled", 191.37, "EUR", "android", "2026-06-29T10:48:02Z"],
  ["c8000000-0000-0000-0000-000000000005", "EU-0184260", "90000000-0000-0000-0000-000000000105", "delivered", 167.17, "EUR", "web", "2026-06-29T11:22:36Z"],
  ["60000000-0000-0000-0000-000000000006", "EU-0184270", "30000000-0000-0000-0000-000000000106", "paid", 15.64, "EUR", "android", "2026-06-29T12:02:54Z"],
  ["90000000-0000-0000-0000-000000000007", "EU-0184280", "70000000-0000-0000-0000-000000000107", "shipped", 421.12, "EUR", "web", "2026-06-29T12:48:11Z"],
  ["12000000-0000-0000-0000-000000000008", "EU-0184290", "20000000-0000-0000-0000-000000000108", "refunded", 88.40, "EUR", "ios", "2026-06-29T13:21:50Z"],
  ["77000000-0000-0000-0000-000000000009", "EU-0184300", "d0000000-0000-0000-0000-000000000109", "fulfilled", 312.08, "EUR", "web", "2026-06-29T13:59:07Z"],
  ["aa000000-0000-0000-0000-000000000010", "EU-0184310", "60000000-0000-0000-0000-000000000110", "paid", 49.99, "EUR", "ios", "2026-06-29T14:30:28Z"],
];

export const mockCommands = {
  listConnections: async (): Promise<Result<ConnectionConfig[], CellarError>> =>
    ok(screenshotFixtures ? [fixtureConnection] : []),

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

  importDatagrip: async (): Promise<Result<DatagripImport, CellarError>> =>
    ok({ connections: [], skipped: [] }),

  introspect: async (
    _connectionId: string,
    _refresh: boolean | null,
  ): Promise<Result<Database[], CellarError>> =>
    ok(screenshotFixtures ? fixtureDatabases : []),

  findUsages: async (
    _connectionId: string,
    _database: string | null,
    _schema: string,
    _objectName: string,
    _columnName: string | null,
    _allSchemas: boolean | null,
  ): Promise<Result<UsageReference[], CellarError>> => ok([]),

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
    request: TableBrowseRequest,
  ): Promise<Result<QueryResult, CellarError>> =>
    ok(
      screenshotFixtures && request.schema === "public" && request.table === "orders"
        ? {
            columns: orderColumns.map((c) => ({
              name: c.name,
              data_type: c.data_type,
              nullable: c.nullable,
            })),
            rows: fixtureRows.map((row) => row.map(cell)),
            notices: [],
            notice_capture: {
              supported: false,
              reason: "Mock IPC mode does not connect to a database server.",
            },
            rows_affected: null,
            duration_ms: 84,
            truncated: false,
            total_rows: request.include_total ? 1840165 : null,
          }
        : {
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
          },
    ),

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

  commitTableImport: async (
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

  // No native save dialog in web mode — callers fall back to a browser download.
  saveTextFile: async (
    _defaultName: string,
    _contents: string,
    _filterName: string,
    _filterExt: string,
  ): Promise<Result<string | null, CellarError>> => ok(null),
};

const mockAiKeys = new Map<string, string>();
const mockTemplates = new Map<string, QueryTemplate>();

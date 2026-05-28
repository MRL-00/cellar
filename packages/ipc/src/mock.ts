// Mock implementations of the generated `commands` for `pnpm dev:web` mode,
// where the Tauri IPC bridge is not present. They return empty datasets so
// the UI can render without throwing. Anything destructive (save, delete) is
// a no-op that resolves with the input — enough to drive UI flow but never
// pretend to persist.

import type {
  CellarError,
  ConnectionConfig,
  Database,
  DriverInfo,
  QueryResult,
  Result,
} from "./generated";

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

  disconnect: async (_id: string): Promise<Result<null, CellarError>> => ok(null),

  introspect: async (
    _connectionId: string,
    _refresh: boolean | null,
  ): Promise<Result<Database[], CellarError>> => ok([]),

  runQuery: async (
    _connectionId: string,
    _sql: string,
    _maxRows: number | null,
  ): Promise<Result<QueryResult, CellarError>> =>
    ok({
      columns: [],
      rows: [],
      rows_affected: null,
      duration_ms: 0,
      truncated: false,
    }),
};

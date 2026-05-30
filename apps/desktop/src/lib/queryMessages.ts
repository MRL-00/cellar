import type { QueryResult } from "@cellar/ipc";

export type QueryMessageSeverity = "info" | "success" | "warning" | "error";
export type QueryMessageSource = "client" | "execution" | "driver";

export interface SqlRange {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
}

export interface QueryMessage {
  id: string;
  tabId: string;
  connectionId: string;
  database?: string;
  severity: QueryMessageSeverity;
  source: QueryMessageSource;
  text: string;
  timestamp: string;
  sql?: string;
  sqlRange?: SqlRange;
  statementIndex?: number;
  durationMs?: number;
  rowCount?: number;
}

export type PendingQueryMessage = Omit<QueryMessage, "id" | "timestamp"> &
  Partial<Pick<QueryMessage, "id" | "timestamp">>;

export const QUERY_MESSAGE_SEVERITIES: QueryMessageSeverity[] = [
  "info",
  "success",
  "warning",
  "error",
];

export interface TableQueryContext {
  tabId: string;
  connectionId: string;
  database: string;
  schema: string;
  table: string;
  sql: string;
  maxRows: number;
}

export function buildTableLoadStartedMessage(
  context: TableQueryContext,
): PendingQueryMessage {
  return {
    tabId: context.tabId,
    connectionId: context.connectionId,
    database: context.database,
    severity: "info",
    source: "client",
    text: `Loading ${formatQualifiedName(context.schema, context.table)} with row limit ${context.maxRows}.`,
    sql: context.sql,
    statementIndex: 0,
  };
}

export function buildQueryResultMessages(
  context: TableQueryContext,
  result: QueryResult,
): PendingQueryMessage[] {
  const rowCount = result.rows_affected ?? result.rows.length;
  const messages: PendingQueryMessage[] = [
    {
      tabId: context.tabId,
      connectionId: context.connectionId,
      database: context.database,
      severity: "success",
      source: "execution",
      text: result.rows_affected == null
        ? `Loaded ${formatQualifiedName(context.schema, context.table)}: ${formatCount(result.rows.length, "row")} in ${formatDuration(result.duration_ms)}.`
        : `Query OK: ${formatCount(result.rows_affected, "row")} affected in ${formatDuration(result.duration_ms)}.`,
      sql: context.sql,
      statementIndex: 0,
      durationMs: result.duration_ms,
      rowCount,
    },
  ];

  if (result.truncated) {
    messages.push({
      tabId: context.tabId,
      connectionId: context.connectionId,
      database: context.database,
      severity: "warning",
      source: "execution",
      text: `Result hit row limit ${context.maxRows}; showing first ${formatCount(result.rows.length, "row")}.`,
      sql: context.sql,
      statementIndex: 0,
      durationMs: result.duration_ms,
      rowCount: result.rows.length,
    });
  }

  return messages;
}

export function buildQueryErrorMessage(
  context: TableQueryContext,
  error: unknown,
): PendingQueryMessage {
  return {
    tabId: context.tabId,
    connectionId: context.connectionId,
    database: context.database,
    severity: "error",
    source: "driver",
    text: `Failed to load ${formatQualifiedName(context.schema, context.table)}: ${errorMessage(error)}`,
    sql: context.sql,
    statementIndex: 0,
  };
}

export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms} ms`;
  return `${(ms / 1000).toFixed(ms < 10_000 ? 2 : 1)} s`;
}

export function formatMessageTime(timestamp: string): string {
  const d = new Date(timestamp);
  if (Number.isNaN(d.getTime())) return "--:--:--.---";
  const h = String(d.getHours()).padStart(2, "0");
  const m = String(d.getMinutes()).padStart(2, "0");
  const s = String(d.getSeconds()).padStart(2, "0");
  const ms = String(d.getMilliseconds()).padStart(3, "0");
  return `${h}:${m}:${s}.${ms}`;
}

export function severityCounts(messages: QueryMessage[]): Record<QueryMessageSeverity, number> {
  return {
    info: messages.filter((m) => m.severity === "info").length,
    success: messages.filter((m) => m.severity === "success").length,
    warning: messages.filter((m) => m.severity === "warning").length,
    error: messages.filter((m) => m.severity === "error").length,
  };
}

function formatQualifiedName(schema: string, table: string): string {
  return `${schema}.${table}`;
}

function formatCount(count: number, noun: string): string {
  return `${count.toLocaleString()} ${noun}${count === 1 ? "" : "s"}`;
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}

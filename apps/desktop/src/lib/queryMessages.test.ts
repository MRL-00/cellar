import { describe, expect, it } from "vitest";
import type { QueryResult } from "@cellar/ipc";
import {
  buildQueryErrorMessage,
  buildQueryResultMessages,
  buildTableLoadStartedMessage,
  formatDuration,
  severityCounts,
  type QueryMessage,
  type TableQueryContext,
} from "./queryMessages";

const context: TableQueryContext = {
  tabId: "tab-1",
  connectionId: "conn-1",
  database: "app",
  schema: "public",
  table: "orders",
  sql: 'SELECT * FROM "public"."orders" LIMIT 500',
  maxRows: 500,
};

const result: QueryResult = {
  columns: [],
  rows: [[], []],
  notices: [],
  notice_capture: { supported: false, reason: null },
  rows_affected: null,
  duration_ms: 12,
  truncated: false,
  total_rows: null,
};

describe("query message builders", () => {
  it("builds an honest table loading message", () => {
    const message = buildTableLoadStartedMessage(context);

    expect(message.severity).toBe("info");
    expect(message.source).toBe("client");
    expect(message.text).toContain("public.orders");
    expect(message.sql).toBe(context.sql);
  });

  it("builds success and truncation messages from QueryResult", () => {
    const messages = buildQueryResultMessages(context, {
      ...result,
      truncated: true,
    });

    expect(messages).toHaveLength(2);
    expect(messages[0]?.severity).toBe("success");
    expect(messages[0]?.durationMs).toBe(12);
    expect(messages[0]?.rowCount).toBe(2);
    expect(messages[1]?.severity).toBe("warning");
    expect(messages[1]?.text).toContain("row limit 500");
  });

  it("uses affected rows for non-row results", () => {
    const messages = buildQueryResultMessages(context, {
      ...result,
      rows: [],
      rows_affected: 3,
    });

    expect(messages[0]?.rowCount).toBe(3);
    expect(messages[0]?.text).toContain("3 rows affected");
  });

  it("formats driver errors without creating notices", () => {
    const message = buildQueryErrorMessage(context, new Error("permission denied"));

    expect(message.severity).toBe("error");
    expect(message.source).toBe("driver");
    expect(message.text).toContain("permission denied");
  });

  it("counts severities for rendering filters", () => {
    const messages: QueryMessage[] = [
      fullMessage("1", "info"),
      fullMessage("2", "success"),
      fullMessage("3", "warning"),
      fullMessage("4", "warning"),
    ];

    expect(severityCounts(messages)).toEqual({
      info: 1,
      success: 1,
      warning: 2,
      error: 0,
    });
  });

  it("formats short and long durations", () => {
    expect(formatDuration(999)).toBe("999 ms");
    expect(formatDuration(1200)).toBe("1.20 s");
  });
});

function fullMessage(
  id: string,
  severity: QueryMessage["severity"],
): QueryMessage {
  return {
    id,
    tabId: "tab-1",
    connectionId: "conn-1",
    severity,
    source: "execution",
    text: id,
    timestamp: "2026-05-30T00:00:00.000Z",
  };
}

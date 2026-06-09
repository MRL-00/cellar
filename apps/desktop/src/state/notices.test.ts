import type { DatabaseNotice, QueryResult } from "@cellar/ipc";
import { beforeEach, describe, expect, it } from "vitest";

import {
  noticeScopeKey,
  useNotices,
  type NoticeLogEntry,
  type NoticeScope,
} from "./notices";

const scope: NoticeScope = {
  tabId: "conn::postgres.public.orders",
  connectionId: "conn",
  database: "postgres",
};

function sampleNotice(message: string): DatabaseNotice {
  return {
    severity: "notice",
    code: "00000",
    message,
    detail: null,
    hint: null,
    timestamp: "2026-05-30T01:02:03Z",
    connection_id: "conn",
    database: "postgres",
    query_id: "q1",
  };
}

function result(notices: DatabaseNotice[]): QueryResult {
  return {
    columns: [],
    rows: [],
    notices,
    notice_capture: { supported: true, reason: null },
    rows_affected: null,
    duration_ms: 1,
    truncated: false,
    total_rows: null,
  };
}

function entryFor(s: NoticeScope): NoticeLogEntry {
  const entry = useNotices.getState().byScope[noticeScopeKey(s)];
  if (!entry) throw new Error(`missing entry for ${noticeScopeKey(s)}`);
  return entry;
}

describe("notice store", () => {
  beforeEach(() => {
    useNotices.setState({ byScope: {} });
  });

  it("retains notices for the same active scope by default", () => {
    useNotices.getState().recordQueryResult(scope, result([sampleNotice("one")]));
    useNotices.getState().recordQueryResult(scope, result([sampleNotice("two")]));

    const entry = entryFor(scope);
    expect(entry.notices.map((n) => n.message)).toEqual(["one", "two"]);
    expect(entry.retain).toBe(true);
  });

  it("replaces notices when retain is disabled", () => {
    useNotices.getState().recordQueryResult(scope, result([sampleNotice("one")]));
    useNotices.getState().setRetain(scope, false);
    useNotices.getState().recordQueryResult(scope, result([sampleNotice("two")]));

    const entry = entryFor(scope);
    expect(entry.notices.map((n) => n.message)).toEqual(["two"]);
    expect(entry.retain).toBe(false);
  });

  it("clears only the requested scope", () => {
    const other = { ...scope, tabId: "conn::postgres.public.customers" };
    useNotices.getState().appendNotice(scope, sampleNotice("orders"));
    useNotices.getState().appendNotice(other, sampleNotice("customers"));

    useNotices.getState().clear(scope);

    expect(entryFor(scope).notices).toEqual([]);
    expect(entryFor(other).notices).toHaveLength(1);
  });
});

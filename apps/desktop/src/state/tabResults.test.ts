import { beforeEach, describe, expect, it } from "vitest";
import type { GridColumn, GridRow } from "@cellar/data-grid";

import {
  maxRowsLabel,
  queryResultSource,
  resultContextLabel,
  rowCountLabel,
  useTabResults,
} from "./tabResults";

const source = queryResultSource(
  "conn-1",
  "app",
  "sql-tab-1",
  "Query 1",
  "SELECT * FROM orders LIMIT 500",
  500,
);

const columns: GridColumn[] = [
  { key: "order_id", name: "order_id", type: "int8", width: 110 },
];
const rows: GridRow[] = [{ id: "0", order_id: 1 }];

describe("tab result state", () => {
  beforeEach(() => {
    useTabResults.setState({ byTabId: {} });
  });

  it("keeps result data scoped to the producing tab", () => {
    useTabResults.getState().setLoading("tab-a", source);
    useTabResults.getState().setReady("tab-a", {
      source,
      columns,
      rows,
      rowCount: rows.length,
      truncated: false,
      durationMs: 12,
    });

    expect(useTabResults.getState().byTabId["tab-a"]).toMatchObject({
      status: "ready",
      rowCount: 1,
      durationMs: 12,
    });
    expect(useTabResults.getState().byTabId["tab-b"]).toBeUndefined();
  });

  it("clears only the closed tab result", () => {
    useTabResults.getState().setLoading("tab-a", source);
    useTabResults.getState().setLoading("tab-b", source);
    useTabResults.getState().clearTab("tab-a");

    expect(useTabResults.getState().byTabId["tab-a"]).toBeUndefined();
    expect(useTabResults.getState().byTabId["tab-b"]?.status).toBe("loading");
  });
});

describe("tab result labels", () => {
  it("formats query context and row limits honestly", () => {
    expect(resultContextLabel(source)).toBe("app.Query 1");
    expect(rowCountLabel(1, false)).toBe("1 row");
    expect(rowCountLabel(500, true)).toBe("500+ rows");
    expect(maxRowsLabel(500, false)).toBe("max 500");
    expect(maxRowsLabel(500, true)).toBe("capped at 500");
  });
});

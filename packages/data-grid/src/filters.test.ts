import { describe, expect, it } from "vitest";
import {
  evaluateFilterClause,
  filterRows,
  operatorsForColumn,
} from "./filters";
import type { FilterClause, GridColumn, GridRow, PendingChanges } from "./types";

const columns: readonly GridColumn[] = [
  { key: "status", name: "status", type: "text", width: 120 },
  { key: "total", name: "total", type: "numeric", width: 90 },
  { key: "note", name: "note", type: "text", width: 160, nullable: true },
  { key: "created_at", name: "created_at", type: "timestamptz", width: 180 },
  { key: "active", name: "active", type: "bool", width: 80 },
];

const rows: readonly GridRow[] = [
  {
    id: "r1",
    status: "paid",
    total: 42,
    note: "VIP customer",
    created_at: "2026-05-01T10:00:00Z",
    active: "true",
  },
  {
    id: "r2",
    status: "pending",
    total: 12,
    note: null,
    created_at: "2026-05-03T10:00:00Z",
    active: "false",
  },
  {
    id: "r3",
    status: "paid",
    total: 99,
    note: "expedite",
    created_at: "2026-05-05T10:00:00Z",
    active: "true",
  },
];

function clause(
  id: string,
  columnKey: string,
  operator: FilterClause["operator"],
  value?: string,
  logic: FilterClause["logic"] = "and",
): FilterClause {
  return { id, columnKey, operator, value, logic };
}

describe("filterRows", () => {
  it("supports multiple clauses against the same column", () => {
    const result = filterRows(rows, columns, [
      clause("a", "status", "equals", "paid"),
      clause("b", "status", "notEquals", "pending"),
      clause("c", "total", "greaterThan", "50"),
    ]);

    expect(result.map((row) => row.id)).toEqual(["r3"]);
  });

  it("combines clauses with OR when requested", () => {
    const result = filterRows(rows, columns, [
      clause("a", "status", "equals", "pending"),
      clause("b", "total", "greaterThan", "90", "or"),
    ]);

    expect(result.map((row) => row.id)).toEqual(["r2", "r3"]);
  });

  it("evaluates null and string operators deterministically", () => {
    expect(
      filterRows(rows, columns, [clause("a", "note", "isNull")]).map(
        (row) => row.id,
      ),
    ).toEqual(["r2"]);
    expect(
      filterRows(rows, columns, [clause("a", "note", "contains", "vip")]).map(
        (row) => row.id,
      ),
    ).toEqual(["r1"]);
  });

  it("uses pending edit values without dropping the pending change", () => {
    const changes: PendingChanges = {
      r2: {
        kind: "update",
        edits: { status: { from: "pending", to: "paid" } },
      },
    };

    const result = filterRows(
      rows,
      columns,
      [clause("a", "status", "equals", "paid")],
      changes,
    );

    expect(result.map((row) => row.id)).toEqual(["r1", "r2", "r3"]);
    expect(changes.r2?.edits.status?.to).toBe("paid");
  });
});

describe("operatorsForColumn", () => {
  it("omits text-only operators for numeric and boolean columns", () => {
    expect(operatorsForColumn(columns[1]!)).toEqual([
      "equals",
      "notEquals",
      "greaterThan",
      "lessThan",
    ]);
    expect(operatorsForColumn(columns[4]!)).toEqual(["equals", "notEquals"]);
  });

  it("adds null operators only for nullable columns", () => {
    expect(operatorsForColumn(columns[2]!)).toContain("isNull");
    expect(operatorsForColumn(columns[0]!)).not.toContain("isNull");
  });
});

describe("evaluateFilterClause", () => {
  it("compares dates through parseable timestamps", () => {
    expect(
      evaluateFilterClause(rows[2]!.created_at, columns[3]!, {
        id: "a",
        columnKey: "created_at",
        operator: "greaterThan",
        value: "2026-05-04T00:00:00Z",
        logic: "and",
      }),
    ).toBe(true);
  });
});

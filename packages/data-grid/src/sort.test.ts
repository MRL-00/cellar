import { describe, expect, it } from "vitest";
import type { GridColumn, GridRow, PendingChanges } from "./types";
import { compareGridValues, cycleSortState, sortGridRows } from "./sort";

const columns: readonly GridColumn[] = [
  { key: "name", name: "name", type: "text", width: 120 },
  { key: "amount", name: "amount", type: "numeric", width: 90 },
  { key: "active", name: "active", type: "bool", width: 70 },
  { key: "created_at", name: "created_at", type: "timestamptz", width: 180 },
];

describe("cycleSortState", () => {
  it("cycles unsorted to ascending to descending to unsorted", () => {
    const asc = cycleSortState(null, "name");
    expect(asc).toEqual({ columnKey: "name", direction: "asc" });
    const desc = cycleSortState(asc, "name");
    expect(desc).toEqual({ columnKey: "name", direction: "desc" });
    expect(cycleSortState(desc, "name")).toBeNull();
  });

  it("starts ascending when switching columns", () => {
    expect(
      cycleSortState({ columnKey: "amount", direction: "desc" }, "name"),
    ).toEqual({ columnKey: "name", direction: "asc" });
  });
});

describe("compareGridValues", () => {
  it("sorts nullish values after real values", () => {
    expect(compareGridValues(null, "A", columns[0]!)).toBeGreaterThan(0);
    expect(compareGridValues(undefined, null, columns[0]!)).toBe(0);
  });

  it("sorts numeric strings by numeric value for numeric columns", () => {
    expect(compareGridValues("9.5", "10", columns[1]!)).toBeLessThan(0);
  });

  it("sorts booleans false before true", () => {
    expect(compareGridValues(false, true, columns[2]!)).toBeLessThan(0);
    expect(compareGridValues("true", "false", columns[2]!)).toBeGreaterThan(0);
  });

  it("sorts practical date-like strings chronologically", () => {
    expect(
      compareGridValues(
        "2026-05-28 09:00:00+00",
        "2026-05-29 08:00:00+00",
        columns[3]!,
      ),
    ).toBeLessThan(0);
  });
});

describe("sortGridRows", () => {
  const rows: readonly GridRow[] = [
    { id: "r1", name: "A", amount: "10", active: true },
    { id: "r2", name: "B", amount: "2", active: false },
    { id: "r3", name: "C", amount: "2", active: true },
    { id: "r4", name: "D", amount: null, active: false },
  ];

  it("sorts stably and keeps row ids attached to their rows", () => {
    expect(
      sortGridRows(rows, columns, { columnKey: "amount", direction: "asc" }).map(
        (row) => row.id,
      ),
    ).toEqual(["r2", "r3", "r1", "r4"]);
  });

  it("uses pending displayed values for local ordering", () => {
    const changes: PendingChanges = {
      r1: { kind: "update", edits: { amount: { from: "10", to: "1" } } },
    };
    expect(
      sortGridRows(
        rows,
        columns,
        { columnKey: "amount", direction: "asc" },
        changes,
      ).map((row) => row.id),
    ).toEqual(["r1", "r2", "r3", "r4"]);
  });
});

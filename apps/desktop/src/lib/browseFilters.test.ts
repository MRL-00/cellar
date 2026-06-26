import { describe, expect, it } from "vitest";
import type { FilterClause, GridColumn } from "@cellar/data-grid";
import {
  advancedFiltersToClauses,
  buildBrowseFilters,
  quickFilterToClause,
  sortToClauses,
} from "./browseFilters";

const columns: GridColumn[] = [
  { key: "id", name: "id", type: "int4", width: 0, pk: true },
  { key: "status", name: "status", type: "text", width: 0 },
  { key: "name", name: "name", type: "varchar", width: 0 },
];

const chip = (over: Partial<FilterClause>): FilterClause => ({
  id: over.id ?? "c1",
  columnKey: over.columnKey ?? "status",
  operator: over.operator ?? "equals",
  logic: over.logic ?? "and",
  value: over.value,
});

describe("advancedFiltersToClauses", () => {
  it("maps a supported chip to a server clause", () => {
    expect(advancedFiltersToClauses([chip({ value: "active" })])).toEqual([
      { column: "status", operator: "equals", value: "active" },
    ]);
  });

  it("emits null value for null-check operators", () => {
    expect(
      advancedFiltersToClauses([chip({ operator: "isNull" })]),
    ).toEqual([{ column: "status", operator: "is_null", value: null }]);
  });

  it("drops value-needing chips with empty values", () => {
    expect(advancedFiltersToClauses([chip({ value: "  " })])).toEqual([]);
  });

  it("skips operators without a server equivalent (startsWith)", () => {
    expect(
      advancedFiltersToClauses([chip({ operator: "startsWith", value: "a" })]),
    ).toEqual([]);
  });

  it("pushes nothing when chips use OR (no server grouping)", () => {
    expect(
      advancedFiltersToClauses([
        chip({ id: "a", value: "x" }),
        chip({ id: "b", columnKey: "name", value: "y", logic: "or" }),
      ]),
    ).toEqual([]);
  });
});

describe("quickFilterToClause", () => {
  it("matches a numeric value against the id/PK column with equals", () => {
    expect(quickFilterToClause("42", columns, null)).toEqual({
      column: "id",
      operator: "equals",
      value: "42",
    });
  });

  it("falls back to contains on the first text column for free text", () => {
    expect(quickFilterToClause("acme", columns, null)).toEqual({
      column: "status",
      operator: "contains",
      value: "acme",
    });
  });

  it("uses the selected text column when provided", () => {
    expect(quickFilterToClause("acme", columns, "name")).toEqual({
      column: "name",
      operator: "contains",
      value: "acme",
    });
  });

  it("returns null for an empty value", () => {
    expect(quickFilterToClause("   ", columns, null)).toBeNull();
  });
});

describe("buildBrowseFilters", () => {
  it("combines advanced clauses with the quick filter (server ANDs them)", () => {
    const result = buildBrowseFilters(
      columns,
      [chip({ value: "active" })],
      "42",
      null,
    );
    expect(result).toEqual([
      { column: "status", operator: "equals", value: "active" },
      { column: "id", operator: "equals", value: "42" },
    ]);
  });
});

describe("sortToClauses", () => {
  it("maps a sort state to a single clause", () => {
    expect(sortToClauses({ columnKey: "name", direction: "desc" })).toEqual([
      { column: "name", direction: "desc" },
    ]);
  });

  it("returns nothing when unsorted", () => {
    expect(sortToClauses(null)).toEqual([]);
  });
});

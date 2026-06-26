import { describe, expect, it } from "vitest";
import type { Database } from "@cellar/ipc";

import {
  inferParamType,
  inputTypeForDataType,
  toCellValue,
} from "./queryParamValues";

/** Minimal Database fixture: one db → one schema → tables of (name, type). */
function db(
  name: string,
  tables: Record<string, Array<[string, string]>>,
): Database {
  return {
    name,
    is_default: true,
    schemas: [
      {
        name: "public",
        tables: Object.entries(tables).map(([table, cols]) => ({
          name: table,
          schema: "public",
          row_count: null,
          columns: cols.map(([col, dataType], i) => ({
            name: col,
            data_type: dataType,
            nullable: true,
            default: null,
            is_primary_key: false,
            ordinal: i + 1,
            comment: null,
          })),
          primary_key: [],
          foreign_keys: [],
          indexes: [],
        })),
        views: [],
      },
    ],
  } as unknown as Database;
}

describe("inputTypeForDataType", () => {
  it("maps engine types to input kinds", () => {
    expect(inputTypeForDataType("int4")).toBe("number");
    expect(inputTypeForDataType("bigint")).toBe("number");
    expect(inputTypeForDataType("numeric")).toBe("number");
    expect(inputTypeForDataType("bool")).toBe("boolean");
    expect(inputTypeForDataType("date")).toBe("date");
    expect(inputTypeForDataType("text")).toBe("text");
    // `interval` contains "int" but must not be treated as a number.
    expect(inputTypeForDataType("interval")).toBe("text");
  });
});

describe("inferParamType", () => {
  it("infers from a unique column match", () => {
    const dbs = [db("shop", { orders: [["id", "int8"]] })];
    expect(inferParamType("id", dbs)).toBe("number");
  });

  it("falls back to text when no hint or no match", () => {
    const dbs = [db("shop", { orders: [["id", "int8"]] })];
    expect(inferParamType(null, dbs)).toBe("text");
    expect(inferParamType("missing", dbs)).toBe("text");
  });

  it("falls back to text when the column name is ambiguous across types", () => {
    const dbs = [
      db("shop", {
        orders: [["id", "int8"]],
        events: [["id", "uuid"]],
      }),
    ];
    // `id` is int8 in one table and uuid in another → don't guess.
    expect(inferParamType("id", dbs)).toBe("text");
  });

  it("scopes to the tab's database before widening", () => {
    const dbs = [
      db("primary", { orders: [["status", "int4"]] }),
      db("legacy", { orders: [["status", "text"]] }),
    ];
    // Scoped to `primary`, `status` is unambiguously numeric.
    expect(inferParamType("status", dbs, "primary")).toBe("number");
    // Without scoping the two databases disagree → text.
    expect(inferParamType("status", dbs)).toBe("text");
  });
});

describe("toCellValue", () => {
  it("validates numbers and dates", () => {
    expect(toCellValue({ type: "number", value: "42" })).toEqual({
      ok: true,
      value: { type: "Int", value: 42 },
    });
    expect(toCellValue({ type: "number", value: "1.5" })).toEqual({
      ok: true,
      value: { type: "Float", value: 1.5 },
    });
    expect(toCellValue({ type: "number", value: "nope" }).ok).toBe(false);
    expect(toCellValue({ type: "date", value: "2026-06-26" })).toEqual({
      ok: true,
      value: { type: "Date", value: "2026-06-26" },
    });
    expect(toCellValue({ type: "date", value: "26/06/2026" }).ok).toBe(false);
    expect(toCellValue({ type: "null", value: "" })).toEqual({
      ok: true,
      value: { type: "Null" },
    });
  });
});

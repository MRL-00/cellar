import { describe, expect, it } from "vitest";

import type { GridColumn, GridRow } from "@cellar/data-grid";
import {
  exportFilename,
  toCsv,
  toJson,
  toSqlInserts,
  toTsv,
} from "./export";

function col(name: string, type = "text"): GridColumn {
  return { key: name, name, type, width: 100 };
}

describe("toCsv", () => {
  const cols = [col("id"), col("note")];

  it("quotes fields with delimiters, quotes, and newlines", () => {
    const data: GridRow[] = [
      { id: "row:0", note: 'say "hi", twice' },
      { id: "row:1", note: "line1\nline2" },
    ];
    const csv = toCsv(cols, data);
    expect(csv).toBe(
      'id,note\r\nrow:0,"say ""hi"", twice"\r\nrow:1,"line1\nline2"\r\n',
    );
  });

  it("distinguishes NULL (empty) from empty string (quoted)", () => {
    const data: GridRow[] = [{ id: "row:0", note: null }, { id: "row:1", note: "" }];
    const csv = toCsv(cols, data);
    expect(csv).toBe('id,note\r\nrow:0,\r\nrow:1,""\r\n');
  });
});

describe("toTsv", () => {
  it("uses tabs and quotes embedded tabs", () => {
    const cols = [col("a"), col("b")];
    const data: GridRow[] = [{ id: "row:0", a: "x", b: "with\ttab" }];
    expect(toTsv(cols, data)).toBe('a\tb\r\nx\t"with\ttab"\r\n');
  });
});

describe("toJson", () => {
  it("emits an array of objects with null for NULL", () => {
    const data: GridRow[] = [
      { id: "row:0", name: "Ada", active: true },
      { id: "row:1", name: null, active: false },
    ];
    const cols = [col("name"), col("active", "bool")];
    expect(JSON.parse(toJson(cols, data))).toEqual([
      { name: "Ada", active: true },
      { name: null, active: false },
    ]);
  });
});

describe("toSqlInserts", () => {
  it("emits one runnable INSERT per row with escaped values", () => {
    const cols = [col("id", "int8"), col("name"), col("active", "bool")];
    const data: GridRow[] = [
      { id: "row:0", name: "O'Brien", active: true },
      { id: "row:1", name: null, active: false },
    ];
    // The grid's synthetic row id collides with a real `id` column here; the
    // exporter reads whatever the column key resolves to, so use numbers.
    const numbered: GridRow[] = data.map((r, i) => ({ ...r, id: String(i + 1) }));
    const sql = toSqlInserts(cols, numbered, { table: "people" });
    expect(sql).toBe(
      `INSERT INTO "people" ("id", "name", "active") VALUES ('1', 'O''Brien', TRUE);\n` +
        `INSERT INTO "people" ("id", "name", "active") VALUES ('2', NULL, FALSE);\n`,
    );
  });

  it("doubles embedded quotes in identifiers", () => {
    const cols = [col('we"ird')];
    const sql = toSqlInserts(cols, [{ id: "row:0", 'we"ird': "v" }]);
    expect(sql).toContain('INSERT INTO "results" ("we""ird") VALUES (\'v\');');
  });
});

describe("exportFilename", () => {
  it("slugs the tab title and appends the extension", () => {
    expect(exportFilename("untitled-1.sql", "csv")).toBe("untitled-1.csv");
    expect(exportFilename("My Query!", "json")).toBe("my-query.json");
    expect(exportFilename("???", "sql")).toBe("result.sql");
  });
});

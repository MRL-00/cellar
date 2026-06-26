import { describe, expect, it } from "vitest";
import {
  autoMap,
  buildImportRequest,
  importCounts,
  parseCsv,
  validateImport,
  type ImportColumn,
  type ImportConfig,
} from "./csvImport";

const COLS: ImportColumn[] = [
  { name: "id", data_type: "int4", nullable: false, is_primary_key: true, has_default: false },
  { name: "name", data_type: "text", nullable: true, is_primary_key: false, has_default: false },
  { name: "status", data_type: "text", nullable: true, is_primary_key: false, has_default: false },
];

function cfg(over: Partial<ImportConfig>): ImportConfig {
  return {
    database: "shop",
    schema: "public",
    table: "users",
    tableColumns: COLS,
    mapping: { id: 0, name: 1, status: 2 },
    matchKeys: ["id"],
    mode: "upsert",
    updateFields: ["name", "status"],
    ...over,
  };
}

describe("parseCsv", () => {
  it("reads a header row and data rows", () => {
    const csv = parseCsv("id,name,status\n1,Ada,active\n2,Grace,paused");
    expect(csv.headers).toEqual(["id", "name", "status"]);
    expect(csv.rows).toEqual([
      ["1", "Ada", "active"],
      ["2", "Grace", "paused"],
    ]);
  });

  it("handles quoted values, embedded commas, newlines and escaped quotes", () => {
    const csv = parseCsv('id,note\n1,"a, b\nc"\n2,"say ""hi"""');
    expect(csv.rows).toEqual([
      ["1", "a, b\nc"],
      ["2", 'say "hi"'],
    ]);
  });

  it("distinguishes unquoted-empty (NULL) from quoted-empty (string)", () => {
    const csv = parseCsv('id,name\n1,\n2,""');
    expect(csv.rows).toEqual([
      ["1", null],
      ["2", ""],
    ]);
  });

  it("tolerates CRLF and a trailing newline", () => {
    const csv = parseCsv("id,name\r\n1,Ada\r\n");
    expect(csv.headers).toEqual(["id", "name"]);
    expect(csv.rows).toEqual([["1", "Ada"]]);
  });

  it("sniffs a tab delimiter", () => {
    const csv = parseCsv("id\tname\n1\tAda");
    expect(csv.headers).toEqual(["id", "name"]);
    expect(csv.rows).toEqual([["1", "Ada"]]);
  });
});

describe("autoMap", () => {
  it("maps by exact then case-insensitive name", () => {
    expect(autoMap(["ID", "name", "extra"], COLS)).toEqual({ id: 0, name: 1 });
  });
});

describe("buildImportRequest", () => {
  const csv = parseCsv("id,name,status\n1,Ada,active\n2,Grace,paused");

  it("update mode never sets the match key", () => {
    const req = buildImportRequest(csv, cfg({ mode: "update" }));
    expect(req.primary_key).toEqual(["id"]);
    const change = req.changes[0]!;
    expect(change.kind).toBe("update");
    if (change.kind !== "update") throw new Error("expected update");
    expect(change.keys.map((k) => k.column)).toEqual(["id"]);
    expect(change.edits.map((e) => e.column)).toEqual(["name", "status"]);
  });

  it("upsert mode excludes the match key from update_columns", () => {
    const req = buildImportRequest(csv, cfg({ updateFields: ["id", "name", "status"] }));
    const change = req.changes[0]!;
    if (change.kind !== "upsert") throw new Error("expected upsert");
    expect(change.conflict_columns).toEqual(["id"]);
    expect(change.values.map((v) => v.column)).toEqual(["id", "name", "status"]);
    expect(change.update_columns).toEqual(["name", "status"]);
  });

  it("insert-only mode emits empty update_columns (DO NOTHING)", () => {
    const req = buildImportRequest(csv, cfg({ mode: "insert" }));
    const change = req.changes[0]!;
    if (change.kind !== "upsert") throw new Error("expected upsert");
    expect(change.update_columns).toEqual([]);
  });

  it("skips rows missing a match-key value", () => {
    const sparse = parseCsv("id,name\n1,Ada\n,Nobody");
    const req = buildImportRequest(
      sparse,
      cfg({ mapping: { id: 0, name: 1 }, updateFields: ["name"] }),
    );
    expect(req.changes).toHaveLength(1);
  });

  it("is deterministic — re-running the same CSV yields identical changes", () => {
    const a = buildImportRequest(csv, cfg({}));
    const b = buildImportRequest(csv, cfg({}));
    expect(a).toEqual(b);
  });
});

describe("importCounts", () => {
  it("splits writable from skipped rows", () => {
    const csv = parseCsv("id,name\n1,Ada\n2,Grace\n,Nobody");
    expect(importCounts(csv, cfg({ mapping: { id: 0, name: 1 } }))).toEqual({
      total: 3,
      toWrite: 2,
      skipped: 1,
    });
  });
});

describe("validateImport", () => {
  it("flags an unmapped match key", () => {
    const csv = parseCsv("name\nAda");
    const errs = validateImport(csv, cfg({ mapping: { name: 0 } }));
    expect(errs.some((e) => e.includes('Match key "id"'))).toBe(true);
  });

  it("flags update/upsert with no fields to write", () => {
    const csv = parseCsv("id\n1");
    const errs = validateImport(csv, cfg({ mapping: { id: 0 }, updateFields: [] }));
    expect(errs.some((e) => e.includes("at least one field"))).toBe(true);
  });

  it("flags a required (NOT NULL, no default) column unmapped on insert", () => {
    const csv = parseCsv("id\n1");
    const required: ImportColumn[] = [
      ...COLS,
      { name: "email", data_type: "text", nullable: false, is_primary_key: false, has_default: false },
    ];
    const errs = validateImport(
      csv,
      cfg({ tableColumns: required, mapping: { id: 0 }, mode: "insert" }),
    );
    expect(errs.some((e) => e.includes('"email"'))).toBe(true);
  });

  it("passes a well-formed upsert", () => {
    const csv = parseCsv("id,name,status\n1,Ada,active");
    expect(validateImport(csv, cfg({}))).toEqual([]);
  });
});

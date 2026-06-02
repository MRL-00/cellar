import { describe, expect, it } from "vitest";
import {
  partitionStatements,
  splitStatements,
  statementAtOffset,
} from "./sqlStatements";

describe("splitStatements", () => {
  it("splits on top-level semicolons and trims each statement", () => {
    const sql = "SELECT 1;\nSELECT 2;";
    const stmts = splitStatements(sql);
    expect(stmts.map((s) => s.text)).toEqual(["SELECT 1", "SELECT 2"]);
  });

  it("keeps a trailing statement with no terminating semicolon", () => {
    const stmts = splitStatements("SELECT 1;\nSELECT 2");
    expect(stmts.map((s) => s.text)).toEqual(["SELECT 1", "SELECT 2"]);
  });

  it("ignores semicolons inside string literals", () => {
    const stmts = splitStatements("SELECT ';not a split';");
    expect(stmts).toHaveLength(1);
    expect(stmts[0]?.text).toBe("SELECT ';not a split'");
  });

  it("ignores semicolons inside line and block comments", () => {
    const sql = "SELECT 1; -- trailing; comment\nSELECT 2 /* a; b */;";
    const stmts = splitStatements(sql);
    expect(stmts.map((s) => s.text)).toEqual([
      "SELECT 1",
      "-- trailing; comment\nSELECT 2 /* a; b */",
    ]);
  });

  it("ignores semicolons inside dollar-quoted bodies", () => {
    const sql = "CREATE FUNCTION f() RETURNS int AS $$ BEGIN; RETURN 1; END; $$;";
    const stmts = splitStatements(sql);
    expect(stmts).toHaveLength(1);
    expect(stmts[0]?.text.startsWith("CREATE FUNCTION")).toBe(true);
  });

  it("drops whitespace-only chunks", () => {
    expect(splitStatements("SELECT 1;\n\n  \n")).toHaveLength(1);
  });
});

describe("partitionStatements", () => {
  it("reports 1-based line ranges for multi-line statements", () => {
    const sql = "SELECT\n  a,\n  b\nFROM t;\nSELECT 1;";
    const [first, second] = partitionStatements(sql);
    expect([first?.startLine, first?.endLine]).toEqual([1, 4]);
    expect([second?.startLine, second?.endLine]).toEqual([5, 5]);
  });
});

describe("statementAtOffset", () => {
  const sql = "SELECT 1;\nSELECT 2;\nSELECT 3";

  it("returns the statement containing the caret", () => {
    expect(statementAtOffset(sql, 13)?.text).toBe("SELECT 2");
  });

  it("assigns blank space after a semicolon to the next statement", () => {
    const padded = "SELECT 1;\n\n\nSELECT 2;";
    // Offset 10 is on the blank line following the first statement's `;`.
    expect(statementAtOffset(padded, 10)?.text).toBe("SELECT 2");
  });

  it("falls back to the preceding statement for caret in trailing blank space", () => {
    // The blank tail after the last `;` is its own empty chunk.
    expect(statementAtOffset("SELECT 1;\n\n", 10)?.text).toBe("SELECT 1");
  });

  it("returns null for an empty buffer", () => {
    expect(statementAtOffset("   \n  ", 2)).toBeNull();
  });
});

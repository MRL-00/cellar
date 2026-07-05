import { describe, expect, it } from "vitest";
import { canRunFromAi, firstRunnableSql, parseSegments } from "./AIMessage";

describe("parseSegments", () => {
  it("returns a single text segment for plain prose", () => {
    expect(parseSegments("just words")).toEqual([
      { kind: "text", text: "just words" },
    ]);
  });

  it("extracts a fenced sql block with its language", () => {
    const segs = parseSegments("Here you go:\n```sql\nSELECT 1;\n```\nDone.");
    expect(segs).toEqual([
      { kind: "text", text: "Here you go:" },
      { kind: "code", lang: "sql", code: "SELECT 1;" },
      { kind: "text", text: "Done." },
    ]);
  });

  it("handles a block with no language tag", () => {
    const segs = parseSegments("```\nplain\n```");
    expect(segs).toEqual([{ kind: "code", lang: "", code: "plain" }]);
  });

  it("handles multiple code blocks", () => {
    const segs = parseSegments("```sql\nA\n```\nmid\n```sql\nB\n```");
    expect(segs.filter((s) => s.kind === "code")).toHaveLength(2);
  });
});

describe("canRunFromAi", () => {
  it("allows read-only SQL blocks", () => {
    expect(canRunFromAi("sql", "SELECT * FROM public.orders")).toBe(true);
    expect(canRunFromAi("postgres", "-- check\nWITH x AS (SELECT 1) SELECT * FROM x")).toBe(true);
  });

  it("blocks write SQL and non-SQL fences", () => {
    expect(canRunFromAi("sql", "DELETE FROM public.orders")).toBe(false);
    expect(canRunFromAi("sql", "EXPLAIN ANALYZE DELETE FROM public.orders")).toBe(false);
    expect(canRunFromAi("sql", "WITH gone AS (DELETE FROM t RETURNING *) SELECT * FROM gone")).toBe(false);
    expect(canRunFromAi("sql", "CREATE TABLE public.x (id int)")).toBe(false);
    expect(canRunFromAi("text", "SELECT 1")).toBe(false);
  });
});

describe("firstRunnableSql", () => {
  it("returns the first safe SQL block", () => {
    expect(firstRunnableSql("x\n```sql\nDELETE FROM t\n```\ny\n```sql\nSELECT 1\n```")).toBe("SELECT 1");
  });
});

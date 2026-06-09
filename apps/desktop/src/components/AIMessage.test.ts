import { describe, expect, it } from "vitest";
import { parseSegments } from "./AIMessage";

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

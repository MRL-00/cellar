import { describe, expect, it } from "vitest";
import { calculateVirtualRows, shouldVirtualizeRows } from "./DataGrid";

describe("shouldVirtualizeRows", () => {
  it("keeps ordinary 500-row pages in normal row flow", () => {
    expect(shouldVirtualizeRows(500)).toBe(false);
  });

  it("only virtualizes larger result sets", () => {
    expect(shouldVirtualizeRows(1_000)).toBe(false);
    expect(shouldVirtualizeRows(1_001)).toBe(true);
  });
});

describe("calculateVirtualRows", () => {
  it("renders a bounded multi-viewport window for a 500-row table", () => {
    const range = calculateVirtualRows({
      rowCount: 500,
      viewportHeight: 550,
      scrollTop: 0,
      rowHeight: 22,
    });

    expect(range).toEqual({
      first: 0,
      last: 189,
      totalHeight: 11_000,
    });
  });

  it("keeps the rendered window bounded when scrolled deep into the table", () => {
    const range = calculateVirtualRows({
      rowCount: 500,
      viewportHeight: 550,
      scrollTop: 8_000,
      rowHeight: 22,
    });

    expect(range.first).toBe(281);
    expect(range.last - range.first).toBe(189);
    expect(range.totalHeight).toBe(11_000);
  });
});

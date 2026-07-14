import { describe, expect, it } from "vitest";
import { parseTime, placeCalendarPopover, ymd } from "./GridDateEditor";

describe("ymd", () => {
  it("formats from local components (no UTC day-shift)", () => {
    // Construct via local Y/M/D so the test is timezone-independent.
    expect(ymd(new Date(2023, 3, 3))).toBe("2023-04-03");
    expect(ymd(new Date(2023, 10, 8))).toBe("2023-11-08");
  });
});

describe("parseTime", () => {
  it("extracts HH:MM:SS from a stored timestamp, padding seconds", () => {
    expect(parseTime("2023-04-03T05:00:31.15863")).toBe("05:00:31");
    expect(parseTime("2023-04-03T05:00")).toBe("05:00:00");
  });

  it("defaults to midnight when no time is present", () => {
    expect(parseTime("2023-04-03")).toBe("00:00:00");
    expect(parseTime("")).toBe("00:00:00");
  });
});

describe("placeCalendarPopover", () => {
  const bounds = { top: 8, bottom: 500, left: 8, right: 800 };
  const panel = { width: 280, height: 320 };

  it("opens below the cell when there is room in the scroll viewport", () => {
    expect(
      placeCalendarPopover(
        { top: 100, left: 40, bottom: 124 },
        panel,
        bounds,
      ),
    ).toEqual({ top: 124, left: 40 });
  });

  it("flips above the cell when a downward open would hit the pending bar", () => {
    // Cell near the bottom of .grid-scroll; calendar would overflow into the
    // pending/pagination bars if placed below.
    expect(
      placeCalendarPopover(
        { top: 400, left: 40, bottom: 424 },
        panel,
        bounds,
      ),
    ).toEqual({ top: 80, left: 40 });
  });

  it("clamps horizontally inside the viewport", () => {
    expect(
      placeCalendarPopover(
        { top: 100, left: 750, bottom: 124 },
        panel,
        bounds,
      ),
    ).toEqual({ top: 124, left: 520 });
  });
});

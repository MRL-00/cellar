import { describe, expect, it } from "vitest";
import { parseTime, ymd } from "./GridDateEditor";

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
  });
});

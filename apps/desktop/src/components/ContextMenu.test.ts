import { describe, expect, it } from "vitest";

import { contextMenuPosition } from "./ContextMenu";

describe("contextMenuPosition", () => {
  it("compensates fixed coordinates for the app UI scale", () => {
    expect(
      contextMenuPosition({
        x: 154,
        y: 67,
        viewportWidth: 800,
        viewportHeight: 600,
        menuWidth: 190,
        menuHeight: 123,
        scale: 1.75,
      }),
    ).toEqual({
      left: 88,
      top: 38.285714285714285,
    });
  });

  it("clamps the menu inside the visible viewport before scale compensation", () => {
    expect(
      contextMenuPosition({
        x: 780,
        y: 590,
        viewportWidth: 800,
        viewportHeight: 600,
        menuWidth: 190,
        menuHeight: 123,
        scale: 2,
      }),
    ).toEqual({
      left: 301,
      top: 234.5,
    });
  });
});

import { describe, expect, it } from "vitest";

describe("@cellar/ipc scaffold", () => {
  it("has no hand-written IPC surface before bindings are generated", async () => {
    const module = await import("./index");

    expect(Object.keys(module)).toEqual([]);
  });
});

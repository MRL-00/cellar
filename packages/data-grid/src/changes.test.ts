import { describe, expect, it } from "vitest";
import { applyCellChange } from "./changes";
import type { PendingChanges } from "./types";

describe("applyCellChange", () => {
  it("records an update edit against an existing row", () => {
    const next = applyCellChange({}, "r1", "id", null, "a-guid");
    expect(next).toEqual({
      r1: { kind: "update", edits: { id: { from: null, to: "a-guid" } } },
    });
  });

  it("preserves insert kind when editing a pending insert", () => {
    const changes: PendingChanges = {
      "insert:1": { kind: "insert", edits: {} },
    };
    const next = applyCellChange(changes, "insert:1", "id", null, "a-guid");
    expect(next["insert:1"]?.kind).toBe("insert");
    expect(next["insert:1"]?.edits.id).toEqual({ from: null, to: "a-guid" });
  });

  it("anchors from to the original value across successive edits", () => {
    const once = applyCellChange({}, "r1", "id", "old", "mid");
    const twice = applyCellChange(once, "r1", "id", "mid", "new");
    expect(twice.r1?.edits.id).toEqual({ from: "old", to: "new" });
  });

  it("drops an update when the value returns to original", () => {
    const once = applyCellChange({}, "r1", "id", "old", "new");
    const next = applyCellChange(once, "r1", "id", "new", "old");
    expect(next).toEqual({});
  });

  it("returns the same object when prev equals next", () => {
    const changes: PendingChanges = {};
    expect(applyCellChange(changes, "r1", "id", "same", "same")).toBe(changes);
  });
});

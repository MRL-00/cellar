import { describe, expect, it } from "vitest";

import { useFilterPresets, type FilterPreset } from "./filterPresets";

const preset = (name: string, extra?: Partial<FilterPreset>): FilterPreset => ({
  name,
  filters: [],
  sort: null,
  quickFilter: "",
  quickColumn: null,
  ...extra,
});

describe("useFilterPresets", () => {
  it("saves, overwrites by name, and deletes per table", () => {
    const { savePreset, deletePreset } = useFilterPresets.getState();

    savePreset("t1", preset("active users", { quickFilter: "active" }));
    savePreset("t2", preset("other table"));
    savePreset("t1", preset("recent"));
    expect(
      useFilterPresets.getState().presets["t1"]?.map((p) => p.name),
    ).toEqual(["active users", "recent"]);

    // Same name overwrites in place rather than duplicating.
    savePreset("t1", preset("recent", { quickFilter: "now" }));
    const t1 = useFilterPresets.getState().presets["t1"] ?? [];
    expect(t1.filter((p) => p.name === "recent")).toHaveLength(1);
    expect(t1.find((p) => p.name === "recent")?.quickFilter).toBe("now");

    deletePreset("t1", "active users");
    deletePreset("t1", "recent");
    expect(useFilterPresets.getState().presets["t1"]).toBeUndefined();
    expect(useFilterPresets.getState().presets["t2"]).toHaveLength(1);
  });
});

import { create } from "zustand";
import type { ColumnFilters, SortState } from "@cellar/data-grid";

const STORAGE_KEY = "cellar.filterPresets.v1";

/** Snapshot of the whole toolbar: quick filter, advanced chips, and order by. */
export interface FilterPreset {
  name: string;
  filters: ColumnFilters;
  sort: SortState;
  quickFilter: string;
  quickColumn: string | null;
}

/** Presets keyed by table id (`tableKey` from tabs.ts). */
export type FilterPresets = Record<string, FilterPreset[]>;

interface FilterPresetsStore {
  presets: FilterPresets;
  /** Save (or overwrite, matched by name) a preset for a table. */
  savePreset: (tableId: string, preset: FilterPreset) => void;
  deletePreset: (tableId: string, name: string) => void;
}

function loadPresets(): FilterPresets {
  if (typeof localStorage === "undefined") return {};
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") return {};
    const presets: FilterPresets = {};
    for (const [key, value] of Object.entries(parsed)) {
      if (!Array.isArray(value)) continue;
      const items = value.filter(
        (item): item is FilterPreset =>
          !!item &&
          typeof item === "object" &&
          typeof (item as FilterPreset).name === "string" &&
          Array.isArray((item as FilterPreset).filters),
      );
      if (items.length > 0) presets[key] = items;
    }
    return presets;
  } catch {
    return {};
  }
}

function savePresets(presets: FilterPresets) {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(STORAGE_KEY, JSON.stringify(presets));
}

export const useFilterPresets = create<FilterPresetsStore>((set) => ({
  presets: loadPresets(),

  savePreset(tableId, preset) {
    set((s) => {
      const existing = s.presets[tableId] ?? [];
      const next = [
        ...existing.filter((item) => item.name !== preset.name),
        preset,
      ];
      const presets = { ...s.presets, [tableId]: next };
      savePresets(presets);
      return { presets };
    });
  },

  deletePreset(tableId, name) {
    set((s) => {
      const next = (s.presets[tableId] ?? []).filter(
        (item) => item.name !== name,
      );
      const presets = { ...s.presets };
      if (next.length > 0) presets[tableId] = next;
      else delete presets[tableId];
      savePresets(presets);
      return { presets };
    });
  },
}));

import { create } from "zustand";
import type { GridColumnLayout, PendingChanges } from "@cellar/data-grid";
import { useTabResults } from "./tabResults";

const TABLE_LAYOUTS_STORAGE_KEY = "cellar.tableLayouts.v1";

export interface TableTab {
  id: string;
  kind: "table";
  connectionId: string;
  database: string;
  schema: string;
  table: string;
}

export interface QueryTab {
  id: string;
  kind: "query";
  connectionId: string;
  database: string;
  title: string;
  sql: string;
  /** `true` when the buffer has unsaved edits since the tab was last run. */
  dirty: boolean;
}

export type WorkspaceTab = TableTab | QueryTab;
export type SplitOrientation = "horizontal" | "vertical";

export interface WorkspaceSplit {
  orientation: SplitOrientation;
  primaryId: string;
  secondaryId: string;
}

export type TableLayouts = Record<string, GridColumnLayout>;

let queryTabSeq = 0;

interface TabsStore {
  tabs: WorkspaceTab[];
  activeId: string | null;
  closedTabs: WorkspaceTab[];
  split: WorkspaceSplit | null;
  tableChanges: Record<string, PendingChanges>;
  tableLayouts: TableLayouts;
  refreshKeys: Record<string, number>;
  openTable: (
    connectionId: string,
    database: string,
    schema: string,
    table: string,
  ) => void;
  newQueryTab: (connectionId: string, database: string) => string;
  setQuerySql: (id: string, sql: string) => void;
  markQueryRun: (id: string) => void;
  closeTab: (id: string) => void;
  reopenClosedTab: () => void;
  splitActiveTab: (orientation: SplitOrientation) => void;
  clearSplit: () => void;
  closeOtherTabs: (id: string) => void;
  closeTabsToRight: (id: string) => void;
  reorderTab: (sourceId: string, targetId: string) => void;
  setActive: (id: string) => void;
  setTableLayout: (id: string, layout: GridColumnLayout) => void;
  setTableChanges: (id: string, changes: PendingChanges) => void;
  clearTableChanges: (id: string) => void;
  refreshTable: (id: string) => void;
}

function tableKey(
  connectionId: string,
  database: string,
  schema: string,
  table: string,
): string {
  return `${connectionId}::${database}.${schema}.${table}`;
}

function loadTableLayouts(): TableLayouts {
  if (typeof localStorage === "undefined") return {};
  try {
    const raw = localStorage.getItem(TABLE_LAYOUTS_STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") return {};
    const layouts: TableLayouts = {};
    for (const [key, value] of Object.entries(parsed)) {
      if (!value || typeof value !== "object") continue;
      const item = value as Partial<GridColumnLayout>;
      layouts[key] = {
        order: Array.isArray(item.order)
          ? item.order.filter(
              (columnKey): columnKey is string =>
                typeof columnKey === "string",
            )
          : [],
        widths:
          item.widths && typeof item.widths === "object"
            ? Object.fromEntries(
                Object.entries(item.widths).filter(
                  ([columnKey, width]) =>
                    typeof columnKey === "string" && typeof width === "number",
                ),
              )
            : {},
      };
    }
    return layouts;
  } catch {
    return {};
  }
}

function saveTableLayouts(layouts: TableLayouts) {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(TABLE_LAYOUTS_STORAGE_KEY, JSON.stringify(layouts));
}

function dropTabScopedState(
  ids: string[],
  tableChanges: Record<string, PendingChanges>,
  refreshKeys: Record<string, number>,
): {
  tableChanges: Record<string, PendingChanges>;
  refreshKeys: Record<string, number>;
} {
  const closed = new Set(ids);
  return {
    tableChanges: Object.fromEntries(
      Object.entries(tableChanges).filter(([id]) => !closed.has(id)),
    ),
    refreshKeys: Object.fromEntries(
      Object.entries(refreshKeys).filter(([id]) => !closed.has(id)),
    ),
  };
}

function clearTabResults(ids: string[]) {
  const results = useTabResults.getState();
  ids.forEach((id) => results.clearTab(id));
}

function splitForTabs(
  tabs: WorkspaceTab[],
  split: WorkspaceSplit | null,
): WorkspaceSplit | null {
  return split &&
    tabs.some((t) => t.id === split.primaryId) &&
    tabs.some((t) => t.id === split.secondaryId)
    ? split
    : null;
}

function stackClosedTabs(
  closed: WorkspaceTab[],
  current: WorkspaceTab[],
): WorkspaceTab[] {
  return [...closed, ...current].slice(0, 12);
}

export const useTabs = create<TabsStore>((set, get) => ({
  tabs: [],
  activeId: null,
  closedTabs: [],
  split: null,
  tableChanges: {},
  tableLayouts: loadTableLayouts(),
  refreshKeys: {},

  openTable(connectionId, database, schema, table) {
    const id = tableKey(connectionId, database, schema, table);
    const existing = get().tabs.find((t) => t.id === id);
    if (existing) {
      set({ activeId: id });
      return;
    }
    set((s) => ({
      tabs: [
        ...s.tabs,
        { id, kind: "table", connectionId, database, schema, table },
      ],
      activeId: id,
    }));
  },

  newQueryTab(connectionId, database) {
    const id = `query:${++queryTabSeq}:${connectionId}`;
    const title = `untitled-${queryTabSeq}.sql`;
    set((s) => ({
      tabs: [
        ...s.tabs,
        {
          id,
          kind: "query",
          connectionId,
          database,
          title,
          sql: "",
          dirty: false,
        },
      ],
      activeId: id,
    }));
    return id;
  },

  setQuerySql(id, sql) {
    set((s) => ({
      tabs: s.tabs.map((t) =>
        t.id === id && t.kind === "query"
          ? { ...t, sql, dirty: sql !== t.sql ? true : t.dirty }
          : t,
      ),
    }));
  },

  markQueryRun(id) {
    set((s) => ({
      tabs: s.tabs.map((t) =>
        t.id === id && t.kind === "query" ? { ...t, dirty: false } : t,
      ),
    }));
  },

  closeTab(id) {
    set((s) => {
      const closed = s.tabs.find((t) => t.id === id);
      const tabs = s.tabs.filter((t) => t.id !== id);
      const activeId =
        s.activeId === id ? tabs[tabs.length - 1]?.id ?? null : s.activeId;
      const { tableChanges, refreshKeys } = dropTabScopedState(
        [id],
        s.tableChanges,
        s.refreshKeys,
      );
      return {
        tabs,
        activeId,
        closedTabs: closed ? [closed, ...s.closedTabs].slice(0, 12) : s.closedTabs,
        split: splitForTabs(tabs, s.split),
        tableChanges,
        refreshKeys,
      };
    });
    clearTabResults([id]);
  },

  closeOtherTabs(id) {
    if (!get().tabs.some((t) => t.id === id)) return;
    const closedIds = get()
      .tabs.filter((t) => t.id !== id)
      .map((t) => t.id);
    set((s) => {
      const tabs = s.tabs.filter((t) => t.id === id);
      const closed = s.tabs.filter((t) => t.id !== id);
      const { tableChanges, refreshKeys } = dropTabScopedState(
        closedIds,
        s.tableChanges,
        s.refreshKeys,
      );
      return {
        tabs,
        activeId: tabs[0]?.id ?? null,
        closedTabs: stackClosedTabs(closed, s.closedTabs),
        split: null,
        tableChanges,
        refreshKeys,
      };
    });
    clearTabResults(closedIds);
  },

  closeTabsToRight(id) {
    const tabIndex = get().tabs.findIndex((t) => t.id === id);
    if (tabIndex < 0) return;
    const closedIds = get()
      .tabs.slice(tabIndex + 1)
      .map((t) => t.id);
    set((s) => {
      const tabs = s.tabs.slice(0, tabIndex + 1);
      const closed = s.tabs.slice(tabIndex + 1);
      const activeId = closedIds.includes(s.activeId ?? "") ? id : s.activeId;
      const { tableChanges, refreshKeys } = dropTabScopedState(
        closedIds,
        s.tableChanges,
        s.refreshKeys,
      );
      return {
        tabs,
        activeId,
        closedTabs: stackClosedTabs(closed, s.closedTabs),
        split: splitForTabs(tabs, s.split),
        tableChanges,
        refreshKeys,
      };
    });
    clearTabResults(closedIds);
  },

  reorderTab(sourceId, targetId) {
    if (sourceId === targetId) return;
    set((s) => {
      const sourceIndex = s.tabs.findIndex((t) => t.id === sourceId);
      const targetIndex = s.tabs.findIndex((t) => t.id === targetId);
      if (sourceIndex === -1 || targetIndex === -1) return {};
      const tabs = [...s.tabs];
      const [moved] = tabs.splice(sourceIndex, 1);
      if (!moved) return {};
      tabs.splice(targetIndex, 0, moved);
      return { tabs, split: splitForTabs(tabs, s.split) };
    });
  },

  reopenClosedTab() {
    set((s) => {
      const [closed, ...closedTabs] = s.closedTabs;
      if (!closed) return {};

      const existing = s.tabs.find((t) => t.id === closed.id);
      if (existing) {
        return { activeId: existing.id, closedTabs };
      }

      return {
        tabs: [...s.tabs, closed],
        activeId: closed.id,
        closedTabs,
      };
    });
  },

  splitActiveTab(orientation) {
    set((s) => {
      if (!s.activeId || s.tabs.length < 2) return {};

      if (s.split?.orientation === orientation) {
        return { split: null };
      }

      const activeIndex = s.tabs.findIndex((t) => t.id === s.activeId);
      if (activeIndex === -1) return {};

      const secondary =
        s.tabs[activeIndex + 1] ?? s.tabs[activeIndex - 1] ?? null;
      if (!secondary) return {};

      return {
        split: {
          orientation,
          primaryId: s.activeId,
          secondaryId: secondary.id,
        },
      };
    });
  },

  clearSplit() {
    set({ split: null });
  },

  setActive(id) {
    set((s) => ({
      activeId: id,
      split:
        s.split && id !== s.split.primaryId && id !== s.split.secondaryId
          ? { ...s.split, primaryId: id }
          : s.split,
    }));
  },

  setTableLayout(id, layout) {
    set((s) => {
      const tableLayouts = { ...s.tableLayouts, [id]: layout };
      saveTableLayouts(tableLayouts);
      return { tableLayouts };
    });
  },

  setTableChanges(id, changes) {
    set((s) => ({ tableChanges: { ...s.tableChanges, [id]: changes } }));
  },

  clearTableChanges(id) {
    set((s) => {
      const { [id]: _changes, ...tableChanges } = s.tableChanges;
      return { tableChanges };
    });
  },

  refreshTable(id) {
    set((s) => ({
      refreshKeys: { ...s.refreshKeys, [id]: (s.refreshKeys[id] ?? 0) + 1 },
    }));
  },
}));

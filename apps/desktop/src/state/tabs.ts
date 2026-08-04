import { create } from "zustand";
import type {
  ColumnFilters,
  GridColumnLayout,
  PendingChanges,
  SortState,
} from "@cellar/data-grid";
import { useNotices } from "./notices";
import { useQueryMessages } from "./queryMessages";
import { useTabResults } from "./tabResults";
import { useSchemaCompare, type CompareConfig } from "./schemaCompare";

const TABLE_LAYOUTS_STORAGE_KEY = "cellar.tableLayouts.v1";
const TABLE_SORTS_STORAGE_KEY = "cellar.tableSorts.v1";

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
  /** Buffer contents at the last run — the baseline `dirty` compares against. */
  savedSql: string;
  /** `true` when the buffer differs from `savedSql`. */
  dirty: boolean;
}

/**
 * A schema comparison view. The live working state (diff, selection, script)
 * lives in `useSchemaCompare`, keyed by this tab's id, but the immutable
 * `config` is carried on the tab itself so the comparison can be re-derived
 * after the tab is closed and reopened (the store entry is disposed on close).
 * `connectionId`/`database` track a live source so connection teardown closes it.
 */
export interface SchemaCompareTab {
  id: string;
  kind: "schema-compare";
  connectionId: string;
  database: string;
  title: string;
  config: CompareConfig;
}

export interface ErDiagramTab {
  id: string;
  kind: "er-diagram";
  connectionId: string;
  database: string;
  title: string;
  /** Schema scope the graph was opened for; `null` means every schema. */
  schemas: string[] | null;
}

export type WorkspaceTab =
  | TableTab
  | QueryTab
  | SchemaCompareTab
  | ErDiagramTab;
export type SplitOrientation = "horizontal" | "vertical";
/** Edge a tab was dropped on to create/redirect a split. */
export type SplitEdge = "left" | "right" | "top" | "bottom";

/** Short label for a tab — title for query/ER tabs, `schema.table` for tables. */
export function tabLabel(tab: WorkspaceTab): string {
  return tab.kind === "table" ? `${tab.schema}.${tab.table}` : tab.title;
}

/** 0 = primary (left/top) pane, 1 = secondary (right/bottom) pane. */
export type PaneIndex = 0 | 1;

export type TableLayouts = Record<string, GridColumnLayout>;
/** Last-used column sort per table id (`tableKey`). `null` means unsorted. */
export type TableSorts = Record<string, SortState>;

/**
 * Active filter toolbar for an open table tab (chips + quick filter).
 * Session-scoped — survives tab swaps while the tab stays open, cleared on close.
 * Named presets live separately in `useFilterPresets`.
 */
export type TableFilterState = {
  filters: ColumnFilters;
  quickFilter: string;
  quickColumn: string | null;
};

export type TableFilters = Record<string, TableFilterState>;

let queryTabSeq = 0;
let schemaCompareSeq = 0;

interface TabsStore {
  tabs: WorkspaceTab[];
  /** The globally focused tab — mirrors the focused pane's active tab. */
  activeId: string | null;
  closedTabs: WorkspaceTab[];
  /** Split orientation, or `null` when the workspace is a single pane. */
  split: SplitOrientation | null;
  /** Pane each tab lives in; absence means the primary pane (0). */
  tabPane: Record<string, PaneIndex>;
  /** Active tab per pane: [primary, secondary]. */
  paneActive: [string | null, string | null];
  /** Which pane has focus — drives `activeId` and where new tabs land. */
  focusedPane: PaneIndex;
  /** Tab id currently being dragged, or `null` — drives the drop-zone overlay. */
  draggingTabId: string | null;
  tableChanges: Record<string, PendingChanges>;
  tableLayouts: TableLayouts;
  tableSorts: TableSorts;
  /** Active filter toolbar per open table tab id. */
  tableFilters: TableFilters;
  refreshKeys: Record<string, number>;
  openTable: (
    connectionId: string,
    database: string,
    schema: string,
    table: string,
  ) => void;
  newQueryTab: (connectionId: string, database: string) => string;
  /**
   * Open a schema-comparison tab. `connectionId`/`database` scope the tab for
   * teardown; `config` is carried on the tab so the comparison can be
   * re-initialized on reopen. Returns the new tab id.
   */
  openSchemaCompare: (
    title: string,
    connectionId: string,
    database: string,
    config: CompareConfig,
  ) => string;
  openErDiagram: (
    connectionId: string,
    database: string,
    schemas: string[] | null,
  ) => void;
  setQuerySql: (id: string, sql: string) => void;
  /** Re-point a query tab at a different database on the same connection. */
  setQueryDatabase: (id: string, database: string) => void;
  markQueryRun: (id: string) => void;
  closeTab: (id: string) => void;
  reopenClosedTab: () => void;
  splitActiveTab: (orientation: SplitOrientation) => void;
  clearSplit: () => void;
  closeOtherTabs: (id: string) => void;
  closeTabsToRight: (id: string) => void;
  closeConnectionTabs: (connectionId: string) => void;
  reorderTab: (sourceId: string, targetId: string) => void;
  /** Move a tab into a pane (used when dragging onto the other pane's strip). */
  moveTabToPane: (id: string, pane: PaneIndex) => void;
  /** Focus a pane without changing its active tab (e.g. clicking its "+"). */
  focusPane: (pane: PaneIndex) => void;
  setDraggingTab: (id: string | null) => void;
  /** Split (or re-split) by dropping a tab on a workspace edge. */
  dropTabToSplit: (id: string, edge: SplitEdge) => void;
  setActive: (id: string) => void;
  setTableLayout: (id: string, layout: GridColumnLayout) => void;
  /** Persist (or clear) the last-used sort for a table browse tab. */
  setTableSort: (id: string, sort: SortState) => void;
  /** Merge imported per-table layouts over the current ones and persist. */
  importTableLayouts: (entries: TableLayouts) => void;
  setTableChanges: (id: string, changes: PendingChanges) => void;
  clearTableChanges: (id: string) => void;
  /** Remember (or clear) the active filter toolbar for an open table tab. */
  setTableFilters: (id: string, state: TableFilterState) => void;
  refreshTable: (id: string) => void;
}

function isEmptyFilterState(state: TableFilterState): boolean {
  return (
    state.filters.length === 0 &&
    state.quickFilter === "" &&
    state.quickColumn === null
  );
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

function coerceSortState(raw: unknown): SortState | undefined {
  if (raw === null) return null;
  if (!raw || typeof raw !== "object") return undefined;
  const sort = raw as { columnKey?: unknown; direction?: unknown };
  if (typeof sort.columnKey !== "string") return undefined;
  if (sort.direction !== "asc" && sort.direction !== "desc") return undefined;
  return { columnKey: sort.columnKey, direction: sort.direction };
}

function loadTableSorts(): TableSorts {
  if (typeof localStorage === "undefined") return {};
  try {
    const raw = localStorage.getItem(TABLE_SORTS_STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") return {};
    const sorts: TableSorts = {};
    for (const [key, value] of Object.entries(parsed)) {
      const sort = coerceSortState(value);
      // Skip malformed entries; keep explicit `null` (cleared sort).
      if (sort === undefined) continue;
      sorts[key] = sort;
    }
    return sorts;
  } catch {
    return {};
  }
}

function saveTableSorts(sorts: TableSorts) {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(TABLE_SORTS_STORAGE_KEY, JSON.stringify(sorts));
  } catch {
    // Persistence failed (quota, private mode) — keep the in-memory update.
  }
}

function dropTabScopedState(
  ids: string[],
  tableChanges: Record<string, PendingChanges>,
  tableFilters: TableFilters,
  refreshKeys: Record<string, number>,
): {
  tableChanges: Record<string, PendingChanges>;
  tableFilters: TableFilters;
  refreshKeys: Record<string, number>;
} {
  const closed = new Set(ids);
  return {
    tableChanges: Object.fromEntries(
      Object.entries(tableChanges).filter(([id]) => !closed.has(id)),
    ),
    tableFilters: Object.fromEntries(
      Object.entries(tableFilters).filter(([id]) => !closed.has(id)),
    ),
    refreshKeys: Object.fromEntries(
      Object.entries(refreshKeys).filter(([id]) => !closed.has(id)),
    ),
  };
}

function clearTabResults(ids: string[]) {
  const results = useTabResults.getState();
  const messages = useQueryMessages.getState();
  const schemaCompare = useSchemaCompare.getState();
  ids.forEach((id) => {
    results.clearTab(id);
    messages.clearForTab(id);
    schemaCompare.dispose(id);
  });
  useNotices.getState().dropTabs(ids);
}

/** The pane a tab lives in (primary unless explicitly placed in secondary). */
function paneOf(tabPane: Record<string, PaneIndex>, id: string): PaneIndex {
  return tabPane[id] ?? 0;
}

function setPaneActive(
  paneActive: [string | null, string | null],
  pane: PaneIndex,
  id: string | null,
): [string | null, string | null] {
  return pane === 0 ? [id, paneActive[1]] : [paneActive[0], id];
}

type SplitState = Pick<
  TabsStore,
  "split" | "tabPane" | "paneActive" | "focusedPane" | "activeId"
>;

/**
 * Re-derive the split-layout fields so they stay consistent with `tabs`:
 * drop stale pane membership, collapse the split if a pane is empty, and pick a
 * valid active tab per pane (keeping the requested one when it's still valid,
 * else the last tab in that pane). `activeId` mirrors the focused pane.
 */
function reconcile(s: { tabs: WorkspaceTab[] } & SplitState): SplitState {
  const ids = new Set(s.tabs.map((t) => t.id));
  let tabPane: Record<string, PaneIndex> = Object.fromEntries(
    Object.entries(s.tabPane).filter(([id]) => ids.has(id)),
  );

  let split = s.split;
  if (split) {
    const hasPrimary = s.tabs.some((t) => paneOf(tabPane, t.id) === 0);
    const hasSecondary = s.tabs.some((t) => paneOf(tabPane, t.id) === 1);
    if (!hasPrimary || !hasSecondary) split = null;
  }
  // Collapsed (or never split): everything lives in the primary pane.
  if (!split) tabPane = {};

  const pickActive = (pane: PaneIndex): string | null => {
    const requested = s.paneActive[pane];
    if (requested && ids.has(requested) && paneOf(tabPane, requested) === pane) {
      return requested;
    }
    const inPane = s.tabs.filter((t) => paneOf(tabPane, t.id) === pane);
    return inPane.length ? inPane[inPane.length - 1]!.id : null;
  };
  const paneActive: [string | null, string | null] = [
    pickActive(0),
    pickActive(1),
  ];

  const focusedPane: PaneIndex = split ? s.focusedPane : 0;
  const activeId = paneActive[focusedPane] ?? paneActive[focusedPane ? 0 : 1];

  return { split, tabPane, paneActive, focusedPane, activeId };
}

function stackClosedTabs(
  closed: WorkspaceTab[],
  current: WorkspaceTab[],
): WorkspaceTab[] {
  return [...closed, ...current].slice(0, 12);
}

/** Append a new tab into the focused pane and make it active there. */
function openInFocusedPane(s: TabsStore, tab: WorkspaceTab): Partial<TabsStore> {
  const pane: PaneIndex = s.split ? s.focusedPane : 0;
  const tabs = [...s.tabs, tab];
  const tabPane = pane === 1 ? { ...s.tabPane, [tab.id]: pane } : s.tabPane;
  const paneActive = setPaneActive(s.paneActive, pane, tab.id);
  return { tabs, ...reconcile({ ...s, tabs, tabPane, paneActive, focusedPane: pane }) };
}

/** Focus an already-open tab in whichever pane it lives. */
function activateExisting(s: TabsStore, id: string): Partial<TabsStore> {
  const pane = paneOf(s.tabPane, id);
  return reconcile({
    ...s,
    focusedPane: pane,
    paneActive: setPaneActive(s.paneActive, pane, id),
  });
}

export const useTabs = create<TabsStore>((set, get) => ({
  tabs: [],
  activeId: null,
  closedTabs: [],
  split: null,
  tabPane: {},
  paneActive: [null, null],
  focusedPane: 0,
  draggingTabId: null,
  tableChanges: {},
  tableLayouts: loadTableLayouts(),
  tableSorts: loadTableSorts(),
  tableFilters: {},
  refreshKeys: {},

  openTable(connectionId, database, schema, table) {
    const id = tableKey(connectionId, database, schema, table);
    set((s) =>
      s.tabs.some((t) => t.id === id)
        ? activateExisting(s, id)
        : openInFocusedPane(s, {
            id,
            kind: "table",
            connectionId,
            database,
            schema,
            table,
          }),
    );
  },

  openErDiagram(connectionId, database, schemas) {
    const scope =
      schemas && schemas.length > 0 ? [...schemas].sort() : null;
    const id = `er:${connectionId}::${database}::${scope?.join(",") ?? "all"}`;
    const title =
      scope && scope.length === 1
        ? `ER: ${scope[0]}`
        : `ER: ${database}`;
    set((s) =>
      s.tabs.some((t) => t.id === id)
        ? activateExisting(s, id)
        : openInFocusedPane(s, {
            id,
            kind: "er-diagram",
            connectionId,
            database,
            title,
            schemas: scope,
          }),
    );
  },

  newQueryTab(connectionId, database) {
    const id = `query:${++queryTabSeq}:${connectionId}`;
    const title = `untitled-${queryTabSeq}.sql`;
    set((s) =>
      openInFocusedPane(s, {
        id,
        kind: "query",
        connectionId,
        database,
        title,
        sql: "",
        savedSql: "",
        dirty: false,
      }),
    );
    return id;
  },

  openSchemaCompare(title, connectionId, database, config) {
    const id = `schema-compare:${++schemaCompareSeq}`;
    set((s) =>
      openInFocusedPane(s, {
        id,
        kind: "schema-compare",
        connectionId,
        database,
        title,
        config,
      }),
    );
    return id;
  },

  setQuerySql(id, sql) {
    set((s) => ({
      tabs: s.tabs.map((t) =>
        t.id === id && t.kind === "query"
          ? { ...t, sql, dirty: sql !== t.savedSql }
          : t,
      ),
    }));
  },

  setQueryDatabase(id, database) {
    const tab = get().tabs.find((t) => t.id === id);
    if (!tab || tab.kind !== "query" || tab.database === database) return;
    set((s) => ({
      tabs: s.tabs.map((t) =>
        t.id === id && t.kind === "query" ? { ...t, database } : t,
      ),
    }));
    // The grid, its `result.source` header, the "Load more" callback, and any
    // messages/notices still describe the previous database — drop them so the
    // user isn't shown stale data until they re-run.
    clearTabResults([id]);
  },

  markQueryRun(id) {
    set((s) => ({
      tabs: s.tabs.map((t) =>
        t.id === id && t.kind === "query"
          ? { ...t, savedSql: t.sql, dirty: false }
          : t,
      ),
    }));
  },

  closeTab(id) {
    set((s) => {
      const closed = s.tabs.find((t) => t.id === id);
      const tabs = s.tabs.filter((t) => t.id !== id);
      const { tableChanges, tableFilters, refreshKeys } = dropTabScopedState(
        [id],
        s.tableChanges,
        s.tableFilters,
        s.refreshKeys,
      );
      return {
        tabs,
        closedTabs: closed ? [closed, ...s.closedTabs].slice(0, 12) : s.closedTabs,
        tableChanges,
        tableFilters,
        refreshKeys,
        ...reconcile({ ...s, tabs }),
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
      const { tableChanges, tableFilters, refreshKeys } = dropTabScopedState(
        closedIds,
        s.tableChanges,
        s.tableFilters,
        s.refreshKeys,
      );
      return {
        tabs,
        closedTabs: stackClosedTabs(closed, s.closedTabs),
        tableChanges,
        tableFilters,
        refreshKeys,
        ...reconcile({ ...s, tabs }),
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
      const { tableChanges, tableFilters, refreshKeys } = dropTabScopedState(
        closedIds,
        s.tableChanges,
        s.tableFilters,
        s.refreshKeys,
      );
      return {
        tabs,
        closedTabs: stackClosedTabs(closed, s.closedTabs),
        tableChanges,
        tableFilters,
        refreshKeys,
        ...reconcile({ ...s, tabs }),
      };
    });
    clearTabResults(closedIds);
  },

  closeConnectionTabs(connectionId) {
    const removedIds = get()
      .tabs.filter((t) => t.connectionId === connectionId)
      .map((t) => t.id);
    set((s) => {
      const removed = new Set(removedIds);
      const tabs = s.tabs.filter((t) => !removed.has(t.id));
      const { tableChanges, tableFilters, refreshKeys } = dropTabScopedState(
        removedIds,
        s.tableChanges,
        s.tableFilters,
        s.refreshKeys,
      );
      return {
        tabs,
        // Drop the connection's tabs from the reopen stack too, so undo-close
        // can't resurrect a tab that points at a connection that no longer exists.
        closedTabs: s.closedTabs.filter(
          (t) => t.connectionId !== connectionId,
        ),
        tableChanges,
        tableFilters,
        refreshKeys,
        ...reconcile({ ...s, tabs }),
      };
    });
    if (removedIds.length > 0) clearTabResults(removedIds);
  },

  reorderTab(sourceId, targetId) {
    if (sourceId === targetId) return;
    set((s) => {
      const sourceIndex = s.tabs.findIndex((t) => t.id === sourceId);
      if (sourceIndex === -1 || !s.tabs.some((t) => t.id === targetId)) {
        return {};
      }
      const tabs = [...s.tabs];
      const [moved] = tabs.splice(sourceIndex, 1);
      if (!moved) return {};
      tabs.splice(tabs.findIndex((t) => t.id === targetId), 0, moved);
      // When split, dropping onto another pane's tab moves the source into it.
      const tabPane = s.split
        ? { ...s.tabPane, [sourceId]: paneOf(s.tabPane, targetId) }
        : s.tabPane;
      return { tabs, ...reconcile({ ...s, tabs, tabPane }) };
    });
  },

  moveTabToPane(id, pane) {
    set((s) => {
      if (!s.split || !s.tabs.some((t) => t.id === id)) return {};
      if (paneOf(s.tabPane, id) === pane) return activateExisting(s, id);
      const tabPane = { ...s.tabPane, [id]: pane };
      return reconcile({
        ...s,
        tabPane,
        paneActive: setPaneActive(s.paneActive, pane, id),
        focusedPane: pane,
      });
    });
  },

  focusPane(pane) {
    set((s) => (s.split ? reconcile({ ...s, focusedPane: pane }) : {}));
  },

  setDraggingTab(id) {
    set({ draggingTabId: id });
  },

  dropTabToSplit(id, edge) {
    set((s) => {
      if (!s.tabs.some((t) => t.id === id)) return { draggingTabId: null };
      const orientation: SplitOrientation =
        edge === "left" || edge === "right" ? "vertical" : "horizontal";
      const targetPane: PaneIndex = edge === "left" || edge === "top" ? 0 : 1;
      let tabPane: Record<string, PaneIndex>;
      if (s.split) {
        // Already split — just move the dropped tab to the chosen pane.
        tabPane = { ...s.tabPane, [id]: targetPane };
      } else {
        // Fresh split — the dropped tab takes its edge, everything else the
        // opposite pane (a no-op when it's the only tab: reconcile collapses).
        const other: PaneIndex = targetPane === 0 ? 1 : 0;
        tabPane = {};
        for (const t of s.tabs) tabPane[t.id] = t.id === id ? targetPane : other;
      }
      return {
        draggingTabId: null,
        ...reconcile({
          ...s,
          split: orientation,
          tabPane,
          paneActive: setPaneActive(s.paneActive, targetPane, id),
          focusedPane: targetPane,
        }),
      };
    });
  },

  reopenClosedTab() {
    set((s) => {
      const [closed, ...closedTabs] = s.closedTabs;
      if (!closed) return {};
      return s.tabs.some((t) => t.id === closed.id)
        ? { closedTabs, ...activateExisting(s, closed.id) }
        : { closedTabs, ...openInFocusedPane(s, closed) };
    });
  },

  splitActiveTab(orientation) {
    set((s) => {
      // Same orientation again → collapse back to a single pane.
      if (s.split === orientation) {
        return reconcile({
          ...s,
          split: null,
          paneActive: [s.activeId, null],
          focusedPane: 0,
        });
      }
      // Already split the other way → just flip the orientation.
      if (s.split) return reconcile({ ...s, split: orientation });
      // Create a split: move the active tab into the secondary pane and keep
      // the rest in the primary (needs another tab so neither pane is empty).
      if (!s.activeId || s.tabs.length < 2) return {};
      const tabPane = { ...s.tabPane, [s.activeId]: 1 as PaneIndex };
      return reconcile({
        ...s,
        split: orientation,
        tabPane,
        paneActive: [null, s.activeId],
        focusedPane: 1,
      });
    });
  },

  clearSplit() {
    set((s) =>
      reconcile({ ...s, split: null, paneActive: [s.activeId, null], focusedPane: 0 }),
    );
  },

  setActive(id) {
    set((s) => activateExisting(s, id));
  },

  setTableLayout(id, layout) {
    set((s) => {
      const tableLayouts = { ...s.tableLayouts, [id]: layout };
      saveTableLayouts(tableLayouts);
      return { tableLayouts };
    });
  },

  setTableSort(id, sort) {
    set((s) => {
      const tableSorts = { ...s.tableSorts, [id]: sort };
      // Drop cleared sorts so the map stays small; absence and null both mean
      // "no remembered sort" when the setting is on.
      if (sort === null) {
        const { [id]: _removed, ...rest } = s.tableSorts;
        saveTableSorts(rest);
        return { tableSorts: rest };
      }
      saveTableSorts(tableSorts);
      return { tableSorts };
    });
  },

  importTableLayouts(entries) {
    set((s) => {
      const tableLayouts = { ...s.tableLayouts, ...entries };
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

  setTableFilters(id, state) {
    set((s) => {
      // Drop empty toolbars so the map stays small while a tab is idle.
      if (isEmptyFilterState(state)) {
        if (!(id in s.tableFilters)) return s;
        const { [id]: _removed, ...rest } = s.tableFilters;
        return { tableFilters: rest };
      }
      return { tableFilters: { ...s.tableFilters, [id]: state } };
    });
  },

  refreshTable(id) {
    set((s) => ({
      refreshKeys: { ...s.refreshKeys, [id]: (s.refreshKeys[id] ?? 0) + 1 },
    }));
  },
}));

import { create } from "zustand";
import type { PendingChanges } from "@cellar/data-grid";
import { useTabResults } from "./tabResults";

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

let queryTabSeq = 0;

interface TabsStore {
  tabs: WorkspaceTab[];
  activeId: string | null;
  tableChanges: Record<string, PendingChanges>;
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
  setActive: (id: string) => void;
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

export const useTabs = create<TabsStore>((set, get) => ({
  tabs: [],
  activeId: null,
  tableChanges: {},
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
      const tabs = s.tabs.filter((t) => t.id !== id);
      const activeId =
        s.activeId === id ? tabs[tabs.length - 1]?.id ?? null : s.activeId;
      const { [id]: _changes, ...tableChanges } = s.tableChanges;
      const { [id]: _refresh, ...refreshKeys } = s.refreshKeys;
      return { tabs, activeId, tableChanges, refreshKeys };
    });
    useTabResults.getState().clearTab(id);
  },

  setActive(id) {
    set({ activeId: id });
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

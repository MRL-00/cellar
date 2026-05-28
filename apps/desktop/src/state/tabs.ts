import { create } from "zustand";

export interface TableTab {
  id: string;
  kind: "table";
  connectionId: string;
  schema: string;
  table: string;
}

interface TabsStore {
  tabs: TableTab[];
  activeId: string | null;
  openTable: (connectionId: string, schema: string, table: string) => void;
  closeTab: (id: string) => void;
  setActive: (id: string) => void;
}

function tableKey(connectionId: string, schema: string, table: string): string {
  return `${connectionId}::${schema}.${table}`;
}

export const useTabs = create<TabsStore>((set, get) => ({
  tabs: [],
  activeId: null,

  openTable(connectionId, schema, table) {
    const id = tableKey(connectionId, schema, table);
    const existing = get().tabs.find((t) => t.id === id);
    if (existing) {
      set({ activeId: id });
      return;
    }
    set((s) => ({
      tabs: [
        ...s.tabs,
        { id, kind: "table", connectionId, schema, table },
      ],
      activeId: id,
    }));
  },

  closeTab(id) {
    set((s) => {
      const tabs = s.tabs.filter((t) => t.id !== id);
      const activeId =
        s.activeId === id ? tabs[tabs.length - 1]?.id ?? null : s.activeId;
      return { tabs, activeId };
    });
  },

  setActive(id) {
    set({ activeId: id });
  },
}));

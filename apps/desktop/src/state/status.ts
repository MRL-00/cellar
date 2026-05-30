import { create } from "zustand";

export interface QueryMetrics {
  connectionId: string;
  tabId?: string | null;
  rowCount: number;
  truncated: boolean;
  durationMs: number;
  completedAtMs: number;
}

type QueryMetricsInput = Omit<QueryMetrics, "completedAtMs">;

interface StatusStore {
  lastQuery: QueryMetrics | null;
  setLastQuery: (m: QueryMetricsInput) => void;
  clear: () => void;
}

export const useStatus = create<StatusStore>((set) => ({
  lastQuery: null,
  setLastQuery: (m) => set({ lastQuery: { ...m, completedAtMs: Date.now() } }),
  clear: () => set({ lastQuery: null }),
}));

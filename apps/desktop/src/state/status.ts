import { create } from "zustand";

export interface QueryMetrics {
  connectionId: string;
  rowCount: number;
  truncated: boolean;
  durationMs: number;
}

interface StatusStore {
  lastQuery: QueryMetrics | null;
  setLastQuery: (m: QueryMetrics) => void;
  clear: () => void;
}

export const useStatus = create<StatusStore>((set) => ({
  lastQuery: null,
  setLastQuery: (m) => set({ lastQuery: m }),
  clear: () => set({ lastQuery: null }),
}));

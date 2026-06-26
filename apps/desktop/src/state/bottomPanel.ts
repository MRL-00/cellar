import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { deferredLocalStorage } from "./deferredStorage";

export type BottomTabId =
  | "results"
  | "messages"
  | "plan"
  | "history"
  | "notices"
  | "findUsages";

interface BottomPanelStore {
  active: BottomTabId;
  /** Bumped to ask the Plan panel to (re)run EXPLAIN for the active query. */
  explainNonce: number;
  setActive: (id: BottomTabId) => void;
  /** Focus the Plan tab and request a fresh EXPLAIN run. */
  requestExplain: () => void;
}

/**
 * Cross-component control for the bottom panel. The SQL editor uses it to focus
 * the Results tab when a query runs and to drive the Plan tab's "Explain plan"
 * toolbar action without lifting the panel's whole tab state into App.
 */
export const useBottomPanel = create<BottomPanelStore>()(
  persist(
    (set) => ({
      active: "results",
      explainNonce: 0,
      setActive: (active) => set({ active }),
      requestExplain: () =>
        set((s) => ({ active: "plan", explainNonce: s.explainNonce + 1 })),
    }),
    {
      name: "cellar.bottomPanel.v1",
      storage: createJSONStorage(deferredLocalStorage),
      partialize: (s) => ({ active: s.active }),
    },
  ),
);

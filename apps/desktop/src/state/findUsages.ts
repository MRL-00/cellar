import { create } from "zustand";
import { commands, unwrap, type UsageReference } from "@cellar/ipc";

import { useBottomPanel } from "./bottomPanel";
import { useLayout } from "./layout";

/** What the user asked to find usages of: a table, or a single column of it. */
export interface UsageTarget {
  connectionId: string;
  database: string;
  schema: string;
  table: string;
  /** `null` for a whole-table search. */
  column: string | null;
}

export type FindUsagesStatus = "idle" | "loading" | "ready" | "error";

interface FindUsagesStore {
  target: UsageTarget | null;
  /** When true, search every schema in the database instead of just the target's. */
  allSchemas: boolean;
  status: FindUsagesStatus;
  results: UsageReference[];
  error: string | null;
  /** Bumped per request so late responses from a superseded search are ignored. */
  runId: number;
  /** Run a fresh search, reveal the bottom panel, and focus the Find Usages tab. */
  findUsages: (target: UsageTarget) => void;
  /** Toggle scope and re-run against the current target. */
  setAllSchemas: (all: boolean) => void;
  /** Re-run the current target's search (e.g. after a schema refresh). */
  refresh: () => void;
  /**
   * React to a connection's schema being refreshed: the backend has just
   * dropped its cached usage definitions, so re-run the current search to
   * avoid showing stale results for that connection.
   */
  onConnectionRefreshed: (connectionId: string) => void;
  clear: () => void;
}

export const useFindUsages = create<FindUsagesStore>((set, get) => {
  const run = (target: UsageTarget, allSchemas: boolean) => {
    const runId = get().runId + 1;
    set({ target, status: "loading", error: null, runId });
    void unwrap(
      commands.findUsages(
        target.connectionId,
        target.database,
        target.schema,
        target.table,
        target.column,
        allSchemas,
      ),
    )
      .then((results) => {
        if (get().runId !== runId) return;
        set({ results, status: "ready", error: null });
      })
      .catch((err) => {
        if (get().runId !== runId) return;
        set({
          results: [],
          status: "error",
          error: err instanceof Error ? err.message : String(err),
        });
      });
  };

  return {
    target: null,
    allSchemas: false,
    status: "idle",
    results: [],
    error: null,
    runId: 0,

    findUsages(target) {
      useLayout.getState().setPanel("bottom", true);
      useBottomPanel.getState().setActive("findUsages");
      run(target, get().allSchemas);
    },

    setAllSchemas(all) {
      set({ allSchemas: all });
      const { target } = get();
      if (target) run(target, all);
    },

    refresh() {
      const { target, allSchemas } = get();
      if (target) run(target, allSchemas);
    },

    onConnectionRefreshed(connectionId) {
      const { target, allSchemas, status } = get();
      if (target?.connectionId === connectionId && status !== "idle") {
        run(target, allSchemas);
      }
    },

    clear() {
      set({ target: null, status: "idle", results: [], error: null });
    },
  };
});

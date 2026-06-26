import { create } from "zustand";
import {
  commands,
  unwrap,
  type MigrationStatement,
  type SchemaComparison,
  type SchemaSource,
} from "@cellar/ipc";

export interface CompareConfig {
  source: SchemaSource;
  target: SchemaSource;
}

/** Per-tab state for a schema comparison. Keyed by the workspace tab id. */
export interface CompareTabState {
  config: CompareConfig;
  loading: boolean;
  error: string | null;
  comparison: SchemaComparison | null;
  /** Statement id → included in the generated script. */
  selected: Record<string, boolean>;
  wrapInTransaction: boolean;
  /** Assembled migration script shown in the editable SQL editor. */
  sql: string;
  /** `true` once the user has hand-edited `sql` since it was generated. */
  sqlDirty: boolean;
  applying: boolean;
  applyError: string | null;
  appliedAt: number | null;
}

interface SchemaCompareStore {
  byTab: Record<string, CompareTabState>;
  start: (tabId: string, config: CompareConfig) => Promise<void>;
  recompare: (tabId: string) => Promise<void>;
  toggle: (tabId: string, id: string) => void;
  setMany: (tabId: string, ids: string[], value: boolean) => void;
  setWrap: (tabId: string, wrap: boolean) => void;
  /** Rebuild `sql` from the current selection, discarding manual edits. */
  generateScript: (tabId: string) => Promise<void>;
  setSql: (tabId: string, sql: string) => void;
  apply: (
    tabId: string,
    connectionId: string,
    database: string,
  ) => Promise<boolean>;
  dispose: (tabId: string) => void;
}

function selectAllChanges(statements: MigrationStatement[]): Record<string, boolean> {
  const selected: Record<string, boolean> = {};
  for (const statement of statements) {
    selected[statement.id] = true;
  }
  return selected;
}

function selectedStatements(state: CompareTabState): MigrationStatement[] {
  const statements = state.comparison?.statements ?? [];
  return statements.filter((s) => state.selected[s.id]);
}

/**
 * Rebuild the migration script after a selection/transaction change — but only
 * when the user hasn't hand-edited the buffer. This keeps the editable script
 * (the source of truth for what `apply` runs, and for the destructive-change
 * count) in sync with the checklist, while preserving manual edits until the
 * user explicitly regenerates.
 */
function regenerateIfClean(get: () => SchemaCompareStore, tabId: string) {
  const state = get().byTab[tabId];
  if (state && !state.sqlDirty) {
    void get().generateScript(tabId);
  }
}

function patch(
  set: (fn: (s: SchemaCompareStore) => Partial<SchemaCompareStore>) => void,
  tabId: string,
  next: Partial<CompareTabState>,
) {
  set((s) => {
    const current = s.byTab[tabId];
    if (!current) return {};
    return { byTab: { ...s.byTab, [tabId]: { ...current, ...next } } };
  });
}

export const useSchemaCompare = create<SchemaCompareStore>((set, get) => ({
  byTab: {},

  async start(tabId, config) {
    set((s) => ({
      byTab: {
        ...s.byTab,
        [tabId]: {
          config,
          loading: true,
          error: null,
          comparison: null,
          selected: {},
          wrapInTransaction: true,
          sql: "",
          sqlDirty: false,
          applying: false,
          applyError: null,
          appliedAt: null,
        },
      },
    }));
    await get().recompare(tabId);
  },

  async recompare(tabId) {
    const state = get().byTab[tabId];
    if (!state) return;
    patch(set, tabId, { loading: true, error: null });
    try {
      const comparison = await unwrap(
        commands.compareSchemas(state.config.source, state.config.target),
      );
      patch(set, tabId, {
        comparison,
        selected: selectAllChanges(comparison.statements),
        loading: false,
        error: null,
        // A fresh comparison supersedes any prior apply outcome.
        applyError: null,
        appliedAt: null,
      });
      // Auto-generate the script once so the editor is populated on open.
      await get().generateScript(tabId);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      patch(set, tabId, { loading: false, error: message, comparison: null });
    }
  },

  toggle(tabId, id) {
    set((s) => {
      const current = s.byTab[tabId];
      if (!current) return {};
      const selected = { ...current.selected, [id]: !current.selected[id] };
      return { byTab: { ...s.byTab, [tabId]: { ...current, selected } } };
    });
    regenerateIfClean(get, tabId);
  },

  setMany(tabId, ids, value) {
    set((s) => {
      const current = s.byTab[tabId];
      if (!current) return {};
      const selected = { ...current.selected };
      for (const id of ids) selected[id] = value;
      return { byTab: { ...s.byTab, [tabId]: { ...current, selected } } };
    });
    regenerateIfClean(get, tabId);
  },

  setWrap(tabId, wrap) {
    patch(set, tabId, { wrapInTransaction: wrap });
    regenerateIfClean(get, tabId);
  },

  async generateScript(tabId) {
    const state = get().byTab[tabId];
    if (!state) return;
    try {
      const sql = await commands.buildMigrationScript(
        selectedStatements(state),
        state.comparison?.dialect ?? "postgres",
        state.wrapInTransaction,
      );
      patch(set, tabId, { sql, sqlDirty: false, error: null });
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      patch(set, tabId, { error: message });
    }
  },

  setSql(tabId, sql) {
    patch(set, tabId, { sql, sqlDirty: true });
  },

  async apply(tabId, connectionId, database) {
    const state = get().byTab[tabId];
    if (!state) return false;
    patch(set, tabId, { applying: true, applyError: null });
    try {
      await unwrap(commands.applyMigration(connectionId, database, state.sql, tabId));
      patch(set, tabId, { applying: false, applyError: null, appliedAt: Date.now() });
      return true;
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      patch(set, tabId, { applying: false, applyError: message });
      return false;
    }
  },

  dispose(tabId) {
    set((s) => {
      if (!s.byTab[tabId]) return {};
      const { [tabId]: _gone, ...byTab } = s.byTab;
      return { byTab };
    });
  },
}));

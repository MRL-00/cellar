import type { GridColumn, GridRow } from "@cellar/data-grid";
import { create } from "zustand";

export type ResultSource =
  | {
      kind: "table";
      connectionId: string;
      database: string;
      schema: string;
      table: string;
      statement: string;
      maxRows: number;
    }
  | {
      kind: "query";
      connectionId: string;
      database: string | null;
      tabId: string;
      label: string;
      statement: string;
      maxRows: number;
    };

export type TabResult =
  | {
      status: "loading";
      tabId: string;
      source: ResultSource;
    }
  | {
      status: "ready";
      tabId: string;
      source: ResultSource;
      columns: GridColumn[];
      rows: GridRow[];
      rowCount: number;
      truncated: boolean;
      durationMs: number;
      /** Callback set by `useQueryRunner` to load the next page of rows. */
      onLoadMore?: (() => void) | null;
    }
  | {
      status: "error";
      tabId: string;
      source: ResultSource;
      message: string;
    };

interface TabResultsStore {
  byTabId: Record<string, TabResult>;
  setLoading: (tabId: string, source: ResultSource) => void;
  setReady: (
    tabId: string,
    result: Omit<Extract<TabResult, { status: "ready" }>, "status" | "tabId">,
  ) => void;
  setError: (tabId: string, source: ResultSource, message: string) => void;
  setLoadMoreCallback: (tabId: string, onLoadMore: (() => void) | null) => void;
  clearTab: (tabId: string) => void;
}

export const useTabResults = create<TabResultsStore>((set) => ({
  byTabId: {},

  setLoading: (tabId, source) =>
    set((s) => ({
      byTabId: {
        ...s.byTabId,
        [tabId]: { status: "loading", tabId, source },
      },
    })),

  setReady: (tabId, result) =>
    set((s) => ({
      byTabId: {
        ...s.byTabId,
        [tabId]: { status: "ready", tabId, ...result },
      },
    })),

  setError: (tabId, source, message) =>
    set((s) => ({
      byTabId: {
        ...s.byTabId,
        [tabId]: { status: "error", tabId, source, message },
      },
    })),

  setLoadMoreCallback: (tabId, onLoadMore) =>
    set((s) => {
      const existing = s.byTabId[tabId];
      if (existing?.status !== "ready") return s;
      return {
        byTabId: {
          ...s.byTabId,
          [tabId]: { ...existing, onLoadMore },
        },
      };
    }),

  clearTab: (tabId) =>
    set((s) => {
      const next = { ...s.byTabId };
      delete next[tabId];
      return { byTabId: next };
    }),
}));

export function tableResultSource(
  connectionId: string,
  database: string,
  schema: string,
  table: string,
  statement: string,
  maxRows: number,
): ResultSource {
  return {
    kind: "table",
    connectionId,
    database,
    schema,
    table,
    statement,
    maxRows,
  };
}

export function queryResultSource(
  connectionId: string,
  database: string | null,
  tabId: string,
  label: string,
  statement: string,
  maxRows: number,
): ResultSource {
  return {
    kind: "query",
    connectionId,
    database,
    tabId,
    label,
    statement,
    maxRows,
  };
}

export function resultContextLabel(source: ResultSource): string {
  switch (source.kind) {
    case "table":
      return `${source.database}.${source.schema}.${source.table}`;
    case "query":
      return source.database ? `${source.database}.${source.label}` : source.label;
  }
}

export function rowCountLabel(rowCount: number, truncated: boolean): string {
  const suffix = truncated ? "+" : "";
  return `${formatCount(rowCount)}${suffix} ${rowCount === 1 && !truncated ? "row" : "rows"}`;
}

export function maxRowsLabel(maxRows: number, truncated: boolean): string {
  return truncated ? `capped at ${formatCount(maxRows)}` : `max ${formatCount(maxRows)}`;
}

function formatCount(value: number): string {
  return new Intl.NumberFormat("en-US").format(value);
}

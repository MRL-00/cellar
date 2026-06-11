import { commands, unwrap } from "@cellar/ipc";
import type { QueryResult } from "@cellar/ipc";
import { useCallback, useEffect, useRef, useState } from "react";

import { queryResultToGrid } from "../lib/gridMapping";
import {
  buildRunErrorMessage,
  buildRunResultMessages,
  buildRunStartedMessage,
  type QueryRunContext,
} from "../lib/queryMessages";
import { useNotices } from "../state/notices";
import { useQueryMessages } from "../state/queryMessages";
import { noteConnectionIssue } from "../state/connections";
import { useStatus } from "../state/status";
import type { QueryTab } from "../state/tabs";
import { useTabs } from "../state/tabs";
import { queryResultSource, useTabResults } from "../state/tabResults";

/** Row cap applied to ad-hoc editor queries before the host truncates. */
export const QUERY_ROW_LIMIT = 1000;

export interface RunOptions {
  /** Human label for the run, surfaced in the Messages panel. */
  label: string;
  /** Line to mark with the error squiggle if the run fails. */
  errorLine?: number | null;
}

export interface QueryRunner {
  running: boolean;
  /** 1-based editor line to squiggle, or null when the last run was clean. */
  errorLine: number | null;
  /** Whether more rows can be fetched (driver truncated the last result). */
  canLoadMore: boolean;
  run: (sql: string, opts: RunOptions) => void;
  /** Fetch the next page and append it to the current result grid. */
  loadMore: () => void;
  /**
   * Ask the server to stop the in-flight statement (best effort). The run
   * itself still settles through its own error path — Postgres reports
   * SQLSTATE 57014 "canceling statement due to user request".
   */
  cancel: () => void;
  clearError: () => void;
}

/**
 * Execute SQL from a query tab through the typed `run_query` IPC and fan the
 * outcome out to every surface that already listens for results: the bottom
 * Results grid (`tabResults`), the Messages log, captured database notices, and
 * the status bar. Stale completions are dropped if the tab unmounts mid-flight.
 */
export function useQueryRunner(tab: QueryTab): QueryRunner {
  const [running, setRunning] = useState(false);
  const [errorLine, setErrorLine] = useState<number | null>(null);
  const [canLoadMore, setCanLoadMore] = useState(false);
  const runToken = useRef(0);
  const mounted = useRef(true);
  // Track the current SQL and next offset for "Load more" appends.
  const loadMoreRef = useRef<{ sql: string; offset: number } | null>(null);
  // Cancellation handle for the in-flight run, passed to the backend so a
  // cancel call can find the statement's connection.
  const activeQueryId = useRef<string | null>(null);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  const clearError = useCallback(() => setErrorLine(null), []);

  const executeQuery = useCallback(
    async (
      sql: string,
      opts: RunOptions,
      offset: number,
      append: boolean,
    ) => {
      const trimmed = sql.trim();
      if (!trimmed) return;

      const token = ++runToken.current;
      const database = tab.database || null;
      const source = queryResultSource(
        tab.connectionId,
        database,
        tab.id,
        tab.title,
        trimmed,
        QUERY_ROW_LIMIT,
      );
      const context: QueryRunContext = {
        tabId: tab.id,
        connectionId: tab.connectionId,
        database: tab.database || undefined,
        label: opts.label,
        sql: trimmed,
        maxRows: QUERY_ROW_LIMIT,
      };

      setRunning(true);
      setErrorLine(null);
      if (!append) {
        useTabResults.getState().setLoading(tab.id, source);
        useQueryMessages
          .getState()
          .replaceForTab(tab.id, [buildRunStartedMessage(context)]);
      }

      const queryId = crypto.randomUUID();
      activeQueryId.current = queryId;

      void (async () => {
        try {
          const result = await unwrap(
            commands.runQuery(
              tab.connectionId,
              trimmed,
              QUERY_ROW_LIMIT,
              offset,
              database,
              tab.id,
              queryId,
            ),
          );
          if (!mounted.current || token !== runToken.current) return;

          const { columns, rows } = queryResultToGrid(result);

          if (append) {
            // Append new rows to the existing result set.
            const existing = useTabResults.getState().byTabId[tab.id];
            if (existing?.status === "ready") {
              const combined = [...existing.rows, ...rows];
              // Preserve the existing onLoadMore callback during the setReady
              // call so the "Load more" button does not flicker off between
              // the store update and the subsequent useEffect that re-registers
              // the callback. The useEffect below will overwrite it on the
              // next render cycle if canLoadMore has changed.
              useTabResults.getState().setReady(tab.id, {
                source: existing.source,
                columns: existing.columns,
                rows: combined,
                rowCount: combined.length,
                truncated: result.truncated,
                durationMs: result.duration_ms,
                rowsAffected: existing.rowsAffected,
                onLoadMore: existing.onLoadMore,
              });
            } else {
              // The tab result is no longer in 'ready' status (e.g. the user
              // started a new query mid-flight). Log a warning and discard the
              // stale appended rows rather than silently dropping them.
              console.warn(
                "[useQueryRunner] Append response arrived but tab result is not ready " +
                  "(status=%s, tabId=%s) — discarding stale rows.",
                existing?.status ?? "missing",
                tab.id,
              );
            }
          } else {
            useTabResults.getState().setReady(tab.id, {
              source,
              columns,
              rows,
              rowCount: rows.length,
              truncated: result.truncated,
              durationMs: result.duration_ms,
              rowsAffected: result.rows_affected,
            });
          }

          // Update the load-more ref for subsequent "Load more" calls.
          if (result.truncated) {
            loadMoreRef.current = {
              sql: trimmed,
              offset: offset + rows.length,
            };
            setCanLoadMore(true);
          } else {
            loadMoreRef.current = null;
            setCanLoadMore(false);
          }

          if (!append) {
            useQueryMessages
              .getState()
              .addMessages(buildRunResultMessages(context, result));
          }
          useNotices.getState().recordQueryResult(
            {
              tabId: tab.id,
              connectionId: tab.connectionId,
              database: tab.database || null,
            },
            result,
          );
          const latestResult = useTabResults.getState().byTabId[tab.id];
          const totalRowCount = latestResult?.status === "ready"
            ? latestResult.rowCount
            : rows.length;
          useStatus.getState().setLastQuery({
            connectionId: tab.connectionId,
            tabId: tab.id,
            rowCount: totalRowCount,
            truncated: result.truncated,
            durationMs: result.duration_ms,
          });
          if (!append) {
            useTabs.getState().markQueryRun(tab.id);
          }
        } catch (err) {
          if (!mounted.current || token !== runToken.current) return;
          noteConnectionIssue(tab.connectionId, err);
          const message = err instanceof Error ? err.message : String(err);
          if (!append) {
            useTabResults.getState().setError(tab.id, source, message);
          }
          useQueryMessages
            .getState()
            .addMessage(buildRunErrorMessage(context, err));
          setErrorLine(opts.errorLine ?? null);
        } finally {
          if (activeQueryId.current === queryId) {
            activeQueryId.current = null;
          }
          if (mounted.current && token === runToken.current) {
            setRunning(false);
          }
        }
      })();
    },
    [tab.connectionId, tab.database, tab.id, tab.title],
  );

  const run = useCallback(
    (sql: string, opts: RunOptions) => {
      if (running) return;
      loadMoreRef.current = null;
      setCanLoadMore(false);
      void executeQuery(sql, opts, 0, false);
    },
    [running, executeQuery],
  );

  const loadMore = useCallback(() => {
    if (running || !loadMoreRef.current) return;
    const { sql, offset } = loadMoreRef.current;
    void executeQuery(sql, { label: "Load more" }, offset, true);
  }, [running, executeQuery]);

  const cancel = useCallback(() => {
    const queryId = activeQueryId.current;
    if (!queryId) return;
    const note = (severity: "info" | "warning", text: string) => {
      useQueryMessages.getState().addMessage({
        tabId: tab.id,
        connectionId: tab.connectionId,
        database: tab.database || undefined,
        severity,
        source: "client",
        text,
      });
    };
    void unwrap(commands.cancelQuery(tab.connectionId, queryId))
      .then((cancelled) => {
        if (cancelled) {
          note("warning", "Cancel requested — the server was asked to stop the running statement.");
        } else {
          note("info", "Nothing to cancel — the statement had already finished.");
        }
      })
      .catch((err) => {
        note(
          "warning",
          `Could not cancel the running statement: ${err instanceof Error ? err.message : String(err)}`,
        );
      });
  }, [tab.connectionId, tab.database, tab.id]);

  // Keep the tabResults store in sync with the load-more callback so the
  // bottom panel's result grid can surface the "Load more" button without
  // needing a direct reference to the hook.
  useEffect(() => {
    useTabResults.getState().setLoadMoreCallback(
      tab.id,
      canLoadMore ? loadMore : null,
    );
  }, [tab.id, canLoadMore, loadMore]);

  return { running, errorLine, canLoadMore, run, loadMore, cancel, clearError };
}

// Re-export for callers that only need the type.
export type { QueryResult };

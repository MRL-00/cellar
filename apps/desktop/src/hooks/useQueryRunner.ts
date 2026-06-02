import { commands, unwrap } from "@cellar/ipc";
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
  run: (sql: string, opts: RunOptions) => void;
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
  const runToken = useRef(0);
  const mounted = useRef(true);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  const clearError = useCallback(() => setErrorLine(null), []);

  const run = useCallback(
    (sql: string, opts: RunOptions) => {
      const trimmed = sql.trim();
      if (!trimmed || running) return;

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
      useTabResults.getState().setLoading(tab.id, source);
      useQueryMessages
        .getState()
        .replaceForTab(tab.id, [buildRunStartedMessage(context)]);

      void (async () => {
        try {
          const result = await unwrap(
            commands.runQuery(
              tab.connectionId,
              trimmed,
              QUERY_ROW_LIMIT,
              database,
              tab.id,
            ),
          );
          if (!mounted.current || token !== runToken.current) return;

          const { columns, rows } = queryResultToGrid(result);
          useTabResults.getState().setReady(tab.id, {
            source,
            columns,
            rows,
            rowCount: rows.length,
            truncated: result.truncated,
            durationMs: result.duration_ms,
          });
          useQueryMessages
            .getState()
            .addMessages(buildRunResultMessages(context, result));
          useNotices.getState().recordQueryResult(
            {
              tabId: tab.id,
              connectionId: tab.connectionId,
              database: tab.database || null,
            },
            result,
          );
          useStatus.getState().setLastQuery({
            connectionId: tab.connectionId,
            tabId: tab.id,
            rowCount: rows.length,
            truncated: result.truncated,
            durationMs: result.duration_ms,
          });
          useTabs.getState().markQueryRun(tab.id);
        } catch (err) {
          if (!mounted.current || token !== runToken.current) return;
          const message = err instanceof Error ? err.message : String(err);
          useTabResults.getState().setError(tab.id, source, message);
          useQueryMessages
            .getState()
            .addMessage(buildRunErrorMessage(context, err));
          setErrorLine(opts.errorLine ?? null);
        } finally {
          if (mounted.current && token === runToken.current) {
            setRunning(false);
          }
        }
      })();
    },
    [running, tab.connectionId, tab.database, tab.id, tab.title],
  );

  return { running, errorLine, run, clearError };
}

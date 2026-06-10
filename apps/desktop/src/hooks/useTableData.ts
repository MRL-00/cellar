import { commands, unwrap } from "@cellar/ipc";
import type { QueryResult, TableBrowseRequest } from "@cellar/ipc";
import type {
  CellAlign,
  GridColumn,
  GridRow,
  GridValue,
} from "@cellar/data-grid";
import { useEffect, useState } from "react";

import {
  cellValueToGrid,
  gridWidthFor,
  isMonoType,
  isNumericType,
} from "../lib/gridMapping";
import { noteConnectionIssue, useConnections } from "../state/connections";
import { useNotices } from "../state/notices";
import { useQueryMessages } from "../state/queryMessages";
import { useStatus } from "../state/status";
import { useTabResults } from "../state/tabResults";
import {
  buildQueryErrorMessage,
  buildQueryResultMessages,
  buildTableLoadStartedMessage,
  type TableQueryContext,
} from "../lib/queryMessages";

interface TableData {
  columns: GridColumn[];
  rows: GridRow[];
  truncated: boolean;
  /** Total row count from the server (only set when include_total was requested). */
  totalRows: number | null;
  loading: boolean;
  fetching: boolean;
  error: string | null;
  durationMs: number;
  offset: number;
  limit: number;
  hasPreviousPage: boolean;
  hasNextPage: boolean;
}

const DEFAULT_LIMIT = 500;

const inflightTableLoads = new Map<string, Promise<QueryResult>>();

/**
 * Pull a page of rows for the given table out of the live connection and map
 * it into the grid's `GridColumn[]` / `GridRow[]` shapes. Lossless types
 * (numeric, uuid, temporals) survive as strings; SQL NULL maps to JS `null`
 * so the grid renders it as the italic NULL marker.
 */
export function useTableData(
  connectionId: string,
  database: string,
  schema: string,
  table: string,
  refreshKey = 0,
  tabId?: string,
  pageIndex = 0,
  pageSize = DEFAULT_LIMIT,
): TableData {
  const offset = pageIndex * pageSize;
  const [state, setState] = useState<TableData>({
    columns: [],
    rows: [],
    truncated: false,
    totalRows: null,
    loading: true,
    fetching: true,
    error: null,
    durationMs: 0,
    offset,
    limit: pageSize,
    hasPreviousPage: false,
    hasNextPage: false,
  });

  useEffect(() => {
    let cancelled = false;
    setState((s) => ({
      ...s,
      loading: s.rows.length === 0,
      fetching: true,
      error: null,
      offset,
      limit: pageSize,
    }));
    const primaryKey = tablePrimaryKey(connectionId, database, schema, table);
    const request: TableBrowseRequest = {
      connection_id: connectionId,
      database,
      schema,
      table,
      limit: pageSize,
      offset,
      sorts: [],
      filters: [],
      primary_key_fallback_ordering: true,
      // Request total count only on the first page so the pagination bar can
      // show "rows X-Y of TOTAL". Subsequent pages re-use the cached total.
      include_total: pageIndex === 0,
    };
    const messageTabId = tabId ?? tableTabId(connectionId, database, schema, table);
    const queryContext: TableQueryContext = {
      tabId: messageTabId,
      connectionId,
      database,
      schema,
      table,
      maxRows: pageSize,
      offset,
    };
    if (tabId) {
      useTabResults.getState().clearTab(tabId);
    }
    useQueryMessages
      .getState()
      .replaceForTab(messageTabId, [buildTableLoadStartedMessage(queryContext)]);
    void (async () => {
      try {
        const result = await loadTableQuery(request);
        if (cancelled) return;
        const columns = columnsFor(connectionId, database, schema, table, result);
        const rows = rowsFor(
          columns,
          result,
          primaryKey,
          offset,
        );
        setState((prev) => ({
          columns,
          rows,
          truncated: result.truncated,
          // Preserve the total count from the first page when navigating to
          // subsequent pages (include_total is only requested on page 0).
          totalRows: result.total_rows !== null && result.total_rows !== undefined
            ? Number(result.total_rows)
            : prev.totalRows,
          loading: false,
          fetching: false,
          error: null,
          durationMs: result.duration_ms,
          offset,
          limit: pageSize,
          hasPreviousPage: offset > 0,
          hasNextPage: result.truncated,
        }));
        useNotices.getState().recordQueryResult(
          { tabId: tableTabId(connectionId, database, schema, table), connectionId, database },
          result,
        );
        useQueryMessages
          .getState()
          .addMessages(buildQueryResultMessages(queryContext, result));
        useStatus.getState().setLastQuery({
          connectionId,
          tabId: tabId ?? null,
          rowCount: rows.length,
          truncated: result.truncated,
          durationMs: result.duration_ms,
        });
      } catch (err) {
        if (cancelled) return;
        noteConnectionIssue(connectionId, err);
        const message = err instanceof Error ? err.message : String(err);
        setState({
          columns: [],
          rows: [],
          truncated: false,
          totalRows: null,
          loading: false,
          fetching: false,
          error: message,
          durationMs: 0,
          offset,
          limit: pageSize,
          hasPreviousPage: offset > 0,
          hasNextPage: false,
        });
        useQueryMessages
          .getState()
          .addMessage(buildQueryErrorMessage(queryContext, err));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [connectionId, database, schema, table, refreshKey, tabId, offset, pageSize]);

  return state;
}

function tableTabId(
  connectionId: string,
  database: string,
  schema: string,
  table: string,
): string {
  return `${connectionId}::${database}.${schema}.${table}`;
}

function loadTableQuery(
  request: TableBrowseRequest,
): Promise<QueryResult> {
  const key = [
    request.connection_id,
    request.database ?? "",
    request.schema,
    request.table,
    request.limit ?? "",
    request.offset ?? "",
    JSON.stringify(request.sorts),
    JSON.stringify(request.filters),
  ].join("\u001f");
  const existing = inflightTableLoads.get(key);
  if (existing) return existing;

  const promise = unwrap(commands.browseTable(request)).finally(() => {
    if (inflightTableLoads.get(key) === promise) {
      inflightTableLoads.delete(key);
    }
  });
  inflightTableLoads.set(key, promise);
  return promise;
}
function columnsFor(
  connectionId: string,
  database: string,
  schema: string,
  table: string,
  result: QueryResult,
): GridColumn[] {
  const cache = useConnections.getState().byId[connectionId];
  const tableMeta = cache?.databases
    .filter((d) => d.name === database)
    .flatMap((d) => d.schemas)
    .find((s) => s.name === schema)
    ?.tables.find((t) => t.name === table);

  return result.columns.map((c) => {
    const meta = tableMeta?.columns.find((mc) => mc.name === c.name);
    const fk = findForeignKey(tableMeta?.foreign_keys ?? [], c.name);
    const numeric = isNumericType(c.data_type);
    const align: CellAlign | undefined = numeric ? "right" : undefined;
    return {
      key: c.name,
      name: c.name,
      type: c.data_type,
      width: gridWidthFor(c.data_type),
      pk: meta?.is_primary_key ?? false,
      fk,
      align,
      mono: numeric || isMonoType(c.data_type),
      nullable: meta ? meta.nullable : c.nullable,
    };
  });
}

function tablePrimaryKey(
  connectionId: string,
  database: string,
  schema: string,
  table: string,
): string[] {
  const cache = useConnections.getState().byId[connectionId];
  return (
    cache?.databases
      .filter((d) => d.name === database)
      .flatMap((d) => d.schemas)
      .find((s) => s.name === schema)
      ?.tables.find((t) => t.name === table)?.primary_key ?? []
  );
}

function findForeignKey(
  fks: {
    columns: string[];
    referenced_schema: string;
    referenced_table: string;
  }[],
  columnName: string,
): string | undefined {
  for (const fk of fks) {
    if (fk.columns.length === 1 && fk.columns[0] === columnName) {
      return `${fk.referenced_schema}.${fk.referenced_table}`;
    }
  }
  return undefined;
}

function rowsFor(
  columns: GridColumn[],
  result: QueryResult,
  primaryKey: string[],
  offset: number,
): GridRow[] {
  return result.rows.map((cells, i) => {
    const row: GridRow = { id: String(i) };
    cells.forEach((cell, ci) => {
      const col = columns[ci];
      if (col) row[col.key] = cellValueToGrid(cell);
    });
    row.id = rowIdFor(row, primaryKey, offset + i);
    return row;
  });
}

function rowIdFor(row: GridRow, primaryKey: string[], index: number): string {
  if (primaryKey.length === 0) return `row:${index}`;
  return JSON.stringify(
    primaryKey.map((column) => ({
      column,
      value: gridValueToString(row[column] ?? null),
    })),
  );
}

function gridValueToString(
  value: GridValue | undefined,
): string | null {
  if (value === null || value === undefined) return null;
  return String(value);
}

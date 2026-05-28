import { commands, unwrap } from "@cellar/ipc";
import type { CellValue, QueryResult } from "@cellar/ipc";
import type {
  GridCellValue,
  GridColumn,
  GridForeignKeyRef,
  GridRow,
} from "@cellar/data-grid";
import { useEffect, useState } from "react";

import { useConnections } from "../state/connections";
import { useStatus } from "../state/status";

interface TableData {
  columns: GridColumn[];
  rows: GridRow[];
  truncated: boolean;
  loading: boolean;
  error: string | null;
  durationMs: number;
}

const DEFAULT_LIMIT = 500;

/**
 * Pull a page of rows for the given table out of the live connection and
 * map it into the grid's columns/rows shape. Lossless types (numeric, uuid,
 * temporals) survive as strings; nulls land as JS `null` so the grid renders
 * them italic.
 */
export function useTableData(
  connectionId: string,
  schema: string,
  table: string,
): TableData {
  const [state, setState] = useState<TableData>({
    columns: [],
    rows: [],
    truncated: false,
    loading: true,
    error: null,
    durationMs: 0,
  });

  useEffect(() => {
    let cancelled = false;
    setState((s) => ({ ...s, loading: true, error: null }));
    const sql = `SELECT * FROM ${quoteIdent(schema)}.${quoteIdent(table)} LIMIT ${DEFAULT_LIMIT}`;
    void (async () => {
      try {
        const result = await unwrap(
          commands.runQuery(connectionId, sql, DEFAULT_LIMIT),
        );
        if (cancelled) return;
        const cols = columnsFor(connectionId, schema, table, result);
        const rows = rowsFor(result);
        setState({
          columns: cols,
          rows,
          truncated: result.truncated,
          loading: false,
          error: null,
          durationMs: result.duration_ms,
        });
        useStatus.getState().setLastQuery({
          connectionId,
          rowCount: rows.length,
          truncated: result.truncated,
          durationMs: result.duration_ms,
        });
      } catch (err) {
        if (cancelled) return;
        const message = err instanceof Error ? err.message : String(err);
        setState({
          columns: [],
          rows: [],
          truncated: false,
          loading: false,
          error: message,
          durationMs: 0,
        });
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [connectionId, schema, table]);

  return state;
}

function columnsFor(
  connectionId: string,
  schema: string,
  table: string,
  result: QueryResult,
): GridColumn[] {
  const cache = useConnections.getState().byId[connectionId];
  const tableMeta = cache?.databases
    .flatMap((d) => d.schemas)
    .find((s) => s.name === schema)
    ?.tables.find((t) => t.name === table);

  return result.columns.map((c) => {
    const meta = tableMeta?.columns.find((mc) => mc.name === c.name);
    const fk = findForeignKey(tableMeta?.foreign_keys ?? [], c.name);
    return {
      id: c.name,
      name: c.name,
      type: c.data_type,
      nullable: meta ? meta.nullable : c.nullable,
      primaryKey: meta ? meta.is_primary_key : false,
      foreignKey: fk,
    };
  });
}

function findForeignKey(
  fks: { name: string; columns: string[]; referenced_schema: string; referenced_table: string; referenced_columns: string[] }[],
  columnName: string,
): GridForeignKeyRef | undefined {
  for (const fk of fks) {
    if (fk.columns.length === 1 && fk.columns[0] === columnName) {
      return {
        schema: fk.referenced_schema,
        table: fk.referenced_table,
        columns: fk.referenced_columns,
      };
    }
  }
  return undefined;
}

function rowsFor(result: QueryResult): GridRow[] {
  return result.rows.map((cells, i) => ({
    id: i,
    cells: cells.map(cellValueToGridCell),
  }));
}

function cellValueToGridCell(value: CellValue): GridCellValue {
  switch (value.type) {
    case "Null":
      return null;
    case "Bool":
      return value.value;
    case "Int":
    case "Float":
      return value.value;
    case "Numeric":
      return value.value;
    case "Text":
      return value.value;
    case "Bytes":
      return bytesToHex(value.value);
    case "Json":
      return JSON.stringify(value.value);
    case "Uuid":
      return value.value;
    case "Date":
    case "Time":
    case "Timestamp":
    case "TimestampTz":
      return value.value;
  }
}

function bytesToHex(bytes: number[]): string {
  if (bytes.length === 0) return "\\x";
  const limit = Math.min(bytes.length, 32);
  let out = "\\x";
  for (let i = 0; i < limit; i++) {
    const b = bytes[i] ?? 0;
    out += b.toString(16).padStart(2, "0");
  }
  if (bytes.length > limit) out += `… (${bytes.length} bytes)`;
  return out;
}

function quoteIdent(s: string): string {
  return `"${s.replaceAll('"', '""')}"`;
}

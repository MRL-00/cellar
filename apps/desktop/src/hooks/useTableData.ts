import { commands, unwrap } from "@cellar/ipc";
import type { CellValue, QueryResult } from "@cellar/ipc";
import type { CellAlign, GridColumn, GridRow } from "@cellar/data-grid";
import { useEffect, useState } from "react";

import { useConnections } from "../state/connections";
import { useNotices } from "../state/notices";
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
          commands.runQuery(connectionId, sql, DEFAULT_LIMIT, database),
        );
        if (cancelled) return;
        const columns = columnsFor(connectionId, database, schema, table, result);
        const rows = rowsFor(columns, result);
        setState({
          columns,
          rows,
          truncated: result.truncated,
          loading: false,
          error: null,
          durationMs: result.duration_ms,
        });
        useNotices.getState().recordQueryResult(
          { tabId: tableTabId(connectionId, database, schema, table), connectionId, database },
          result,
        );
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
  }, [connectionId, database, schema, table]);

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
    const numeric = isNumeric(c.data_type);
    const align: CellAlign | undefined = numeric ? "right" : undefined;
    return {
      key: c.name,
      name: c.name,
      type: c.data_type,
      width: widthFor(c.data_type),
      pk: meta?.is_primary_key ?? false,
      fk,
      align,
      mono: numeric || isMonoType(c.data_type),
      nullable: meta ? meta.nullable : c.nullable,
    };
  });
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

function rowsFor(columns: GridColumn[], result: QueryResult): GridRow[] {
  return result.rows.map((cells, i) => {
    const row: GridRow = { id: String(i) };
    cells.forEach((cell, ci) => {
      const col = columns[ci];
      if (col) row[col.key] = cellValueToGrid(cell);
    });
    return row;
  });
}

function cellValueToGrid(value: CellValue): string | number | null {
  switch (value.type) {
    case "Null":
      return null;
    case "Bool":
      return value.value ? "true" : "false";
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

function isNumeric(type: string): boolean {
  const t = type.toLowerCase();
  return (
    t === "int2" ||
    t === "int4" ||
    t === "int8" ||
    t === "oid" ||
    t === "float4" ||
    t === "float8" ||
    t === "numeric"
  );
}

function isMonoType(type: string): boolean {
  const t = type.toLowerCase();
  return (
    t === "uuid" ||
    t === "json" ||
    t === "jsonb" ||
    t === "bytea" ||
    t === "date" ||
    t === "time" ||
    t === "timetz" ||
    t === "timestamp" ||
    t === "timestamptz"
  );
}

function widthFor(type: string): number {
  const t = type.toLowerCase();
  if (t === "uuid") return 290;
  if (t === "timestamptz" || t === "timestamp") return 210;
  if (t === "date") return 110;
  if (t === "json" || t === "jsonb" || t === "bytea") return 260;
  if (isNumeric(t)) return 110;
  if (t === "bool") return 80;
  return 180;
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

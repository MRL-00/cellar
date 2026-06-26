import type { CellValue, QueryResult } from "@cellar/ipc";
import type { CellAlign, GridColumn, GridRow, GridValue } from "@cellar/data-grid";

// Shared mapping from the typed IPC `QueryResult` surface into the grid's
// `GridColumn` / `GridRow` shapes. Both the table-browse path (`useTableData`)
// and the SQL-editor run path go through these helpers so column typing,
// alignment, and value fidelity stay consistent.

/** Postgres numeric type names that render right-aligned in a monospace cell. */
export function isNumericType(type: string): boolean {
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

/** Types we render in monospace even when they are not numeric. */
export function isMonoType(type: string): boolean {
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

/** A sensible default column width keyed off the engine-native type. */
export function gridWidthFor(type: string): number {
  const t = type.toLowerCase();
  if (t === "uuid") return 290;
  if (t === "timestamptz" || t === "timestamp") return 210;
  if (t === "date") return 110;
  if (t === "json" || t === "jsonb" || t === "bytea") return 260;
  if (isNumericType(t)) return 110;
  if (t === "bool") return 80;
  return 180;
}

/**
 * Full `\x`-prefixed hex for a `bytea` payload. The grid's bytea renderer
 * decodes this back into the exact bytes for hex-dump, magic-byte image
 * detection, and save-to-file — all without a re-fetch. The bytes are already
 * resident from the bounded result page, so this only doubles that cell's
 * string size; the renderer caps what it actually paints.
 */
export function bytesToHexFull(bytes: number[]): string {
  let out = "\\x";
  for (let i = 0; i < bytes.length; i++) {
    out += (bytes[i] ?? 0).toString(16).padStart(2, "0");
  }
  return out;
}

/**
 * Convert one typed cell into the grid's value space. Lossless types (numeric,
 * uuid, temporals) survive as strings; SQL NULL maps to JS `null` so the grid
 * renders the italic NULL marker.
 */
export function cellValueToGrid(value: CellValue): GridValue {
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
      return bytesToHexFull(value.value);
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

/**
 * Map a freeform query result into read-only grid columns and rows. Unlike the
 * table path, result columns have no catalog metadata, so primary-key/foreign-
 * key adornments are absent and column typing comes straight from the result
 * descriptor.
 */
export function queryResultToGrid(result: QueryResult): {
  columns: GridColumn[];
  rows: GridRow[];
} {
  const columns: GridColumn[] = result.columns.map((c) => {
    const numeric = isNumericType(c.data_type);
    const align: CellAlign | undefined = numeric ? "right" : undefined;
    return {
      key: c.name,
      name: c.name,
      type: c.data_type,
      width: gridWidthFor(c.data_type),
      align,
      mono: numeric || isMonoType(c.data_type),
      nullable: c.nullable,
    };
  });

  const rows: GridRow[] = result.rows.map((cells, i) => {
    const row: GridRow = { id: `row:${i}` };
    cells.forEach((cell, ci) => {
      const col = columns[ci];
      if (col) row[col.key] = cellValueToGrid(cell);
    });
    return row;
  });

  return { columns, rows };
}

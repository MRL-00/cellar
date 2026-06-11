import type { GridColumn, GridRow, GridValue } from "@cellar/data-grid";

// Pure exporters from the grid's column/row shapes into portable text formats.
// Shared by the bottom-panel "Export" button (file download) and the result
// grid's "Copy as" context menu (clipboard), per SPEC §9.1.

export type ExportFormat = "csv" | "tsv" | "json" | "sql";

export const EXPORT_FORMATS: { format: ExportFormat; label: string }[] = [
  { format: "csv", label: "CSV" },
  { format: "tsv", label: "TSV" },
  { format: "json", label: "JSON" },
  { format: "sql", label: "SQL INSERT" },
];

const EXTENSIONS: Record<ExportFormat, string> = {
  csv: "csv",
  tsv: "tsv",
  json: "json",
  sql: "sql",
};

const MIME_TYPES: Record<ExportFormat, string> = {
  csv: "text/csv",
  tsv: "text/tab-separated-values",
  json: "application/json",
  sql: "application/sql",
};

export interface SqlInsertOptions {
  /** Target table for the generated INSERT statements. */
  table?: string;
}

export interface DelimitedOptions {
  /** Emit a header row of column names. Defaults to true. */
  header?: boolean;
}

export function toCsv(
  columns: readonly GridColumn[],
  rows: readonly GridRow[],
  opts: DelimitedOptions = {},
): string {
  return delimited(columns, rows, ",", opts);
}

export function toTsv(
  columns: readonly GridColumn[],
  rows: readonly GridRow[],
  opts: DelimitedOptions = {},
): string {
  return delimited(columns, rows, "\t", opts);
}

/**
 * RFC 4180-style delimited text. Fields containing the delimiter, quotes, or
 * line breaks are quoted with embedded quotes doubled. SQL NULL becomes an
 * unquoted empty field while an empty string is quoted (`""`), preserving the
 * distinction the way `psql \copy ... csv` does.
 */
function delimited(
  columns: readonly GridColumn[],
  rows: readonly GridRow[],
  delimiter: string,
  { header = true }: DelimitedOptions = {},
): string {
  const escape = (value: GridValue | undefined): string => {
    if (value === null || value === undefined) return "";
    const text = String(value);
    if (text === "") return '""';
    if (
      text.includes(delimiter) ||
      text.includes('"') ||
      text.includes("\n") ||
      text.includes("\r")
    ) {
      return `"${text.replaceAll('"', '""')}"`;
    }
    return text;
  };

  const lines = header
    ? [columns.map((c) => escape(c.name)).join(delimiter)]
    : [];
  for (const row of rows) {
    lines.push(columns.map((c) => escape(row[c.key])).join(delimiter));
  }
  return lines.join("\r\n") + "\r\n";
}

/** Array of objects keyed by column name; SQL NULL maps to JSON null. */
export function toJson(
  columns: readonly GridColumn[],
  rows: readonly GridRow[],
): string {
  const objects = rows.map((row) => {
    const out: Record<string, GridValue> = {};
    for (const c of columns) {
      out[c.name] = row[c.key] ?? null;
    }
    return out;
  });
  return JSON.stringify(objects, null, 2) + "\n";
}

/**
 * One runnable INSERT per row. Identifiers are double-quoted with embedded
 * quotes doubled; string values single-quoted with embedded quotes doubled
 * (standard-conforming literals — no backslash escapes). Numbers and booleans
 * pass through bare; grid values that arrived as strings (numerics, temporals,
 * uuids) are quoted, which every engine accepts for those types.
 */
export function toSqlInserts(
  columns: readonly GridColumn[],
  rows: readonly GridRow[],
  opts: SqlInsertOptions = {},
): string {
  const table = quoteIdent(opts.table ?? "results");
  const columnList = columns.map((c) => quoteIdent(c.name)).join(", ");
  const statements = rows.map((row) => {
    const values = columns.map((c) => sqlLiteral(row[c.key])).join(", ");
    return `INSERT INTO ${table} (${columnList}) VALUES (${values});`;
  });
  return statements.join("\n") + (statements.length > 0 ? "\n" : "");
}

function quoteIdent(ident: string): string {
  return `"${ident.replaceAll('"', '""')}"`;
}

function sqlLiteral(value: GridValue | undefined): string {
  if (value === null || value === undefined) return "NULL";
  if (typeof value === "boolean") return value ? "TRUE" : "FALSE";
  if (typeof value === "number") {
    return Number.isFinite(value) ? String(value) : `'${String(value)}'`;
  }
  return `'${value.replaceAll("'", "''")}'`;
}

export function exportText(
  format: ExportFormat,
  columns: readonly GridColumn[],
  rows: readonly GridRow[],
  opts: SqlInsertOptions = {},
): string {
  switch (format) {
    case "csv":
      return toCsv(columns, rows);
    case "tsv":
      return toTsv(columns, rows);
    case "json":
      return toJson(columns, rows);
    case "sql":
      return toSqlInserts(columns, rows, opts);
  }
}

/** `label` sanitized into `label.{ext}` for the download attribute. */
export function exportFilename(label: string, format: ExportFormat): string {
  const base =
    label
      .toLowerCase()
      .replace(/\.sql$/, "")
      .replace(/[^a-z0-9._-]+/g, "-")
      .replace(/^-+|-+$/g, "") || "result";
  return `${base}.${EXTENSIONS[format]}`;
}

/** Trigger a browser-style download — same pattern as ExportSetupModal. */
export function downloadText(
  filename: string,
  format: ExportFormat,
  contents: string,
): void {
  const blob = new Blob([contents], { type: MIME_TYPES[format] });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  // Revoke on the next tick so the download has time to start.
  setTimeout(() => URL.revokeObjectURL(url), 0);
}

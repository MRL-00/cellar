//! CSV parsing + mapping for the table data-import wizard. Pure, UI-free, and
//! testable: the modal handles file picking and IPC, this builds the
//! `TableChangeRequest` the existing commit engine already understands.

import type { CellAssignment, RowChange, TableChangeRequest } from "@cellar/ipc";

export type ImportMode = "update" | "insert" | "upsert";

export interface ParsedCsv {
  headers: string[];
  /** One entry per data row. `null` = an unquoted empty field (→ SQL NULL);
   *  `""` = a quoted empty field (→ empty string). */
  rows: (string | null)[][];
}

/** The subset of table column metadata the wizard reasons about. */
export interface ImportColumn {
  name: string;
  data_type: string;
  nullable: boolean;
  is_primary_key: boolean;
  has_default: boolean;
}

/** table column name → CSV header index that supplies it. */
export type ColumnMapping = Record<string, number>;

export interface ImportConfig {
  database: string;
  schema: string;
  table: string;
  tableColumns: ImportColumn[];
  mapping: ColumnMapping;
  matchKeys: string[];
  mode: ImportMode;
  /** table columns to write on update/upsert (match keys are excluded). */
  updateFields: string[];
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/** Pick a delimiter by counting candidates in the first line. Comma by default;
 *  handles TSV / semicolon exports without a config knob. */
function sniffDelimiter(text: string): string {
  const nl = text.indexOf("\n");
  const firstLine = nl === -1 ? text : text.slice(0, nl);
  let best = ",";
  let bestCount = -1;
  for (const d of [",", "\t", ";"]) {
    const count = firstLine.split(d).length - 1;
    if (count > bestCount) {
      bestCount = count;
      best = d;
    }
  }
  return best;
}

// ponytail: single-pass parser, no dep. Handles quotes, embedded commas/
// newlines, "" escapes, CRLF. Fine to ~100k rows synchronously; move to a
// worker only if the wizard visibly janks on a real file.
export function parseCsv(text: string): ParsedCsv {
  const delimiter = sniffDelimiter(text);
  const rows: (string | null)[][] = [];
  let row: (string | null)[] = [];
  let field = "";
  let quoted = false; // this field opened with a quote
  let inQuotes = false;
  let started = false; // any char (or quote) seen for the current field
  const n = text.length;

  const endField = () => {
    row.push(quoted ? field : field.length === 0 ? null : field);
    field = "";
    quoted = false;
    started = false;
  };
  const endRow = () => {
    endField();
    rows.push(row);
    row = [];
  };

  for (let i = 0; i < n; i++) {
    const c = text[i];
    if (inQuotes) {
      if (c === '"') {
        if (text[i + 1] === '"') {
          field += '"';
          i++;
        } else {
          inQuotes = false;
        }
      } else {
        field += c;
      }
      continue;
    }
    if (c === '"' && !started) {
      inQuotes = true;
      quoted = true;
      started = true;
    } else if (c === delimiter) {
      endField();
    } else if (c === "\r") {
      // swallow CR; the following LF (or EOF) ends the row
    } else if (c === "\n") {
      endRow();
    } else {
      field += c;
      started = true;
    }
  }
  // Trailing row unless the file ended on a clean newline with nothing pending.
  if (started || quoted || field.length > 0 || row.length > 0) endRow();

  const header = rows.shift() ?? [];
  return { headers: header.map((h) => (h ?? "").trim()), rows };
}

// ---------------------------------------------------------------------------
// Mapping
// ---------------------------------------------------------------------------

/** Auto-map table columns to CSV headers by exact, then case-insensitive name. */
export function autoMap(
  headers: string[],
  tableColumns: ImportColumn[],
): ColumnMapping {
  const lower = headers.map((h) => h.toLowerCase());
  const map: ColumnMapping = {};
  for (const col of tableColumns) {
    let idx = headers.indexOf(col.name);
    if (idx === -1) idx = lower.indexOf(col.name.toLowerCase());
    if (idx !== -1) map[col.name] = idx;
  }
  return map;
}

// ---------------------------------------------------------------------------
// Request building + validation
// ---------------------------------------------------------------------------

/** A row can't be matched/inserted without a value for every match key. */
function rowKeyMissing(row: (string | null)[], cfg: ImportConfig): boolean {
  return cfg.matchKeys.some((k) => {
    const idx = cfg.mapping[k];
    const v = idx === undefined ? null : row[idx];
    return v === null || v === undefined || v === "";
  });
}

export interface ImportCounts {
  total: number;
  /** rows with a usable match key — sent to the DB. */
  toWrite: number;
  /** rows skipped because a match-key value is empty/missing. */
  skipped: number;
}

export function importCounts(csv: ParsedCsv, cfg: ImportConfig): ImportCounts {
  let skipped = 0;
  for (const row of csv.rows) if (rowKeyMissing(row, cfg)) skipped++;
  return { total: csv.rows.length, toWrite: csv.rows.length - skipped, skipped };
}

export function buildImportRequest(
  csv: ParsedCsv,
  cfg: ImportConfig,
): TableChangeRequest {
  const mappedCols = cfg.tableColumns.filter(
    (c) => cfg.mapping[c.name] !== undefined,
  );
  // Match-key columns are never overwritten on update/upsert.
  const setCols = cfg.updateFields.filter((f) => !cfg.matchKeys.includes(f));

  const cellFor = (row: (string | null)[], col: string): CellAssignment => {
    const idx = cfg.mapping[col];
    return {
      column: col,
      value: { value: idx === undefined ? null : row[idx] ?? null },
    };
  };

  const changes: RowChange[] = [];
  csv.rows.forEach((row, i) => {
    if (rowKeyMissing(row, cfg)) return;
    const row_id = `csv:${i + 2}`; // header is line 1; data starts at line 2
    if (cfg.mode === "update") {
      changes.push({
        kind: "update",
        row_id,
        keys: cfg.matchKeys.map((k) => cellFor(row, k)),
        edits: setCols.map((c) => cellFor(row, c)),
      });
    } else {
      changes.push({
        kind: "upsert",
        row_id,
        conflict_columns: cfg.matchKeys,
        values: mappedCols.map((c) => cellFor(row, c.name)),
        update_columns: cfg.mode === "upsert" ? setCols : [],
      });
    }
  });

  return {
    database: cfg.database,
    schema: cfg.schema,
    table: cfg.table,
    primary_key: cfg.matchKeys,
    columns: cfg.tableColumns.map((c) => ({
      name: c.name,
      data_type: c.data_type,
      nullable: c.nullable,
    })),
    changes,
  };
}

/** Human-readable blockers; empty array means the import is ready to preview. */
export function validateImport(
  csv: ParsedCsv | null,
  cfg: ImportConfig,
): string[] {
  if (!csv) return ["Choose a CSV file to import."];
  if (csv.rows.length === 0) return ["The CSV has no data rows."];

  const errs: string[] = [];
  if (cfg.matchKeys.length === 0) {
    errs.push("Select at least one match-key column.");
  }
  for (const k of cfg.matchKeys) {
    if (cfg.mapping[k] === undefined) {
      errs.push(`Match key "${k}" is not mapped from any CSV column.`);
    }
  }
  if (cfg.mode === "update" || cfg.mode === "upsert") {
    const set = cfg.updateFields.filter((f) => !cfg.matchKeys.includes(f));
    if (set.length === 0) errs.push("Select at least one field to update.");
  }
  if (cfg.mode === "insert" || cfg.mode === "upsert") {
    for (const c of cfg.tableColumns) {
      if (!c.nullable && !c.has_default && cfg.mapping[c.name] === undefined) {
        errs.push(
          `Required column "${c.name}" (NOT NULL, no default) must be mapped before inserting.`,
        );
      }
    }
  }
  return errs;
}

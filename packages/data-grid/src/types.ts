/** Public types for the Cellar data grid. */

export type CellAlign = "left" | "right" | "center";

/**
 * A single column definition. Mirrors a SQL column with enough metadata for the
 * grid to render the type icon, format the value, and offer the right inline
 * editor.
 */
export type GridColumn = {
  key: string;
  name: string;
  type: string;
  width: number;
  pk?: boolean;
  fk?: string;
  align?: CellAlign;
  mono?: boolean;
  nullable?: boolean;
  enum?: readonly string[];
};

/** Row values are keyed by column key. Use `null` for SQL NULL. */
export type GridRow = {
  id: string;
  [key: string]: string | number | null | undefined;
};

export type ChangeKind = "insert" | "update" | "delete";

export type CellChange = {
  from: string | number | null;
  to: string | number | null;
};

/**
 * A pending change against a row. `edits` is sparse — only the columns the user
 * has touched — and is empty for `delete` and dense for `insert`.
 */
export type PendingChange = {
  kind: ChangeKind;
  edits: Record<string, CellChange>;
};

export type PendingChanges = Record<string, PendingChange>;

export type CellAddress = {
  row: number;
  col: number;
};

export type ColumnFilters = Record<string, string>;

export type GridStatusCounts = {
  total: number;
  inserts: number;
  updates: number;
  deletes: number;
};

/**
 * Cell value that the grid knows how to render. The IPC layer hands us
 * `CellValue`; the workspace lowers it to one of these shapes (numerics and
 * uuids as strings, dates as ISO strings, etc.) before passing into the grid.
 */
export type GridCellValue = string | number | boolean | null;

export interface GridForeignKeyRef {
  schema: string;
  table: string;
  columns: string[];
}

export interface GridColumn {
  id: string;
  name: string;
  /** Engine-native type tag, e.g. `int4`, `text`, `timestamptz`. */
  type: string;
  nullable: boolean;
  primaryKey: boolean;
  foreignKey?: GridForeignKeyRef;
}

export interface GridRow {
  /** Stable identity for React reconciliation. */
  id: string | number;
  cells: GridCellValue[];
}

/** Pending in-memory edits, kept by the grid until commit. SPEC §6.5. */
export interface PendingChanges {
  inserts: number;
  updates: number;
  deletes: number;
}

export interface DataGridProps {
  columns: GridColumn[];
  rows: GridRow[];
  pendingChanges?: PendingChanges;
  /** `true` when the result was capped — the footer surfaces a "+ more" badge. */
  truncated?: boolean;
  loading?: boolean;
  error?: string | null;
}

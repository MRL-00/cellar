/** Public types for the Cellar data grid. */

export type CellAlign = "left" | "right" | "center";
export type GridValue = string | number | boolean | null;

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

export type GridColumnLayout = {
  order: string[];
  widths: Record<string, number>;
};

/** Row values are keyed by column key. Use `null` for SQL NULL. */
export type GridRow = {
  id: string;
  [key: string]: GridValue | undefined;
};

export type ChangeKind = "insert" | "update" | "delete";

export type CellChange = {
  from: GridValue;
  to: GridValue;
};

export type SortDirection = "asc" | "desc";

/**
 * Single-column sort intent. The host can map this to a server request later;
 * the grid uses the same model for local page sorting in the current slice.
 */
export type SortClause = {
  columnKey: string;
  direction: SortDirection;
};

export type SortState = SortClause | null;

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

export type FilterLogic = "and" | "or";

export type FilterOperator =
  | "equals"
  | "notEquals"
  | "contains"
  | "startsWith"
  | "greaterThan"
  | "lessThan"
  | "isNull"
  | "isNotNull";

export type FilterClause = {
  id: string;
  columnKey: string;
  operator: FilterOperator;
  value?: string;
  /** Joins this clause to the previous clause. Ignored for the first clause. */
  logic: FilterLogic;
};

export type ColumnFilters = FilterClause[];

export type GridStatusCounts = {
  total: number;
  inserts: number;
  updates: number;
  deletes: number;
};

export type GridPagination = {
  offset: number;
  limit: number;
  pageSizeOptions?: readonly number[];
  hasPrevious: boolean;
  hasNext: boolean;
  loading?: boolean;
  onPrevious: () => void;
  onNext: () => void;
  onPageSizeChange?: (next: number) => void;
};

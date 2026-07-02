import type {
  MouseEvent as ReactMouseEvent,
  ReactNode,
} from "react";
import type { CellEditorProps } from "./Cell";
import type { SavedFilterControls } from "./FilterBar";
import type { RendererRegistry, SaveBlob } from "./renderers/types";
import type {
  CellAddress,
  ColumnFilters,
  GridColumn,
  GridColumnLayout,
  GridPagination,
  GridRow,
  PendingChanges,
  SortState,
} from "./types";

/**
 * Renders a custom inline editor for a cell, or returns `null` to defer to the
 * built-in editor. See {@link DataGridProps.renderEditor}.
 */
export type CellEditorRenderer = (props: CellEditorProps) => ReactNode | null;

export type DataGridProps = {
  columns: readonly GridColumn[];
  rows: readonly GridRow[];

  /** Live pending edits. Drives the row tint and the bottom pending bar. */
  changes: PendingChanges;
  onChange: (next: PendingChanges) => void;

  /** Active selection / inline editor. Both controlled to support keyboard nav. */
  selection: CellAddress | null;
  onSelect: (next: CellAddress | null) => void;
  editing: CellAddress | null;
  onEdit: (next: CellAddress | null) => void;

  filters: ColumnFilters;
  onFiltersChange: (next: ColumnFilters) => void;

  /**
   * Quick filter pinned in the toolbar. Independent of the advanced filter
   * chips: clearing one never touches the other. When omitted, the quick filter
   * input is hidden. The compiled clause is applied server-side by the host.
   */
  quickFilter?: string;
  onQuickFilterChange?: (next: string) => void;
  quickFilterColumn?: string | null;
  onQuickFilterColumnChange?: (next: string | null) => void;

  sort?: SortState;
  onSortChange?: (next: SortState) => void;

  /** Saved filter presets shown in the toolbar. Hidden when omitted. */
  savedFilters?: SavedFilterControls;

  columnLayout?: GridColumnLayout;
  onColumnLayoutChange?: (next: GridColumnLayout) => void;

  /**
   * Number of leftmost columns to freeze. The frozen columns stick to the left
   * edge as the user scrolls horizontally — same idea as Excel's freeze panes.
   */
  frozenCount?: number;

  /** Total row count on the server (the grid usually only holds a page). */
  totalRows?: number;
  pagination?: GridPagination;

  onCommit?: () => void;
  onRevert?: () => void;

  /** Read-only result grids can still select/filter, but do not expose edits. */
  readOnly?: boolean;

  /** Text to display for NULL cell values. Defaults to "NULL". */
  nullDisplay?: string;

  /**
   * Pluggable rich-type cell renderers. Defaults to the built-in set (JSON,
   * arrays, bytea, geometry). Pass `null` to disable rich rendering entirely
   * and fall back to plain stringified display.
   */
  renderers?: RendererRegistry | null;

  /**
   * Optional override for the inline cell editor. Receives the same props as the
   * built-in editor; return an element to take over editing for a cell, or
   * `null` to fall back to the built-in text/number/native-picker editor. Lets a
   * host supply a richer editor — e.g. a calendar date picker — without pulling
   * its UI dependencies into this dependency-free package.
   */
  renderEditor?: CellEditorRenderer;

  /** Override how a renderer persists binary payloads (e.g. a native dialog). */
  saveBlob?: SaveBlob;

  /** Stripe alternating data rows. Defaults to false. */
  stripeRows?: boolean;

  /**
   * Right-click on a data cell. The host receives the row object and column so
   * it can render its own context menu (e.g. copy-as) without depending on the
   * grid's internal filter/sort order.
   */
  onCellContextMenu?: (
    event: ReactMouseEvent<HTMLDivElement>,
    row: GridRow,
    column: GridColumn,
  ) => void;

  /**
   * Full-row selection, driven by clicking the row-number gutter. Independent of
   * the single-cell `selection` (the two are mutually exclusive). The index is
   * into the grid's current visible (filtered + sorted) order; the callback also
   * hands back the row object so the host can copy it without re-deriving order.
   */
  selectedRow?: number | null;
  onRowSelect?: (rowIndex: number | null, row: GridRow | null) => void;

  /** Right-click on the row-number gutter. Selects the row, then notifies. */
  onRowContextMenu?: (
    event: ReactMouseEvent<HTMLDivElement>,
    row: GridRow,
    rowIndex: number,
  ) => void;

  /**
   * Right-click on a column header. The host receives the column so it can
   * offer column-scoped actions (e.g. "Find Usages") without the grid knowing
   * the source table/schema.
   */
  onHeaderContextMenu?: (
    event: ReactMouseEvent<HTMLDivElement>,
    column: GridColumn,
  ) => void;
};

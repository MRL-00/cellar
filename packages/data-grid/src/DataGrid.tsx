import {
  memo,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent as ReactMouseEvent,
  type PointerEvent as ReactPointerEvent,
  type UIEvent,
} from "react";
import { CellEditor, CellValue } from "./Cell";
import {
  emptyColumnLayout,
  layoutForColumns,
  MIN_COLUMN_WIDTH,
  pruneColumnLayout,
  sameColumnLayout,
} from "./columnLayout";
import { FilterBar } from "./FilterBar";
import { filterRows } from "./filters";
import { GridIcon } from "./icons";
import { PendingBar } from "./PendingBar";
import { cycleSortState, sortGridRows } from "./sort";
import { TypeIcon } from "./TypeIcon";
import type {
  CellAddress,
  CellChange,
  ColumnFilters,
  GridColumn,
  GridColumnLayout,
  GridPagination,
  GridRow,
  PendingChange,
  PendingChanges,
  SortState,
} from "./types";

const ROWNO_WIDTH = 36;
const COLUMN_AUTOFIT_PADDING = 32;
const HEADER_AUTOFIT_PADDING = 58;
const HEADER_HEIGHT = 26;
const DEFAULT_ROW_HEIGHT = 22;
const MIN_ROW_OVERSCAN = 24;
const OVERSCAN_VIEWPORTS = 3;
const VIRTUAL_ROW_THRESHOLD = 1_000;

export function calculateVirtualRows({
  rowCount,
  viewportHeight,
  scrollTop,
  rowHeight,
}: {
  rowCount: number;
  viewportHeight: number;
  scrollTop: number;
  rowHeight: number;
}) {
  const measuredRowHeight = rowHeight || DEFAULT_ROW_HEIGHT;
  const bodyScrollTop = Math.max(0, scrollTop - HEADER_HEIGHT);
  const measuredViewportHeight = viewportHeight || measuredRowHeight * 30;
  const visibleCount = Math.ceil(
    (measuredViewportHeight + HEADER_HEIGHT) / measuredRowHeight,
  );
  const overscan = Math.max(
    MIN_ROW_OVERSCAN,
    visibleCount * OVERSCAN_VIEWPORTS,
  );
  const first = Math.max(
    0,
    Math.floor(bodyScrollTop / measuredRowHeight) - overscan,
  );
  const last = Math.min(rowCount, first + visibleCount + overscan * 2);

  return {
    first,
    last,
    totalHeight: rowCount * measuredRowHeight,
  };
}

export function shouldVirtualizeRows(rowCount: number): boolean {
  return rowCount > VIRTUAL_ROW_THRESHOLD;
}

type ColumnResizeState = {
  columnKey: string;
  startX: number;
  startWidth: number;
  nextWidth: number;
};

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

  sort?: SortState;
  onSortChange?: (next: SortState) => void;

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
};

/**
 * Editable, virtualization-ready data grid. Renders one page of rows at a time
 * — virtualization will be layered in once we wire result streaming and a
 * scroller library. Today the grid is honest about pending edits, supports
 * inline editing for plain + enum cells, and emits intent through callbacks so
 * the host can plug it into Zustand or a Tauri command later.
 */
export function DataGrid({
  columns,
  rows,
  changes,
  onChange,
  selection,
  onSelect,
  editing,
  onEdit,
  filters,
  onFiltersChange,
  sort,
  onSortChange,
  columnLayout,
  onColumnLayoutChange,
  frozenCount = 2,
  totalRows,
  pagination,
  onCommit,
  onRevert,
  readOnly = false,
  nullDisplay = "NULL",
  stripeRows = false,
  onCellContextMenu,
}: DataGridProps) {
  const [internalSort, setInternalSort] = useState<SortState>(null);
  const [internalColumnLayout, setInternalColumnLayout] =
    useState<GridColumnLayout>(() => emptyColumnLayout());
  const [resizing, setResizing] = useState<ColumnResizeState | null>(null);
  const [draggedColumnKey, setDraggedColumnKey] = useState<string | null>(null);
  const [columnDropTargetKey, setColumnDropTargetKey] = useState<string | null>(null);
  const activeSort = sort ?? internalSort;
  const activeColumnLayout = columnLayout ?? internalColumnLayout;
  const isResizing = resizing !== null;
  const resizingRef = useRef<ColumnResizeState | null>(null);
  const suppressNextSortRef = useRef(false);
  const measureCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const insertCounterRef = useRef(0);

  const renderedColumns = useMemo(
    () => layoutForColumns(columns, activeColumnLayout),
    [activeColumnLayout, columns],
  );

  // Local search across visible page. Server-side filtering happens upstream
  // for large tables; the chips drive both.
  const filteredRows = useMemo(() => {
    return filterRows(rows, renderedColumns, filters, changes);
  }, [rows, renderedColumns, filters, changes]);

  const insertedRows = useMemo(() => {
    const existingIds = new Set(rows.map((row) => row.id));
    return Object.entries(changes)
      .filter(
        ([rowId, change]) => change.kind === "insert" && !existingIds.has(rowId),
      )
      .map(([rowId, change]) => {
        const row: GridRow = { id: rowId };
        for (const column of renderedColumns) {
          row[column.key] = change.edits[column.key]?.to ?? null;
        }
        return row;
      });
  }, [changes, renderedColumns, rows]);

  const visibleRows = useMemo(() => {
    return [
      ...sortGridRows(filteredRows, renderedColumns, activeSort, changes),
      ...insertedRows,
    ];
  }, [activeSort, changes, filteredRows, insertedRows, renderedColumns]);

  const minTableWidth = useMemo(
    () => renderedColumns.reduce((acc, c) => acc + c.width, ROWNO_WIDTH + 8),
    [renderedColumns],
  );
  const virtualized = shouldVirtualizeRows(visibleRows.length);

  const scrollRef = useRef<HTMLDivElement | null>(null);
  const [viewport, setViewport] = useState({
    height: 0,
    scrollTop: 0,
    rowHeight: DEFAULT_ROW_HEIGHT,
  });

  const measureViewport = useCallback((scroller: HTMLDivElement) => {
    const raw = getComputedStyle(scroller).getPropertyValue("--row-h").trim();
    const parsed = Number.parseFloat(raw);
    const rowHeight =
      Number.isFinite(parsed) && parsed > 0 ? parsed : DEFAULT_ROW_HEIGHT;

    setViewport((current) => {
      const next = {
        height: scroller.clientHeight,
        scrollTop: scroller.scrollTop,
        rowHeight,
      };
      if (
        current.height === next.height &&
        current.scrollTop === next.scrollTop &&
        current.rowHeight === next.rowHeight
      ) {
        return current;
      }
      return next;
    });
  }, []);

  useLayoutEffect(() => {
    const scroller = scrollRef.current;
    if (!scroller) return;

    measureViewport(scroller);

    let frame = 0;
    const scheduleSync = () => {
      if (frame) return;
      frame = window.requestAnimationFrame(() => {
        frame = 0;
        measureViewport(scroller);
      });
    };

    const resizeObserver = new ResizeObserver(scheduleSync);
    resizeObserver.observe(scroller);

    return () => {
      if (frame) window.cancelAnimationFrame(frame);
      resizeObserver.disconnect();
    };
  }, [measureViewport]);

  const handleScroll = useCallback((event: UIEvent<HTMLDivElement>) => {
    const scrollTop = event.currentTarget.scrollTop;
    setViewport((current) =>
      current.scrollTop === scrollTop ? current : { ...current, scrollTop },
    );
  }, []);

  const virtualRows = useMemo(() => {
    const rowHeight = viewport.rowHeight || DEFAULT_ROW_HEIGHT;
    if (!virtualized) {
      return {
        first: 0,
        rows: visibleRows,
        rowHeight,
        totalHeight: undefined,
      };
    }

    const range = calculateVirtualRows({
      rowCount: visibleRows.length,
      viewportHeight: viewport.height,
      scrollTop: viewport.scrollTop,
      rowHeight,
    });

    return {
      first: range.first,
      rows: visibleRows.slice(range.first, range.last),
      rowHeight,
      totalHeight: range.totalHeight,
    };
  }, [viewport, visibleRows, virtualized]);

  const updateColumnLayout = useCallback(
    (next: GridColumnLayout) => {
      const pruned = pruneColumnLayout(columns, next);
      if (columnLayout === undefined) setInternalColumnLayout(pruned);
      onColumnLayoutChange?.(pruned);
    },
    [columnLayout, columns, onColumnLayoutChange],
  );

  useEffect(() => {
    const pruned = pruneColumnLayout(columns, activeColumnLayout);
    if (sameColumnLayout(pruned, activeColumnLayout)) return;
    updateColumnLayout(pruned);
  }, [activeColumnLayout, columns, updateColumnLayout]);

  useEffect(() => {
    resizingRef.current = resizing;
  }, [resizing]);

  useEffect(() => {
    if (!isResizing) return;
    const previousCursor = document.body.style.cursor;
    const previousUserSelect = document.body.style.userSelect;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";

    const onPointerMove = (event: PointerEvent) => {
      const active = resizingRef.current;
      if (!active) return;

      const nextWidth = Math.max(
        MIN_COLUMN_WIDTH,
        Math.round(active.startWidth + event.clientX - active.startX),
      );
      const nextState = { ...active, nextWidth };
      resizingRef.current = nextState;
      setResizing(nextState);
      updateColumnLayout({
        ...activeColumnLayout,
        widths: {
          ...activeColumnLayout.widths,
          [active.columnKey]: nextWidth,
        },
      });
    };

    const onPointerUp = () => {
      resizingRef.current = null;
      setResizing(null);
    };

    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp, { once: true });
    window.addEventListener("pointercancel", onPointerUp, { once: true });
    return () => {
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onPointerUp);
      window.removeEventListener("pointercancel", onPointerUp);
      document.body.style.cursor = previousCursor;
      document.body.style.userSelect = previousUserSelect;
    };
  }, [activeColumnLayout, isResizing, updateColumnLayout]);

  const applySort = useCallback(
    (next: SortState) => {
      if (sort === undefined) setInternalSort(next);
      onSortChange?.(next);
    },
    [onSortChange, sort],
  );

  const translateAddress = useCallback(
    (
      address: CellAddress | null,
      before: readonly GridRow[],
      after: readonly GridRow[],
    ): CellAddress | null => {
      if (!address) return null;
      const row = before[address.row];
      if (!row) return after[address.row] ? address : null;
      const nextRow = after.findIndex((candidate) => candidate.id === row.id);
      if (nextRow === -1) return null;
      return { row: nextRow, col: address.col };
    },
    [],
  );

  const sameAddress = useCallback(
    (left: CellAddress | null, right: CellAddress | null) =>
      left?.row === right?.row && left?.col === right?.col,
    [],
  );

  const previousVisibleRows = useRef<readonly GridRow[]>(visibleRows);
  useEffect(() => {
    const before = previousVisibleRows.current;
    previousVisibleRows.current = visibleRows;
    if (before === visibleRows) return;

    const nextSelection = translateAddress(selection, before, visibleRows);
    const nextEditing = translateAddress(editing, before, visibleRows);
    if (!sameAddress(nextSelection, selection)) onSelect(nextSelection);
    if (!sameAddress(nextEditing, editing)) onEdit(nextEditing);
  }, [
    editing,
    onEdit,
    onSelect,
    sameAddress,
    selection,
    translateAddress,
    visibleRows,
  ]);

  const handleSort = useCallback(
    (columnKey: string) => {
      if (suppressNextSortRef.current) {
        suppressNextSortRef.current = false;
        return;
      }
      const next = cycleSortState(activeSort, columnKey);
      applySort(next);
    },
    [activeSort, applySort],
  );

  const beginColumnResize = useCallback(
    (event: ReactPointerEvent<HTMLSpanElement>, column: GridColumn) => {
      event.preventDefault();
      event.stopPropagation();
      suppressNextSortRef.current = true;
      const next: ColumnResizeState = {
        columnKey: column.key,
        startX: event.clientX,
        startWidth: column.width,
        nextWidth: column.width,
      };
      resizingRef.current = next;
      setResizing(next);
    },
    [],
  );

  const measureGridText = useCallback((text: string): number => {
    if (typeof document === "undefined") return text.length * 8;
    const canvas =
      measureCanvasRef.current ??
      (measureCanvasRef.current = document.createElement("canvas"));
    const context = canvas.getContext("2d");
    if (!context) return text.length * 8;
    const rootStyle = getComputedStyle(document.documentElement);
    const monoFont = rootStyle.getPropertyValue("--font-mono").trim() || "monospace";
    context.font = `11px ${monoFont}`;
    return context.measureText(text).width;
  }, []);

  const autofitColumn = useCallback(
    (column: GridColumn) => {
      const headerWidth =
        measureGridText(column.name) +
        measureGridText(column.type) +
        HEADER_AUTOFIT_PADDING;
      const valueWidth = visibleRows.reduce((maxWidth, row) => {
        const change = changes[row.id]?.edits?.[column.key];
        const value = change ? change.to : row[column.key];
        const text = value === null || value === undefined ? "NULL" : String(value);
        const adornmentWidth = column.fk ? 24 : column.enum ? 18 : 0;
        return Math.max(
          maxWidth,
          measureGridText(text) + COLUMN_AUTOFIT_PADDING + adornmentWidth,
        );
      }, 0);
      const nextWidth = Math.max(
        MIN_COLUMN_WIDTH,
        Math.ceil(Math.max(headerWidth, valueWidth)),
      );
      updateColumnLayout({
        ...activeColumnLayout,
        widths: {
          ...activeColumnLayout.widths,
          [column.key]: nextWidth,
        },
      });
    },
    [activeColumnLayout, changes, measureGridText, updateColumnLayout, visibleRows],
  );

  const reorderColumn = useCallback(
    (sourceKey: string, targetKey: string) => {
      if (sourceKey === targetKey) return;
      const order = renderedColumns.map((column) => column.key);
      const sourceIndex = order.indexOf(sourceKey);
      const targetIndex = order.indexOf(targetKey);
      if (sourceIndex === -1 || targetIndex === -1) return;
      const nextOrder = [...order];
      const [moved] = nextOrder.splice(sourceIndex, 1);
      if (!moved) return;
      nextOrder.splice(targetIndex, 0, moved);
      updateColumnLayout({ ...activeColumnLayout, order: nextOrder });
    },
    [activeColumnLayout, renderedColumns, updateColumnLayout],
  );

  const handleCellEdit = useCallback(
    (rowId: string, colKey: string, prev: CellChange["from"], next: CellChange["to"]) => {
      if (prev === next) return;
      const existing: PendingChange = changes[rowId] ?? { kind: "update", edits: {} };
      const baseEdit = existing.edits[colKey];
      const fromValue = baseEdit ? baseEdit.from : prev;

      // If the user edited back to the original value, drop the per-cell change.
      const nextEdits = { ...existing.edits };
      if (fromValue === next) {
        delete nextEdits[colKey];
      } else {
        nextEdits[colKey] = { from: fromValue, to: next };
      }

      const updated: PendingChanges = { ...changes };
      const editKeys = Object.keys(nextEdits);
      if (editKeys.length === 0 && existing.kind === "update") {
        delete updated[rowId];
      } else {
        updated[rowId] = { kind: existing.kind, edits: nextEdits };
      }
      onChange(updated);
    },
    [changes, onChange],
  );

  const handleInsertRow = useCallback(() => {
    if (readOnly || renderedColumns.length === 0) return;
    insertCounterRef.current += 1;
    const rowId = `insert:${Date.now()}:${insertCounterRef.current}`;
    onChange({
      ...changes,
      [rowId]: { kind: "insert", edits: {} },
    });
    const nextRowIndex = visibleRows.length;
    onSelect({ row: nextRowIndex, col: 0 });
    onEdit({ row: nextRowIndex, col: 0 });
    window.requestAnimationFrame(() => {
      scrollRef.current?.scrollTo({
        top: scrollRef.current.scrollHeight,
        behavior: "smooth",
      });
    });
  }, [
    changes,
    onChange,
    onEdit,
    onSelect,
    readOnly,
    renderedColumns.length,
    visibleRows.length,
  ]);

  return (
    <div
      className={
        "grid-root mono" +
        (virtualized ? "" : " grid-stable-scroll") +
        (stripeRows ? " grid-stripe-rows" : "")
      }
    >
      <FilterBar
        columns={renderedColumns}
        filters={filters}
        setFilters={onFiltersChange}
        totalRows={rows.length}
        filteredRows={visibleRows.length}
        serverRows={totalRows}
      />

      <div
        className="grid-scroll"
        ref={scrollRef}
        onScroll={virtualized ? handleScroll : undefined}
      >
        <div className="grid-table" style={{ minWidth: minTableWidth }}>
          <div className="grid-row grid-header-row">
            <div className="grid-cell grid-cell-rowno">
              <GridIcon.hash size={9} stroke="var(--fg-3)" />
            </div>
            {renderedColumns.map((c, ci) => {
              const sorted = activeSort?.columnKey === c.key ? activeSort : null;
              const ariaSort =
                sorted?.direction === "asc"
                  ? "ascending"
                  : sorted?.direction === "desc"
                    ? "descending"
                    : "none";
              const SortIcon =
                sorted?.direction === "desc"
                  ? GridIcon.sortDesc
                  : GridIcon.sortAsc;
              const nextSortLabel =
                sorted?.direction === "asc"
                  ? "Sort descending"
                  : sorted?.direction === "desc"
                    ? "Clear sort"
                    : "Sort ascending";
              return (
                <div
                  key={c.key}
                  className={
                    "grid-cell grid-header-cell" +
                    (ci < frozenCount ? " frozen" : "") +
                    (sorted ? " is-sorted" : "") +
                    (draggedColumnKey === c.key ? " is-dragging" : "") +
                    (columnDropTargetKey === c.key ? " is-drop-target" : "")
                  }
                  role="columnheader"
                  aria-sort={ariaSort}
                  aria-label={`${c.name}, ${c.type}. ${nextSortLabel}.`}
                  tabIndex={0}
                  style={{ width: c.width, flexBasis: c.width }}
                  title={nextSortLabel}
                  draggable={!isResizing}
                  onDragStart={(event) => {
                    event.dataTransfer.effectAllowed = "move";
                    event.dataTransfer.setData("text/plain", c.key);
                    setDraggedColumnKey(c.key);
                    suppressNextSortRef.current = true;
                  }}
                  onDragOver={(event) => {
                    if (!draggedColumnKey || draggedColumnKey === c.key) return;
                    event.preventDefault();
                    event.dataTransfer.dropEffect = "move";
                    setColumnDropTargetKey(c.key);
                  }}
                  onDragLeave={() => {
                    setColumnDropTargetKey((current) =>
                      current === c.key ? null : current,
                    );
                  }}
                  onDrop={(event) => {
                    event.preventDefault();
                    const sourceKey =
                      event.dataTransfer.getData("text/plain") || draggedColumnKey;
                    if (sourceKey) reorderColumn(sourceKey, c.key);
                    setDraggedColumnKey(null);
                    setColumnDropTargetKey(null);
                    suppressNextSortRef.current = true;
                  }}
                  onDragEnd={() => {
                    setDraggedColumnKey(null);
                    setColumnDropTargetKey(null);
                  }}
                  onClick={() => handleSort(c.key)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      handleSort(c.key);
                    }
                  }}
                >
                  <span className="grid-header-icon">
                    <TypeIcon col={c} />
                  </span>
                  <span className="grid-header-name">{c.name}</span>
                  <span className="grid-header-type">{c.type}</span>
                  <span className="grid-header-sort" aria-hidden="true">
                    <SortIcon size={10} />
                  </span>
                  <span
                    className={
                      "grid-col-resize" +
                      (resizing?.columnKey === c.key ? " is-resizing" : "")
                    }
                    role="separator"
                    aria-orientation="vertical"
                    aria-label={`Resize ${c.name} column`}
                    aria-valuemin={MIN_COLUMN_WIDTH}
                    aria-valuenow={
                      resizing?.columnKey === c.key ? resizing.nextWidth : c.width
                    }
                    tabIndex={-1}
                    onPointerDown={(event) => beginColumnResize(event, c)}
                    onDoubleClick={(event) => {
                      event.preventDefault();
                      event.stopPropagation();
                      suppressNextSortRef.current = true;
                      resizingRef.current = null;
                      setResizing(null);
                      autofitColumn(c);
                    }}
                    onClick={(event) => event.stopPropagation()}
                  />
                </div>
              );
            })}
          </div>

          <div
            className={
              virtualRows.totalHeight === undefined ? "" : "grid-virtual-body"
            }
            style={{ height: virtualRows.totalHeight }}
          >
            {virtualRows.rows.map((row, virtualIndex) => {
              const ri = virtualRows.first + virtualIndex;
              const change = changes[row.id];
              return (
                <GridRowView
                  key={row.id}
                  row={row}
                  rowIndex={ri}
                  rowNumber={(pagination?.offset ?? 0) + ri + 1}
                  columns={renderedColumns}
                  change={change}
                  selected={selection?.row === ri ? selection : null}
                  editing={editing?.row === ri ? editing : null}
                  frozenCount={frozenCount}
                  readOnly={readOnly}
                  nullDisplay={nullDisplay}
                  stripeRows={stripeRows}
                  top={
                    virtualRows.totalHeight === undefined
                      ? undefined
                      : ri * virtualRows.rowHeight
                  }
                  onSelect={onSelect}
                  onEdit={onEdit}
                  onCellEdit={handleCellEdit}
                  onCellContextMenu={onCellContextMenu}
                />
              );
            })}
          </div>
          {!readOnly && (
            <div
              className="grid-row grid-row-add"
              role="button"
              tabIndex={0}
              onClick={handleInsertRow}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  handleInsertRow();
                }
              }}
              title="Insert new row"
            >
              <div className="grid-cell grid-cell-rowno">
                <GridIcon.plus size={9} stroke="var(--fg-3)" />
              </div>
              <div className="grid-cell" style={{ width: 600 }}>
                <span style={{ color: "var(--fg-3)" }}>Insert new row…</span>
              </div>
            </div>
          )}
        </div>
      </div>

      {pagination && (
        <PaginationBar pagination={pagination} rowCount={rows.length} />
      )}

      {!readOnly && (
        <PendingBar changes={changes} onCommit={onCommit} onRevert={onRevert} />
      )}
    </div>
  );
}

type GridRowViewProps = {
  row: GridRow;
  rowIndex: number;
  rowNumber: number;
  columns: readonly GridColumn[];
  change: PendingChange | undefined;
  selected: CellAddress | null;
  editing: CellAddress | null;
  frozenCount: number;
  readOnly: boolean;
  nullDisplay: string;
  stripeRows: boolean;
  top: number | undefined;
  onSelect: (next: CellAddress | null) => void;
  onEdit: (next: CellAddress | null) => void;
  onCellEdit: (
    rowId: string,
    colKey: string,
    prev: CellChange["from"],
    next: CellChange["to"],
  ) => void;
  onCellContextMenu: DataGridProps["onCellContextMenu"];
};

const GridRowView = memo(function GridRowView({
  row,
  rowIndex,
  rowNumber,
  columns,
  change,
  selected,
  editing,
  frozenCount,
  readOnly,
  nullDisplay,
  stripeRows,
  top,
  onSelect,
  onEdit,
  onCellEdit,
  onCellContextMenu,
}: GridRowViewProps) {
  const kind = change?.kind;
  const rowSelected = selected !== null;
  // Compute stripe class from absolute rowIndex so it stays correct in virtual
  // scroll mode (where nth-child reflects only the current render window).
  // Only apply the stripe when there is no pending-change tint (is-update,
  // is-insert, is-delete), so the change indicators are never hidden.
  const isStripe = stripeRows && !kind && rowIndex % 2 === 1;

  return (
    <div
      className={
        "grid-row" +
        (kind ? " is-" + kind : "") +
        (isStripe ? " is-stripe" : "") +
        (rowSelected ? " is-selected-row" : "")
      }
      style={top === undefined ? undefined : { top }}
    >
      <div className="grid-cell grid-cell-rowno">
        <span className="grid-rowno-num tnum">{rowNumber}</span>
        {kind === "update" && (
          <span
            className="grid-gutter-mark"
            style={{ background: "var(--update)" }}
            title="Updated"
          />
        )}
        {kind === "insert" && (
          <span
            className="grid-gutter-mark"
            style={{ background: "var(--insert)" }}
            title="Inserted"
          />
        )}
        {kind === "delete" && (
          <span
            className="grid-gutter-mark"
            style={{ background: "var(--delete)" }}
            title="Marked for delete"
          />
        )}
      </div>
      {columns.map((c, ci) => {
        const isSel = selected?.col === ci;
        const isEdit = !readOnly && editing?.col === ci;
        const cellChange = change?.edits?.[c.key];
        const displayed = cellChange ? cellChange.to : row[c.key];
        const original = row[c.key] ?? null;

        return (
          <div
            key={c.key}
            className={
              "grid-cell" +
              (ci < frozenCount ? " frozen" : "") +
              (cellChange ? " is-edited" : "") +
              (isSel ? " is-selected" : "") +
              (isEdit ? " is-editing" : "")
            }
            style={{ width: c.width, flexBasis: c.width }}
            onClick={() => onSelect({ row: rowIndex, col: ci })}
            onDoubleClick={() => {
              if (!readOnly) onEdit({ row: rowIndex, col: ci });
            }}
            onContextMenu={
              onCellContextMenu &&
              ((event) => {
                onSelect({ row: rowIndex, col: ci });
                onCellContextMenu(event, row, c);
              })
            }
          >
            {isEdit ? (
              <CellEditor
                col={c}
                value={displayed}
                onCommit={(v) => {
                  onCellEdit(
                    row.id,
                    c.key,
                    (original ?? null) as CellChange["from"],
                    (v ?? null) as CellChange["to"],
                  );
                  onEdit(null);
                }}
                onCancel={() => onEdit(null)}
              />
            ) : (
              <CellValue col={c} value={displayed} nullDisplay={nullDisplay} />
            )}
            {cellChange && !isEdit && (
              <span
                className="grid-cell-prev"
                title={`Was: ${cellChange.from ?? "NULL"}`}
              >
                <span className="grid-cell-prev-strike">
                  {cellChange.from === null ? "NULL" : String(cellChange.from)}
                </span>
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
});

function PaginationBar({
  pagination,
  rowCount,
}: {
  pagination: GridPagination;
  rowCount: number;
}) {
  const options = pagination.pageSizeOptions ?? [100, 250, 500];
  const firstRow = rowCount === 0 ? pagination.offset : pagination.offset + 1;
  const lastRow = pagination.offset + rowCount;
  const totalRows = pagination.totalRows ?? null;

  const range =
    rowCount === 0
      ? `No rows at offset ${pagination.offset}`
      : `Rows ${formatNumber(firstRow)}–${formatNumber(lastRow)}${
          totalRows !== null ? ` of ${formatNumber(totalRows)}` : ""
        }`;
  const nextRangeStart = pagination.offset + pagination.limit + 1;
  const nextRangeEnd = pagination.offset + pagination.limit * 2;

  return (
    <div className="grid-pagination">
      <div className="grid-pagination-range">
        <span className="tnum">{range}</span>
        {pagination.hasNext && totalRows === null && (
          <span className="grid-pagination-more">more available</span>
        )}
      </div>
      <div className="grid-pagination-controls">
        {pagination.onPageSizeChange && (
          <label className="grid-pagination-size">
            <span>Page size</span>
            <select
              value={pagination.limit}
              onChange={(e) => pagination.onPageSizeChange?.(Number(e.target.value))}
              disabled={pagination.loading}
              aria-label="Page size"
            >
              {options.map((option) => (
                <option key={option} value={option}>
                  {option}
                </option>
              ))}
            </select>
          </label>
        )}
        <button
          className="grid-pagination-btn"
          type="button"
          onClick={pagination.onPrevious}
          disabled={!pagination.hasPrevious || pagination.loading}
          title="Previous page"
          aria-label="Previous page"
        >
          <GridIcon.chevronLeft size={12} />
        </button>
        <button
          className="grid-pagination-btn"
          type="button"
          onClick={pagination.onNext}
          disabled={!pagination.hasNext || pagination.loading}
          title={
            pagination.hasNext
              ? `Next page: rows ${formatNumber(nextRangeStart)}–${formatNumber(nextRangeEnd)}`
              : "Next page"
          }
          aria-label="Next page"
        >
          <GridIcon.chevronRight size={12} />
        </button>
      </div>
    </div>
  );
}

function formatNumber(value: number): string {
  return new Intl.NumberFormat("en-US").format(value);
}

export type UseGridStateOptions = {
  initialFilters?: ColumnFilters;
  initialChanges?: PendingChanges;
  initialSort?: SortState;
};

/**
 * Minimal local controller for the grid. Suits tabs that don't have a backing
 * Zustand store yet; once we wire `useTabs`, the same pieces can be lifted into
 * a store and the grid stays exactly the same.
 */
export function useGridState({
  initialFilters = [],
  initialChanges = {},
  initialSort = null,
}: UseGridStateOptions = {}) {
  const [filters, setFilters] = useState<ColumnFilters>(initialFilters);
  const [changes, setChanges] = useState<PendingChanges>(initialChanges);
  const [sort, setSort] = useState<SortState>(initialSort);
  const [selection, setSelection] = useState<CellAddress | null>(null);
  const [editing, setEditing] = useState<CellAddress | null>(null);

  const revert = useCallback(() => setChanges({}), []);

  return {
    filters,
    setFilters,
    changes,
    setChanges,
    sort,
    setSort,
    selection,
    setSelection,
    editing,
    setEditing,
    revert,
  };
}

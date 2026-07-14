import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
  type UIEvent,
} from "react";
import { applyCellChange } from "./changes";
import {
  emptyColumnLayout,
  layoutForColumns,
  MIN_COLUMN_WIDTH,
  pruneColumnLayout,
  sameColumnLayout,
} from "./columnLayout";
import type { CellEditorRenderer, DataGridProps } from "./DataGridProps";
import { FilterBar } from "./FilterBar";
import { filterRows } from "./filters";
import { GridHeaderRow, type ColumnResizeState } from "./GridHeaderRow";
import { GridIcon } from "./icons";
import { GridRowView } from "./GridRowView";
import { PaginationBar } from "./PaginationBar";
import { PendingBar } from "./PendingBar";
import { defaultRendererRegistry } from "./renderers/registry";
import { cycleSortState, sortGridRows } from "./sort";
import type {
  CellAddress,
  CellChange,
  GridColumn,
  GridColumnLayout,
  GridRow,
  PendingChanges,
  SortState,
} from "./types";
import {
  calculateVirtualRows,
  DEFAULT_ROW_HEIGHT,
  shouldVirtualizeRows,
} from "./virtualRows";

export type { CellEditorRenderer, DataGridProps } from "./DataGridProps";
export { calculateVirtualRows, shouldVirtualizeRows } from "./virtualRows";
export { useGridState, type UseGridStateOptions } from "./useGridState";

const ROWNO_WIDTH = 36;
const COLUMN_AUTOFIT_PADDING = 32;
const HEADER_AUTOFIT_PADDING = 58;
/* Default (data-driven) column widths are capped so one long text column can't
   eat the viewport; explicit double-click autofit is uncapped. */
const MAX_AUTO_WIDTH = 420;
/* ponytail: sample the first N loaded rows for default widths; scanning the
   whole page buys little once widths have converged. */
const AUTO_WIDTH_SAMPLE_ROWS = 200;

let measureCanvas: HTMLCanvasElement | null = null;
/** Measure `text` in the grid's mono font. Cell data renders at 13px
    (--fs-sm), header names at 11px. */
function measureGridText(text: string, fontSize = 13): number {
  if (typeof document === "undefined") return text.length * 8;
  measureCanvas ??= document.createElement("canvas");
  const context = measureCanvas.getContext("2d");
  if (!context) return text.length * 8;
  const monoFont =
    getComputedStyle(document.documentElement)
      .getPropertyValue("--font-mono")
      .trim() || "monospace";
  context.font = `${fontSize}px ${monoFont}`;
  return context.measureText(text).width;
}

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
  quickFilter,
  onQuickFilterChange,
  quickFilterColumn,
  onQuickFilterColumnChange,
  sort,
  onSortChange,
  savedFilters,
  columnLayout,
  onColumnLayoutChange,
  frozenCount = 0,
  pagination,
  onCommit,
  onRevert,
  readOnly = false,
  nullDisplay = "NULL",
  renderers = defaultRendererRegistry,
  renderEditor,
  saveBlob,
  stripeRows = false,
  onCellContextMenu,
  selectedRow = null,
  onRowSelect,
  onRowContextMenu,
  onHeaderContextMenu,
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
  const insertCounterRef = useRef(0);

  /* Data-driven default widths: any column WITHOUT a user-set width (resize /
     double-click autofit, persisted in the layout) is sized to fit its header
     and the visible data, capped at MAX_AUTO_WIDTH. Computed at render time so
     nothing is written back into the saved layout. */
  /* The widths object is re-spread on every pointermove during a resize drag,
     so keying the memo on it would re-measure every unsized column per move.
     Depend on the stable set of unsized column keys instead. */
  const unsizedColumnsKey = columns
    .map((column) => column.key)
    .filter((key) => activeColumnLayout.widths[key] === undefined)
    .join("\u0000");

  const autoWidths = useMemo(() => {
    const unsized = new Set(
      unsizedColumnsKey === "" ? [] : unsizedColumnsKey.split("\u0000"),
    );
    const widths: Record<string, number> = {};
    const sample = rows.slice(0, AUTO_WIDTH_SAMPLE_ROWS);
    /* Inserted rows live only in `changes` (appended to visibleRows later), so
       sample their pending values too or long inserted text gets clipped. */
    const inserts = Object.values(changes).filter(
      (change) => change.kind === "insert",
    );
    for (const column of columns) {
      if (!unsized.has(column.key)) continue;
      const adornment = column.fk ? 24 : column.enum ? 18 : 0;
      let width =
        measureGridText(column.name, 11) +
        measureGridText(column.type, 11) +
        HEADER_AUTOFIT_PADDING;
      for (const row of sample) {
        const edit = changes[row.id]?.edits?.[column.key];
        const value = edit ? edit.to : row[column.key];
        const text = value === null || value === undefined ? "NULL" : String(value);
        width = Math.max(
          width,
          measureGridText(text) + COLUMN_AUTOFIT_PADDING + adornment,
        );
        if (width >= MAX_AUTO_WIDTH) break;
      }
      for (const change of inserts) {
        if (width >= MAX_AUTO_WIDTH) break;
        const value = change.edits[column.key]?.to;
        const text = value === null || value === undefined ? "NULL" : String(value);
        width = Math.max(
          width,
          measureGridText(text) + COLUMN_AUTOFIT_PADDING + adornment,
        );
      }
      widths[column.key] = Math.min(
        MAX_AUTO_WIDTH,
        Math.max(MIN_COLUMN_WIDTH, Math.ceil(width)),
      );
    }
    return widths;
  }, [changes, columns, rows, unsizedColumnsKey]);

  const renderedColumns = useMemo(
    () =>
      layoutForColumns(columns, activeColumnLayout).map((column) => {
        const auto = autoWidths[column.key];
        return auto === undefined ? column : { ...column, width: auto };
      }),
    [activeColumnLayout, autoWidths, columns],
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

    /* --row-h comes from the density attribute on <html> (see tokens.css);
       remeasure when it flips so virtual row offsets track the CSS height. */
    const densityObserver = new MutationObserver(scheduleSync);
    densityObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-density"],
    });

    return () => {
      if (frame) window.cancelAnimationFrame(frame);
      resizeObserver.disconnect();
      densityObserver.disconnect();
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

  const translateRow = useCallback(
    (
      index: number | null,
      before: readonly GridRow[],
      after: readonly GridRow[],
    ): number | null => {
      if (index === null) return null;
      const row = before[index];
      if (!row) return after[index] ? index : null;
      const nextRow = after.findIndex((candidate) => candidate.id === row.id);
      return nextRow === -1 ? null : nextRow;
    },
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

    const nextSelectedRow = translateRow(selectedRow, before, visibleRows);
    if (nextSelectedRow !== selectedRow) {
      onRowSelect?.(
        nextSelectedRow,
        nextSelectedRow === null ? null : visibleRows[nextSelectedRow] ?? null,
      );
    }
  }, [
    editing,
    onEdit,
    onRowSelect,
    onSelect,
    sameAddress,
    selectedRow,
    selection,
    translateAddress,
    translateRow,
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

  const autofitColumn = useCallback(
    (column: GridColumn) => {
      const headerWidth =
        measureGridText(column.name, 11) +
        measureGridText(column.type, 11) +
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
    [activeColumnLayout, changes, updateColumnLayout, visibleRows],
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
      const updated = applyCellChange(changes, rowId, colKey, prev, next);
      if (updated !== changes) onChange(updated);
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
    // Inserting starts editing a cell, so clear any full-row selection to keep
    // row- and cell-selection mutually exclusive.
    onRowSelect?.(null, null);
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
    onRowSelect,
    onSelect,
    readOnly,
    renderedColumns.length,
    visibleRows.length,
  ]);

  return (
    <div
      className={
        "grid-root" +
        (virtualized ? "" : " grid-stable-scroll") +
        (stripeRows ? " grid-stripe-rows" : "")
      }
    >
      <FilterBar
        columns={renderedColumns}
        filters={filters}
        setFilters={onFiltersChange}
        quickFilter={quickFilter}
        onQuickFilterChange={onQuickFilterChange}
        quickFilterColumn={quickFilterColumn}
        onQuickFilterColumnChange={onQuickFilterColumnChange}
        totalRows={rows.length}
        filteredRows={visibleRows.length}
        serverRows={pagination?.totalRows ?? undefined}
        sort={activeSort}
        onSortChange={applySort}
        savedFilters={savedFilters}
      />

      <div
        className="grid-scroll"
        ref={scrollRef}
        onScroll={virtualized ? handleScroll : undefined}
      >
        <div className="grid-table" style={{ minWidth: minTableWidth }}>
          <GridHeaderRow
            columns={renderedColumns}
            activeSort={activeSort}
            frozenCount={frozenCount}
            isResizing={isResizing}
            resizing={resizing}
            draggedColumnKey={draggedColumnKey}
            columnDropTargetKey={columnDropTargetKey}
            setDraggedColumnKey={setDraggedColumnKey}
            setColumnDropTargetKey={setColumnDropTargetKey}
            suppressNextSort={() => {
              suppressNextSortRef.current = true;
            }}
            reorderColumn={reorderColumn}
            handleSort={handleSort}
            beginColumnResize={beginColumnResize}
            cancelResize={() => {
              resizingRef.current = null;
              setResizing(null);
            }}
            autofitColumn={autofitColumn}
            onHeaderContextMenu={onHeaderContextMenu}
          />

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
                  rowSelected={selectedRow === ri}
                  editing={editing?.row === ri ? editing : null}
                  frozenCount={frozenCount}
                  readOnly={readOnly}
                  nullDisplay={nullDisplay}
                  renderers={renderers}
                  renderEditor={renderEditor}
                  saveBlob={saveBlob}
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
                  onRowSelect={onRowSelect}
                  onRowContextMenu={onRowContextMenu}
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

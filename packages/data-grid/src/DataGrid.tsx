import { useCallback, useMemo, useState } from "react";
import { CellEditor, CellValue } from "./Cell";
import { FilterBar } from "./FilterBar";
import { GridIcon } from "./icons";
import { PendingBar } from "./PendingBar";
import { TypeIcon } from "./TypeIcon";
import type {
  CellAddress,
  CellChange,
  ColumnFilters,
  GridColumn,
  GridRow,
  PendingChange,
  PendingChanges,
} from "./types";

const ROWNO_WIDTH = 36;

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
   * Number of leftmost columns to freeze. The frozen columns stick to the left
   * edge as the user scrolls horizontally — same idea as Excel's freeze panes.
   */
  frozenCount?: number;

  /** Total row count on the server (the grid usually only holds a page). */
  totalRows?: number;

  onCommit?: () => void;
  onRevert?: () => void;

  /** Read-only result grids can still select/filter, but do not expose edits. */
  readOnly?: boolean;
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
  frozenCount = 2,
  totalRows,
  onCommit,
  onRevert,
  readOnly = false,
}: DataGridProps) {
  // Local search across visible page. Server-side filtering happens upstream
  // for large tables; the chips drive both.
  const visibleRows = useMemo(() => {
    const entries = Object.entries(filters);
    if (entries.length === 0) return rows;
    return rows.filter((row) =>
      entries.every(([k, needle]) =>
        String(row[k] ?? "")
          .toLowerCase()
          .includes(String(needle).toLowerCase()),
      ),
    );
  }, [rows, filters]);

  const minTableWidth = useMemo(
    () => columns.reduce((acc, c) => acc + c.width, ROWNO_WIDTH + 8),
    [columns],
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

  return (
    <div className="grid-root mono">
      <FilterBar
        filters={filters}
        setFilters={onFiltersChange}
        totalRows={totalRows ?? rows.length}
        filteredRows={visibleRows.length}
      />

      <div className="grid-scroll">
        <div className="grid-table" style={{ minWidth: minTableWidth }}>
          <div className="grid-row grid-header-row">
            <div className="grid-cell grid-cell-rowno">
              <GridIcon.hash size={9} stroke="var(--fg-3)" />
            </div>
            {columns.map((c, ci) => (
              <div
                key={c.key}
                className={
                  "grid-cell grid-header-cell" +
                  (ci < frozenCount ? " frozen" : "")
                }
                style={{ width: c.width, flexBasis: c.width }}
              >
                <span className="grid-header-icon">
                  <TypeIcon col={c} />
                </span>
                <span className="grid-header-name">{c.name}</span>
                <span className="grid-header-type">{c.type}</span>
                <button
                  className="grid-header-sort"
                  aria-label={`Sort ${c.name}`}
                  disabled
                  title="Sorting is not available yet"
                >
                  <GridIcon.sortAsc size={10} />
                </button>
                <span className="grid-col-resize" />
              </div>
            ))}
          </div>

          {visibleRows.map((row, ri) => {
            const change = changes[row.id];
            const kind = change?.kind;
            const rowSelected = selection?.row === ri;
            return (
              <div
                key={row.id}
                className={
                  "grid-row" +
                  (kind ? " is-" + kind : "") +
                  (rowSelected ? " is-selected-row" : "")
                }
              >
                <div className="grid-cell grid-cell-rowno">
                  <span className="grid-rowno-num tnum">{ri + 1}</span>
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
                  const isSel =
                    selection?.row === ri && selection?.col === ci;
                  const isEdit =
                    !readOnly && editing?.row === ri && editing?.col === ci;
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
                      onClick={() => onSelect({ row: ri, col: ci })}
                      onDoubleClick={() => {
                        if (!readOnly) onEdit({ row: ri, col: ci });
                      }}
                    >
                      {isEdit ? (
                        <CellEditor
                          col={c}
                          value={displayed}
                          onCommit={(v) => {
                            handleCellEdit(
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
                        <CellValue col={c} value={displayed} />
                      )}
                      {cellChange && !isEdit && (
                        <span
                          className="grid-cell-prev"
                          title={`Was: ${cellChange.from ?? "NULL"}`}
                        >
                          <span className="grid-cell-prev-strike">
                            {cellChange.from === null ? "NULL" : cellChange.from}
                          </span>
                        </span>
                      )}
                    </div>
                  );
                })}
              </div>
            );
          })}
          {!readOnly && (
            <div className="grid-row grid-row-add">
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

      {!readOnly && (
        <PendingBar changes={changes} onCommit={onCommit} onRevert={onRevert} />
      )}
    </div>
  );
}

export type UseGridStateOptions = {
  initialFilters?: ColumnFilters;
  initialChanges?: PendingChanges;
};

/**
 * Minimal local controller for the grid. Suits tabs that don't have a backing
 * Zustand store yet; once we wire `useTabs`, the same pieces can be lifted into
 * a store and the grid stays exactly the same.
 */
export function useGridState({
  initialFilters = {},
  initialChanges = {},
}: UseGridStateOptions = {}) {
  const [filters, setFilters] = useState<ColumnFilters>(initialFilters);
  const [changes, setChanges] = useState<PendingChanges>(initialChanges);
  const [selection, setSelection] = useState<CellAddress | null>(null);
  const [editing, setEditing] = useState<CellAddress | null>(null);

  const revert = useCallback(() => setChanges({}), []);

  return {
    filters,
    setFilters,
    changes,
    setChanges,
    selection,
    setSelection,
    editing,
    setEditing,
    revert,
  };
}

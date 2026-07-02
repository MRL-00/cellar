import type { PointerEvent as ReactPointerEvent } from "react";
import { MIN_COLUMN_WIDTH } from "./columnLayout";
import type { DataGridProps } from "./DataGridProps";
import { GridIcon } from "./icons";
import { TypeIcon } from "./TypeIcon";
import type { GridColumn, SortState } from "./types";

export type ColumnResizeState = {
  columnKey: string;
  startX: number;
  startWidth: number;
  nextWidth: number;
};

export type GridHeaderRowProps = {
  columns: readonly GridColumn[];
  activeSort: SortState;
  frozenCount: number;
  isResizing: boolean;
  resizing: ColumnResizeState | null;
  draggedColumnKey: string | null;
  columnDropTargetKey: string | null;
  setDraggedColumnKey: (key: string | null) => void;
  setColumnDropTargetKey: (
    updater: string | null | ((current: string | null) => string | null),
  ) => void;
  suppressNextSort: () => void;
  reorderColumn: (sourceKey: string, targetKey: string) => void;
  handleSort: (columnKey: string) => void;
  beginColumnResize: (
    event: ReactPointerEvent<HTMLSpanElement>,
    column: GridColumn,
  ) => void;
  cancelResize: () => void;
  autofitColumn: (column: GridColumn) => void;
  onHeaderContextMenu: DataGridProps["onHeaderContextMenu"];
};

export function GridHeaderRow({
  columns,
  activeSort,
  frozenCount,
  isResizing,
  resizing,
  draggedColumnKey,
  columnDropTargetKey,
  setDraggedColumnKey,
  setColumnDropTargetKey,
  suppressNextSort,
  reorderColumn,
  handleSort,
  beginColumnResize,
  cancelResize,
  autofitColumn,
  onHeaderContextMenu,
}: GridHeaderRowProps) {
  return (
    <div className="grid-row grid-header-row">
      <div className="grid-cell grid-cell-rowno">
        <GridIcon.hash size={9} stroke="var(--fg-3)" />
      </div>
      {columns.map((c, ci) => {
        const sorted = activeSort?.columnKey === c.key ? activeSort : null;
        const ariaSort =
          sorted?.direction === "asc"
            ? "ascending"
            : sorted?.direction === "desc"
              ? "descending"
              : "none";
        const SortIcon =
          sorted?.direction === "desc" ? GridIcon.sortDesc : GridIcon.sortAsc;
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
              suppressNextSort();
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
              suppressNextSort();
            }}
            onDragEnd={() => {
              setDraggedColumnKey(null);
              setColumnDropTargetKey(null);
            }}
            onClick={() => handleSort(c.key)}
            onContextMenu={
              onHeaderContextMenu
                ? (event) => onHeaderContextMenu(event, c)
                : undefined
            }
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
                suppressNextSort();
                cancelResize();
                autofitColumn(c);
              }}
              onClick={(event) => event.stopPropagation()}
            />
          </div>
        );
      })}
    </div>
  );
}

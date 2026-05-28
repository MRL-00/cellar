import type { ReactNode } from "react";

import type {
  DataGridProps,
  GridCellValue,
  GridColumn,
  GridRow,
} from "./types";

/**
 * Read-only result grid. SPEC §6.5 calls for virtualized rendering, editing,
 * and a pending-changes layer — this is the v0 scaffold: HTML table, dense
 * monospace cells, NULL distinguishable from empty string, and a footer with
 * row count and truncation badge. Virtualization swaps in via TanStack Table
 * once the read path is stable.
 */
export function DataGrid({
  columns,
  rows,
  pendingChanges,
  truncated,
  loading,
  error,
}: DataGridProps) {
  return (
    <div className="flex h-full min-h-0 w-full flex-col bg-bg-1 text-fg-1">
      <div className="min-h-0 flex-1 overflow-auto">
        <table className="w-full border-separate border-spacing-0 text-[11.5px]">
          <thead className="sticky top-0 z-10 bg-bg-2">
            <tr>
              <th className="sticky left-0 z-20 w-10 border-b border-r border-border-default bg-bg-2 px-2 py-1 text-left text-[10.5px] font-medium text-fg-3">
                #
              </th>
              {columns.map((c) => (
                <ColumnHeader key={c.id} column={c} />
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.map((r, i) => (
              <tr
                key={`${r.id}-${i}`}
                className="group hover:bg-bg-2/40"
              >
                <td className="sticky left-0 z-10 border-b border-r border-border-default bg-bg-1 px-2 py-[3px] text-right font-mono text-[10.5px] text-fg-3 group-hover:bg-bg-2">
                  {i + 1}
                </td>
                {r.cells.map((cell, ci) => (
                  <Cell
                    key={ci}
                    value={cell}
                    type={columns[ci]?.type ?? ""}
                  />
                ))}
              </tr>
            ))}
            {!loading && rows.length === 0 && !error && (
              <tr>
                <td
                  colSpan={columns.length + 1}
                  className="px-3 py-6 text-center text-[11px] text-fg-3"
                >
                  no rows
                </td>
              </tr>
            )}
            {error && (
              <tr>
                <td
                  colSpan={Math.max(columns.length, 0) + 1}
                  className="px-3 py-4 text-[11px] text-warn"
                >
                  {error}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
      <Footer
        rowCount={rows.length}
        truncated={truncated}
        pending={pendingChanges}
        loading={loading}
      />
    </div>
  );
}

function ColumnHeader({ column }: { column: GridColumn }) {
  return (
    <th className="border-b border-r border-border-default bg-bg-2 px-2 py-1 text-left text-[11px] font-medium text-fg-1">
      <div className="flex items-center gap-1.5 whitespace-nowrap">
        <span>{column.name}</span>
        {column.primaryKey && (
          <span
            className="rounded-[3px] bg-accent-soft px-1 py-px font-mono text-[9px] text-accent"
            title="primary key"
          >
            pk
          </span>
        )}
        {column.foreignKey && (
          <span
            className="rounded-[3px] bg-bg-3 px-1 py-px font-mono text-[9px] text-fg-2"
            title={`fk → ${column.foreignKey.schema}.${column.foreignKey.table}`}
          >
            fk
          </span>
        )}
        <span className="font-mono text-[9.5px] text-fg-3">{column.type}</span>
        {!column.nullable && (
          <span
            className="font-mono text-[9.5px] text-fg-3"
            title="not null"
          >
            ·!
          </span>
        )}
      </div>
    </th>
  );
}

function Cell({ value, type }: { value: GridCellValue; type: string }) {
  return (
    <td className="border-b border-r border-border-default px-2 py-[3px] align-top font-mono text-[11px] text-fg-1">
      {renderValue(value, type)}
    </td>
  );
}

function renderValue(value: GridCellValue, type: string): ReactNode {
  if (value === null) {
    return <span className="italic text-fg-3">NULL</span>;
  }
  if (typeof value === "boolean") {
    return <span>{value ? "true" : "false"}</span>;
  }
  if (typeof value === "number") {
    return <span>{value}</span>;
  }
  // Timestamps come through as ISO strings; show them dimmed for fast scanning.
  if (isTemporal(type)) {
    return <span className="text-fg-2">{value}</span>;
  }
  return <span>{value}</span>;
}

function isTemporal(type: string): boolean {
  const t = type.toLowerCase();
  return (
    t === "date" ||
    t === "time" ||
    t === "timetz" ||
    t === "timestamp" ||
    t === "timestamptz"
  );
}

function Footer({
  rowCount,
  truncated,
  pending,
  loading,
}: {
  rowCount: number;
  truncated?: boolean;
  pending?: { inserts: number; updates: number; deletes: number };
  loading?: boolean;
}) {
  return (
    <div className="flex h-6 shrink-0 items-center justify-between border-t border-border-default bg-bg-2 px-2 text-[10.5px] text-fg-3">
      <div className="flex items-center gap-2 font-mono">
        <span>{loading ? "loading…" : `${rowCount} rows`}</span>
        {truncated && (
          <span className="rounded-[3px] bg-bg-3 px-1 py-px text-fg-2">
            + more (truncated)
          </span>
        )}
      </div>
      {pending && pending.inserts + pending.updates + pending.deletes > 0 && (
        <div className="flex items-center gap-1.5 text-fg-2">
          <span>{pending.inserts} ins</span>
          <span>·</span>
          <span>{pending.updates} upd</span>
          <span>·</span>
          <span>{pending.deletes} del</span>
        </div>
      )}
    </div>
  );
}

export type { GridColumn, GridRow, GridCellValue, DataGridProps } from "./types";

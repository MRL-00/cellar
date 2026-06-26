import { useMemo, useState } from "react";
import type {
  ChangeStatus,
  Column,
  ColumnDiff,
  SchemaDiff,
  TableDiff,
  ViewDiff,
} from "@cellar/ipc";

import { Icon } from "../icons";

/**
 * Side-by-side schema diff: source on the left, target on the right, with a
 * per-object status (added / removed / changed / unchanged). Tables expand to
 * show column-level differences. This is the read-only "what differs" view;
 * statement selection lives in the migration panel.
 */
export function DiffTree({ diff }: { diff: SchemaDiff }) {
  const [showUnchanged, setShowUnchanged] = useState(false);

  const tables = useMemo(
    () =>
      showUnchanged
        ? diff.tables
        : diff.tables.filter((t) => t.status !== "unchanged"),
    [diff.tables, showUnchanged],
  );
  const views = useMemo(
    () =>
      showUnchanged
        ? diff.views
        : diff.views.filter((v) => v.status !== "unchanged"),
    [diff.views, showUnchanged],
  );

  const hiddenCount =
    diff.summary.tables_unchanged + diff.summary.views_unchanged;

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex h-[26px] shrink-0 items-center gap-2 border-b border-border-divider bg-bg-1 px-3 text-[10px] font-semibold uppercase tracking-[0.05em] text-fg-3">
        <span className="flex-1 truncate" title={diff.source_label}>
          source · {diff.source_label}
        </span>
        <span className="flex-1 truncate" title={diff.target_label}>
          target · {diff.target_label}
        </span>
        {hiddenCount > 0 && (
          <label className="flex shrink-0 cursor-pointer items-center gap-1 normal-case tracking-normal text-fg-3">
            <input
              type="checkbox"
              checked={showUnchanged}
              onChange={(e) => setShowUnchanged(e.target.checked)}
            />
            show unchanged ({hiddenCount})
          </label>
        )}
      </div>

      <div className="flex-1 overflow-auto py-1">
        {tables.length === 0 && views.length === 0 ? (
          <div className="px-3 py-3 text-[11px] text-fg-3">
            No differences between these schemas.
          </div>
        ) : (
          <>
            {tables.map((table) => (
              <TableRow key={`t:${table.name}`} table={table} />
            ))}
            {views.map((view) => (
              <ViewRow key={`v:${view.name}`} view={view} />
            ))}
          </>
        )}
      </div>
    </div>
  );
}

function TableRow({ table }: { table: TableDiff }) {
  const [open, setOpen] = useState(table.status !== "unchanged");
  const changedCols = table.columns.filter((c) => c.status !== "unchanged");
  // For a freshly added/dropped table, show its full column list; for a
  // modified table, only the columns that actually differ.
  const detailCols = !open
    ? []
    : table.status === "modified"
      ? changedCols
      : table.columns;

  return (
    <div className="border-b border-dashed border-border-divider">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center gap-1.5 px-2 py-1 text-left hover:bg-bg-2"
      >
        <span className="inline-flex w-3 shrink-0 text-fg-3">
          {open ? <Icon.chevronDown size={10} /> : <Icon.chevronRight size={10} />}
        </span>
        <StatusBadge status={table.status} />
        <Icon.table size={11} stroke="var(--fg-3)" />
        <span className="font-mono text-[11.5px] text-fg-1">{table.name}</span>
        {table.status === "modified" && changedCols.length > 0 && (
          <span className="ml-1 text-[10px] text-fg-3">
            {changedCols.length} column{changedCols.length === 1 ? "" : "s"}
          </span>
        )}
        {table.primary_key.status !== "unchanged" && (
          <span className="ml-1 rounded-[3px] bg-bg-2 px-1 text-[9.5px] text-fg-3">
            pk changed
          </span>
        )}
      </button>
      {open && detailCols.length > 0 && (
        <div className="pb-1">
          {detailCols.map((col) => (
            <ColumnRow key={col.name} col={col} />
          ))}
        </div>
      )}
    </div>
  );
}

function ColumnRow({ col }: { col: ColumnDiff }) {
  return (
    <div className="grid grid-cols-2 gap-2 px-2 pl-7 text-[10.5px]">
      <ColumnCell side="source" status={col.status} column={col.source} />
      <ColumnCell
        side="target"
        status={col.status}
        column={col.target}
        changes={col.changes}
      />
    </div>
  );
}

function ColumnCell({
  side,
  status,
  column,
  changes,
}: {
  side: "source" | "target";
  status: ChangeStatus;
  column: Column | null;
  changes?: string[];
}) {
  // Emphasize the side that carries the meaningful difference: the source for
  // removals, the target for additions, both for modifications.
  const emphasized =
    status === "modified" ||
    (status === "added" && side === "target") ||
    (status === "removed" && side === "source");
  const color = emphasized ? statusVar(status) : "var(--fg-3)";

  if (!column) {
    return <span className="font-mono text-fg-4">—</span>;
  }
  return (
    <span className="flex min-w-0 items-baseline gap-1.5 font-mono">
      <span className="truncate" style={{ color }}>
        {column.name}
      </span>
      <span className="truncate text-fg-3">{column.data_type}</span>
      {!column.nullable && <span className="text-fg-4">NOT NULL</span>}
      {side === "target" && changes && changes.length > 0 && (
        <span className="truncate text-[10px]" style={{ color: "var(--update)" }}>
          {changes.join(", ")}
        </span>
      )}
    </span>
  );
}

function ViewRow({ view }: { view: ViewDiff }) {
  return (
    <div className="flex items-center gap-1.5 border-b border-dashed border-border-divider px-2 py-1 pl-5">
      <StatusBadge status={view.status} />
      <Icon.tree size={11} stroke="var(--fg-3)" />
      <span className="font-mono text-[11.5px] text-fg-1">{view.name}</span>
      <span className="text-[10px] text-fg-3">view</span>
    </div>
  );
}

function StatusBadge({ status }: { status: ChangeStatus }) {
  const { label, color, bg } = statusStyle(status);
  return (
    <span
      className="inline-flex h-[14px] shrink-0 items-center rounded-[3px] px-1 font-mono text-[9px] font-semibold uppercase tracking-[0.04em]"
      style={{ background: bg, color }}
    >
      {label}
    </span>
  );
}

function statusVar(status: ChangeStatus): string {
  switch (status) {
    case "added":
      return "var(--insert)";
    case "removed":
      return "var(--delete)";
    case "modified":
      return "var(--update)";
    case "unchanged":
      return "var(--fg-3)";
  }
}

function statusStyle(status: ChangeStatus): {
  label: string;
  color: string;
  bg: string;
} {
  switch (status) {
    case "added":
      return { label: "add", color: "var(--insert)", bg: "var(--insert-bg)" };
    case "removed":
      return { label: "drop", color: "var(--delete)", bg: "var(--delete-bg)" };
    case "modified":
      return { label: "chg", color: "var(--update)", bg: "var(--update-bg)" };
    case "unchanged":
      return { label: "—", color: "var(--fg-3)", bg: "var(--bg-2)" };
  }
}

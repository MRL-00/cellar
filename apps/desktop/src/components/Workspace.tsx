import {
  DataGrid,
  useGridState,
  type PendingChanges,
} from "@cellar/data-grid";

import { SqlEditor } from "./SqlEditor";
import { useTabs, type TableTab } from "../state/tabs";
import { useTableData } from "../hooks/useTableData";

const EMPTY_CHANGES: PendingChanges = {};

export function Workspace({ onCommit }: { onCommit?: () => void } = {}) {
  const tabs = useTabs((s) => s.tabs);
  const activeId = useTabs((s) => s.activeId);
  const active = tabs.find((t) => t.id === activeId) ?? null;

  if (!active) {
    return <EmptyWorkspace />;
  }
  if (active.kind === "query") {
    // `key` gives each query tab its own caret/wrap state.
    return <SqlEditor key={active.id} tab={active} />;
  }
  // `key` resets the grid's local state (filters/selection) when the user
  // switches to a different table tab.
  return <TableTabPane key={active.id} tab={active} onCommit={onCommit} />;
}

function TableTabPane({
  tab,
  onCommit,
}: {
  tab: TableTab;
  onCommit?: () => void;
}) {
  const refreshKey = useTabs((s) => s.refreshKeys[tab.id] ?? 0);
  const changes = useTabs((s) => s.tableChanges[tab.id] ?? EMPTY_CHANGES);
  const setTableChanges = useTabs((s) => s.setTableChanges);
  const clearTableChanges = useTabs((s) => s.clearTableChanges);
  const data = useTableData(
    tab.connectionId,
    tab.database,
    tab.schema,
    tab.table,
    refreshKey,
    tab.id,
  );
  const grid = useGridState();

  if (data.loading) {
    return (
      <PaneMessage>
        <span className="animate-sb-pulse">loading {tab.schema}.{tab.table}…</span>
      </PaneMessage>
    );
  }
  if (data.error) {
    return (
      <PaneMessage>
        <span className="text-warn">Table load failed. See Messages for details.</span>
      </PaneMessage>
    );
  }

  return (
    <div className="flex flex-1 min-h-0 overflow-hidden">
      <DataGrid
        columns={data.columns}
        rows={data.rows}
        totalRows={data.truncated ? undefined : data.rows.length}
        changes={changes}
        onChange={(next) => setTableChanges(tab.id, next)}
        selection={grid.selection}
        onSelect={grid.setSelection}
        editing={grid.editing}
        onEdit={grid.setEditing}
        filters={grid.filters}
        onFiltersChange={grid.setFilters}
        sort={grid.sort}
        onSortChange={grid.setSort}
        onCommit={onCommit}
        onRevert={() => clearTableChanges(tab.id)}
      />
    </div>
  );
}

function PaneMessage({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex flex-1 items-center justify-center bg-bg-inset text-[11.5px] text-fg-3">
      {children}
    </div>
  );
}

function EmptyWorkspace() {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-[14px] bg-bg-inset px-10 py-10 text-center text-[12.5px] text-fg-2">
      <span
        className="relative h-9 w-9 rounded-lg"
        style={{
          background:
            "linear-gradient(135deg, #c4b5fd 0%, var(--accent) 55%, #6d4ed1 100%)",
          boxShadow: "0 0 24px var(--accent-soft)",
        }}
      >
        <span
          className="absolute inset-[5px] rounded bg-bg-inset"
          style={{
            clipPath:
              "polygon(0 0, 100% 0, 100% 35%, 35% 35%, 35% 65%, 100% 65%, 100% 100%, 0 100%)",
          }}
        />
      </span>
      <div>
        <div className="text-[14px] font-semibold text-fg-0">
          Open a table to begin
        </div>
        <div className="max-w-[360px] text-[11.5px] leading-[1.5] text-fg-3">
          Add a Postgres connection in the sidebar, expand it, and click a
          table to load real rows — or hit <span className="font-mono">+</span>{" "}
          in the tab bar to open a SQL editor.
        </div>
      </div>
      <div className="flex gap-3 text-[10.5px] text-fg-3">
        <span className="inline-flex items-center gap-1">
          <kbd className="kbd">⌘</kbd>
          <kbd className="kbd">N</kbd>&nbsp;new connection
        </span>
        <span className="inline-flex items-center gap-1">
          <kbd className="kbd">⌘</kbd>
          <kbd className="kbd">K</kbd>&nbsp;command palette
        </span>
      </div>
    </div>
  );
}

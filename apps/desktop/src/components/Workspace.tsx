import { DataGrid } from "@cellar/data-grid";

import { Icon } from "./icons";
import { useTabs, type TableTab } from "../state/tabs";
import { useTableData } from "../hooks/useTableData";

export function Workspace({ onCommit }: { onCommit?: () => void } = {}) {
  const tabs = useTabs((s) => s.tabs);
  const activeId = useTabs((s) => s.activeId);
  const active = tabs.find((t) => t.id === activeId) ?? null;

  if (!active) {
    return <EmptyWorkspace onCommit={onCommit} />;
  }
  return <TableTabPane tab={active} />;
}

function TableTabPane({ tab }: { tab: TableTab }) {
  const data = useTableData(tab.connectionId, tab.schema, tab.table);
  return (
    <div className="flex flex-1 flex-col min-h-0 overflow-hidden">
      <DataGrid
        columns={data.columns}
        rows={data.rows}
        truncated={data.truncated}
        loading={data.loading}
        error={data.error}
      />
    </div>
  );
}

function EmptyWorkspace({ onCommit }: { onCommit?: () => void }) {
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
          Add a Postgres connection in the sidebar, expand it, and double-click
          a table to load real rows. The SQL editor and pending-changes flow
          land in the next slice.
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
      {onCommit && (
        <button
          onClick={onCommit}
          title="Open the commit review modal"
          className="mt-1 inline-flex h-[26px] items-center gap-[5px] whitespace-nowrap rounded-[4px] border border-border-default bg-transparent px-2.5 text-[11.5px] font-medium text-fg-1 transition-[background,color,border-color] duration-[120ms] hover:border-border-strong hover:bg-bg-3 hover:text-fg-0"
        >
          <Icon.commit size={11} />
          <span>Review &amp; commit (preview)</span>
          <kbd className="kbd ml-1">⌘S</kbd>
        </button>
      )}
    </div>
  );
}

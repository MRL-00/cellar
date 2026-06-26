import {
  DataGrid,
  useGridState,
  type GridRow,
  type PendingChanges,
} from "@cellar/data-grid";
import {
  useCallback,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
} from "react";

import { SqlEditor } from "./SqlEditor";
import { SchemaComparePane } from "./SchemaComparePane";
import { ErDiagram } from "./er/ErDiagram";
import { ContextMenu, type ContextMenuState } from "./ContextMenu";
import { Icon } from "./icons";
import { useTabs, tabLabel, type TableTab, type WorkspaceTab } from "../state/tabs";
import { useFindUsages } from "../state/findUsages";
import { useTableData } from "../hooks/useTableData";
import { useSettings } from "../lib/settings";
import { toCsv, toJson, toSqlInserts, toTsv } from "../lib/export";

const EMPTY_CHANGES: PendingChanges = {};
const TABLE_PAGE_SIZE_OPTIONS = [100, 250, 500, 1000, 2000] as const;

export function Workspace({ onCommit }: { onCommit?: () => void } = {}) {
  const tabs = useTabs((s) => s.tabs);
  const activeId = useTabs((s) => s.activeId);
  const split = useTabs((s) => s.split);
  const setActive = useTabs((s) => s.setActive);
  const clearSplit = useTabs((s) => s.clearSplit);
  const active = tabs.find((t) => t.id === activeId) ?? null;

  if (!active) {
    return <EmptyWorkspace />;
  }

  if (split) {
    const primary = tabs.find((t) => t.id === split.primaryId) ?? active;
    const secondary = tabs.find((t) => t.id === split.secondaryId) ?? null;
    if (primary && secondary && primary.id !== secondary.id) {
      return (
        <div
          className={
            "flex min-h-0 flex-1 overflow-hidden bg-bg-inset " +
            (split.orientation === "horizontal" ? "flex-col" : "flex-row")
          }
        >
          <SplitPane
            tab={primary}
            active={primary.id === activeId}
            onActivate={() => setActive(primary.id)}
            onCloseSplit={clearSplit}
            onCommit={onCommit}
          />
          <div
            className={
              "shrink-0 bg-border-default " +
              (split.orientation === "horizontal" ? "h-px" : "w-px")
            }
          />
          <SplitPane
            tab={secondary}
            active={secondary.id === activeId}
            onActivate={() => setActive(secondary.id)}
            onCloseSplit={clearSplit}
            onCommit={onCommit}
          />
        </div>
      );
    }
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      {renderTab(active, onCommit)}
    </div>
  );
}

function SplitPane({
  tab,
  active,
  onActivate,
  onCloseSplit,
  onCommit,
}: {
  tab: WorkspaceTab;
  active: boolean;
  onActivate: () => void;
  onCloseSplit: () => void;
  onCommit?: () => void;
}) {
  return (
    <section
      className={
        "flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden " +
        (active ? "bg-bg-0" : "bg-bg-inset")
      }
      onMouseDown={onActivate}
    >
      <div
        className={
          "flex h-6 shrink-0 items-center justify-between border-b border-border-default px-2 text-[10.5px] " +
          (active ? "bg-bg-1 text-fg-1" : "bg-bg-inset text-fg-3")
        }
      >
        <button
          type="button"
          className="flex min-w-0 items-center gap-1.5 text-left"
          onClick={onActivate}
          title={tabTitle(tab)}
        >
          <span className="inline-flex shrink-0 text-fg-3">
            <TabIcon tab={tab} />
          </span>
          <span className="truncate font-mono">{tabTitle(tab)}</span>
        </button>
        <button
          type="button"
          className="icon-btn h-[18px] w-[18px]"
          onClick={(event) => {
            event.stopPropagation();
            onCloseSplit();
          }}
          title="Close split"
        >
          <Icon.close size={10} />
        </button>
      </div>
      <div className="flex min-h-0 flex-1 overflow-hidden">
        {renderTab(tab, onCommit)}
      </div>
    </section>
  );
}

function renderTab(tab: WorkspaceTab, onCommit?: () => void) {
  if (tab.kind === "query") {
    // `key` gives each query tab its own caret/wrap state.
    return <SqlEditor key={tab.id} tab={tab} />;
  }
  if (tab.kind === "schema-compare") {
    return <SchemaComparePane key={tab.id} tab={tab} />;
  }
  if (tab.kind === "er-diagram") {
    // `key` resets zoom/pan and node positions per diagram tab.
    return <ErDiagram key={tab.id} tab={tab} />;
  }
  // `key` resets the grid's local state (filters/selection) when the user
  // switches to a different table tab.
  return <TableTabPane key={tab.id} tab={tab} onCommit={onCommit} />;
}

function TabIcon({ tab }: { tab: WorkspaceTab }) {
  if (tab.kind === "query") return <Icon.terminal size={11} />;
  if (tab.kind === "schema-compare") return <Icon.diff size={11} />;
  if (tab.kind === "er-diagram") return <Icon.diagram size={11} />;
  return <Icon.table size={11} />;
}

function tabTitle(tab: WorkspaceTab): string {
  return tabLabel(tab);
}

function TableTabPane({
  tab,
  onCommit,
}: {
  tab: TableTab;
  onCommit?: () => void;
}) {
  const { settings } = useSettings();
  const refreshKey = useTabs((s) => s.refreshKeys[tab.id] ?? 0);
  const changes = useTabs((s) => s.tableChanges[tab.id] ?? EMPTY_CHANGES);
  const columnLayout = useTabs((s) => s.tableLayouts[tab.id]);
  const setTableChanges = useTabs((s) => s.setTableChanges);
  const setTableLayout = useTabs((s) => s.setTableLayout);
  const clearTableChanges = useTabs((s) => s.clearTableChanges);
  const [pageIndex, setPageIndex] = useState(0);
  const [pageSize, setPageSize] = useState(500);
  const data = useTableData(
    tab.connectionId,
    tab.database,
    tab.schema,
    tab.table,
    refreshKey,
    tab.id,
    pageIndex,
    pageSize,
  );
  const grid = useGridState();
  const [rowMenu, setRowMenu] = useState<ContextMenuState | null>(null);
  const [headerMenu, setHeaderMenu] = useState<ContextMenuState | null>(null);
  const findUsages = useFindUsages((s) => s.findUsages);
  // The grid hands back the row object on selection, so copy never has to
  // re-derive the grid's filter/sort/insert order.
  const [selectedRowData, setSelectedRowData] = useState<GridRow | null>(null);
  const handleGridChange = useCallback(
    (next: PendingChanges) => setTableChanges(tab.id, next),
    [setTableChanges, tab.id],
  );
  const handleRevert = useCallback(
    () => clearTableChanges(tab.id),
    [clearTableChanges, tab.id],
  );
  const containerRef = useRef<HTMLDivElement | null>(null);
  const copyText = useCallback((text: string) => {
    if (!navigator.clipboard) return;
    void navigator.clipboard.writeText(text);
  }, []);

  // ⌘/Ctrl+C copies the selected row as TSV (drops cleanly into spreadsheets).
  // Scoped to the grid container's focus so it never hijacks copy from the SQL
  // editor or another pane; native text selections and inline editors are left
  // to the browser.
  const handleCopyKey = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== "c") {
      return;
    }
    const active = document.activeElement;
    if (
      active instanceof HTMLElement &&
      (active.tagName === "INPUT" ||
        active.tagName === "TEXTAREA" ||
        active.isContentEditable)
    ) {
      return;
    }
    if (!window.getSelection()?.isCollapsed) return;
    if (!selectedRowData) return;
    event.preventDefault();
    copyText(
      toTsv(data.columns, [selectedRowData], { header: false }).trimEnd(),
    );
  };
  const pagination = useMemo(
    () => ({
      offset: data.offset,
      limit: data.limit,
      pageSizeOptions: TABLE_PAGE_SIZE_OPTIONS,
      hasPrevious: data.hasPreviousPage,
      hasNext: data.hasNextPage,
      loading: data.fetching,
      totalRows: data.totalRows,
      onPrevious: () => setPageIndex((page) => Math.max(0, page - 1)),
      onNext: () => setPageIndex((page) => page + 1),
      onPageSizeChange: (next: number) => {
        setPageSize(next);
        setPageIndex(0);
      },
    }),
    [
      data.fetching,
      data.hasNextPage,
      data.hasPreviousPage,
      data.limit,
      data.offset,
      data.totalRows,
    ],
  );

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
    <div
      ref={containerRef}
      tabIndex={-1}
      onKeyDown={handleCopyKey}
      className="flex flex-1 min-h-0 overflow-hidden outline-none"
    >
      <DataGrid
        columns={data.columns}
        rows={data.rows}
        totalRows={data.totalRows ?? (data.truncated ? undefined : data.rows.length)}
        pagination={pagination}
        changes={changes}
        onChange={handleGridChange}
        selection={grid.selection}
        onSelect={grid.setSelection}
        editing={grid.editing}
        onEdit={grid.setEditing}
        filters={grid.filters}
        onFiltersChange={grid.setFilters}
        sort={grid.sort}
        onSortChange={grid.setSort}
        columnLayout={columnLayout}
        onColumnLayoutChange={(next) => setTableLayout(tab.id, next)}
        onCommit={onCommit}
        onRevert={handleRevert}
        nullDisplay={settings.grid.nullDisplay}
        stripeRows={settings.grid.stripeRows}
        selectedRow={grid.selectedRow}
        onRowSelect={(rowIndex, row) => {
          grid.setSelectedRow(rowIndex);
          setSelectedRowData(row);
          if (rowIndex !== null) {
            containerRef.current?.focus({ preventScroll: true });
          }
        }}
        onHeaderContextMenu={(event, column) => {
          event.preventDefault();
          setHeaderMenu({
            x: event.clientX,
            y: event.clientY,
            items: [
              {
                label: `Find Usages of ${column.name}`,
                icon: <Icon.search size={12} />,
                onClick: () =>
                  findUsages({
                    connectionId: tab.connectionId,
                    database: tab.database,
                    schema: tab.schema,
                    table: tab.table,
                    column: column.key,
                  }),
              },
              {
                label: `Find Usages of ${tab.table}`,
                icon: <Icon.search size={12} />,
                onClick: () =>
                  findUsages({
                    connectionId: tab.connectionId,
                    database: tab.database,
                    schema: tab.schema,
                    table: tab.table,
                    column: null,
                  }),
              },
              {
                label: "Copy column name",
                icon: <Icon.copy size={12} />,
                onClick: () => copyText(column.name),
              },
            ],
          });
        }}
        onRowContextMenu={(event, row) => {
          event.preventDefault();
          setRowMenu({
            x: event.clientX,
            y: event.clientY,
            items: [
              {
                label: "Copy row as CSV",
                onClick: () =>
                  copyText(
                    toCsv(data.columns, [row], { header: false }).trimEnd(),
                  ),
              },
              {
                label: "Copy row as TSV",
                onClick: () =>
                  copyText(
                    toTsv(data.columns, [row], { header: false }).trimEnd(),
                  ),
              },
              {
                label: "Copy row as JSON",
                onClick: () => copyText(toJson(data.columns, [row]).trimEnd()),
              },
              {
                label: "Copy row as SQL INSERT",
                onClick: () =>
                  copyText(toSqlInserts(data.columns, [row]).trimEnd()),
              },
            ],
          });
        }}
      />
      <ContextMenu state={rowMenu} onClose={() => setRowMenu(null)} />
      <ContextMenu state={headerMenu} onClose={() => setHeaderMenu(null)} />
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

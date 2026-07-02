import {
  DataGrid,
  useGridState,
  type GridRow,
  type PendingChanges,
} from "@cellar/data-grid";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
} from "react";

import { SqlEditor } from "./SqlEditor";
import { renderGridEditor } from "./grid/GridDateEditor";
import { SchemaComparePane } from "./SchemaComparePane";
import { ErDiagram } from "./er/ErDiagram";
import { ContextMenu, type ContextMenuState } from "./ContextMenu";
import { TabBar } from "./TabBar";
import { Icon } from "./icons";
import {
  useTabs,
  type PaneIndex,
  type SplitEdge,
  type TableTab,
  type WorkspaceTab,
} from "../state/tabs";
import { useFindUsages } from "../state/findUsages";
import { useFilterPresets } from "../state/filterPresets";
import { useTableData } from "../hooks/useTableData";
import { useSettings } from "../lib/settings";
import { toCsv, toJson, toSqlInserts, toTsv } from "../lib/export";

const EMPTY_CHANGES: PendingChanges = {};
const TABLE_PAGE_SIZE_OPTIONS = [100, 250, 500, 1000, 2000] as const;

export function Workspace({ onCommit }: { onCommit?: () => void } = {}) {
  const tabs = useTabs((s) => s.tabs);
  const activeId = useTabs((s) => s.activeId);
  const split = useTabs((s) => s.split);
  const paneActive = useTabs((s) => s.paneActive);
  const focusedPane = useTabs((s) => s.focusedPane);
  const active = tabs.find((t) => t.id === activeId) ?? null;
  // Fraction of the split given to the primary pane; dragging the divider moves it.
  const [ratio, setRatio] = useState(0.5);
  const containerRef = useRef<HTMLDivElement | null>(null);

  const startDividerDrag = (e: React.MouseEvent) => {
    e.preventDefault();
    const el = containerRef.current;
    if (!el) return;
    const sideBySide = split === "vertical";
    const onMove = (ev: MouseEvent) => {
      const rect = el.getBoundingClientRect();
      const r = sideBySide
        ? (ev.clientX - rect.left) / rect.width
        : (ev.clientY - rect.top) / rect.height;
      setRatio(Math.min(0.85, Math.max(0.15, r)));
    };
    const onUp = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
    document.body.style.cursor = sideBySide ? "col-resize" : "row-resize";
    document.body.style.userSelect = "none";
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  };

  if (!active) {
    return <EmptyWorkspace />;
  }

  const primary =
    split && (tabs.find((t) => t.id === paneActive[0]) ?? null);
  const secondary =
    split && (tabs.find((t) => t.id === paneActive[1]) ?? null);

  const content =
    primary && secondary && primary.id !== secondary.id ? (
      <div
        ref={containerRef}
        className={
          "flex min-h-0 flex-1 overflow-hidden bg-bg-inset " +
          (split === "horizontal" ? "flex-col" : "flex-row")
        }
      >
        <SplitPane
          pane={0}
          tab={primary}
          focused={focusedPane === 0}
          grow={ratio}
          onCommit={onCommit}
        />
        <div
          role="separator"
          onMouseDown={startDividerDrag}
          className={
            "group relative z-10 shrink-0 " +
            (split === "vertical"
              ? "w-[7px] -mx-[3px] cursor-col-resize"
              : "h-[7px] -my-[3px] cursor-row-resize")
          }
        >
          <div
            className={
              "absolute bg-border-separator transition-colors duration-100 group-hover:bg-accent-line group-active:bg-accent " +
              (split === "vertical"
                ? "inset-y-0 left-1/2 w-px -translate-x-1/2"
                : "inset-x-0 top-1/2 h-px -translate-y-1/2")
            }
          />
        </div>
        <SplitPane
          pane={1}
          tab={secondary}
          focused={focusedPane === 1}
          grow={1 - ratio}
          onCommit={onCommit}
        />
      </div>
    ) : (
      <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
        {renderTab(active, onCommit)}
      </div>
    );

  return (
    <div className="relative flex min-h-0 flex-1 flex-col overflow-hidden">
      {content}
      <SplitDropZones />
    </div>
  );
}

function SplitPane({
  pane,
  tab,
  focused,
  grow,
  onCommit,
}: {
  pane: PaneIndex;
  tab: WorkspaceTab;
  focused: boolean;
  grow: number;
  onCommit?: () => void;
}) {
  const focusPane = useTabs((s) => s.focusPane);
  return (
    <section
      style={{ flexGrow: grow }}
      className={
        "flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden " +
        (focused ? "bg-bg-0" : "bg-bg-inset")
      }
      onMouseDown={() => focusPane(pane)}
    >
      <TabBar pane={pane} />
      <div className="flex min-h-0 flex-1 overflow-hidden">
        {renderTab(tab, onCommit)}
      </div>
    </section>
  );
}

/** Edge bands that turn a dropped tab into a split. Shown only during a drag. */
function SplitDropZones() {
  const draggingTabId = useTabs((s) => s.draggingTabId);
  const tabCount = useTabs((s) => s.tabs.length);
  const dropTabToSplit = useTabs((s) => s.dropTabToSplit);
  const [edge, setEdge] = useState<SplitEdge | null>(null);

  // Need at least two tabs to leave both panes non-empty.
  if (!draggingTabId || tabCount < 2) return null;

  const zone = (e: SplitEdge, className: string) => (
    <div
      className={"pointer-events-auto absolute " + className}
      onDragOver={(ev) => {
        ev.preventDefault();
        ev.dataTransfer.dropEffect = "move";
        setEdge(e);
      }}
      onDragLeave={() => setEdge((cur) => (cur === e ? null : cur))}
      onDrop={(ev) => {
        ev.preventDefault();
        const id = ev.dataTransfer.getData("text/plain") || draggingTabId;
        if (id) dropTabToSplit(id, e);
        setEdge(null);
      }}
    />
  );

  const previewClass: Record<SplitEdge, string> = {
    left: "inset-y-0 left-0 w-1/2",
    right: "inset-y-0 right-0 w-1/2",
    top: "inset-x-0 top-0 h-1/2",
    bottom: "inset-x-0 bottom-0 h-1/2",
  };

  return (
    <div className="pointer-events-none absolute inset-0 z-20">
      {edge && (
        <div
          className={
            "absolute border-2 border-accent bg-accent-soft transition-all " +
            previewClass[edge]
          }
        />
      )}
      {zone("left", "inset-y-0 left-0 w-[14%]")}
      {zone("right", "inset-y-0 right-0 w-[14%]")}
      {zone("bottom", "inset-x-0 bottom-0 h-[18%]")}
    </div>
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
  const grid = useGridState();
  // Quick filter is kept separate from the advanced chips (`grid.filters`) so
  // clearing one never disturbs the other. FilterBar owns the typed text and
  // debounces it, so `quickFilter` only updates (and re-queries) once the user
  // pauses — keystrokes never re-render the grid.
  const [quickFilter, setQuickFilter] = useState("");
  const [quickColumn, setQuickColumn] = useState<string | null>(null);
  // Saved presets snapshot the whole toolbar (quick filter, chips, order by)
  // per table, persisted to localStorage so they survive restarts.
  const presets = useFilterPresets((s) => s.presets[tab.id]);
  const savePreset = useFilterPresets((s) => s.savePreset);
  const deletePreset = useFilterPresets((s) => s.deletePreset);
  const savedFilters = useMemo(
    () => ({
      names: (presets ?? []).map((preset) => preset.name),
      // Derived, not tracked: the dropdown names whichever preset the toolbar
      // currently matches, and clears itself the moment anything is edited.
      activeName:
        (presets ?? []).find(
          (preset) =>
            JSON.stringify([
              preset.filters,
              preset.sort,
              preset.quickFilter,
              preset.quickColumn,
            ]) ===
            JSON.stringify([grid.filters, grid.sort, quickFilter, quickColumn]),
        )?.name ?? null,
      onSave: (name: string) =>
        savePreset(tab.id, {
          name,
          filters: grid.filters,
          sort: grid.sort,
          quickFilter,
          quickColumn,
        }),
      onApply: (name: string) => {
        const preset = (presets ?? []).find((item) => item.name === name);
        if (!preset) return;
        grid.setFilters(preset.filters);
        grid.setSort(preset.sort);
        setQuickFilter(preset.quickFilter);
        setQuickColumn(preset.quickColumn);
      },
      onDelete: (name: string) => deletePreset(tab.id, name),
      onClear: () => {
        grid.setFilters([]);
        grid.setSort(null);
        setQuickFilter("");
        setQuickColumn(null);
      },
    }),
    [
      presets,
      savePreset,
      deletePreset,
      tab.id,
      grid.filters,
      grid.sort,
      grid.setFilters,
      grid.setSort,
      quickFilter,
      quickColumn,
    ],
  );
  const data = useTableData(
    tab.connectionId,
    tab.database,
    tab.schema,
    tab.table,
    refreshKey,
    tab.id,
    pageIndex,
    pageSize,
    grid.filters,
    quickFilter,
    quickColumn,
    grid.sort,
  );
  // Filters/sort apply to the whole table, so any change must jump back to the
  // first page (page size already resets in its own handler below).
  const filtersKey = JSON.stringify(grid.filters);
  const sortKey = JSON.stringify(grid.sort);
  useEffect(() => {
    setPageIndex(0);
  }, [filtersKey, sortKey, quickFilter, quickColumn]);
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

  // Toggle the given row's pending delete. A pending insert is cancelled
  // outright (nothing in the DB to delete); a row already marked delete toggles
  // back off. Otherwise mark it (dropping any pending edits — the delete wins).
  const handleDeleteRow = useCallback(
    (row: GridRow) => {
      const existing = changes[row.id];
      const next = { ...changes };
      if (existing?.kind === "insert" || existing?.kind === "delete") {
        delete next[row.id];
      } else {
        next[row.id] = { kind: "delete", edits: {} };
      }
      setTableChanges(tab.id, next);
    },
    [changes, setTableChanges, tab.id],
  );

  // Keyboard shortcuts active while the grid container holds focus. Native text
  // selections and inline editors (input/textarea/contenteditable) are left to
  // the browser so we never hijack copy/delete from the SQL editor or a cell.
  const handleGridKey = useCallback(
    (event: ReactKeyboardEvent<HTMLDivElement>) => {
      const active = document.activeElement;
      const inEditor =
        active instanceof HTMLElement &&
        (active.tagName === "INPUT" ||
          active.tagName === "TEXTAREA" ||
          active.isContentEditable);
      if (inEditor) return;

      // Delete/Backspace marks the selected row for deletion.
      if (event.key === "Delete" || event.key === "Backspace") {
        if (!selectedRowData) return;
        event.preventDefault();
        handleDeleteRow(selectedRowData);
        return;
      }

      // ⌘/Ctrl+C copies the selected row as TSV (drops cleanly into spreadsheets).
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "c") {
        if (!window.getSelection()?.isCollapsed) return;
        if (!selectedRowData) return;
        event.preventDefault();
        copyText(
          toTsv(data.columns, [selectedRowData], { header: false }).trimEnd(),
        );
      }
    },
    [handleDeleteRow, selectedRowData, data.columns, copyText],
  );
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
        <span className="animate-sb-pulse">
          loading {tab.schema}.{tab.table}…
        </span>
      </PaneMessage>
    );
  }
  if (data.error) {
    return (
      <PaneMessage>
        <span className="text-warn">
          Table load failed. See Messages for details.
        </span>
      </PaneMessage>
    );
  }

  return (
    <div
      ref={containerRef}
      tabIndex={-1}
      onKeyDown={handleGridKey}
      className="flex flex-1 min-h-0 overflow-hidden outline-none"
    >
      <DataGrid
        columns={data.columns}
        rows={data.rows}
        pagination={pagination}
        changes={changes}
        onChange={handleGridChange}
        selection={grid.selection}
        onSelect={grid.setSelection}
        editing={grid.editing}
        onEdit={grid.setEditing}
        renderEditor={renderGridEditor}
        filters={grid.filters}
        onFiltersChange={grid.setFilters}
        quickFilter={quickFilter}
        onQuickFilterChange={setQuickFilter}
        quickFilterColumn={quickColumn}
        onQuickFilterColumnChange={setQuickColumn}
        sort={grid.sort}
        onSortChange={grid.setSort}
        savedFilters={savedFilters}
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
              {
                label:
                  changes[row.id]?.kind === "delete"
                    ? "Unmark row for delete"
                    : changes[row.id]?.kind === "insert"
                      ? "Cancel insert"
                      : "Delete row",
                icon: <Icon.trash size={12} />,
                danger: changes[row.id]?.kind !== "delete",
                onClick: () => handleDeleteRow(row),
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
    <div className="flex flex-1 items-center justify-center bg-bg-inset text-[12.5px] text-fg-3">
      {children}
    </div>
  );
}

function EmptyWorkspace() {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-[14px] bg-bg-inset px-10 py-10 text-center text-sm text-fg-2">
      <span
        className="relative h-9 w-9 rounded-lg"
        style={{
          background:
            "linear-gradient(135deg, #c4b5fd 0%, #a78bfa 55%, #6d4ed1 100%)",
          boxShadow: "0 0 24px rgba(167, 139, 250, 0.14)",
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
        <div className="text-[15px] font-semibold text-fg-0">
          Open a table to begin
        </div>
        <div className="max-w-[360px] text-[12.5px] leading-[1.5] text-fg-3">
          Add a Postgres connection in the sidebar, expand it, and click a table
          to load real rows — or hit <span className="font-mono">+</span> in the
          tab bar to open a SQL editor.
        </div>
      </div>
      <div className="flex gap-3 text-[11.5px] text-fg-3">
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

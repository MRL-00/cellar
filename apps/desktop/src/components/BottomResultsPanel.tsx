import {
  DataGrid,
  filterRows,
  sortGridRows,
  useGridState,
  type GridColumn,
  type GridRow,
} from "@cellar/data-grid";
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type MutableRefObject,
} from "react";
import { useSettings } from "../lib/settings";
import { toCsv, toJson, toSqlInserts, toTsv } from "../lib/export";
import { ContextMenu, type ContextMenuState } from "./ContextMenu";
import { renderFilterValue, renderGridEditor } from "./grid/GridDateEditor";
import { Icon } from "./icons";
import { useTabs, type TableTab } from "../state/tabs";
import { maxRowsLabel, resultContextLabel, rowCountLabel, type TabResult } from "../state/tabResults";

/**
 * The rows/columns as the user currently sees them — after the result grid's
 * local filters and sort. Registered by ReadOnlyResultGrid so the Export
 * button (which lives outside the grid) exports what is on screen.
 */
export type ExportView = { columns: readonly GridColumn[]; rows: readonly GridRow[] };
export type ExportViewRef = MutableRefObject<(() => ExportView) | null>;

export function HeaderMeta({
  activeTab,
  result,
}: {
  activeTab: ReturnType<typeof useTabs.getState>["tabs"][number] | null;
  result: TabResult | null;
}) {
  const items = headerItems(activeTab, result);

  return (
    <div className="inline-flex min-w-0 items-center gap-1.5 overflow-hidden font-mono text-[10.5px]">
      {items.map((item, i) => (
        <span
          key={`${item}-${i}`}
          className={i === 0 ? "truncate text-fg-2" : "shrink-0 text-fg-3"}
        >
          {i > 0 && <span className="text-fg-4">· </span>}
          {item}
        </span>
      ))}
    </div>
  );
}

function headerItems(
  activeTab: ReturnType<typeof useTabs.getState>["tabs"][number] | null,
  result: TabResult | null,
): string[] {
  if (!activeTab) return ["no active tab"];
  if (activeTab.kind === "query") return [activeTab.title, "query tab"];
  if (activeTab.kind === "schema-compare")
    return [activeTab.title, "schema compare tab"];
  if (activeTab.kind === "er-diagram") return [activeTab.title, "ER diagram"];
  if (!result) return [tableLabel(activeTab), "table rows shown above"];

  const context = resultContextLabel(result.source);
  if (result.source.kind === "table") {
    return [context, "table rows shown above"];
  }
  if (result.status === "loading") {
    return [context, "loading", maxRowsLabel(result.source.maxRows, false)];
  }
  if (result.status === "error") {
    return [context, "failed"];
  }
  if (result.columns.length === 0 && result.rowsAffected != null) {
    return [
      context,
      `${result.rowsAffected.toLocaleString("en-US")} affected`,
      `${result.durationMs} ms`,
    ];
  }
  return [
    context,
    rowCountLabel(result.rowCount, result.truncated),
    maxRowsLabel(result.source.maxRows, result.truncated),
    `${result.durationMs} ms`,
  ];
}

export function ResultsBody({
  activeTab,
  result,
  exportViewRef,
}: {
  activeTab: ReturnType<typeof useTabs.getState>["tabs"][number] | null;
  result: TabResult | null;
  exportViewRef: ExportViewRef;
}) {
  if (!activeTab) {
    return (
      <EmptyPanel
        title="No active tab"
        detail="Open a table from the sidebar to load rows, or open a SQL editor with + in the tab bar and run a query to populate this panel."
      />
    );
  }

  if (!result) {
    if (activeTab.kind === "query") {
      return (
        <EmptyPanel
          title="Run a query to see results"
          detail={`${activeTab.title} has not produced a result set yet.`}
        />
      );
    }
    if (activeTab.kind === "schema-compare") {
      return (
        <EmptyPanel
          title="Schema comparison"
          detail={`${activeTab.title} is a schema-compare tab. Its diff and generated migration are shown in the main pane above.`}
        />
      );
    }
    if (activeTab.kind === "er-diagram") {
      return (
        <EmptyPanel
          title="ER diagram"
          detail={`${activeTab.title} renders the foreign-key graph above. The Results grid is reserved for SQL query output.`}
        />
      );
    }
    return (
      <EmptyPanel
        title="Table rows are already shown"
        detail={`${tableLabel(activeTab)} is a table-browsing tab. The Results grid is reserved for SQL query output — open a query tab and run a statement to use it.`}
      />
    );
  }

  if (result.source.kind === "table") {
    return (
      <EmptyPanel
        title="Table rows are already shown"
        detail="This tab's table data lives in the main grid. The bottom Results grid is reserved for SQL query output."
      />
    );
  }

  if (result.status === "loading") {
    return <EmptyPanel busy title="Loading rows" />;
  }

  if (result.status === "error") {
    return (
      <EmptyPanel
        title="Could not load results"
        detail={result.message}
        tone="warn"
      />
    );
  }

  if (result.columns.length === 0) {
    return (
      <EmptyPanel
        title={
          result.rowsAffected != null
            ? `Query OK — ${result.rowsAffected.toLocaleString("en-US")} ${
                result.rowsAffected === 1 ? "row" : "rows"
              } affected`
            : "Statement returned no columns"
        }
        detail={
          result.rowsAffected != null
            ? `Completed in ${result.durationMs} ms. The statement did not return a result set.`
            : "The statement completed without producing a result set."
        }
      />
    );
  }

  return (
    <ReadOnlyResultGrid
      key={result.tabId}
      result={result}
      exportViewRef={exportViewRef}
    />
  );
}

function ReadOnlyResultGrid({
  result,
  exportViewRef,
}: {
  result: Extract<TabResult, { status: "ready" }>;
  exportViewRef: ExportViewRef;
}) {
  const grid = useGridState();
  const { settings } = useSettings();
  const onLoadMore = result.onLoadMore ?? null;
  const [copyMenu, setCopyMenu] = useState<ContextMenuState | null>(null);

  // The rows in the exact order the grid shows them (local filter + sort). Row
  // selection indexes into this list, so it must match the grid's own order.
  const visibleRows = useMemo(
    () =>
      sortGridRows(
        filterRows(result.rows, result.columns, grid.filters, grid.changes),
        result.columns,
        grid.sort,
        grid.changes,
      ),
    [result.rows, result.columns, grid.filters, grid.sort, grid.changes],
  );

  useEffect(() => {
    exportViewRef.current = () => ({
      columns: result.columns,
      rows: visibleRows,
    });
    return () => {
      exportViewRef.current = null;
    };
  }, [exportViewRef, result.columns, visibleRows]);

  const containerRef = useRef<HTMLDivElement | null>(null);

  const copy = (text: string) => {
    if (!navigator.clipboard) return;
    void navigator.clipboard.writeText(text);
  };

  // ⌘/Ctrl+C copies the selected row as TSV (pastes cleanly into spreadsheets),
  // or the selected single cell's value. Scoped to the grid container's focus so
  // it never hijacks copy from the SQL editor or another pane, and leaves native
  // text selections (and copies from inline editors) alone.
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
    if (grid.selectedRow !== null) {
      const row = visibleRows[grid.selectedRow];
      if (!row) return;
      event.preventDefault();
      copy(toTsv(result.columns, [row], { header: false }).trimEnd());
    } else if (grid.selection) {
      const column = result.columns[grid.selection.col];
      const row = visibleRows[grid.selection.row];
      if (!column || !row) return;
      const cell = row[column.key];
      event.preventDefault();
      copy(cell == null ? "" : String(cell));
    }
  };

  return (
    <div
      ref={containerRef}
      tabIndex={-1}
      onKeyDown={handleCopyKey}
      className="flex h-full min-h-0 flex-col overflow-hidden outline-none"
    >
      <div className="min-h-0 flex-1 overflow-hidden">
        <DataGrid
          columns={result.columns}
          rows={result.rows}
          totalRows={result.truncated ? undefined : result.rows.length}
          changes={grid.changes}
          onChange={grid.setChanges}
          selection={grid.selection}
          onSelect={(next) => {
            grid.setSelection(next);
            // Pull keyboard focus into the grid so ⌘C targets this pane.
            if (next) containerRef.current?.focus({ preventScroll: true });
          }}
          editing={grid.editing}
          onEdit={grid.setEditing}
          filters={grid.filters}
          onFiltersChange={grid.setFilters}
          sort={grid.sort}
          onSortChange={grid.setSort}
          renderEditor={renderGridEditor}
          renderFilterValue={renderFilterValue}
          readOnly
          nullDisplay={settings.grid.nullDisplay}
          stripeRows={settings.grid.stripeRows}
          selectedRow={grid.selectedRow}
          onRowSelect={(rowIndex) => {
            grid.setSelectedRow(rowIndex);
            if (rowIndex !== null) {
              containerRef.current?.focus({ preventScroll: true });
            }
          }}
          onRowContextMenu={(event, row) => {
            event.preventDefault();
            setCopyMenu({
              x: event.clientX,
              y: event.clientY,
              items: [
                {
                  label: "Copy row as CSV",
                  onClick: () =>
                    copy(
                      toCsv(result.columns, [row], { header: false }).trimEnd(),
                    ),
                },
                {
                  label: "Copy row as TSV",
                  onClick: () =>
                    copy(
                      toTsv(result.columns, [row], { header: false }).trimEnd(),
                    ),
                },
                {
                  label: "Copy row as JSON",
                  onClick: () => copy(toJson(result.columns, [row]).trimEnd()),
                },
                {
                  label: "Copy row as SQL INSERT",
                  onClick: () =>
                    copy(toSqlInserts(result.columns, [row]).trimEnd()),
                },
                {
                  label: "Copy all rows as CSV",
                  onClick: () => copy(toCsv(result.columns, visibleRows).trimEnd()),
                },
                {
                  label: "Copy all rows as JSON",
                  onClick: () =>
                    copy(toJson(result.columns, visibleRows).trimEnd()),
                },
                {
                  label: "Copy all rows as SQL INSERT",
                  onClick: () =>
                    copy(toSqlInserts(result.columns, visibleRows).trimEnd()),
                },
              ],
            });
          }}
          onCellContextMenu={(event, row, column) => {
            event.preventDefault();
            const cell = row[column.key];
            setCopyMenu({
              x: event.clientX,
              y: event.clientY,
              items: [
                {
                  label: "Copy cell",
                  onClick: () => copy(cell == null ? "" : String(cell)),
                },
                {
                  label: "Copy row as CSV",
                  onClick: () =>
                    copy(
                      toCsv(result.columns, [row], { header: false }).trimEnd(),
                    ),
                },
                {
                  label: "Copy row as TSV",
                  onClick: () =>
                    copy(
                      toTsv(result.columns, [row], { header: false }).trimEnd(),
                    ),
                },
                {
                  label: "Copy row as JSON",
                  onClick: () => copy(toJson(result.columns, [row]).trimEnd()),
                },
                {
                  label: "Copy row as SQL INSERT",
                  onClick: () =>
                    copy(toSqlInserts(result.columns, [row]).trimEnd()),
                },
                {
                  label: "Copy all rows as CSV",
                  onClick: () => copy(toCsv(result.columns, visibleRows).trimEnd()),
                },
                {
                  label: "Copy all rows as JSON",
                  onClick: () =>
                    copy(toJson(result.columns, visibleRows).trimEnd()),
                },
                {
                  label: "Copy all rows as SQL INSERT",
                  onClick: () =>
                    copy(toSqlInserts(result.columns, visibleRows).trimEnd()),
                },
              ],
            });
          }}
        />
      </div>
      <ContextMenu state={copyMenu} onClose={() => setCopyMenu(null)} />
      {result.truncated && (
        <div className="flex shrink-0 items-center justify-between border-t border-border-default bg-bg-1 px-3 py-1.5 text-[11px] text-fg-3">
          <span>
            First {result.rowCount.toLocaleString("en-US")} rows shown
            {" — "}query result was truncated at the row cap.
          </span>
          {onLoadMore && (
            <button
              type="button"
              className="rounded px-2 py-0.5 text-[11px] text-accent hover:bg-accent-soft"
              onClick={onLoadMore}
            >
              Load more
            </button>
          )}
        </div>
      )}
    </div>
  );
}

function EmptyPanel({
  title,
  detail,
  tone = "muted",
  busy = false,
}: {
  title: string;
  detail?: string;
  tone?: "muted" | "warn";
  busy?: boolean;
}) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-2 bg-bg-inset p-6 text-center text-[11.5px] text-fg-3">
      {busy && (
        <span className="mb-0.5 inline-flex text-accent animate-spinner" aria-hidden>
          <Icon.spinner size={20} sw={1.6} />
        </span>
      )}
      <div
        className={
          "text-[12px] font-medium " +
          (tone === "warn" ? "text-warn" : "text-fg-1")
        }
      >
        {title}
      </div>
      {detail ? (
        <div className="max-w-[460px] line-clamp-4 text-[10.5px] leading-[1.5] text-fg-3">
          {detail}
        </div>
      ) : null}
    </div>
  );
}

function tableLabel(tab: TableTab): string {
  return `${tab.database}.${tab.schema}.${tab.table}`;
}

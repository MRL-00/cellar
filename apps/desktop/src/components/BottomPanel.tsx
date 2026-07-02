import {
  commands,
  unwrap,
  type DatabaseNotice,
  type NoticeSeverity,
  type QueryHistoryRecord,
} from "@cellar/ipc";
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
  type ReactNode,
} from "react";
import { useSettings } from "../lib/settings";
import {
  downloadText,
  exportFilename,
  exportText,
  EXPORT_FORMATS,
  toCsv,
  toJson,
  toSqlInserts,
  toTsv,
} from "../lib/export";
import { ContextMenu, type ContextMenuState } from "./ContextMenu";

import {
  countNoticeSeverities,
  formatNoticeTime,
  toneForSeverity,
  type NoticeTone,
} from "../lib/notices";
import { useConnections } from "../state/connections";
import {
  EMPTY_ENTRY,
  noticeScopeKey,
  useNotices,
  type NoticeLogEntry,
  type NoticeScope,
} from "../state/notices";
import { useBottomPanel, type BottomTabId } from "../state/bottomPanel";
import { useStatus } from "../state/status";
import { useTabs, tabLabel, type TableTab } from "../state/tabs";
import {
  maxRowsLabel,
  resultContextLabel,
  rowCountLabel,
  useTabResults,
  type TabResult,
} from "../state/tabResults";
import { formatDuration } from "../lib/queryMessages";
import { useQueryMessages } from "../state/queryMessages";
import { PlanPanel } from "./BottomPlanPanel";
import { MessagesView } from "./BottomMessagesPanel";
import { FindUsagesPanel } from "./FindUsagesPanel";
import { useFindUsages } from "../state/findUsages";
import { Icon } from "./icons";

type BPTab = {
  id: BottomTabId;
  label: string;
  count: number | null;
  icon: ReactNode;
  enabled: boolean;
};

const BASE_TABS: Omit<BPTab, "count">[] = [
  { id: "results", label: "Results", icon: <Icon.table size={11} />, enabled: true },
  { id: "messages", label: "Messages", icon: <Icon.info size={11} />, enabled: true },
  { id: "plan", label: "Plan", icon: <Icon.tree size={11} />, enabled: true },
  { id: "history", label: "History", icon: <Icon.history size={11} />, enabled: true },
  { id: "notices", label: "Notices", icon: <Icon.warn size={11} />, enabled: true },
  { id: "findUsages", label: "Find Usages", icon: <Icon.search size={11} />, enabled: true },
];

/**
 * The rows/columns as the user currently sees them — after the result grid's
 * local filters and sort. Registered by ReadOnlyResultGrid so the Export
 * button (which lives outside the grid) exports what is on screen.
 */
type ExportView = { columns: readonly GridColumn[]; rows: readonly GridRow[] };
type ExportViewRef = MutableRefObject<(() => ExportView) | null>;

export function BottomPanel({ onClose }: { onClose: () => void }) {
  const active = useBottomPanel((s) => s.active);
  const setActive = useBottomPanel((s) => s.setActive);
  const [historyCount, setHistoryCount] = useState<number | null>(null);
  const activeTabId = useTabs((s) => s.activeId);
  const tabs = useTabs((s) => s.tabs);
  const connections = useConnections((s) => s.connections);
  const messages = useQueryMessages((s) => s.messages);
  const activeTab = tabs.find((t) => t.id === activeTabId) ?? null;
  const activeMessages = useMemo(
    () => messages.filter((m) => m.tabId === activeTabId),
    [activeTabId, messages],
  );
  const usageCount = useFindUsages((s) =>
    s.status === "ready" ? s.results.length : null,
  );
  const result = useTabResults((s) =>
    activeTabId ? s.byTabId[activeTabId] ?? null : null,
  );
  const resultCount =
    result?.status === "ready" && result.source.kind === "query"
      ? result.rowCount
      : null;
  const activeConnection = activeTab
    ? connections.find((c) => c.id === activeTab.connectionId) ?? null
    : null;
  const noticeScope = useMemo<NoticeScope>(
    () => ({
      tabId: activeTab?.id ?? null,
      connectionId: activeTab?.connectionId ?? null,
      database: activeTab?.database ?? null,
    }),
    [activeTab],
  );
  const scopeKey = noticeScopeKey(noticeScope);
  const noticeEntry =
    useNotices((s) => s.byScope[scopeKey]) ?? structuredClone(EMPTY_ENTRY);
  const clearNotices = useNotices((s) => s.clear);
  const setRetain = useNotices((s) => s.setRetain);
  const exportViewRef: ExportViewRef = useRef(null);
  const [exportMenu, setExportMenu] = useState<ContextMenuState | null>(null);
  const exportable =
    result?.status === "ready" &&
    result.source.kind === "query" &&
    result.columns.length > 0
      ? result
      : null;
  const bottomTabs: BPTab[] = BASE_TABS.map((tab) => ({
    ...tab,
    count:
      tab.id === "notices"
        ? noticeEntry.notices.length
        : tab.id === "messages"
          ? activeMessages.length || null
        : tab.id === "history"
          ? historyCount
          : tab.id === "results"
            ? resultCount
            : tab.id === "findUsages"
              ? usageCount
              : null,
  }));

  return (
    <div className="flex h-full flex-col bg-bg-1">
      <div className="flex shrink-0 items-stretch justify-between h-7 pl-1.5 pr-1 border-b border-border-default">
        <div className="flex flex-1 min-w-0 items-center gap-0.5">
          {bottomTabs.map((t) => {
            const isActive = active === t.id;
            return (
              <button
                key={t.id}
                onClick={() => {
                  if (t.enabled) setActive(t.id);
                }}
                disabled={!t.enabled}
                title={t.enabled ? t.label : `${t.label} is not wired yet`}
                className={
                  "mt-[3px] inline-flex h-[22px] items-center gap-1.5 rounded-[4px] px-2 text-sm disabled:cursor-default disabled:opacity-45 " +
                  (isActive
                    ? "bg-bg-3 text-fg-0"
                    : "text-fg-2 hover:bg-bg-2 hover:text-fg-0 disabled:hover:bg-transparent disabled:hover:text-fg-2")
                }
              >
                <span className={"inline-flex " + (isActive ? "text-accent" : "text-fg-3")}>
                  {t.icon}
                </span>
                <span>{t.label}</span>
                {t.count != null && (
                  <span
                    className={
                      "rounded-[8px] px-1 py-px font-mono text-[10.5px] " +
                      (isActive ? "bg-bg-1 text-fg-0" : "bg-bg-2 text-fg-2")
                    }
                  >
                    {t.count}
                  </span>
                )}
              </button>
            );
          })}
          <div className="mx-1.5 h-[18px] w-px self-center bg-border-divider" />
          <HeaderMeta activeTab={activeTab} result={result} />
        </div>
        <div className="flex items-center gap-px">
          <button
            className={"icon-btn" + (exportable ? "" : " opacity-45")}
            disabled={!exportable}
            title={
              exportable
                ? "Export results…"
                : "Export — run a query that returns rows first"
            }
            onClick={(e) => {
              if (!exportable) return;
              const rect = e.currentTarget.getBoundingClientRect();
              const label =
                activeTab?.kind === "query" ? activeTab.title : "result";
              setExportMenu({
                x: rect.right,
                y: rect.bottom + 4,
                items: EXPORT_FORMATS.map(({ format, label: name }) => ({
                  label: `Export as ${name}`,
                  onClick: () => {
                    const view = exportViewRef.current?.() ?? {
                      columns: exportable.columns,
                      rows: exportable.rows,
                    };
                    downloadText(
                      exportFilename(label, format),
                      format,
                      exportText(format, view.columns, view.rows),
                    );
                  },
                })),
              });
            }}
          >
            <Icon.fileText size={11} />
          </button>
          <button className="icon-btn opacity-45" disabled title="Pop out not implemented yet">
            <Icon.expand size={11} />
          </button>
          <button className="icon-btn" onClick={onClose} title="Hide">
            <Icon.chevronsDown size={12} />
          </button>
        </div>
      </div>

      <ContextMenu state={exportMenu} onClose={() => setExportMenu(null)} />

      <div className="min-h-0 flex-1 overflow-hidden">
        {active === "results" ? (
          <ResultsBody
            activeTab={activeTab}
            result={result}
            exportViewRef={exportViewRef}
          />
        ) : active === "messages" ? (
          <MessagesView
            messages={activeMessages}
            hasActiveTab={activeTabId != null}
          />
        ) : active === "history" ? (
          <HistoryPanel activeTab={activeTab} onCountChange={setHistoryCount} />
        ) : active === "notices" ? (
          <NoticesPanel
            activeConnectionName={activeConnection?.name ?? null}
            activeEngine={activeConnection?.engine ?? null}
            activeTabLabel={
              activeTab ? `${activeTab.database}.${tabLabel(activeTab)}` : null
            }
            entry={noticeEntry}
            onClear={() => clearNotices(noticeScope)}
            onRetainChange={(retain) => setRetain(noticeScope, retain)}
          />
        ) : active === "plan" ? (
          <PlanPanel activeTab={activeTab} />
        ) : active === "findUsages" ? (
          <FindUsagesPanel />
        ) : (
          <Placeholder tab={active} />
        )}
      </div>
    </div>
  );
}

function HeaderMeta({
  activeTab,
  result,
}: {
  activeTab: ReturnType<typeof useTabs.getState>["tabs"][number] | null;
  result: TabResult | null;
}) {
  const items = headerItems(activeTab, result);

  return (
    <div className="inline-flex min-w-0 items-center gap-1.5 overflow-hidden font-mono text-[11.5px]">
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
  if (!result) return [tableLabel(activeTab)];

  const context = resultContextLabel(result.source);
  if (result.source.kind === "table") {
    return [context];
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

function ResultsBody({
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
    return (
      <EmptyPanel
        title="Loading rows"
        detail={`Running ${result.source.statement}`}
      />
    );
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
        <div className="flex shrink-0 items-center justify-between border-t border-border-default bg-bg-1 px-3 py-1.5 text-sm text-fg-3">
          <span>
            First {result.rowCount.toLocaleString("en-US")} rows shown
            {" — "}query result was truncated at the row cap.
          </span>
          {onLoadMore && (
            <button
              type="button"
              className="rounded px-2 py-0.5 text-sm text-accent hover:bg-accent-soft"
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
}: {
  title: string;
  detail: string;
  tone?: "muted" | "warn";
}) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-1.5 bg-bg-inset p-6 text-center text-sm text-fg-3">
      <div
        className={
          "text-sm font-medium " +
          (tone === "warn" ? "text-warn" : "text-fg-1")
        }
      >
        {title}
      </div>
      <div className="max-w-[460px] text-[11.5px] leading-[1.5] text-fg-3">
        {detail}
      </div>
    </div>
  );
}

function tableLabel(tab: TableTab): string {
  return `${tab.database}.${tab.schema}.${tab.table}`;
}

function HistoryPanel({
  activeTab,
  onCountChange,
}: {
  activeTab: ReturnType<typeof useTabs.getState>["tabs"][number] | null;
  onCountChange: (count: number | null) => void;
}) {
  const [search, setSearch] = useState("");
  const [records, setRecords] = useState<QueryHistoryRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copiedId, setCopiedId] = useState<number | null>(null);
  const lastQuery = useStatus((s) => s.lastQuery);
  const newQueryTab = useTabs((s) => s.newQueryTab);
  const setQuerySql = useTabs((s) => s.setQuerySql);
  const trimmedSearch = search.trim();
  const refreshKey =
    activeTab &&
    lastQuery &&
    (lastQuery.tabId === activeTab.id ||
      (!lastQuery.tabId && lastQuery.connectionId === activeTab.connectionId))
      ? lastQuery.completedAtMs
      : 0;

  useEffect(() => {
    if (!activeTab) {
      setRecords([]);
      onCountChange(null);
      return;
    }

    let cancelled = false;
    const timer = window.setTimeout(() => {
      setLoading(true);
      setError(null);
      void unwrap(
        commands.listQueryHistory(
          activeTab.connectionId,
          activeTab.database,
          activeTab.id,
          trimmedSearch || null,
          100,
        ),
      )
        .then((rows) => {
          if (cancelled) return;
          setRecords(rows);
          onCountChange(rows.length);
        })
        .catch((err) => {
          if (cancelled) return;
          setRecords([]);
          onCountChange(0);
          setError(err instanceof Error ? err.message : String(err));
        })
        .finally(() => {
          if (!cancelled) setLoading(false);
        });
    }, 150);

    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [activeTab, onCountChange, refreshKey, trimmedSearch]);

  const scopeLabel = useMemo(() => {
    if (!activeTab) return "No active tab";
    return `${activeTab.database}.${tabLabel(activeTab)}`;
  }, [activeTab]);

  return (
    <div className="flex h-full min-h-0 flex-col bg-bg-inset">
      <div className="flex shrink-0 items-center gap-2 border-b border-border-divider px-2 py-1.5">
        <div className="relative min-w-[180px] max-w-[360px] flex-1">
          <span className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-fg-3">
            <Icon.search size={11} />
          </span>
          <input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search SQL or errors"
            className="h-6 w-full rounded-[4px] border border-border-default bg-bg-1 pl-6 pr-2 font-mono text-sm text-fg-1 outline-none placeholder:text-fg-4 focus:border-accent-line"
          />
        </div>
        <div className="min-w-0 truncate font-mono text-[11.5px] text-fg-3">
          {scopeLabel}
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-auto">
        {!activeTab ? (
          <EmptyHistory title="Open a tab to view scoped history" />
        ) : error ? (
          <EmptyHistory title="History unavailable" detail={error} />
        ) : loading && records.length === 0 ? (
          <EmptyHistory title="Loading history..." />
        ) : records.length === 0 ? (
          <EmptyHistory
            title={trimmedSearch ? "No matching queries" : "No queries recorded for this tab"}
            detail="History starts filling as queries execute locally."
          />
        ) : (
          <div className="divide-y divide-border-divider">
            {records.map((record) => (
              <HistoryRow
                key={record.id}
                record={record}
                copied={copiedId === record.id}
                onCopy={() => {
                  if (!navigator.clipboard) return;
                  void navigator.clipboard.writeText(record.sql).then(() => {
                    setCopiedId(record.id);
                    window.setTimeout(() => setCopiedId(null), 1100);
                  });
                }}
                onReuse={() => {
                  const id = newQueryTab(
                    record.connection_id,
                    record.database ?? "",
                  );
                  setQuerySql(id, record.sql);
                }}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function HistoryRow({
  record,
  copied,
  onCopy,
  onReuse,
}: {
  record: QueryHistoryRecord;
  copied: boolean;
  onCopy: () => void;
  onReuse: () => void;
}) {
  return (
    <div className="group grid grid-cols-[minmax(0,1fr)_auto] gap-3 px-2.5 py-2 hover:bg-bg-1">
      <div className="min-w-0">
        <div className="mb-1 flex min-w-0 items-center gap-1.5 text-[11.5px]">
          <span
            className={
              "rounded-[3px] px-1 py-px font-mono " +
              (record.success
                ? "bg-insert-bg text-insert"
                : "bg-delete-bg text-delete")
            }
          >
            {record.success ? "ok" : "error"}
          </span>
          <span className="font-mono text-fg-2">{formatDuration(record.duration_ms)}</span>
          <span className="text-fg-4">·</span>
          <span className="font-mono text-fg-2">{formatRows(record)}</span>
          {record.truncated && <span className="font-mono text-warn">truncated</span>}
          <span className="text-fg-4">·</span>
          <span className="truncate font-mono text-fg-3">{formatTimestamp(record.executed_at_ms)}</span>
        </div>
        <pre className="m-0 max-h-[58px] overflow-hidden whitespace-pre-wrap break-words font-mono text-sm leading-[1.45] text-fg-1">
          {record.sql}
        </pre>
        {record.error_summary && (
          <div className="mt-1 truncate font-mono text-[11.5px] text-delete">
            {record.error_summary}
          </div>
        )}
      </div>
      <div className="flex items-start gap-1 opacity-100 sm:opacity-0 sm:group-hover:opacity-100">
        <button className="icon-btn" onClick={onCopy} title="Copy SQL">
          {copied ? <Icon.check size={11} /> : <Icon.copy size={11} />}
        </button>
        <button className="icon-btn" onClick={onReuse} title="Open in a new query tab">
          <Icon.edit size={11} />
        </button>
      </div>
    </div>
  );
}

function EmptyHistory({ title, detail }: { title: string; detail?: string }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-1.5 p-6 text-center text-[12.5px] text-fg-3">
      <div className="text-sm font-medium text-fg-1">{title}</div>
      {detail && <div className="max-w-[360px] text-[11.5px] leading-[1.5]">{detail}</div>}
    </div>
  );
}

function Placeholder({ tab }: { tab: BottomTabId }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-1.5 p-6 text-center text-[12.5px] text-fg-3">
      <div className="text-sm font-medium text-fg-1">{labelFor(tab)}</div>
      <div className="max-w-[320px] text-[11.5px] leading-[1.5] text-fg-3">
        {subFor(tab)}
      </div>
    </div>
  );
}

function NoticesPanel({
  activeConnectionName,
  activeEngine,
  activeTabLabel,
  entry,
  onClear,
  onRetainChange,
}: {
  activeConnectionName: string | null;
  activeEngine: string | null;
  activeTabLabel: string | null;
  entry: NoticeLogEntry;
  onClear: () => void;
  onRetainChange: (retain: boolean) => void;
}) {
  const notices = entry.notices;
  const counts = countNoticeSeverities(notices);
  const visibleCounts = (Object.entries(counts) as [NoticeSeverity, number][])
    .filter(([, count]) => count > 0)
    .slice(0, 4);
  const unsupported = entry.capture?.supported === false;

  return (
    <div className="flex h-full min-h-0 flex-col text-[12px]">
      <div className="flex h-8 shrink-0 items-center justify-between gap-3 border-b border-border-divider px-2">
        <div className="flex min-w-0 items-center gap-2">
          <span className="font-medium text-fg-1">Database notices</span>
          <span className="min-w-0 truncate font-mono text-[11.5px] text-fg-3">
            {activeConnectionName && activeTabLabel
              ? `${activeConnectionName} / ${activeTabLabel}`
              : "no active query tab"}
          </span>
          {activeEngine && (
            <span className="rounded-[4px] border border-border-default px-1.5 py-px font-mono text-[10.5px] text-fg-3">
              {activeEngine}
            </span>
          )}
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <div className="hidden items-center gap-1 sm:flex">
            {visibleCounts.length === 0 ? (
              <span className="font-mono text-[11px] text-fg-3">0</span>
            ) : (
              visibleCounts.map(([severity, count]) => (
                <span
                  key={severity}
                  className={
                    "rounded-[4px] px-1.5 py-px font-mono text-[10.5px] " +
                    severityPillClass(toneForSeverity(severity))
                  }
                >
                  {severity}:{count}
                </span>
              ))
            )}
          </div>
          <label className="inline-flex items-center gap-1.5 text-[11.5px] text-fg-2">
            <input
              type="checkbox"
              checked={entry.retain}
              onChange={(e) => onRetainChange(e.target.checked)}
              className="h-3 w-3 accent-[var(--accent)]"
            />
            <span>Retain</span>
          </label>
          <button
            className="h-5 rounded-[4px] border border-border-default px-2 text-sm text-fg-2 hover:bg-bg-2 hover:text-fg-0 disabled:opacity-45"
            disabled={notices.length === 0}
            onClick={onClear}
          >
            Clear
          </button>
        </div>
      </div>
      <div className="min-h-0 flex-1 overflow-auto">
        {!activeTabLabel ? (
          <NoticeState
            title="No active query tab"
            body="Open a table or query tab to collect database-emitted notices for that scope."
          />
        ) : unsupported ? (
          <NoticeState
            title="Notice capture unavailable"
            body={
              entry.capture?.reason ??
              "The current driver path cannot observe database notice frames."
            }
          />
        ) : notices.length === 0 ? (
          <NoticeState
            title="No database notices"
            body="This scope has not received Postgres NOTICE/RAISE output or engine-equivalent messages."
          />
        ) : (
          <NoticeRows notices={notices} />
        )}
      </div>
    </div>
  );
}

function NoticeState({ title, body }: { title: string; body: string }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-1.5 p-6 text-center">
      <div className="text-sm font-medium text-fg-1">{title}</div>
      <div className="max-w-[520px] text-[11.5px] leading-[1.5] text-fg-3">
        {body}
      </div>
    </div>
  );
}

function NoticeRows({ notices }: { notices: DatabaseNotice[] }) {
  return (
    <div className="min-w-[720px]">
      {notices.map((notice, index) => (
        <div
          key={`${notice.timestamp}:${index}`}
          className="grid grid-cols-[76px_84px_82px_minmax(0,1fr)] items-start border-b border-border-divider px-2 py-1.5 font-mono text-sm leading-[1.45]"
        >
          <span className="text-fg-3">{formatNoticeTime(notice.timestamp)}</span>
          <span>
            <span
              className={
                "rounded-[4px] px-1.5 py-px text-[10.5px] " +
                severityPillClass(toneForSeverity(notice.severity))
              }
            >
              {notice.severity}
            </span>
          </span>
          <span className="text-fg-3">{notice.code ?? ""}</span>
          <span className="min-w-0 text-fg-1">
            {notice.message}
            {notice.detail && (
              <span className="ml-2 text-fg-3">detail: {notice.detail}</span>
            )}
            {notice.hint && (
              <span className="ml-2 text-fg-3">hint: {notice.hint}</span>
            )}
          </span>
        </div>
      ))}
    </div>
  );
}

function severityPillClass(tone: NoticeTone) {
  switch (tone) {
    case "danger":
      return "bg-delete-bg text-delete";
    case "warning":
      return "bg-update-bg text-warn";
    case "info":
      return "bg-accent-soft text-accent";
    case "muted":
      return "bg-bg-2 text-fg-2";
  }
}

function formatRows(record: QueryHistoryRecord) {
  if (record.row_count == null) return "no rows";
  return `${record.row_count.toLocaleString()} row${record.row_count === 1 ? "" : "s"}`;
}

function formatTimestamp(ms: number) {
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(ms));
}

function labelFor(id: BottomTabId) {
  switch (id) {
    case "results":
      return "Run a query to see results here";
    case "messages":
      return "No execution messages";
    case "plan":
      return "Execution plans appear here";
    case "history":
      return "Query history is empty";
    case "notices":
      return "No notices yet";
    default:
      return "";
  }
}

function subFor(id: BottomTabId) {
  switch (id) {
    case "results":
      return "Open a query tab, ⌘⏎ to run the statement under the cursor.";
    case "messages":
      return "Execution status, row-limit warnings, and failures appear here.";
    case "plan":
      return "EXPLAIN ANALYZE renders as a tree with a cost heatmap.";
    case "history":
      return "Every executed query is logged with duration, row count, and outcome.";
    case "notices":
      return "Database-emitted output such as Postgres NOTICE/RAISE and SQL Server PRINT lands here when the driver can capture it.";
    default:
      return "";
  }
}

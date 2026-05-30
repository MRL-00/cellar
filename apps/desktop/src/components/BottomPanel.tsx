import {
  commands,
  unwrap,
  type DatabaseNotice,
  type NoticeSeverity,
  type QueryHistoryRecord,
} from "@cellar/ipc";
import { DataGrid, useGridState } from "@cellar/data-grid";
import { useEffect, useMemo, useState, type ReactNode } from "react";

import {
  countNoticeSeverities,
  formatNoticeTime,
  toneForSeverity,
  type NoticeTone,
} from "../lib/notices";
import { useConnections } from "../state/connections";
import {
  emptyNoticeEntry,
  noticeScopeKey,
  useNotices,
  type NoticeLogEntry,
  type NoticeScope,
} from "../state/notices";
import { useStatus } from "../state/status";
import { useTabs, type TableTab } from "../state/tabs";
import {
  maxRowsLabel,
  resultContextLabel,
  rowCountLabel,
  useTabResults,
  type TabResult,
} from "../state/tabResults";
import { useQueryMessages } from "../state/queryMessages";
import { MessagesView } from "./BottomMessagesPanel";
import { Icon } from "./icons";

type BottomTabId = "results" | "messages" | "plan" | "history" | "notices";

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
  { id: "plan", label: "Plan", icon: <Icon.tree size={11} />, enabled: false },
  { id: "history", label: "History", icon: <Icon.history size={11} />, enabled: true },
  { id: "notices", label: "Notices", icon: <Icon.warn size={11} />, enabled: true },
];

export function BottomPanel({ onClose }: { onClose: () => void }) {
  const [active, setActive] = useState<BottomTabId>("results");
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
    useNotices((s) => s.byScope[scopeKey]) ?? emptyNoticeEntry();
  const clearNotices = useNotices((s) => s.clear);
  const setRetain = useNotices((s) => s.setRetain);
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
                  "mt-[3px] inline-flex h-[22px] items-center gap-1.5 rounded-[4px] px-2 text-[11px] disabled:cursor-default disabled:opacity-45 " +
                  (isActive
                    ? "bg-accent-soft text-accent"
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
                      "rounded-[8px] px-1 py-px font-mono text-[9.5px] " +
                      (isActive ? "bg-bg-1 text-accent" : "bg-bg-2 text-fg-2")
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
          <button className="icon-btn opacity-45" disabled title="Export not implemented yet">
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

      <div className="min-h-0 flex-1 overflow-hidden">
        {active === "results" ? (
          <ResultsBody activeTab={activeTab} result={result} />
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
              activeTab
                ? `${activeTab.database}.${activeTab.schema}.${activeTab.table}`
                : null
            }
            entry={noticeEntry}
            onClear={() => clearNotices(noticeScope)}
            onRetainChange={(retain) => setRetain(noticeScope, retain)}
          />
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
  activeTab: TableTab | null;
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

function headerItems(activeTab: TableTab | null, result: TabResult | null): string[] {
  if (!activeTab) return ["no active tab"];
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
}: {
  activeTab: TableTab | null;
  result: TabResult | null;
}) {
  if (!activeTab) {
    return (
      <EmptyPanel
        title="No active tab"
        detail="Open a table from the sidebar to load rows. SQL query-tab results will use this panel when the editor slice lands."
      />
    );
  }

  if (!result) {
    return (
      <EmptyPanel
        title="Table rows are already shown"
        detail={`${tableLabel(activeTab)} is a table-browsing tab. The Results grid is reserved for SQL query output, so it will light up when SQL editor tabs land.`}
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
        title="Statement returned no columns"
        detail="Rows-affected messages are not surfaced in the bottom panel yet."
      />
    );
  }

  return <ReadOnlyResultGrid key={result.tabId} result={result} />;
}

function ReadOnlyResultGrid({
  result,
}: {
  result: Extract<TabResult, { status: "ready" }>;
}) {
  const grid = useGridState();

  return (
    <div className="flex h-full min-h-0 overflow-hidden">
      <DataGrid
        columns={result.columns}
        rows={result.rows}
        totalRows={result.truncated ? undefined : result.rows.length}
        changes={grid.changes}
        onChange={grid.setChanges}
        selection={grid.selection}
        onSelect={grid.setSelection}
        editing={grid.editing}
        onEdit={grid.setEditing}
        filters={grid.filters}
        onFiltersChange={grid.setFilters}
        readOnly
      />
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
    <div className="flex h-full flex-col items-center justify-center gap-1.5 bg-bg-inset p-6 text-center text-[11.5px] text-fg-3">
      <div
        className={
          "text-[12px] font-medium " +
          (tone === "warn" ? "text-warn" : "text-fg-1")
        }
      >
        {title}
      </div>
      <div className="max-w-[460px] text-[10.5px] leading-[1.5] text-fg-3">
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
  activeTab: TableTab | null;
  onCountChange: (count: number | null) => void;
}) {
  const [search, setSearch] = useState("");
  const [records, setRecords] = useState<QueryHistoryRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copiedId, setCopiedId] = useState<number | null>(null);
  const lastQuery = useStatus((s) => s.lastQuery);
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
    return `${activeTab.database}.${activeTab.schema}.${activeTab.table}`;
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
            className="h-6 w-full rounded-[4px] border border-border-default bg-bg-1 pl-6 pr-2 font-mono text-[11px] text-fg-1 outline-none placeholder:text-fg-4 focus:border-accent-line"
          />
        </div>
        <div className="min-w-0 truncate font-mono text-[10.5px] text-fg-3">
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
}: {
  record: QueryHistoryRecord;
  copied: boolean;
  onCopy: () => void;
}) {
  return (
    <div className="group grid grid-cols-[minmax(0,1fr)_auto] gap-3 px-2.5 py-2 hover:bg-bg-1">
      <div className="min-w-0">
        <div className="mb-1 flex min-w-0 items-center gap-1.5 text-[10.5px]">
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
        <pre className="m-0 max-h-[58px] overflow-hidden whitespace-pre-wrap break-words font-mono text-[11px] leading-[1.45] text-fg-1">
          {record.sql}
        </pre>
        {record.error_summary && (
          <div className="mt-1 truncate font-mono text-[10.5px] text-delete">
            {record.error_summary}
          </div>
        )}
      </div>
      <div className="flex items-start gap-1 opacity-100 sm:opacity-0 sm:group-hover:opacity-100">
        <button className="icon-btn" onClick={onCopy} title="Copy SQL">
          {copied ? <Icon.check size={11} /> : <Icon.copy size={11} />}
        </button>
        <button className="icon-btn opacity-45" disabled title="Reuse waits for the SQL editor">
          <Icon.edit size={11} />
        </button>
      </div>
    </div>
  );
}

function EmptyHistory({ title, detail }: { title: string; detail?: string }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-1.5 p-6 text-center text-[11.5px] text-fg-3">
      <div className="text-[12px] font-medium text-fg-1">{title}</div>
      {detail && <div className="max-w-[360px] text-[10.5px] leading-[1.5]">{detail}</div>}
    </div>
  );
}

function Placeholder({ tab }: { tab: BottomTabId }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-1.5 p-6 text-center text-[11.5px] text-fg-3">
      <div className="text-[12px] font-medium text-fg-1">{labelFor(tab)}</div>
      <div className="max-w-[320px] text-[10.5px] leading-[1.5] text-fg-3">
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
    <div className="flex h-full min-h-0 flex-col text-[11px]">
      <div className="flex h-8 shrink-0 items-center justify-between gap-3 border-b border-border-divider px-2">
        <div className="flex min-w-0 items-center gap-2">
          <span className="font-medium text-fg-1">Database notices</span>
          <span className="min-w-0 truncate font-mono text-[10.5px] text-fg-3">
            {activeConnectionName && activeTabLabel
              ? `${activeConnectionName} / ${activeTabLabel}`
              : "no active query tab"}
          </span>
          {activeEngine && (
            <span className="rounded-[4px] border border-border-default px-1.5 py-px font-mono text-[9.5px] text-fg-3">
              {activeEngine}
            </span>
          )}
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <div className="hidden items-center gap-1 sm:flex">
            {visibleCounts.length === 0 ? (
              <span className="font-mono text-[10px] text-fg-3">0</span>
            ) : (
              visibleCounts.map(([severity, count]) => (
                <span
                  key={severity}
                  className={
                    "rounded-[4px] px-1.5 py-px font-mono text-[9.5px] " +
                    severityPillClass(toneForSeverity(severity))
                  }
                >
                  {severity}:{count}
                </span>
              ))
            )}
          </div>
          <label className="inline-flex items-center gap-1.5 text-[10.5px] text-fg-2">
            <input
              type="checkbox"
              checked={entry.retain}
              onChange={(e) => onRetainChange(e.target.checked)}
              className="h-3 w-3 accent-[var(--accent)]"
            />
            <span>Retain</span>
          </label>
          <button
            className="h-5 rounded-[4px] border border-border-default px-2 text-[10.5px] text-fg-2 hover:bg-bg-2 hover:text-fg-0 disabled:opacity-45"
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
      <div className="text-[12px] font-medium text-fg-1">{title}</div>
      <div className="max-w-[520px] text-[10.5px] leading-[1.5] text-fg-3">
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
          className="grid grid-cols-[76px_84px_82px_minmax(0,1fr)] items-start border-b border-border-divider px-2 py-1.5 font-mono text-[10.5px] leading-[1.45]"
        >
          <span className="text-fg-3">{formatNoticeTime(notice.timestamp)}</span>
          <span>
            <span
              className={
                "rounded-[4px] px-1.5 py-px text-[9.5px] " +
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

function formatDuration(ms: number) {
  if (ms < 1000) return `${ms} ms`;
  return `${(ms / 1000).toFixed(ms < 10_000 ? 2 : 1)} s`;
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

import { commands, unwrap, type QueryHistoryRecord } from "@cellar/ipc";
import { useEffect, useMemo, useState } from "react";
import { useStatus } from "../state/status";
import { useTabs, tabLabel } from "../state/tabs";
import { formatDuration } from "../lib/queryMessages";
import { Icon } from "./icons";

export function HistoryPanel({
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
        <button className="icon-btn" onClick={onReuse} title="Open in a new query tab">
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

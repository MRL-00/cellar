import type { DatabaseNotice, NoticeSeverity } from "@cellar/ipc";
import { useMemo, useState, type ReactNode } from "react";

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
import { useTabs } from "../state/tabs";
import { Icon } from "./icons";

type BottomTabId = "results" | "messages" | "plan" | "history" | "notices";

type BPTab = {
  id: BottomTabId;
  label: string;
  count: number | null;
  icon: ReactNode;
};

const BASE_TABS: Omit<BPTab, "count">[] = [
  { id: "results", label: "Results", icon: <Icon.table size={11} /> },
  { id: "messages", label: "Messages", icon: <Icon.info size={11} /> },
  { id: "plan", label: "Plan", icon: <Icon.tree size={11} /> },
  { id: "history", label: "History", icon: <Icon.history size={11} /> },
  { id: "notices", label: "Notices", icon: <Icon.warn size={11} /> },
];

export function BottomPanel({ onClose }: { onClose: () => void }) {
  const [active, setActive] = useState<BottomTabId>("results");
  const activeTabId = useTabs((s) => s.activeId);
  const tabs = useTabs((s) => s.tabs);
  const connections = useConnections((s) => s.connections);
  const activeTab = tabs.find((t) => t.id === activeTabId) ?? null;
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
    count: tab.id === "notices" ? noticeEntry.notices.length : null,
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
                onClick={() => setActive(t.id)}
                className={
                  "mt-[3px] inline-flex h-[22px] items-center gap-1.5 rounded-[4px] px-2 text-[11px] " +
                  (isActive
                    ? "bg-accent-soft text-accent"
                    : "text-fg-2 hover:bg-bg-2 hover:text-fg-0")
                }
              >
                <span
                  className={
                    "inline-flex " + (isActive ? "text-accent" : "text-fg-3")
                  }
                >
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
          <div className="inline-flex items-center gap-1.5 font-mono text-[10.5px]">
            <span className="text-fg-3">no query run yet</span>
          </div>
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
        {active === "notices" ? (
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

function labelFor(id: BottomTabId) {
  switch (id) {
    case "results":
      return "Run a query to see results here";
    case "messages":
      return "No messages yet";
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
      return "Query truncation, validation, and app-side execution status land here as this panel grows.";
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

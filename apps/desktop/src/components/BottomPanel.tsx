import { useMemo, useRef, useState, type ReactNode } from "react";
import {
  downloadText,
  exportFilename,
  exportText,
  EXPORT_FORMATS,
} from "../lib/export";
import { ContextMenu, type ContextMenuState } from "./ContextMenu";

import { useConnections } from "../state/connections";
import { EMPTY_ENTRY, noticeScopeKey, useNotices, type NoticeScope } from "../state/notices";
import { useBottomPanel, type BottomTabId } from "../state/bottomPanel";
import { useTabs, tabLabel } from "../state/tabs";
import { useTabResults } from "../state/tabResults";
import { useQueryMessages } from "../state/queryMessages";
import { PlanPanel } from "./BottomPlanPanel";
import { MessagesView } from "./BottomMessagesPanel";
import { FindUsagesPanel } from "./FindUsagesPanel";
import { useFindUsages } from "../state/findUsages";
import { Icon } from "./icons";
import {
  HeaderMeta,
  ResultsBody,
  type ExportViewRef,
} from "./BottomResultsPanel";
import { HistoryPanel } from "./BottomHistoryPanel";
import { NoticesPanel } from "./BottomNoticesPanel";

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

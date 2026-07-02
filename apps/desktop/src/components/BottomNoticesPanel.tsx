import type { DatabaseNotice, NoticeSeverity } from "@cellar/ipc";
import {
  countNoticeSeverities,
  formatNoticeTime,
  toneForSeverity,
  type NoticeTone,
} from "../lib/notices";
import type { NoticeLogEntry } from "../state/notices";

export function NoticesPanel({
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

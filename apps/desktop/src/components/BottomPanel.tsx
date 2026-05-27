import { useState, type ReactNode } from "react";
import { Icon } from "./icons";

type BPTab = {
  id: string;
  label: string;
  count: number | null;
  icon: ReactNode;
};

const TABS: BPTab[] = [
  { id: "results", label: "Results", count: null, icon: <Icon.table size={11} /> },
  { id: "messages", label: "Messages", count: null, icon: <Icon.info size={11} /> },
  { id: "plan", label: "Plan", count: null, icon: <Icon.tree size={11} /> },
  { id: "history", label: "History", count: null, icon: <Icon.history size={11} /> },
  { id: "notices", label: "Notices", count: null, icon: <Icon.warn size={11} /> },
];

export function BottomPanel({ onClose }: { onClose: () => void }) {
  const [active, setActive] = useState("results");
  return (
    <div className="flex h-full flex-col bg-bg-1">
      <div className="flex shrink-0 items-stretch justify-between h-7 pl-1.5 pr-1 border-b border-border-default">
        <div className="flex flex-1 min-w-0 items-center gap-0.5">
          {TABS.map((t) => {
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
          <button className="icon-btn" title="Export">
            <Icon.fileText size={11} />
          </button>
          <button className="icon-btn" title="Pop out">
            <Icon.expand size={11} />
          </button>
          <button className="icon-btn" onClick={onClose} title="Hide">
            <Icon.chevronsDown size={12} />
          </button>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-hidden">
        <div className="flex h-full flex-col items-center justify-center gap-1.5 p-6 text-center text-[11.5px] text-fg-3">
          <div className="text-[12px] font-medium text-fg-1">
            {labelFor(active)}
          </div>
          <div className="max-w-[320px] text-[10.5px] leading-[1.5] text-fg-3">
            {subFor(active)}
          </div>
        </div>
      </div>
    </div>
  );
}

function labelFor(id: string) {
  switch (id) {
    case "results":
      return "Run a query to see results here";
    case "messages":
      return "Server notices appear here";
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

function subFor(id: string) {
  switch (id) {
    case "results":
      return "Open a query tab, ⌘⏎ to run the statement under the cursor.";
    case "messages":
      return "Postgres NOTICE/RAISE output, SQL Server PRINT, and warnings will land here.";
    case "plan":
      return "EXPLAIN ANALYZE renders as a tree with a cost heatmap.";
    case "history":
      return "Every executed query is logged with duration, row count, and outcome.";
    case "notices":
      return "Auto-promoted warnings such as missing indexes or sequential scans.";
    default:
      return "";
  }
}

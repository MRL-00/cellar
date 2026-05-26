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
    <div className="bp-root">
      <div className="bp-head">
        <div className="bp-tabs">
          {TABS.map((t) => (
            <button
              key={t.id}
              className={"bp-tab" + (active === t.id ? " active" : "")}
              onClick={() => setActive(t.id)}
            >
              <span className="bp-tab-icon">{t.icon}</span>
              <span>{t.label}</span>
              {t.count != null && (
                <span className="bp-tab-count mono">{t.count}</span>
              )}
            </button>
          ))}
          <div className="bp-tab-divider" />
          <div className="bp-runinfo mono">
            <span style={{ color: "var(--fg-3)" }}>no query run yet</span>
          </div>
        </div>
        <div className="bp-head-actions">
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

      <div className="bp-body">
        <div className="bp-placeholder">
          <div className="bp-placeholder-title">{labelFor(active)}</div>
          <div className="bp-placeholder-sub">{subFor(active)}</div>
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

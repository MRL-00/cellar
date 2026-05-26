import { useState } from "react";
import { Icon } from "./icons";
import { EngineBadge, type Engine } from "./EngineBadge";

type ConnStatus = "connected" | "connecting" | "disconnected";

type SampleConnection = {
  id: string;
  name: string;
  engine: Engine;
  status: ConnStatus;
};

const SAMPLE_CONNECTIONS: SampleConnection[] = [
  { id: "prod", name: "shop-eu (prod)", engine: "postgres", status: "connected" },
  { id: "stage", name: "shop-eu (stage)", engine: "postgres", status: "disconnected" },
  { id: "warehouse", name: "analytics-warehouse", engine: "mssql", status: "disconnected" },
  { id: "billing", name: "billing-mysql", engine: "mysql", status: "disconnected" },
  { id: "local", name: "local.sqlite", engine: "sqlite", status: "disconnected" },
];

const SAMPLE_TABLES = [
  { id: "orders", name: "orders", rows: "1.8M", fks: 2, active: true },
  { id: "order_items", name: "order_items", rows: "7.2M" },
  { id: "customers", name: "customers", rows: "184k" },
  { id: "products", name: "products", rows: "12k" },
  { id: "payments", name: "payments", rows: "1.8M" },
  { id: "refunds", name: "refunds", rows: "18k" },
];

function StatusDot({ status }: { status: ConnStatus }) {
  const color =
    status === "connected"
      ? "var(--accent)"
      : status === "connecting"
        ? "var(--warn)"
        : "var(--fg-4)";
  return (
    <span
      className={"sb-status-dot" + (status === "connecting" ? " pulse" : "")}
      style={{ background: color }}
      title={status}
    />
  );
}

export function Sidebar() {
  const [filter, setFilter] = useState("");
  const meta = { color: "var(--eng-postgres)" };
  return (
    <div className="sb-root">
      <div className="sb-header">
        <div className="sb-header-title">
          <span>Connections</span>
          <span className="sb-header-count mono">{SAMPLE_CONNECTIONS.length}</span>
        </div>
        <div className="sb-header-actions">
          <button className="icon-btn" title="New connection">
            <Icon.plus size={12} />
          </button>
          <button className="icon-btn" title="More">
            <Icon.more size={12} />
          </button>
        </div>
      </div>

      <div className="sb-filter">
        <Icon.search size={11} style={{ color: "var(--fg-3)" }} />
        <input
          className="sb-filter-input"
          placeholder="Filter…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        />
        <span className="kbd">⌘F</span>
      </div>

      <div className="sb-scroll">
        {/* expanded prod connection */}
        <div className="sb-connection">
          <div
            className="sb-row sb-conn-row"
            style={{ borderLeftColor: meta.color }}
          >
            <button className="sb-twisty">
              <Icon.chevronDown size={10} />
            </button>
            <EngineBadge engine="postgres" size={12} />
            <span className="sb-label sb-conn-name">shop-eu (prod)</span>
            <StatusDot status="connected" />
            <button className="icon-btn sb-row-action" title="Disconnect">
              <Icon.power size={11} />
            </button>
          </div>

          {/* database row */}
          <div className="sb-row" style={{ paddingLeft: 18 }}>
            <button className="sb-twisty">
              <Icon.chevronDown size={10} />
            </button>
            <span className="sb-icon">
              <Icon.database size={12} />
            </span>
            <span className="sb-label">shop_eu</span>
            <span className="sb-meta mono">2 schemas</span>
          </div>

          {/* schema row */}
          <div className="sb-row" style={{ paddingLeft: 30 }}>
            <button className="sb-twisty">
              <Icon.chevronDown size={10} />
            </button>
            <span className="sb-icon">
              <Icon.schema size={12} />
            </span>
            <span className="sb-label">public</span>
            <span className="sb-meta mono">{SAMPLE_TABLES.length}</span>
          </div>

          {/* tables group */}
          <div className="sb-row sb-group" style={{ paddingLeft: 42 }}>
            <button className="sb-twisty">
              <Icon.chevronDown size={10} />
            </button>
            <span className="sb-group-label">tables</span>
            <span className="sb-group-count mono">{SAMPLE_TABLES.length}</span>
          </div>

          {SAMPLE_TABLES.map((t) => (
            <div
              key={t.id}
              className={"sb-row" + (t.active ? " active" : "")}
              style={{ paddingLeft: 54 }}
            >
              <span className="sb-twisty" style={{ visibility: "hidden" }} />
              <span className="sb-icon" style={{ color: "var(--fg-1)" }}>
                <Icon.table size={11} />
              </span>
              <span className="sb-label">{t.name}</span>
              <span className="sb-meta mono">{t.rows}</span>
              {t.fks ? (
                <span className="sb-pill" title={`${t.fks} foreign keys`}>
                  fk·{t.fks}
                </span>
              ) : null}
            </div>
          ))}

          <div className="sb-row sb-group" style={{ paddingLeft: 42 }}>
            <button className="sb-twisty">
              <Icon.chevronRight size={10} />
            </button>
            <span className="sb-group-label">views</span>
            <span className="sb-group-count mono">3</span>
          </div>
          <div className="sb-row sb-group" style={{ paddingLeft: 42 }}>
            <button className="sb-twisty">
              <Icon.chevronRight size={10} />
            </button>
            <span className="sb-group-label">functions</span>
            <span className="sb-group-count mono">2</span>
          </div>
          <div className="sb-row sb-group" style={{ paddingLeft: 42 }}>
            <button className="sb-twisty">
              <Icon.chevronRight size={10} />
            </button>
            <span className="sb-group-label">procedures</span>
            <span className="sb-group-count mono">1</span>
          </div>
        </div>

        {/* collapsed connections */}
        {SAMPLE_CONNECTIONS.slice(1).map((c) => (
          <div key={c.id} className="sb-connection">
            <div className="sb-row sb-conn-row">
              <button className="sb-twisty">
                <Icon.chevronRight size={10} />
              </button>
              <EngineBadge engine={c.engine} size={12} />
              <span className="sb-label sb-conn-name">{c.name}</span>
              <StatusDot status={c.status} />
            </div>
          </div>
        ))}

        <button className="sb-add-connection">
          <Icon.plus size={11} />
          <span>New connection</span>
        </button>
      </div>
    </div>
  );
}

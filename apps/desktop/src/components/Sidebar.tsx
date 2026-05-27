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
      className={
        "ml-1 h-1.5 w-1.5 shrink-0 rounded-full" +
        (status === "connecting" ? " animate-sb-pulse" : "")
      }
      style={{ background: color }}
      title={status}
    />
  );
}

const ROW_BASE =
  "group relative flex h-[22px] select-none items-center gap-1 pr-1.5 text-fg-1 cursor-default hover:bg-bg-2";

const ROW_ACTIVE = "bg-accent-soft text-accent [&_.sb-icon-slot]:!text-accent";

const ICON_SLOT =
  "sb-icon-slot inline-flex h-[14px] w-[14px] shrink-0 items-center justify-center";

const TWISTY =
  "inline-flex h-[14px] w-[14px] shrink-0 items-center justify-center text-fg-3 hover:text-fg-1";

const META = "ml-auto pr-1 whitespace-nowrap text-[10px] text-fg-3 shrink-0";

const PILL =
  "ml-1 rounded-[3px] bg-bg-2 px-1 py-px font-mono text-[9px] text-fg-3";

export function Sidebar({
  onNewConnection,
}: {
  onNewConnection?: () => void;
} = {}) {
  const [filter, setFilter] = useState("");
  const meta = { color: "var(--eng-postgres)" };
  return (
    <div className="flex h-full flex-col text-[11.5px]">
      <div className="flex shrink-0 items-center justify-between pt-[7px] pb-[5px] pl-2.5 pr-2">
        <div className="flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-[0.04em] text-fg-2">
          <span>Connections</span>
          <span className="rounded-[8px] bg-bg-2 px-1.5 py-px font-mono text-[10px] text-fg-3">
            {SAMPLE_CONNECTIONS.length}
          </span>
        </div>
        <div className="flex gap-px">
          <button className="icon-btn" title="New connection" onClick={onNewConnection}>
            <Icon.plus size={12} />
          </button>
          <button className="icon-btn" title="More">
            <Icon.more size={12} />
          </button>
        </div>
      </div>

      <div className="mx-2 mb-1.5 flex h-6 shrink-0 items-center gap-1.5 rounded-[4px] border border-border-default bg-bg-inset px-2 focus-within:border-accent-line">
        <Icon.search size={11} style={{ color: "var(--fg-3)" }} />
        <input
          placeholder="Filter…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          className="flex-1 border-none bg-transparent text-[11.5px] text-fg-0 outline-none placeholder:text-fg-3"
        />
        <span className="kbd">⌘F</span>
      </div>

      <div className="flex-1 overflow-y-auto pb-3">
        <div>
          <div
            className={
              ROW_BASE + " h-[26px] border-l-2 pl-1 font-medium text-fg-0"
            }
            style={{ borderLeftColor: meta.color }}
          >
            <button className={TWISTY}>
              <Icon.chevronDown size={10} />
            </button>
            <EngineBadge engine="postgres" size={12} />
            <span className="flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-[12px] font-medium">
              shop-eu (prod)
            </span>
            <StatusDot status="connected" />
            <button
              className="icon-btn ml-1 opacity-0 transition-opacity duration-100 group-hover:opacity-100"
              title="Disconnect"
            >
              <Icon.power size={11} />
            </button>
          </div>

          <div className={ROW_BASE} style={{ paddingLeft: 18 }}>
            <button className={TWISTY}>
              <Icon.chevronDown size={10} />
            </button>
            <span className={ICON_SLOT}>
              <Icon.database size={12} />
            </span>
            <span className="flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-[11.5px]">
              shop_eu
            </span>
            <span className={META + " font-mono"}>2 schemas</span>
          </div>

          <div className={ROW_BASE} style={{ paddingLeft: 30 }}>
            <button className={TWISTY}>
              <Icon.chevronDown size={10} />
            </button>
            <span className={ICON_SLOT}>
              <Icon.schema size={12} />
            </span>
            <span className="flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-[11.5px]">
              public
            </span>
            <span className={META + " font-mono"}>{SAMPLE_TABLES.length}</span>
          </div>

          <div
            className={
              ROW_BASE +
              " mt-0.5 h-5 text-[10px] uppercase tracking-[0.05em] text-fg-3 hover:bg-transparent hover:text-fg-2"
            }
            style={{ paddingLeft: 42 }}
          >
            <button className={TWISTY}>
              <Icon.chevronDown size={10} />
            </button>
            <span className="flex-1 font-semibold">tables</span>
            <span className="font-mono text-[10px] text-fg-3">
              {SAMPLE_TABLES.length}
            </span>
          </div>

          {SAMPLE_TABLES.map((t) => (
            <div
              key={t.id}
              className={ROW_BASE + (t.active ? " " + ROW_ACTIVE : "")}
              style={{ paddingLeft: 54 }}
            >
              <span className={TWISTY + " invisible"} />
              <span className={ICON_SLOT} style={{ color: "var(--fg-1)" }}>
                <Icon.table size={11} />
              </span>
              <span className="flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-[11.5px]">
                {t.name}
              </span>
              <span className={META + " font-mono"}>{t.rows}</span>
              {t.fks ? (
                <span className={PILL} title={`${t.fks} foreign keys`}>
                  fk·{t.fks}
                </span>
              ) : null}
            </div>
          ))}

          {(["views", "functions", "procedures"] as const).map((group, i) => (
            <div
              key={group}
              className={
                ROW_BASE +
                " mt-0.5 h-5 text-[10px] uppercase tracking-[0.05em] text-fg-3 hover:bg-transparent hover:text-fg-2"
              }
              style={{ paddingLeft: 42 }}
            >
              <button className={TWISTY}>
                <Icon.chevronRight size={10} />
              </button>
              <span className="flex-1 font-semibold">{group}</span>
              <span className="font-mono text-[10px] text-fg-3">
                {[3, 2, 1][i]}
              </span>
            </div>
          ))}
        </div>

        {SAMPLE_CONNECTIONS.slice(1).map((c) => (
          <div key={c.id}>
            <div
              className={
                ROW_BASE +
                " h-[26px] border-l-2 border-transparent pl-1 font-medium text-fg-0"
              }
            >
              <button className={TWISTY}>
                <Icon.chevronRight size={10} />
              </button>
              <EngineBadge engine={c.engine} size={12} />
              <span className="flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-[12px] font-medium">
                {c.name}
              </span>
              <StatusDot status={c.status} />
            </div>
          </div>
        ))}

        <button
          onClick={onNewConnection}
          className="m-2 flex w-[calc(100%-16px)] items-center gap-1.5 rounded-[4px] border border-dashed border-border-default px-2 py-1.5 text-[11.5px] text-fg-2 transition-[border-color,color,background] duration-150 hover:border-solid hover:border-accent-line hover:bg-accent-soft hover:text-accent"
        >
          <Icon.plus size={11} />
          <span>New connection</span>
        </button>
      </div>
    </div>
  );
}

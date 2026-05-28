import { useEffect, useMemo, useState } from "react";
import type { ConnectionConfig, Schema, Table } from "@cellar/ipc";

import { Icon } from "./icons";
import { EngineBadge, type Engine } from "./EngineBadge";
import { ContextMenu, type ContextMenuState } from "./ContextMenu";
import { useConnections, type ConnStatus } from "../state/connections";
import { useTabs } from "../state/tabs";

function StatusDot({ status }: { status: ConnStatus }) {
  const color =
    status === "connected"
      ? "var(--accent)"
      : status === "connecting"
        ? "var(--warn)"
        : status === "error"
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

export interface SidebarProps {
  onNewConnection?: () => void;
  onEditConnection?: (config: ConnectionConfig) => void;
  onDuplicateConnection?: (config: ConnectionConfig) => void;
}

export function Sidebar({
  onNewConnection,
  onEditConnection,
  onDuplicateConnection,
}: SidebarProps = {}) {
  const [filter, setFilter] = useState("");
  const [menu, setMenu] = useState<ContextMenuState | null>(null);
  const connections = useConnections((s) => s.connections);
  const byId = useConnections((s) => s.byId);
  const loaded = useConnections((s) => s.loaded);
  const load = useConnections((s) => s.load);
  const toggleExpand = useConnections((s) => s.toggleExpand);
  const connect = useConnections((s) => s.connect);
  const disconnect = useConnections((s) => s.disconnect);
  const deleteConnection = useConnections((s) => s.deleteConnection);
  const openTable = useTabs((s) => s.openTable);
  const activeTabId = useTabs((s) => s.activeId);

  useEffect(() => {
    if (!loaded) {
      void load();
    }
  }, [loaded, load]);

  const filtered = useMemo(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return connections;
    return connections.filter((c) => c.name.toLowerCase().includes(q));
  }, [filter, connections]);

  const openConnectionMenu = (e: React.MouseEvent, config: ConnectionConfig) => {
    e.preventDefault();
    e.stopPropagation();
    const status = byId[config.id]?.status ?? "disconnected";
    const connected = status === "connected" || status === "connecting";
    setMenu({
      x: e.clientX,
      y: e.clientY,
      items: [
        {
          label: "Edit…",
          icon: <Icon.edit size={12} />,
          onClick: () => onEditConnection?.(config),
        },
        {
          label: "Duplicate",
          icon: <Icon.copy size={12} />,
          onClick: () => onDuplicateConnection?.(config),
        },
        connected
          ? {
              label: "Disconnect",
              icon: <Icon.power size={12} />,
              onClick: () => void disconnect(config.id),
            }
          : {
              label: "Connect",
              icon: <Icon.power size={12} />,
              onClick: () => void connect(config.id),
            },
        {
          label: "Remove",
          icon: <Icon.trash size={12} />,
          danger: true,
          onClick: () => {
            if (
              window.confirm(
                `Remove connection "${config.name}"? This deletes its saved password from the keychain.`,
              )
            ) {
              void deleteConnection(config.id);
            }
          },
        },
      ],
    });
  };

  return (
    <div className="flex h-full flex-col text-[11.5px]">
      <div className="flex shrink-0 items-center justify-between pt-[7px] pb-[5px] pl-2.5 pr-2">
        <div className="flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-[0.04em] text-fg-2">
          <span>Connections</span>
          <span className="rounded-[8px] bg-bg-2 px-1.5 py-px font-mono text-[10px] text-fg-3">
            {connections.length}
          </span>
        </div>
        <div className="flex gap-px">
          <button
            className="icon-btn"
            title="New connection"
            onClick={onNewConnection}
          >
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
        {filtered.length === 0 && (
          <div className="px-3 py-6 text-center text-[11px] text-fg-3">
            no connections yet
          </div>
        )}
        {filtered.map((c) => {
          const state = byId[c.id];
          return (
            <ConnectionRow
              key={c.id}
              config={c}
              status={state?.status ?? "disconnected"}
              expanded={state?.expanded ?? false}
              loadingSchema={state?.loadingSchema ?? false}
              databases={state?.databases ?? []}
              error={state?.error ?? null}
              onToggle={() => toggleExpand(c.id)}
              onDisconnect={() => void disconnect(c.id)}
              onContextMenu={(e) => openConnectionMenu(e, c)}
              onOpenTable={(database, schema, table) =>
                openTable(c.id, database, schema, table)
              }
              activeTabId={activeTabId}
            />
          );
        })}

        <button
          onClick={onNewConnection}
          className="m-2 flex w-[calc(100%-16px)] items-center gap-1.5 rounded-[4px] border border-dashed border-border-default px-2 py-1.5 text-[11.5px] text-fg-2 transition-[border-color,color,background] duration-150 hover:border-solid hover:border-accent-line hover:bg-accent-soft hover:text-accent"
        >
          <Icon.plus size={11} />
          <span>New connection</span>
        </button>
      </div>

      <ContextMenu state={menu} onClose={() => setMenu(null)} />
    </div>
  );
}

interface ConnectionRowProps {
  config: ConnectionConfig;
  status: ConnStatus;
  expanded: boolean;
  loadingSchema: boolean;
  databases: { name: string; is_default: boolean; schemas: Schema[] }[];
  error: string | null;
  onToggle: () => void;
  onDisconnect: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
  onOpenTable: (database: string, schema: string, table: string) => void;
  activeTabId: string | null;
}

function ConnectionRow({
  config,
  status,
  expanded,
  loadingSchema,
  databases,
  error,
  onToggle,
  onDisconnect,
  onContextMenu,
  onOpenTable,
  activeTabId,
}: ConnectionRowProps) {
  const accent = config.color ?? engineDefaultColor(config.engine as Engine);
  return (
    <div>
      <div
        className={
          ROW_BASE +
          " h-[26px] border-l-2 pl-1 font-medium text-fg-0 cursor-pointer"
        }
        style={{
          borderLeftColor:
            expanded && status === "connected" ? accent : "transparent",
        }}
        onClick={onToggle}
        onContextMenu={onContextMenu}
      >
        <button
          className={TWISTY}
          onClick={(e) => {
            e.stopPropagation();
            onToggle();
          }}
        >
          {expanded ? (
            <Icon.chevronDown size={10} />
          ) : (
            <Icon.chevronRight size={10} />
          )}
        </button>
        <EngineBadge engine={config.engine as Engine} size={12} />
        <span className="flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-[12px] font-medium">
          {config.name}
        </span>
        {config.env_tag === "prod" && (
          <span
            className="rounded-[3px] px-1 py-px font-mono text-[8.5px] uppercase"
            style={{ color: "var(--warn)", background: "color-mix(in oklab, var(--warn) 16%, transparent)" }}
            title="production"
          >
            prod
          </span>
        )}
        <StatusDot status={status} />
        <button
          className="icon-btn ml-1 opacity-0 transition-opacity duration-100 group-hover:opacity-100"
          title="Actions"
          onClick={(e) => {
            e.stopPropagation();
            onContextMenu(e);
          }}
        >
          <Icon.more size={11} />
        </button>
        {status === "connected" && (
          <button
            className="icon-btn opacity-0 transition-opacity duration-100 group-hover:opacity-100"
            title="Disconnect"
            onClick={(e) => {
              e.stopPropagation();
              onDisconnect();
            }}
          >
            <Icon.power size={11} />
          </button>
        )}
      </div>

      {expanded && error && (
        <div className="px-3 py-1 text-[10.5px] text-warn" style={{ paddingLeft: 32 }}>
          {error}
        </div>
      )}

      {expanded && loadingSchema && (
        <div
          className="px-3 py-1 text-[10.5px] text-fg-3 animate-sb-pulse"
          style={{ paddingLeft: 32 }}
        >
          loading schemas…
        </div>
      )}

      {expanded &&
        !loadingSchema &&
        databases.map((db) => (
          <DatabaseRow
            key={db.name}
            dbName={db.name}
            isDefault={db.is_default}
            schemas={db.schemas}
            onOpenTable={onOpenTable}
            activeTabId={activeTabId}
          />
        ))}
    </div>
  );
}

function DatabaseRow({
  dbName,
  isDefault,
  schemas,
  onOpenTable,
  activeTabId,
}: {
  dbName: string;
  isDefault: boolean;
  schemas: Schema[];
  onOpenTable: (database: string, schema: string, table: string) => void;
  activeTabId: string | null;
}) {
  // Default database opens expanded; others stay collapsed to keep the tree tidy.
  const [open, setOpen] = useState(isDefault);
  const empty = schemas.length === 0;
  return (
    <div>
      <div
        className={ROW_BASE + " cursor-pointer"}
        style={{ paddingLeft: 18 }}
        onClick={() => !empty && setOpen((v) => !v)}
        title={empty ? "no accessible schemas" : undefined}
      >
        <button className={TWISTY}>
          {empty ? (
            <span className={TWISTY + " invisible"} />
          ) : open ? (
            <Icon.chevronDown size={10} />
          ) : (
            <Icon.chevronRight size={10} />
          )}
        </button>
        <span className={ICON_SLOT}>
          <Icon.database size={12} />
        </span>
        <span
          className={
            "flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-[11.5px] " +
            (empty ? "text-fg-3" : "")
          }
        >
          {dbName}
        </span>
        <span className={META + " font-mono"}>
          {empty ? "—" : `${schemas.length} schemas`}
        </span>
      </div>
      {open &&
        schemas.map((sch) => (
          <SchemaRow
            key={sch.name}
            database={dbName}
            schema={sch}
            onOpenTable={onOpenTable}
            activeTabId={activeTabId}
          />
        ))}
    </div>
  );
}

function SchemaRow({
  database,
  schema,
  onOpenTable,
  activeTabId,
}: {
  database: string;
  schema: Schema;
  onOpenTable: (database: string, schema: string, table: string) => void;
  activeTabId: string | null;
}) {
  const [open, setOpen] = useState(true);
  return (
    <div>
      <div
        className={ROW_BASE + " cursor-pointer"}
        style={{ paddingLeft: 30 }}
        onClick={() => setOpen((v) => !v)}
      >
        <button className={TWISTY}>
          {open ? (
            <Icon.chevronDown size={10} />
          ) : (
            <Icon.chevronRight size={10} />
          )}
        </button>
        <span className={ICON_SLOT}>
          <Icon.schema size={12} />
        </span>
        <span className="flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-[11.5px]">
          {schema.name}
        </span>
        <span className={META + " font-mono"}>{schema.tables.length}</span>
      </div>
      {open && (
        <>
          {schema.tables.length > 0 && (
            <GroupHeader label="tables" count={schema.tables.length} />
          )}
          {schema.tables.map((t) => (
            <TableRow
              key={t.name}
              database={database}
              schema={schema.name}
              table={t}
              onOpen={() => onOpenTable(database, schema.name, t.name)}
              activeTabId={activeTabId}
            />
          ))}
          {schema.views.length > 0 && (
            <>
              <GroupHeader label="views" count={schema.views.length} />
              {schema.views.map((v) => (
                <div
                  key={v.name}
                  className={ROW_BASE + " cursor-pointer"}
                  style={{ paddingLeft: 54 }}
                  onClick={() => onOpenTable(database, schema.name, v.name)}
                  title="click to open"
                >
                  <span className={TWISTY + " invisible"} />
                  <span className={ICON_SLOT}>
                    <Icon.tree size={11} />
                  </span>
                  <span className="flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-[11.5px]">
                    {v.name}
                  </span>
                </div>
              ))}
            </>
          )}
        </>
      )}
    </div>
  );
}

function GroupHeader({ label, count }: { label: string; count: number }) {
  return (
    <div
      className={
        ROW_BASE +
        " mt-0.5 h-5 text-[10px] uppercase tracking-[0.05em] text-fg-3 hover:bg-transparent hover:text-fg-2"
      }
      style={{ paddingLeft: 42 }}
    >
      <span className={TWISTY + " invisible"} />
      <span className="flex-1 font-semibold">{label}</span>
      <span className="font-mono text-[10px] text-fg-3">{count}</span>
    </div>
  );
}

function TableRow({
  database,
  schema,
  table,
  onOpen,
  activeTabId,
}: {
  database: string;
  schema: string;
  table: Table;
  onOpen: () => void;
  activeTabId: string | null;
}) {
  const tabId = `${database}.${schema}.${table.name}`;
  const active = activeTabId?.endsWith(tabId) ?? false;
  const fkCount = table.foreign_keys.length;
  return (
    <div
      className={ROW_BASE + " cursor-pointer" + (active ? " " + ROW_ACTIVE : "")}
      style={{ paddingLeft: 54 }}
      onClick={onOpen}
      title="click to open"
    >
      <span className={TWISTY + " invisible"} />
      <span className={ICON_SLOT} style={{ color: "var(--fg-1)" }}>
        <Icon.table size={11} />
      </span>
      <span className="flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-[11.5px]">
        {table.name}
      </span>
      {table.row_count != null && (
        <span className={META + " font-mono"}>
          {formatRowCount(table.row_count)}
        </span>
      )}
      {fkCount > 0 && (
        <span className={PILL} title={`${fkCount} foreign keys`}>
          fk·{fkCount}
        </span>
      )}
    </div>
  );
}

function formatRowCount(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
  if (n >= 1_000) return (n / 1_000).toFixed(1) + "k";
  return String(n);
}

function engineDefaultColor(engine: Engine): string {
  switch (engine) {
    case "postgres":
      return "var(--eng-postgres)";
    case "mysql":
      return "var(--eng-mysql)";
    case "mssql":
      return "var(--eng-mssql)";
    case "azure":
      return "var(--eng-azure)";
    case "sqlite":
      return "var(--eng-sqlite)";
  }
}

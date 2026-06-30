import { useState } from "react";
import type { ConnectionConfig, Schema, Table } from "@cellar/ipc";

import { EngineBadge, type Engine } from "./EngineBadge";
import { Icon } from "./icons";
import type { ConnStatus } from "../state/connections";

/** A right-clickable node in the schema tree. */
export type SidebarNode =
  | {
      kind: "database";
      connectionId: string;
      database: string;
      schemas: Schema[];
      showHiddenSchemas: boolean;
    }
  | {
      kind: "schema";
      connectionId: string;
      database: string;
      schema: string;
      hidden: boolean;
    }
  | {
      kind: "relation";
      connectionId: string;
      database: string;
      schema: string;
      name: string;
      isView: boolean;
    };

export type NodeMenuHandler = (e: React.MouseEvent, node: SidebarNode) => void;

export type SchemaVisibilityPrefs = {
  hidden: string[];
  showHidden: boolean;
};

export type SchemaVisibilityState = Record<string, SchemaVisibilityPrefs>;

const SCHEMA_VISIBILITY_STORAGE_KEY = "cellar.schemaVisibility.v1";

export const ROW_BASE =
  "group relative flex h-[22px] select-none items-center gap-1 pr-1.5 text-fg-1 cursor-default hover:bg-bg-2";

const ROW_ACTIVE = "bg-accent-soft text-accent [&_.sb-icon-slot]:!text-accent";

export const ICON_SLOT =
  "sb-icon-slot inline-flex h-[14px] w-[14px] shrink-0 items-center justify-center";

export const TWISTY =
  "inline-flex h-[14px] w-[14px] shrink-0 items-center justify-center text-fg-3 hover:text-fg-1";

export const META =
  "ml-auto pr-1 whitespace-nowrap text-[10px] text-fg-3 shrink-0";

const PILL =
  "ml-1 rounded-[3px] bg-bg-2 px-1 py-px font-mono text-[9px] text-fg-3";

/** Drag/drop wiring the sidebar list attaches to a connection header row. */
export interface ConnectionDragHandles {
  draggable: boolean;
  onDragStart: React.DragEventHandler;
  onDragOver: React.DragEventHandler;
  onDragEnd: () => void;
  dropIndicator: "before" | "after" | null;
}

export interface ConnectionRowProps {
  config: ConnectionConfig;
  status: ConnStatus;
  expanded: boolean;
  loadingSchema: boolean;
  databases: { name: string; is_default: boolean; schemas: Schema[] }[];
  error: string | null;
  onToggle: () => void;
  onReconnect: () => void;
  onDisconnect: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
  onNodeContextMenu: NodeMenuHandler;
  onOpenTable: (database: string, schema: string, table: string) => void;
  activeTabId: string | null;
  schemaVisibility: SchemaVisibilityState;
  onManageSchemas: (
    connectionId: string,
    database: string,
    schemas: Schema[],
  ) => void;
  drag?: ConnectionDragHandles;
}

export function ConnectionRow({
  config,
  status,
  expanded,
  loadingSchema,
  databases,
  error,
  onToggle,
  onReconnect,
  onDisconnect,
  onContextMenu,
  onNodeContextMenu,
  onOpenTable,
  activeTabId,
  schemaVisibility,
  onManageSchemas,
  drag,
}: ConnectionRowProps) {
  const accent = config.color ?? engineDefaultColor(config.engine as Engine);
  return (
    <div>
      <div
        className={
          ROW_BASE +
          " h-[26px] border-l-2 pl-1 font-medium text-fg-0 cursor-pointer"
        }
        style={{ borderLeftColor: accent }}
        onClick={onToggle}
        onContextMenu={onContextMenu}
        draggable={drag?.draggable}
        onDragStart={drag?.onDragStart}
        onDragOver={drag?.onDragOver}
        onDragEnd={drag?.onDragEnd}
      >
        {drag?.dropIndicator && (
          <span
            className={
              "pointer-events-none absolute inset-x-0 z-10 h-[2px] rounded " +
              (drag.dropIndicator === "before" ? "top-0" : "bottom-0")
            }
            style={{ background: "var(--accent)" }}
          />
        )}
        <button
          type="button"
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
        <EngineBadge engine={config.engine as Engine} size={12} color={accent} />
        <span className="flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-[13px] font-medium">
          {config.name}
        </span>
        {config.env_tag === "prod" && (
          <span
            className="rounded-[3px] px-1 py-px font-mono text-[8.5px] uppercase"
            style={{
              color: "var(--warn)",
              background: "color-mix(in oklab, var(--warn) 16%, transparent)",
            }}
            title="production"
          >
            prod
          </span>
        )}
        <StatusDot status={status} />
        <button
          type="button"
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
            type="button"
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
        <div
          className="flex items-start gap-2 px-3 py-1 text-[10.5px] text-warn"
          style={{ paddingLeft: 32 }}
        >
          <span className="min-w-0 flex-1">{error}</span>
          <button
            className="inline-flex h-[20px] shrink-0 items-center gap-1 rounded-[4px] border border-warn/40 px-1.5 text-[10px] text-warn hover:bg-bg-2"
            title="Reconnect"
            onClick={(e) => {
              e.stopPropagation();
              onReconnect();
            }}
          >
            <Icon.history size={10} />
            Retry
          </button>
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
            connectionId={config.id}
            dbName={db.name}
            isDefault={db.is_default}
            schemas={db.schemas}
            onNodeContextMenu={onNodeContextMenu}
            onOpenTable={onOpenTable}
            activeTabId={activeTabId}
            schemaVisibility={schemaVisibility}
            onManageSchemas={onManageSchemas}
          />
        ))}
    </div>
  );
}

function DatabaseRow({
  connectionId,
  dbName,
  isDefault,
  schemas,
  onNodeContextMenu,
  onOpenTable,
  activeTabId,
  schemaVisibility,
  onManageSchemas,
}: {
  connectionId: string;
  dbName: string;
  isDefault: boolean;
  schemas: Schema[];
  onNodeContextMenu: NodeMenuHandler;
  onOpenTable: (database: string, schema: string, table: string) => void;
  activeTabId: string | null;
  schemaVisibility: SchemaVisibilityState;
  onManageSchemas: (
    connectionId: string,
    database: string,
    schemas: Schema[],
  ) => void;
}) {
  const [open, setOpen] = useState(isDefault);
  const prefs = visibilityPrefs(
    schemaVisibility,
    schemaVisibilityKey(connectionId, dbName),
  );
  const visibility = visibleSchemas(schemas, prefs);
  const empty = schemas.length === 0;
  const visibleEmpty = visibility.schemas.length === 0;
  return (
    <div>
      <div
        className={ROW_BASE + " cursor-pointer"}
        style={{ paddingLeft: 18 }}
        onClick={() => !visibleEmpty && setOpen((v) => !v)}
        onContextMenu={(e) =>
          onNodeContextMenu(e, {
            kind: "database",
            connectionId,
            database: dbName,
            schemas,
            showHiddenSchemas: prefs.showHidden,
          })
        }
        title={empty ? "no accessible schemas" : undefined}
      >
        <button
          type="button"
          className={TWISTY}
          aria-label={open ? "Collapse database" : "Expand database"}
        >
          {visibleEmpty ? (
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
            "flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-[13px] " +
            (visibleEmpty ? "text-fg-3" : "")
          }
        >
          {dbName}
        </span>
        <span className={META + " font-mono"}>
          {empty
            ? "—"
            : visibility.hiddenCount > 0
              ? `${visibility.schemas.length}/${schemas.length} schemas`
              : `${schemas.length} schemas`}
        </span>
        {schemas.length > 0 && (
          <button
            type="button"
            className="icon-btn ml-0.5 opacity-0 transition-opacity duration-100 group-hover:opacity-100"
            title="Choose visible schemas"
            onClick={(e) => {
              e.stopPropagation();
              onManageSchemas(connectionId, dbName, schemas);
            }}
          >
            <Icon.eye size={11} />
          </button>
        )}
      </div>
      {open &&
        visibility.schemas.map((sch) => (
          <SchemaRow
            key={sch.name}
            connectionId={connectionId}
            database={dbName}
            schema={sch}
            hidden={visibility.hiddenNames.has(sch.name)}
            onNodeContextMenu={onNodeContextMenu}
            onOpenTable={onOpenTable}
            activeTabId={activeTabId}
          />
        ))}
    </div>
  );
}

function SchemaRow({
  connectionId,
  database,
  schema,
  onNodeContextMenu,
  onOpenTable,
  activeTabId,
  hidden,
}: {
  connectionId: string;
  database: string;
  schema: Schema;
  hidden: boolean;
  onNodeContextMenu: NodeMenuHandler;
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
        onContextMenu={(e) =>
          onNodeContextMenu(e, {
            kind: "schema",
            connectionId,
            database,
            schema: schema.name,
            hidden,
          })
        }
      >
        <button
          type="button"
          className={TWISTY}
          aria-label={open ? "Collapse schema" : "Expand schema"}
        >
          {open ? (
            <Icon.chevronDown size={10} />
          ) : (
            <Icon.chevronRight size={10} />
          )}
        </button>
        <span className={ICON_SLOT}>
          <Icon.schema size={12} />
        </span>
        <span
          className={
            "flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-[13px] " +
            (hidden ? "text-fg-3" : "")
          }
        >
          {schema.name}
        </span>
        <span className={META + " font-mono"}>{schema.tables.length}</span>
      </div>
      {open && (
        <>
          {schema.tables.length > 0 && (
            <GroupFolder
              storageKey={`${connectionId}.${database}.${schema.name}.tables`}
              label="tables"
              count={schema.tables.length}
            >
              {schema.tables.map((t) => (
                <TableRow
                  key={t.name}
                  connectionId={connectionId}
                  database={database}
                  schema={schema.name}
                  table={t}
                  onOpen={() => onOpenTable(database, schema.name, t.name)}
                  onNodeContextMenu={onNodeContextMenu}
                  activeTabId={activeTabId}
                />
              ))}
            </GroupFolder>
          )}
          {schema.views.length > 0 && (
            <GroupFolder
              storageKey={`${connectionId}.${database}.${schema.name}.views`}
              label="views"
              count={schema.views.length}
            >
              {schema.views.map((v) => (
                <div
                  key={v.name}
                  className={ROW_BASE + " cursor-pointer"}
                  style={{ paddingLeft: 66 }}
                  onClick={() => onOpenTable(database, schema.name, v.name)}
                  onContextMenu={(e) =>
                    onNodeContextMenu(e, {
                      kind: "relation",
                      connectionId,
                      database,
                      schema: schema.name,
                      name: v.name,
                      isView: true,
                    })
                  }
                  title="click to open"
                >
                  <span className={TWISTY + " invisible"} />
                  <span className={ICON_SLOT}>
                    <Icon.tree size={11} />
                  </span>
                  <span className="flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-[13px]">
                    {v.name}
                  </span>
                </div>
              ))}
            </GroupFolder>
          )}
        </>
      )}
    </div>
  );
}

function GroupFolder({
  storageKey,
  label,
  count,
  children,
}: {
  storageKey: string;
  label: string;
  count: number;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(() => readFolderOpen(storageKey, true));
  const toggle = () =>
    setOpen((v) => {
      const next = !v;
      writeFolderOpen(storageKey, next);
      return next;
    });
  return (
    <div>
      <div
        className={ROW_BASE + " cursor-pointer"}
        style={{ paddingLeft: 42 }}
        onClick={toggle}
      >
        <button
          type="button"
          className={TWISTY}
          aria-label={open ? `Collapse ${label}` : `Expand ${label}`}
          onClick={(e) => {
            e.stopPropagation();
            toggle();
          }}
        >
          {open ? (
            <Icon.chevronDown size={10} />
          ) : (
            <Icon.chevronRight size={10} />
          )}
        </button>
        <span className={ICON_SLOT}>
          {open ? <Icon.folderOpen size={12} /> : <Icon.folder size={12} />}
        </span>
        <span className="flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-[13px]">
          {label}
        </span>
        <span className={META + " font-mono"}>{count}</span>
      </div>
      {open && children}
    </div>
  );
}

function TableRow({
  connectionId,
  database,
  schema,
  table,
  onOpen,
  onNodeContextMenu,
  activeTabId,
}: {
  connectionId: string;
  database: string;
  schema: string;
  table: Table;
  onOpen: () => void;
  onNodeContextMenu: NodeMenuHandler;
  activeTabId: string | null;
}) {
  const tabId = `${database}.${schema}.${table.name}`;
  const active = activeTabId?.endsWith(tabId) ?? false;
  const fkCount = table.foreign_keys.length;
  return (
    <div
      className={ROW_BASE + " cursor-pointer" + (active ? " " + ROW_ACTIVE : "")}
      style={{ paddingLeft: 66 }}
      onClick={onOpen}
      onContextMenu={(e) =>
        onNodeContextMenu(e, {
          kind: "relation",
          connectionId,
          database,
          schema,
          name: table.name,
          isView: false,
        })
      }
      title="click to open"
    >
      <span className={TWISTY + " invisible"} />
      <span className={ICON_SLOT} style={{ color: "var(--fg-1)" }}>
        <Icon.table size={11} />
      </span>
      <span className="flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-[13px]">
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

function formatRowCount(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
  if (n >= 1_000) return (n / 1_000).toFixed(1) + "k";
  return String(n);
}

export function schemaVisibilityKey(
  connectionId: string,
  database: string,
): string {
  return `${connectionId}::${database}`;
}

export function visibilityPrefs(
  state: SchemaVisibilityState,
  key: string,
): SchemaVisibilityPrefs {
  return state[key] ?? { hidden: [], showHidden: false };
}

function visibleSchemas(schemas: Schema[], prefs: SchemaVisibilityPrefs) {
  const hiddenNames = new Set(prefs.hidden);
  const hasNonEmptySchema = schemas.some(schemaHasObjects);
  const shouldAutoHideEmpty = hasNonEmptySchema && !prefs.showHidden;
  const visible = schemas.filter((schema) => {
        if (hiddenNames.has(schema.name)) return false;
        if (shouldAutoHideEmpty && !schemaHasObjects(schema)) return false;
        return true;
      });

  return {
    schemas: visible,
    hiddenNames,
    hiddenCount: schemas.length - visible.length,
  };
}

export function schemaHasObjects(schema: Schema): boolean {
  return schema.tables.length > 0 || schema.views.length > 0;
}

export function loadSchemaVisibility(): SchemaVisibilityState {
  if (typeof window === "undefined") return {};
  try {
    const raw = window.localStorage.getItem(SCHEMA_VISIBILITY_STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as unknown;
    if (!parsed || typeof parsed !== "object") return {};
    const out: SchemaVisibilityState = {};
    for (const [key, value] of Object.entries(parsed)) {
      if (!value || typeof value !== "object") continue;
      const candidate = value as Partial<SchemaVisibilityPrefs>;
      out[key] = {
        hidden: Array.isArray(candidate.hidden)
          ? candidate.hidden.filter(
              (name): name is string => typeof name === "string",
            )
          : [],
        showHidden: Boolean(candidate.showHidden),
      };
    }
    return out;
  } catch {
    return {};
  }
}

export function saveSchemaVisibility(state: SchemaVisibilityState) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(
    SCHEMA_VISIBILITY_STORAGE_KEY,
    JSON.stringify(state),
  );
}

const FOLDER_OPEN_STORAGE_KEY = "cellar.folderOpen.v1";

function readFolderOpenMap(): Record<string, boolean> {
  if (typeof window === "undefined") return {};
  try {
    const raw = window.localStorage.getItem(FOLDER_OPEN_STORAGE_KEY);
    const parsed = raw ? (JSON.parse(raw) as unknown) : {};
    return parsed && typeof parsed === "object"
      ? (parsed as Record<string, boolean>)
      : {};
  } catch {
    return {};
  }
}

/** Persisted open/closed state for a sidebar folder, keyed by a stable id. */
function readFolderOpen(key: string, fallback: boolean): boolean {
  const v = readFolderOpenMap()[key];
  return typeof v === "boolean" ? v : fallback;
}

function writeFolderOpen(key: string, open: boolean) {
  if (typeof window === "undefined") return;
  const map = readFolderOpenMap();
  map[key] = open;
  window.localStorage.setItem(FOLDER_OPEN_STORAGE_KEY, JSON.stringify(map));
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
    case "firestore":
      return "var(--eng-firestore)";
  }
}

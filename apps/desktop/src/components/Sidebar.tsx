import { useEffect, useMemo, useState } from "react";
import type { ConnectionConfig, Schema } from "@cellar/ipc";

import { Icon } from "./icons";
import {
  ContextMenu,
  type ContextMenuState,
  type MenuItem,
} from "./ContextMenu";
import {
  ConnectionRow,
  loadSchemaVisibility,
  saveSchemaVisibility,
  schemaVisibilityKey,
  visibilityPrefs,
  type NodeMenuHandler,
  type SchemaVisibilityState,
  type SidebarNode,
} from "./SidebarTree";
import { SidebarConnectionList } from "./SidebarConnectionList";
import {
  SchemaVisibilityManager,
  type SchemaManagerState,
} from "./SidebarSchemaVisibility";
import { useConnections } from "../state/connections";
import {
  useSidebarLayout,
  type SidebarFolderItem,
} from "../state/sidebarLayout";
import { useTabs } from "../state/tabs";
import { useFindUsages } from "../state/findUsages";
import { useConfirm } from "../state/confirm";
import { qualifiedName, selectAllStatement } from "../lib/sqlIdent";

export interface SidebarProps {
  onNewConnection?: () => void;
  onImportDatagrip?: () => void;
  onEditConnection?: (config: ConnectionConfig) => void;
  onDuplicateConnection?: (config: ConnectionConfig) => void;
  onOpenSettings?: () => void;
  onCompareSchemas?: (preset?: {
    connectionId: string;
    database: string;
    schema?: string;
  }) => void;
  onImportData?: () => void;
}

export function Sidebar({
  onNewConnection,
  onImportDatagrip,
  onEditConnection,
  onDuplicateConnection,
  onOpenSettings,
  onCompareSchemas,
  onImportData,
}: SidebarProps = {}) {
  const [filter, setFilter] = useState("");
  const [menu, setMenu] = useState<ContextMenuState | null>(null);
  const [schemaManager, setSchemaManager] =
    useState<SchemaManagerState | null>(null);
  const [schemaVisibility, setSchemaVisibility] =
    useState<SchemaVisibilityState>(() => loadSchemaVisibility());
  const connections = useConnections((s) => s.connections);
  const byId = useConnections((s) => s.byId);
  const loaded = useConnections((s) => s.loaded);
  const load = useConnections((s) => s.load);
  const toggleExpand = useConnections((s) => s.toggleExpand);
  const connect = useConnections((s) => s.connect);
  const reconnect = useConnections((s) => s.reconnect);
  const disconnect = useConnections((s) => s.disconnect);
  const deleteConnection = useConnections((s) => s.deleteConnection);
  const askConfirm = useConfirm((s) => s.ask);
  const refreshSchema = useConnections((s) => s.refreshSchema);
  const openTable = useTabs((s) => s.openTable);
  const openErDiagram = useTabs((s) => s.openErDiagram);
  const newQueryTab = useTabs((s) => s.newQueryTab);
  const setQuerySql = useTabs((s) => s.setQuerySql);
  const findUsages = useFindUsages((s) => s.findUsages);
  const activeTabId = useTabs((s) => s.activeId);
  const layoutItems = useSidebarLayout((s) => s.items);
  const reconcileLayout = useSidebarLayout((s) => s.reconcile);
  const createFolder = useSidebarLayout((s) => s.createFolder);
  const renameFolder = useSidebarLayout((s) => s.renameFolder);
  const removeFolder = useSidebarLayout((s) => s.removeFolder);
  const toggleFolder = useSidebarLayout((s) => s.toggleFolder);
  const moveConnection = useSidebarLayout((s) => s.moveConnection);
  const moveFolder = useSidebarLayout((s) => s.moveFolder);
  const moveToFolder = useSidebarLayout((s) => s.moveToFolder);
  const [renamingFolderId, setRenamingFolderId] = useState<string | null>(null);

  useEffect(() => {
    if (!loaded) {
      void load();
    }
  }, [loaded, load]);

  useEffect(() => {
    if (loaded) reconcileLayout(connections.map((c) => c.id));
  }, [loaded, connections, reconcileLayout]);

  useEffect(() => {
    saveSchemaVisibility(schemaVisibility);
  }, [schemaVisibility]);

  const configById = useMemo(
    () => new Map(connections.map((c) => [c.id, c] as const)),
    [connections],
  );

  const matchCount = useMemo(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return connections.length;
    return connections.filter((c) => c.name.toLowerCase().includes(q)).length;
  }, [filter, connections]);

  const copyText = (text: string) => {
    if (navigator.clipboard) void navigator.clipboard.writeText(text);
  };

  const queryFor = (connectionId: string, database: string, sql?: string) => {
    const id = newQueryTab(connectionId, database);
    if (sql) setQuerySql(id, sql);
  };

  const nodeMenuItems = (node: SidebarNode): MenuItem[] => {
    switch (node.kind) {
      case "database":
        return [
          {
            label: "New SQL query",
            icon: <Icon.terminal size={12} />,
            onClick: () => queryFor(node.connectionId, node.database),
          },
          {
            label: "Open ER diagram",
            icon: <Icon.diagram size={12} />,
            onClick: () => openErDiagram(node.connectionId, node.database, null),
          },
          {
            label: "Refresh schemas",
            icon: <Icon.history size={12} />,
            onClick: () => void refreshSchema(node.connectionId),
          },
          {
            label: "Compare schema…",
            icon: <Icon.diff size={12} />,
            onClick: () =>
              onCompareSchemas?.({
                connectionId: node.connectionId,
                database: node.database,
                schema: node.schemas[0]?.name,
              }),
          },
          {
            label: "Choose visible schemas…",
            icon: <Icon.eye size={12} />,
            onClick: () =>
              setSchemaManager({
                connectionId: node.connectionId,
                database: node.database,
                schemas: node.schemas,
              }),
          },
          {
            label: node.showHiddenSchemas
              ? "Hide empty schemas"
              : "Show empty schemas",
            icon: node.showHiddenSchemas ? (
              <Icon.eyeOff size={12} />
            ) : (
              <Icon.eye size={12} />
            ),
            onClick: () =>
              setDatabaseShowHidden(
                node.connectionId,
                node.database,
                !node.showHiddenSchemas,
              ),
          },
          {
            label: "Copy name",
            icon: <Icon.copy size={12} />,
            onClick: () => copyText(node.database),
          },
        ];
      case "schema":
        return [
          {
            label: "New SQL query",
            icon: <Icon.terminal size={12} />,
            onClick: () => queryFor(node.connectionId, node.database),
          },
          {
            label: "Compare schema…",
            icon: <Icon.diff size={12} />,
            onClick: () =>
              onCompareSchemas?.({
                connectionId: node.connectionId,
                database: node.database,
                schema: node.schema,
              }),
          },
          {
            label: "Open ER diagram",
            icon: <Icon.diagram size={12} />,
            onClick: () =>
              openErDiagram(node.connectionId, node.database, [node.schema]),
          },
          {
            label: node.hidden ? "Show in sidebar" : "Hide from sidebar",
            icon: node.hidden ? (
              <Icon.eye size={12} />
            ) : (
              <Icon.eyeOff size={12} />
            ),
            onClick: () =>
              node.hidden
                ? showSchema(node.connectionId, node.database, node.schema)
                : hideSchema(node.connectionId, node.database, node.schema),
          },
          {
            label: "Copy qualified name",
            icon: <Icon.copy size={12} />,
            onClick: () => copyText(qualifiedName(node.database, node.schema)),
          },
          {
            label: "Copy name",
            icon: <Icon.copy size={12} />,
            onClick: () => copyText(node.schema),
          },
        ];
      case "relation":
        return [
          {
            label: "Open",
            icon: node.isView ? (
              <Icon.tree size={12} />
            ) : (
              <Icon.table size={12} />
            ),
            onClick: () =>
              openTable(
                node.connectionId,
                node.database,
                node.schema,
                node.name,
              ),
          },
          {
            label: "Query SELECT *",
            icon: <Icon.terminal size={12} />,
            onClick: () =>
              queryFor(
                node.connectionId,
                node.database,
                selectAllStatement(node.schema, node.name),
              ),
          },
          {
            label: "Import data…",
            icon: <Icon.upload size={12} />,
            // Views aren't directly writable; only offer this on base tables.
            disabled: node.isView,
            onClick: () => {
              openTable(node.connectionId, node.database, node.schema, node.name);
              onImportData?.();
            },
          },
          {
            label: "Find Usages",
            icon: <Icon.search size={12} />,
            onClick: () =>
              findUsages({
                connectionId: node.connectionId,
                database: node.database,
                schema: node.schema,
                table: node.name,
                column: null,
              }),
          },
          {
            label: "Copy qualified name",
            icon: <Icon.copy size={12} />,
            onClick: () => copyText(qualifiedName(node.schema, node.name)),
          },
          {
            label: "Copy name",
            icon: <Icon.copy size={12} />,
            onClick: () => copyText(node.name),
          },
        ];
    }
  };

  const openNodeMenu: NodeMenuHandler = (e, node) => {
    e.preventDefault();
    e.stopPropagation();
    setMenu({ x: e.clientX, y: e.clientY, items: nodeMenuItems(node) });
  };

  const setDatabaseShowHidden = (
    connectionId: string,
    database: string,
    showHidden: boolean,
  ) => {
    const key = schemaVisibilityKey(connectionId, database);
    setSchemaVisibility((state) => ({
      ...state,
      [key]: {
        ...visibilityPrefs(state, key),
        showHidden,
      },
    }));
  };

  const hideSchema = (
    connectionId: string,
    database: string,
    schema: string,
  ) => {
    const key = schemaVisibilityKey(connectionId, database);
    setSchemaVisibility((state) => {
      const prefs = visibilityPrefs(state, key);
      const hidden = new Set(prefs.hidden);
      hidden.add(schema);
      return {
        ...state,
        [key]: {
          ...prefs,
          hidden: [...hidden].sort(),
          showHidden: false,
        },
      };
    });
  };

  const showSchema = (
    connectionId: string,
    database: string,
    schema: string,
  ) => {
    const key = schemaVisibilityKey(connectionId, database);
    setSchemaVisibility((state) => {
      const prefs = visibilityPrefs(state, key);
      return {
        ...state,
        [key]: {
          ...prefs,
          hidden: prefs.hidden.filter((name) => name !== schema),
        },
      };
    });
  };

  const setVisibleSchemas = (
    connectionId: string,
    database: string,
    schemas: Schema[],
    visible: Set<string>,
  ) => {
    const key = schemaVisibilityKey(connectionId, database);
    setSchemaVisibility((state) => {
      const prefs = visibilityPrefs(state, key);
      return {
        ...state,
        [key]: {
          ...prefs,
          hidden: schemas
            .map((schema) => schema.name)
            .filter((name) => !visible.has(name))
            .sort(),
          showHidden: true,
        },
      };
    });
  };

  const openSchemaManager = (
    connectionId: string,
    database: string,
    schemas: Schema[],
  ) => {
    setSchemaManager({ connectionId, database, schemas });
  };

  const startNewFolder = (connectionId?: string) => {
    const id = createFolder("New folder");
    if (connectionId) moveToFolder(connectionId, id);
    setRenamingFolderId(id);
  };

  const commitFolderRename = (folderId: string, name: string) => {
    renameFolder(folderId, name);
    setRenamingFolderId(null);
  };

  const openFolderMenu = (e: React.MouseEvent, folder: SidebarFolderItem) => {
    e.preventDefault();
    e.stopPropagation();
    setMenu({
      x: e.clientX,
      y: e.clientY,
      items: [
        {
          label: "Rename folder",
          icon: <Icon.edit size={12} />,
          onClick: () => setRenamingFolderId(folder.id),
        },
        {
          label:
            folder.children.length > 0
              ? "Remove folder (keep connections)"
              : "Remove folder",
          icon: <Icon.trash size={12} />,
          danger: true,
          onClick: () => removeFolder(folder.id),
        },
      ],
    });
  };

  const openConnectionMenu = (e: React.MouseEvent, config: ConnectionConfig) => {
    e.preventDefault();
    e.stopPropagation();
    const status = byId[config.id]?.status ?? "disconnected";
    const connected = status === "connected";
    const connecting = status === "connecting";
    const items: MenuItem[] = [
      {
        label: "New SQL query",
        icon: <Icon.terminal size={12} />,
        onClick: () => {
          const dbs = byId[config.id]?.databases ?? [];
          const database =
            dbs.find((d) => d.is_default)?.name ??
            dbs[0]?.name ??
            config.database;
          newQueryTab(config.id, database);
        },
      },
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
    ];
    const folders = layoutItems.filter(
      (it): it is SidebarFolderItem => it.kind === "folder",
    );
    const currentFolder =
      folders.find((f) => f.children.includes(config.id)) ?? null;
    const otherFolders = folders.filter((f) => f.id !== currentFolder?.id);
    if (otherFolders.length > 0) {
      // ponytail: second-page menu instead of real submenus; the folder list
      // reuses the same ContextMenu at the same position.
      const x = e.clientX;
      const y = e.clientY;
      items.push({
        label: "Move to folder…",
        icon: <Icon.folder size={12} />,
        onClick: () =>
          setMenu({
            x,
            y,
            items: otherFolders.map((f) => ({
              label: `Move to "${f.name}"`,
              icon: <Icon.folder size={12} />,
              onClick: () => moveToFolder(config.id, f.id),
            })),
          }),
      });
    }
    items.push({
      label: "Move to new folder",
      icon: <Icon.folderPlus size={12} />,
      onClick: () => startNewFolder(config.id),
    });
    if (currentFolder) {
      items.push({
        label: "Remove from folder",
        icon: <Icon.folderOpen size={12} />,
        onClick: () => moveToFolder(config.id, null),
      });
    }
    if (status === "connected" || status === "error") {
      items.push({
        label: status === "error" ? "Retry connection" : "Reconnect",
        icon: <Icon.history size={12} />,
        onClick: () => void reconnect(config.id),
      });
    }
    items.push(
      connecting
        ? {
            label: "Connecting...",
            icon: <Icon.history size={12} />,
            disabled: true,
            onClick: () => {},
          }
        : connected
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
          void (async () => {
            const ok = await askConfirm({
              title: "Remove connection",
              message: `Remove connection "${config.name}"?\n\nThis deletes its saved password from the keychain.`,
              confirmLabel: "Remove",
              danger: true,
            });
            if (ok) void deleteConnection(config.id);
          })();
        },
      },
    );
    setMenu({ x: e.clientX, y: e.clientY, items });
  };

  const openSidebarMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const connected = connections.filter(
      (c) => byId[c.id]?.status === "connected",
    );
    setMenu({
      x: e.clientX,
      y: e.clientY,
      items: [
        {
          label: "New connection",
          icon: <Icon.plus size={12} />,
          onClick: () => onNewConnection?.(),
        },
        {
          label: "New folder",
          icon: <Icon.folderPlus size={12} />,
          onClick: () => startNewFolder(),
        },
        {
          label: "Import from DataGrip",
          icon: <Icon.database size={12} />,
          onClick: () => onImportDatagrip?.(),
        },
        {
          label: "Refresh connected schemas",
          icon: <Icon.history size={12} />,
          disabled: connected.length === 0,
          onClick: () => {
            for (const c of connected) void refreshSchema(c.id);
          },
        },
      ],
    });
  };

  return (
    <div
      className="flex h-full flex-col text-[9px]"
      style={{ fontFamily: "var(--font-sans)" }}
    >
      <div className="flex shrink-0 items-center justify-between pt-[7px] pb-[5px] pl-2.5 pr-2">
        <div className="flex items-center gap-1.5 text-[12px] font-semibold uppercase tracking-[0.04em] text-fg-2">
          <span>Connections</span>
          <span className="rounded-[8px] bg-bg-2 px-1.5 py-px text-[11px] tabular-nums text-fg-3">
            {connections.length}
          </span>
        </div>
        <div className="flex gap-px">
          <button
            type="button"
            className="icon-btn"
            title="New connection"
            onClick={onNewConnection}
          >
            <Icon.plus size={12} />
          </button>
          <button
            type="button"
            className="icon-btn"
            title="Connection actions"
            onClick={openSidebarMenu}
          >
            <Icon.more size={12} />
          </button>
        </div>
      </div>

      <div className="mx-2 mb-1.5 flex min-h-7 shrink-0 items-center gap-1.5 rounded-[4px] border border-border-default bg-bg-inset px-2 py-1 focus-within:border-accent-line">
        <Icon.search size={11} style={{ color: "var(--fg-3)" }} />
        <input
          placeholder="Filter…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          className="flex-1 border-none bg-transparent py-0.5 text-sm leading-4 text-fg-0 outline-none placeholder:text-fg-3"
        />
        <span className="kbd">⌘F</span>
      </div>

      <div className="flex flex-1 flex-col overflow-y-auto pb-3">
        {connections.length === 0 && (
          <button
            type="button"
            onClick={onNewConnection}
            className="mx-2 mt-1 mb-3 flex w-[calc(100%-16px)] items-center gap-1.5 rounded-[4px] border border-dashed border-border-default px-2 py-1.5 text-sm text-fg-2 transition-[border-color,color,background] duration-150 hover:border-solid hover:border-accent-line hover:bg-accent-soft hover:text-accent"
          >
            <Icon.plus size={11} />
            <span>New connection</span>
          </button>
        )}

        {matchCount === 0 && (
          <div className="px-3 py-5 text-center text-[12px] text-fg-3">
            {connections.length === 0 ? "no connections yet" : "no matches"}
          </div>
        )}
        <SidebarConnectionList
          items={layoutItems}
          configs={configById}
          filter={filter}
          renamingFolderId={renamingFolderId}
          onCommitRename={commitFolderRename}
          onCancelRename={() => setRenamingFolderId(null)}
          onToggleFolder={toggleFolder}
          onFolderContextMenu={openFolderMenu}
          onMoveConnection={moveConnection}
          onMoveFolder={moveFolder}
          renderConnection={(c, drag) => {
            const state = byId[c.id];
            return (
              <ConnectionRow
                config={c}
                status={state?.status ?? "disconnected"}
                expanded={state?.expanded ?? false}
                loadingSchema={state?.loadingSchema ?? false}
                databases={state?.databases ?? []}
                error={state?.error ?? null}
                onToggle={() => toggleExpand(c.id)}
                onReconnect={() => void reconnect(c.id)}
                onDisconnect={() => void disconnect(c.id)}
                onContextMenu={(e) => openConnectionMenu(e, c)}
                onNodeContextMenu={openNodeMenu}
                onOpenTable={(database, schema, table) =>
                  openTable(c.id, database, schema, table)
                }
                activeTabId={activeTabId}
                schemaVisibility={schemaVisibility}
                onManageSchemas={openSchemaManager}
                drag={drag}
              />
            );
          }}
        />
      </div>

      <div className="flex shrink-0 items-center border-t border-border-default px-2 py-1.5">
        <button
          type="button"
          onClick={onOpenSettings}
          title="Settings (⌘,)"
          className="flex items-center gap-1.5 rounded-[4px] px-1.5 py-1 text-sm text-fg-2 transition-colors duration-150 hover:bg-bg-2 hover:text-fg-0"
        >
          <Icon.settings size={13} />
          <span>Settings</span>
        </button>
      </div>

      <ContextMenu state={menu} onClose={() => setMenu(null)} />
      {schemaManager && (
        <SchemaVisibilityManager
          state={schemaManager}
          prefs={visibilityPrefs(
            schemaVisibility,
            schemaVisibilityKey(
              schemaManager.connectionId,
              schemaManager.database,
            ),
          )}
          onClose={() => setSchemaManager(null)}
          onChangeVisible={setVisibleSchemas}
        />
      )}
    </div>
  );
}

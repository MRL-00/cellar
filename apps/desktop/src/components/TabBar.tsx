import { useState, type MouseEvent as ReactMouseEvent } from "react";

import {
  ContextMenu,
  type ContextMenuState,
  type MenuItem,
} from "./ContextMenu";
import { Icon } from "./icons";
import { qualifiedName, selectAllStatement } from "../lib/sqlIdent";
import { useConnections } from "../state/connections";
import {
  useTabs,
  tabLabel,
  type PaneIndex,
  type WorkspaceTab,
} from "../state/tabs";

/**
 * Pick the connection + database a new query tab should bind to: the active
 * tab's, else the first connected connection, else the first configured one.
 */
function pickQueryTarget(): { connectionId: string; database: string } | null {
  const tabState = useTabs.getState();
  const active = tabState.tabs.find((t) => t.id === tabState.activeId);
  if (active) {
    return { connectionId: active.connectionId, database: active.database };
  }
  const conn = useConnections.getState();
  const connectedId = conn.connections.find(
    (c) => conn.byId[c.id]?.status === "connected",
  )?.id;
  const targetId = connectedId ?? conn.connections[0]?.id;
  if (!targetId) return null;
  const cfg = conn.connections.find((c) => c.id === targetId);
  if (!cfg) return null;
  const dbs = conn.byId[targetId]?.databases ?? [];
  const database =
    dbs.find((d) => d.is_default)?.name ?? dbs[0]?.name ?? cfg.database;
  return { connectionId: targetId, database };
}

/**
 * The workspace tab strip. With no `pane`, it shows every open tab and is used
 * as the single bar above an unsplit workspace (it renders nothing while a
 * split is active — each pane draws its own strip then). With a `pane`, it
 * shows only that pane's tabs and lands new tabs in it.
 */
export function TabBar({ pane }: { pane?: PaneIndex } = {}) {
  const [menu, setMenu] = useState<ContextMenuState | null>(null);
  const [draggedTabId, setDraggedTabId] = useState<string | null>(null);
  const [tabDropTargetId, setTabDropTargetId] = useState<string | null>(null);
  const allTabs = useTabs((s) => s.tabs);
  const tabPane = useTabs((s) => s.tabPane);
  const paneActive = useTabs((s) => s.paneActive);
  const activeId = useTabs((s) => s.activeId);
  const focusedPane = useTabs((s) => s.focusedPane);
  const split = useTabs((s) => s.split);
  const closedCount = useTabs((s) => s.closedTabs.length);
  const setActive = useTabs((s) => s.setActive);
  const focusPane = useTabs((s) => s.focusPane);
  const setDraggingTab = useTabs((s) => s.setDraggingTab);
  const moveTabToPane = useTabs((s) => s.moveTabToPane);
  const closeTab = useTabs((s) => s.closeTab);
  const splitActiveTab = useTabs((s) => s.splitActiveTab);
  const reopenClosedTab = useTabs((s) => s.reopenClosedTab);
  const closeOtherTabs = useTabs((s) => s.closeOtherTabs);
  const closeTabsToRight = useTabs((s) => s.closeTabsToRight);
  const reorderTab = useTabs((s) => s.reorderTab);
  const newQueryTab = useTabs((s) => s.newQueryTab);
  const setQuerySql = useTabs((s) => s.setQuerySql);
  const refreshTable = useTabs((s) => s.refreshTable);
  const hasConnections = useConnections((s) => s.connections.length > 0);

  // The single top bar disappears once split — the panes own their strips then.
  if (pane == null && split) return null;

  const tabs =
    pane == null
      ? allTabs
      : allTabs.filter((t) => (tabPane[t.id] ?? 0) === pane);
  // Highlight this strip's active tab; the unsplit bar tracks the global one.
  const stripActiveId = pane == null ? activeId : paneActive[pane];
  const isStripFocused = pane == null || pane === focusedPane;
  const canSplit = split != null || (tabs.length > 1 && activeId != null);

  const onNewQuery = () => {
    if (pane != null) focusPane(pane);
    const target = pickQueryTarget();
    if (target) newQueryTab(target.connectionId, target.database);
  };

  const copyText = (text: string) => {
    if (navigator.clipboard) void navigator.clipboard.writeText(text);
  };

  const queryFor = (tab: WorkspaceTab, sql?: string) => {
    // Snapshot-only schema-compare tabs carry no connection; a query tab bound
    // to nothing would be useless and error on run.
    if (!tab.connectionId) return;
    const id = newQueryTab(tab.connectionId, tab.database);
    if (sql) {
      setQuerySql(id, sql);
    }
  };

  const menuItemsFor = (tab: WorkspaceTab): MenuItem[] => {
    const index = tabs.findIndex((t) => t.id === tab.id);
    const hasRightTabs = index >= 0 && index < tabs.length - 1;
    const name =
      tab.kind === "table" ? qualifiedName(tab.schema, tab.table) : tab.title;

    return [
      {
        label: "New SQL query",
        icon: <Icon.terminal size={12} />,
        disabled: !tab.connectionId,
        onClick: () => queryFor(tab),
      },
      ...(tab.kind === "table"
        ? [
            {
              label: "Refresh",
              icon: <Icon.history size={12} />,
              onClick: () => refreshTable(tab.id),
            },
            {
              label: "Query SELECT *",
              icon: <Icon.terminal size={12} />,
              onClick: () =>
                queryFor(tab, selectAllStatement(tab.schema, tab.table)),
            },
          ]
        : []),
      {
        label: tab.kind === "table" ? "Copy qualified name" : "Copy title",
        icon: <Icon.copy size={12} />,
        onClick: () => copyText(name),
      },
      {
        label: "Close",
        icon: <Icon.close size={12} />,
        onClick: () => closeTab(tab.id),
      },
      {
        label: "Close Others",
        icon: <Icon.close size={12} />,
        disabled: tabs.length < 2,
        onClick: () => closeOtherTabs(tab.id),
      },
      {
        label: "Close Tabs to the Right",
        icon: <Icon.close size={12} />,
        disabled: !hasRightTabs,
        onClick: () => closeTabsToRight(tab.id),
      },
    ];
  };

  const openTabMenu = (e: ReactMouseEvent, tab: WorkspaceTab) => {
    e.preventDefault();
    e.stopPropagation();
    setActive(tab.id);
    setMenu({ x: e.clientX, y: e.clientY, items: menuItemsFor(tab) });
  };

  return (
    <div className="flex h-[30px] items-stretch shrink-0 border-b border-border-default bg-bg-1">
      <div
        className="flex flex-1 min-w-0 overflow-x-auto"
        // Dropping a tab onto blank strip space moves it into this pane.
        onDragOver={(e) => {
          if (pane == null || !useTabs.getState().draggingTabId) return;
          e.preventDefault();
          e.dataTransfer.dropEffect = "move";
        }}
        onDrop={(e) => {
          if (pane == null) return;
          const sourceId =
            e.dataTransfer.getData("text/plain") ||
            useTabs.getState().draggingTabId;
          if (sourceId) moveTabToPane(sourceId, pane);
          setDraggingTab(null);
          setDraggedTabId(null);
          setTabDropTargetId(null);
        }}
      >
        {tabs.length === 0 && (
          <div className="inline-flex items-center px-3 text-[11px] text-fg-3">
            no tabs — double-click a table in the sidebar
          </div>
        )}
        {tabs.map((t) => {
          const isActive = t.id === stripActiveId;
          return (
            <div
              key={t.id}
              draggable
              onDragStart={(e) => {
                e.dataTransfer.effectAllowed = "move";
                e.dataTransfer.setData("text/plain", t.id);
                setDraggingTab(t.id);
                setDraggedTabId(t.id);
              }}
              onDragOver={(e) => {
                const dragging = useTabs.getState().draggingTabId;
                if (!dragging || dragging === t.id) return;
                e.preventDefault();
                e.dataTransfer.dropEffect = "move";
                setTabDropTargetId(t.id);
              }}
              onDragLeave={() => {
                setTabDropTargetId((current) => (current === t.id ? null : current));
              }}
              onDrop={(e) => {
                e.preventDefault();
                e.stopPropagation();
                const sourceId =
                  e.dataTransfer.getData("text/plain") ||
                  useTabs.getState().draggingTabId;
                if (sourceId) reorderTab(sourceId, t.id);
                if (sourceId) setActive(sourceId);
                setDraggingTab(null);
                setDraggedTabId(null);
                setTabDropTargetId(null);
              }}
              onDragEnd={() => {
                setDraggingTab(null);
                setDraggedTabId(null);
                setTabDropTargetId(null);
              }}
              onClick={() => setActive(t.id)}
              onContextMenu={(e) => openTabMenu(e, t)}
              className={
                "group relative inline-flex items-center gap-1.5 h-full pl-2.5 pr-2 max-w-[260px] shrink-0 border-r border-border-default text-[11.5px] cursor-pointer transition-[background,color] duration-100 " +
                (isActive
                  ? "bg-bg-0 text-fg-0 border-b border-bg-0 -mb-px"
                  : "bg-bg-1 text-fg-2 hover:bg-bg-2 hover:text-fg-1") +
                (draggedTabId === t.id ? " opacity-60" : "") +
                (tabDropTargetId === t.id ? " shadow-[inset_2px_0_0_var(--accent-line)]" : "")
              }
            >
              <span
                className={
                  "absolute left-0 top-0 h-full w-0.5 transition-opacity duration-150 " +
                  (isActive && isStripFocused ? "opacity-100" : "opacity-0")
                }
                style={{ background: "var(--eng-postgres)" }}
              />
              <span className="inline-flex" style={{ color: "var(--fg-1)" }}>
                {t.kind === "query" ? (
                  <Icon.terminal size={11} />
                ) : t.kind === "schema-compare" ? (
                  <Icon.diff size={11} />
                ) : t.kind === "er-diagram" ? (
                  <Icon.diagram size={11} />
                ) : (
                  <Icon.table size={11} />
                )}
              </span>
              <span className="overflow-hidden text-ellipsis whitespace-nowrap font-mono">
                {tabLabel(t)}
              </span>
              {t.kind === "query" && t.dirty && (
                <span
                  title="Unsaved edits"
                  className="h-1.5 w-1.5 shrink-0 rounded-full bg-fg-3 group-hover:opacity-0"
                />
              )}
              <button
                type="button"
                draggable={false}
                onClick={(e) => {
                  e.stopPropagation();
                  closeTab(t.id);
                }}
                title="Close"
                className={
                  "ml-0.5 inline-flex h-4 w-4 items-center justify-center rounded-[3px] text-fg-3 transition-opacity duration-100 hover:bg-bg-3 hover:text-fg-0 " +
                  (isActive ? "opacity-100" : "opacity-0 group-hover:opacity-100")
                }
              >
                <Icon.close size={10} />
              </button>
            </div>
          );
        })}
        <button
          type="button"
          onClick={onNewQuery}
          disabled={!hasConnections}
          title={
            hasConnections
              ? "New SQL query"
              : "Add a connection to open a query tab"
          }
          className="inline-flex h-full w-7 shrink-0 items-center justify-center text-fg-3 hover:bg-bg-2 hover:text-fg-0 disabled:cursor-default disabled:opacity-40 disabled:hover:bg-transparent disabled:hover:text-fg-3"
        >
          <Icon.plus size={12} />
        </button>
      </div>
      <div className="flex items-center gap-px border-l border-border-default px-1.5">
        <button
          type="button"
          className={"icon-btn" + (split === "horizontal" ? " active" : "")}
          onClick={() => splitActiveTab("horizontal")}
          disabled={!canSplit}
          aria-pressed={split === "horizontal"}
          title={
            canSplit
              ? split === "horizontal"
                ? "Close horizontal split"
                : "Split active tab horizontally"
              : "Open another tab to split the workspace"
          }
        >
          <Icon.splitH size={12} />
        </button>
        <button
          type="button"
          className={"icon-btn" + (split === "vertical" ? " active" : "")}
          onClick={() => splitActiveTab("vertical")}
          disabled={!canSplit}
          aria-pressed={split === "vertical"}
          title={
            canSplit
              ? split === "vertical"
                ? "Close vertical split"
                : "Split active tab vertically"
              : "Open another tab to split the workspace"
          }
        >
          <Icon.splitV size={12} />
        </button>
        <button
          type="button"
          className="icon-btn"
          onClick={reopenClosedTab}
          disabled={closedCount === 0}
          title={closedCount > 0 ? "Re-open closed tab" : "No closed tabs"}
        >
          <Icon.history size={12} />
        </button>
      </div>
      <ContextMenu state={menu} onClose={() => setMenu(null)} />
    </div>
  );
}

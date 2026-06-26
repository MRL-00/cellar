import { countChanges } from "@cellar/data-grid";
import { commands, unwrap } from "@cellar/ipc";
import type { QueryTemplate } from "@cellar/ipc";
import { useEffect, useMemo, useState, type ReactNode } from "react";

import { useBottomPanel, type BottomTabId } from "../../state/bottomPanel";
import { useConnections } from "../../state/connections";
import type { PanelId, Panels } from "../../state/layout";
import { useTabs, type WorkspaceTab } from "../../state/tabs";
import { Icon } from "../icons";

type Group =
  | "Actions"
  | "Templates"
  | "Tabs"
  | "Connections"
  | "Catalog"
  | "Columns"
  | "View";

type Entry = {
  id: string;
  grp: Group;
  label: string;
  hint?: string;
  kbd?: string[];
  search: string;
  action: () => void;
};

type CommandPaletteProps = {
  panels: Panels;
  onClose: () => void;
  onNewConnection: () => void;
  onOpenCommit: () => void;
  onOpenSettings: () => void;
  onTogglePanel: (k: PanelId) => void;
  onExportSetup: () => void;
  onImportSetup: () => void;
  onCompareSchemas: () => void;
};

const GROUP_ORDER: Group[] = [
  "Actions",
  "Templates",
  "Tabs",
  "Connections",
  "Catalog",
  "Columns",
  "View",
];

export function CommandPalette({
  panels,
  onClose,
  onNewConnection,
  onOpenCommit,
  onOpenSettings,
  onTogglePanel,
  onExportSetup,
  onImportSetup,
  onCompareSchemas,
}: CommandPaletteProps) {
  const [q, setQ] = useState("");
  const [active, setActive] = useState(0);
  const connections = useConnections((s) => s.connections);
  const byId = useConnections((s) => s.byId);
  const loaded = useConnections((s) => s.loaded);
  const load = useConnections((s) => s.load);
  const connect = useConnections((s) => s.connect);
  const disconnect = useConnections((s) => s.disconnect);
  const refreshSchema = useConnections((s) => s.refreshSchema);
  const tabs = useTabs((s) => s.tabs);
  const activeTabId = useTabs((s) => s.activeId);
  const tableChanges = useTabs((s) => s.tableChanges);
  const openTable = useTabs((s) => s.openTable);
  const newQueryTab = useTabs((s) => s.newQueryTab);
  const setQuerySql = useTabs((s) => s.setQuerySql);
  const setActiveTab = useTabs((s) => s.setActive);
  const clearTableChanges = useTabs((s) => s.clearTableChanges);
  const bottomTab = useBottomPanel((s) => s.active);
  const setBottomTab = useBottomPanel((s) => s.setActive);
  const [templates, setTemplates] = useState<QueryTemplate[]>([]);

  useEffect(() => {
    if (!loaded) void load();
  }, [loaded, load]);

  // Load the local query-template library when the palette opens.
  useEffect(() => {
    let cancelled = false;
    void unwrap(commands.listQueryTemplates())
      .then((list) => {
        if (!cancelled) setTemplates(list);
      })
      .catch(() => {
        if (!cancelled) setTemplates([]);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const pending = useMemo(() => {
    let total = 0;
    for (const changes of Object.values(tableChanges)) {
      total += countChanges(changes).total;
    }
    return total;
  }, [tableChanges]);

  const entries = useMemo<Entry[]>(() => {
    const list: Entry[] = [];
    const target = pickQueryTarget(tabs, activeTabId, connections, byId);

    const add = (entry: Omit<Entry, "search"> & { search?: string }) => {
      list.push({
        ...entry,
        search: `${entry.label} ${entry.hint ?? ""} ${entry.search ?? ""}`.toLowerCase(),
      });
    };

    add({
      id: "new-connection",
      grp: "Actions",
      label: "New connection",
      hint: "create a saved database connection",
      kbd: ["⌘", "N"],
      action: onNewConnection,
    });

    if (target) {
      add({
        id: "new-query",
        grp: "Actions",
        label: "New SQL query",
        hint: target.database,
        action: () => newQueryTab(target.connectionId, target.database),
      });
    }

    add({
      id: "commit",
      grp: "Actions",
      label: "Review pending changes",
      hint: pending > 0 ? `${pending} pending` : "no pending changes",
      kbd: ["⌘", "S"],
      action: onOpenCommit,
    });

    add({
      id: "compare-schemas",
      grp: "Actions",
      label: "Compare schemas…",
      hint: "diff two schemas and generate migration DDL",
      search: "schema diff migration ddl snapshot compare",
      action: onCompareSchemas,
    });

    add({
      id: "export-setup",
      grp: "Actions",
      label: "Export setup…",
      hint: "share connections, settings, layouts",
      search: "backup transfer share download",
      action: onExportSetup,
    });

    add({
      id: "import-setup",
      grp: "Actions",
      label: "Import setup…",
      hint: "load a shared setup file",
      search: "restore transfer upload merge",
      action: onImportSetup,
    });

    if (target) {
      for (const template of templates) {
        add({
          id: `template:${template.name}`,
          grp: "Templates",
          label: template.name,
          hint: template.description || "open in a new query tab",
          search: `${template.sql} saved query`,
          action: () => {
            const id = newQueryTab(target.connectionId, target.database);
            setQuerySql(id, template.sql);
          },
        });
      }
    }

    if (pending > 0) {
      add({
        id: "revert-active",
        grp: "Actions",
        label: "Revert active table changes",
        hint: "discard pending edits on the active table",
        action: () => {
          const activeTab = tabs.find((t) => t.id === activeTabId);
          if (activeTab?.kind === "table") clearTableChanges(activeTab.id);
        },
      });
    }

    for (const tab of tabs) {
      add({
        id: `tab:${tab.id}`,
        grp: "Tabs",
        label: tabLabel(tab),
        hint: tab.kind === "query" ? "query tab" : tab.database,
        search: `${tab.connectionId} ${tab.database}`,
        action: () => setActiveTab(tab.id),
      });
    }

    for (const connection of connections) {
      const state = byId[connection.id];
      const connected = state?.status === "connected";
      add({
        id: `conn:${connection.id}`,
        grp: "Connections",
        label: connection.name,
        hint: connected ? "connected" : state?.status ?? "disconnected",
        search: `${connection.engine} ${connection.host} ${connection.database}`,
        action: () => {
          if (connected) void disconnect(connection.id);
          else void connect(connection.id);
        },
      });
      if (connected) {
        add({
          id: `conn-refresh:${connection.id}`,
          grp: "Connections",
          label: `Refresh ${connection.name} schema`,
          hint: state?.loadingSchema ? "loading" : "introspect catalog",
          search: connection.database,
          action: () => void refreshSchema(connection.id),
        });
      }

      for (const db of state?.databases ?? []) {
        for (const schema of db.schemas) {
          for (const table of schema.tables) {
            const rel = `${schema.name}.${table.name}`;
            add({
              id: `table:${connection.id}:${db.name}:${rel}`,
              grp: "Catalog",
              label: rel,
              hint: `${connection.name} · ${db.name}`,
              search: `${connection.name} ${db.name} table`,
              action: () =>
                openTable(connection.id, db.name, schema.name, table.name),
            });
            for (const column of table.columns) {
              add({
                id: `column:${connection.id}:${db.name}:${rel}.${column.name}`,
                grp: "Columns",
                label: `${rel}.${column.name}`,
                hint: column.data_type,
                search: `${connection.name} ${db.name} column ${column.comment ?? ""}`,
                action: () =>
                  openTable(connection.id, db.name, schema.name, table.name),
              });
            }
          }
          for (const view of schema.views) {
            const rel = `${schema.name}.${view.name}`;
            add({
              id: `view:${connection.id}:${db.name}:${rel}`,
              grp: "Catalog",
              label: rel,
              hint: `${connection.name} · view`,
              search: `${connection.name} ${db.name} view`,
              action: () =>
                openTable(connection.id, db.name, schema.name, view.name),
            });
          }
        }
      }
    }

    addPanelEntry(add, panels.left, "left", "Connections panel", onTogglePanel);
    addPanelEntry(add, panels.bottom, "bottom", "Output panel", onTogglePanel);
    addPanelEntry(add, panels.right, "right", "AI panel", onTogglePanel);

    for (const id of ["results", "messages", "plan", "history", "notices"] as BottomTabId[]) {
      add({
        id: `bottom:${id}`,
        grp: "View",
        label: `Show ${titleCase(id)}`,
        hint: bottomTab === id ? "active" : "output panel",
        action: () => {
          setBottomTab(id);
          if (!panels.bottom) onTogglePanel("bottom");
        },
      });
    }

    add({
      id: "settings",
      grp: "View",
      label: "Open settings",
      kbd: ["⌘", ","],
      action: onOpenSettings,
    });

    return list;
  }, [
    activeTabId,
    bottomTab,
    byId,
    clearTableChanges,
    connect,
    connections,
    disconnect,
    onCompareSchemas,
    newQueryTab,
    onExportSetup,
    onImportSetup,
    onNewConnection,
    onOpenCommit,
    onOpenSettings,
    onTogglePanel,
    openTable,
    panels,
    pending,
    refreshSchema,
    setActiveTab,
    setBottomTab,
    setQuerySql,
    tabs,
    templates,
  ]);

  const filtered = useMemo(() => {
    const needle = q.trim().toLowerCase();
    if (!needle) {
      return entries.filter((e) => e.grp !== "Columns").slice(0, 80);
    }
    return entries.filter((e) => e.search.includes(needle)).slice(0, 120);
  }, [entries, q]);

  useEffect(() => {
    setActive(0);
  }, [q]);

  useEffect(() => {
    if (active >= filtered.length) setActive(Math.max(0, filtered.length - 1));
  }, [active, filtered.length]);

  const runEntry = (entry: Entry | undefined) => {
    if (!entry) return;
    entry.action();
    onClose();
  };

  const grouped = useMemo(() => {
    const groups = new Map<Group, Entry[]>();
    for (const entry of filtered) {
      const items = groups.get(entry.grp) ?? [];
      items.push(entry);
      groups.set(entry.grp, items);
    }
    return GROUP_ORDER.flatMap((grp) => {
      const items = groups.get(grp);
      return items?.length ? [[grp, items] as const] : [];
    });
  }, [filtered]);

  return (
    <div
      className="fixed inset-0 z-[100] flex items-start justify-center bg-bg-overlay pt-[14vh] backdrop-blur-[4px] animate-scrim-in"
      onClick={onClose}
    >
      <div
        className="flex w-[580px] flex-col overflow-hidden rounded-lg border border-border-default bg-bg-1 shadow-lg animate-modal-in-fast"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex h-[38px] items-center gap-[9px] border-b border-border-default px-3">
          <Icon.search size={13} stroke="var(--fg-3)" />
          <input
            placeholder="Search tables, columns, commands…"
            value={q}
            onChange={(e) => setQ(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") onClose();
              else if (e.key === "ArrowDown") {
                e.preventDefault();
                setActive((i) => Math.min(filtered.length - 1, i + 1));
              } else if (e.key === "ArrowUp") {
                e.preventDefault();
                setActive((i) => Math.max(0, i - 1));
              } else if (e.key === "Enter") {
                e.preventDefault();
                runEntry(filtered[active]);
              }
            }}
            autoFocus
            className="flex-1 border-none bg-transparent text-[13px] text-fg-0 outline-none placeholder:text-fg-3"
          />
          <span className="kbd">esc</span>
        </div>

        <div className="max-h-[420px] overflow-y-auto pt-1 pb-2">
          {grouped.map(([grp, items]) => (
            <div key={grp} className="pt-1.5 pb-1">
              <div className="px-3.5 py-0.5 text-[10px] font-semibold uppercase tracking-[0.06em] text-fg-3">
                {grp}
              </div>
              {items.map((entry) => {
                const isActive = filtered[active]?.id === entry.id;
                return (
                  <button
                    key={entry.id}
                    onClick={() => runEntry(entry)}
                    onMouseEnter={() =>
                      setActive(filtered.findIndex((e) => e.id === entry.id))
                    }
                    className={
                      "flex w-full items-center gap-2.5 px-3.5 py-1.5 text-left text-[12px] " +
                      (isActive
                        ? "bg-accent-soft text-accent"
                        : "text-fg-1 hover:bg-bg-2 hover:text-fg-0")
                    }
                  >
                    <span className="inline-flex w-[18px] shrink-0 items-center justify-center">
                      {groupIcon(entry.grp)}
                    </span>
                    <span className="min-w-0 shrink overflow-hidden text-ellipsis whitespace-nowrap font-medium">
                      {entry.label}
                    </span>
                    {entry.hint && (
                      <span className="ml-auto min-w-0 overflow-hidden text-ellipsis whitespace-nowrap pr-1.5 text-[11px] text-fg-3">
                        {entry.hint}
                      </span>
                    )}
                    {entry.kbd && (
                      <span className="inline-flex shrink-0 gap-0.5">
                        {entry.kbd.map((k, j) => (
                          <kbd key={j} className="kbd">
                            {k}
                          </kbd>
                        ))}
                      </span>
                    )}
                  </button>
                );
              })}
            </div>
          ))}
          {filtered.length === 0 && (
            <div className="px-3.5 py-5 text-center text-[11.5px] text-fg-3">
              No matches for &ldquo;{q}&rdquo;
            </div>
          )}
        </div>

        <div className="flex items-center gap-3 border-t border-border-default bg-bg-2 px-3 py-1.5 text-[10.5px] text-fg-3">
          <span className="inline-flex items-center gap-1">
            <kbd className="kbd">↑↓</kbd>
            <span>navigate</span>
          </span>
          <span className="inline-flex items-center gap-1">
            <kbd className="kbd">⏎</kbd>
            <span>select</span>
          </span>
          <div className="flex-1" />
          <span>{filtered.length} matches</span>
        </div>
      </div>
    </div>
  );
}

function groupIcon(grp: Group): ReactNode {
  switch (grp) {
    case "Actions":
      return <Icon.bolt size={11} stroke="var(--update)" />;
    case "Templates":
      return <Icon.star size={11} stroke="var(--fg-2)" />;
    case "Catalog":
      return <Icon.table size={11} stroke="var(--fg-2)" />;
    case "Columns":
      return <Icon.bracket size={11} stroke="var(--fg-2)" />;
    case "Connections":
      return <Icon.database size={11} stroke="var(--fg-2)" />;
    case "Tabs":
      return <Icon.terminal size={11} stroke="var(--fg-2)" />;
    case "View":
      return <Icon.layout size={11} stroke="var(--fg-2)" />;
  }
}

function addPanelEntry(
  add: (entry: Omit<Entry, "search"> & { search?: string }) => void,
  visible: boolean,
  panel: keyof Panels,
  label: string,
  onTogglePanel: (k: keyof Panels) => void,
) {
  add({
    id: `panel:${panel}`,
    grp: "View",
    label: `${visible ? "Hide" : "Show"} ${label}`,
    hint: visible ? "visible" : "hidden",
    action: () => onTogglePanel(panel),
  });
}

function pickQueryTarget(
  tabs: WorkspaceTab[],
  activeTabId: string | null,
  connections: ReturnType<typeof useConnections.getState>["connections"],
  byId: ReturnType<typeof useConnections.getState>["byId"],
): { connectionId: string; database: string } | null {
  const active = tabs.find((t) => t.id === activeTabId);
  if (active) return { connectionId: active.connectionId, database: active.database };
  const connected = connections.find((c) => byId[c.id]?.status === "connected");
  const target = connected ?? connections[0];
  if (!target) return null;
  const dbs = byId[target.id]?.databases ?? [];
  const database =
    dbs.find((d) => d.is_default)?.name ?? dbs[0]?.name ?? target.database;
  return { connectionId: target.id, database };
}

function tabLabel(tab: WorkspaceTab): string {
  return tab.kind === "table" ? `${tab.schema}.${tab.table}` : tab.title;
}

function titleCase(value: string): string {
  return value.slice(0, 1).toUpperCase() + value.slice(1);
}

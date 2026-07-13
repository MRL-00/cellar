import { useEffect, useMemo, useState } from "react";
import type { ConnectionConfig } from "@cellar/ipc";

import { useConnections } from "../../state/connections";
import { useTabs } from "../../state/tabs";
import { useSettings } from "../../lib/settings";
import {
  buildBundle,
  sectionCounts,
  serializeBundle,
  type SetupSectionKey,
  type SetupSelection,
} from "../../lib/setupTransfer";
import { EngineBadge, type Engine } from "../EngineBadge";
import { Icon } from "../icons";
import { ED_RUN_PRIMARY, ED_RUN_SUBTLE } from "./settingsPrimitives";
import { Modal } from "./Modal";

const APP_VERSION = "0.1.0";

type SectionMeta = {
  key: SetupSectionKey;
  label: string;
  describe: (count: number) => string;
};

const SECTIONS: SectionMeta[] = [
  {
    key: "settings",
    label: "Appearance & settings",
    describe: () => "Theme, accent, density, fonts, font size",
  },
  {
    key: "connections",
    label: "Connections",
    describe: (n) =>
      `${n} saved ${n === 1 ? "connection" : "connections"} — passwords excluded`,
  },
  {
    key: "tableLayouts",
    label: "Table grid layouts",
    describe: (n) =>
      `${n} saved ${n === 1 ? "layout" : "layouts"} (column order & widths)`,
  },
];

function todayStamp(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

function connectionHint(c: ConnectionConfig): string {
  if (c.engine === "sqlite") return c.database || c.host || "SQLite";
  const auth = c.user ? `${c.user}@` : "";
  const host = c.host || "localhost";
  const port = c.port ? `:${c.port}` : "";
  const db = c.database ? `/${c.database}` : "";
  return `${auth}${host}${port}${db}`;
}

function allSelectedMap(connections: ConnectionConfig[]): Record<string, boolean> {
  return Object.fromEntries(connections.map((c) => [c.id, true]));
}

export function ExportSetupModal({ onClose }: { onClose: () => void }) {
  const { settings } = useSettings();
  const connections = useConnections((s) => s.connections);
  const tableLayouts = useTabs((s) => s.tableLayouts);
  const [selection, setSelection] = useState<SetupSelection>({
    settings: true,
    connections: true,
    tableLayouts: true,
  });
  const [selectedConnectionIds, setSelectedConnectionIds] = useState<
    Record<string, boolean>
  >(() => allSelectedMap(connections));
  const [copied, setCopied] = useState(false);

  // Keep the selection map in sync when connections appear/disappear while the
  // modal is open. New connections default to selected.
  useEffect(() => {
    setSelectedConnectionIds((prev) => {
      const next: Record<string, boolean> = {};
      let changed = false;
      for (const c of connections) {
        if (Object.prototype.hasOwnProperty.call(prev, c.id)) {
          next[c.id] = prev[c.id]!;
        } else {
          next[c.id] = true;
          changed = true;
        }
      }
      if (!changed && Object.keys(prev).length === connections.length) return prev;
      return next;
    });
  }, [connections]);

  const sources = useMemo(
    () => ({ settings, connections, tableLayouts }),
    [settings, connections, tableLayouts],
  );
  const counts = useMemo(() => sectionCounts(sources), [sources]);

  const chosenConnections = useMemo(
    () => connections.filter((c) => selectedConnectionIds[c.id]),
    [connections, selectedConnectionIds],
  );
  const connectionIdSet = useMemo(
    () => new Set(chosenConnections.map((c) => c.id)),
    [chosenConnections],
  );
  const allConnectionsSelected =
    connections.length > 0 && chosenConnections.length === connections.length;

  const connectionsExportable =
    selection.connections && chosenConnections.length > 0;
  const anySelected =
    selection.settings || connectionsExportable || selection.tableLayouts;

  const json = useMemo(() => {
    if (!anySelected) return "";
    return serializeBundle(
      buildBundle(
        {
          ...selection,
          // Treat "connections on but none picked" as off so the section is
          // omitted rather than written as an empty array.
          connections: connectionsExportable,
        },
        sources,
        {
          app: APP_VERSION,
          exportedAt: new Date().toISOString(),
        },
        selection.connections
          ? { connectionIds: connectionIdSet }
          : undefined,
      ),
    );
  }, [
    anySelected,
    selection,
    sources,
    connectionsExportable,
    connectionIdSet,
  ]);

  const toggle = (key: SetupSectionKey) =>
    setSelection((s) => ({ ...s, [key]: !s[key] }));

  const toggleAllConnections = () => {
    const next = !allConnectionsSelected;
    setSelectedConnectionIds(
      Object.fromEntries(connections.map((c) => [c.id, next])),
    );
  };

  const onDownload = () => {
    if (!json) return;
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `cellar-setup-${todayStamp()}.json`;
    document.body.appendChild(a);
    a.click();
    a.remove();
    // Revoke on the next tick so the download has time to start.
    setTimeout(() => URL.revokeObjectURL(url), 0);
  };

  const onCopy = async () => {
    if (!json) return;
    try {
      await navigator.clipboard.writeText(json);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      setCopied(false);
    }
  };

  return (
    <Modal onClose={onClose} width={560}>
      <div className="flex h-[38px] shrink-0 items-center justify-between border-b border-border-default pl-3.5 pr-2">
        <div className="flex items-center gap-2">
          <span className="inline-flex text-accent">
            <Icon.download size={14} />
          </span>
          <span className="whitespace-nowrap text-sm font-semibold text-fg-0">
            Export setup
          </span>
        </div>
        <button className="icon-btn" onClick={onClose} title="Close">
          <Icon.close size={13} />
        </button>
      </div>

      <div className="max-h-[calc(90vh_-_160px)] flex-1 overflow-y-auto px-4 py-3.5">
        <p className="m-0 mb-3 max-w-[52ch] text-sm text-fg-2 text-pretty">
          Pick what to include, then download a <code>.json</code> file you can
          share or move to another machine.
        </p>

        <div className="flex flex-col gap-1.5">
          {SECTIONS.map((section) => {
            const count = counts[section.key];
            const on = selection[section.key];
            const empty = count === 0;
            const isConnections = section.key === "connections";
            const badgeCount =
              isConnections && on && !empty
                ? chosenConnections.length
                : count;
            return (
              <div key={section.key} className="flex flex-col gap-1">
                <button
                  type="button"
                  onClick={() => !empty && toggle(section.key)}
                  disabled={empty}
                  className={
                    "flex items-start gap-2.5 rounded-[5px] border px-3 py-2 text-left transition-colors " +
                    (empty
                      ? "cursor-not-allowed border-border-default bg-bg-inset opacity-60"
                      : on
                        ? "border-accent-line bg-accent-soft"
                        : "border-border-default bg-bg-2 hover:border-border-strong")
                  }
                >
                  <span
                    className={
                      "mt-px inline-flex h-[15px] w-[15px] shrink-0 items-center justify-center rounded-[4px] border " +
                      (on && !empty
                        ? "border-accent bg-accent text-accent-fg"
                        : "border-border-strong bg-bg-inset")
                    }
                  >
                    {on && !empty && <Icon.check size={10} />}
                  </span>
                  <span className="min-w-0">
                    <span className="flex items-center gap-1.5 text-sm font-medium text-fg-0">
                      {section.label}
                      <span className="font-mono text-[11px] text-fg-3">
                        {empty
                          ? "none saved"
                          : isConnections && on && chosenConnections.length !== count
                            ? `${chosenConnections.length}/${count}`
                            : `×${badgeCount}`}
                      </span>
                    </span>
                    <span className="block text-[11.5px] text-fg-3">
                      {isConnections && on && !empty
                        ? `${chosenConnections.length} of ${count} selected — passwords excluded`
                        : section.describe(count)}
                    </span>
                  </span>
                </button>

                {isConnections && on && !empty && (
                  <div className="ml-2 flex flex-col gap-1 border-l border-border-default pl-2.5">
                    <div className="flex items-center justify-end px-0.5">
                      <button
                        type="button"
                        onClick={toggleAllConnections}
                        className="text-[11.5px] font-medium text-accent hover:underline"
                      >
                        {allConnectionsSelected ? "Unselect all" : "Select all"}
                      </button>
                    </div>
                    {connections.map((c) => {
                      const picked = !!selectedConnectionIds[c.id];
                      return (
                        <label
                          key={c.id}
                          className={
                            "flex cursor-pointer items-center gap-2 rounded-[5px] border px-2.5 py-1.5 transition-colors " +
                            (picked
                              ? "border-border-default bg-bg-2"
                              : "border-border-default bg-bg-inset opacity-70")
                          }
                        >
                          <input
                            type="checkbox"
                            checked={picked}
                            onChange={(e) =>
                              setSelectedConnectionIds((s) => ({
                                ...s,
                                [c.id]: e.target.checked,
                              }))
                            }
                          />
                          <EngineBadge engine={c.engine as Engine} size={14} />
                          <span className="min-w-0 flex-1">
                            <span className="block truncate text-sm font-medium text-fg-0">
                              {c.name || c.host || c.id}
                            </span>
                            <span className="block truncate font-mono text-[11px] text-fg-3">
                              {connectionHint(c)}
                            </span>
                          </span>
                        </label>
                      );
                    })}
                    {selection.tableLayouts && (
                      <p className="m-0 px-0.5 pt-0.5 text-[11px] text-fg-3">
                        Table layouts for unselected connections are omitted.
                      </p>
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>

        <div className="mt-3 flex items-center gap-1.5 rounded-[4px] border border-dashed border-border-default bg-bg-inset px-3 py-2 text-sm text-fg-2">
          <Icon.lock size={12} stroke="var(--fg-3)" />
          <span>
            Passwords and API keys are never exported — recipients re-enter their
            own.
          </span>
        </div>
      </div>

      <div className="flex h-11 shrink-0 items-center justify-between gap-3 border-t border-border-default bg-bg-2 px-3">
        <span className="font-mono text-[11.5px] text-fg-3">
          {anySelected ? `${json.length.toLocaleString()} bytes` : "nothing selected"}
        </span>
        <div className="flex items-center gap-2">
          <button className={ED_RUN_SUBTLE} onClick={onClose}>
            Cancel
          </button>
          <button
            className={ED_RUN_SUBTLE + " disabled:cursor-not-allowed disabled:opacity-40"}
            onClick={() => void onCopy()}
            disabled={!anySelected}
          >
            <Icon.copy size={11} />
            <span>{copied ? "Copied" : "Copy JSON"}</span>
          </button>
          <button
            className={ED_RUN_PRIMARY + " disabled:cursor-not-allowed disabled:opacity-40"}
            onClick={onDownload}
            disabled={!anySelected}
          >
            <Icon.download size={11} />
            <span>Download .json</span>
          </button>
        </div>
      </div>
    </Modal>
  );
}

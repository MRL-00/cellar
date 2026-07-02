import { useEffect, useMemo, useState } from "react";
import type { DatagripImport } from "@cellar/ipc";
import { commands, unwrap } from "@cellar/ipc";

import { Icon } from "../icons";
import { EngineBadge, type Engine } from "../EngineBadge";
import { Modal } from "./Modal";
import { useConnections } from "../../state/connections";

const ROW_INPUT =
  "h-[24px] rounded-[4px] border border-border-default bg-bg-inset px-2 text-sm font-mono text-fg-0 outline-none focus:border-accent-line focus:bg-bg-2 disabled:opacity-40";

const BTN_BASE =
  "inline-flex h-[26px] items-center gap-[5px] whitespace-nowrap rounded-[4px] border px-2.5 text-sm font-medium transition-[background,color,border-color,filter] duration-[120ms]";
const BTN_SUBTLE =
  BTN_BASE +
  " text-fg-1 bg-transparent border-border-default hover:bg-bg-3 hover:border-border-strong hover:text-fg-0";
const BTN_PRIMARY =
  BTN_BASE +
  " bg-accent text-accent-fg border-transparent hover:brightness-[1.07] disabled:opacity-40 disabled:cursor-not-allowed";

type Load =
  | { kind: "loading" }
  | { kind: "error"; message: string }
  | { kind: "ready"; data: DatagripImport };

export function ImportDatagripDialog({ onClose }: { onClose: () => void }) {
  const saveConnection = useConnections((s) => s.saveConnection);
  const existingIds = useConnections((s) => s.connections);
  const existing = useMemo(
    () => new Set(existingIds.map((c) => c.id)),
    [existingIds],
  );

  const [load, setLoad] = useState<Load>({ kind: "loading" });
  const [selected, setSelected] = useState<Record<string, boolean>>({});
  const [passwords, setPasswords] = useState<Record<string, string>>({});
  // Editable per-row database override. DataGrip's JDBC URLs often omit the
  // database (esp. SQL Server/Azure), so we default to the parsed value but let
  // the user point each connection at the real database before importing.
  const [databases, setDatabases] = useState<Record<string, string>>({});
  const [importing, setImporting] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    void (async () => {
      try {
        const data = await unwrap(commands.importDatagrip());
        if (!alive) return;
        setLoad({ kind: "ready", data });
        // Default to importing everything, prefilled with the parsed database.
        const sel: Record<string, boolean> = {};
        const dbs: Record<string, string> = {};
        for (const c of data.connections) {
          sel[c.id] = true;
          dbs[c.id] = c.database;
        }
        setSelected(sel);
        setDatabases(dbs);
      } catch (err) {
        if (alive)
          setLoad({
            kind: "error",
            message: err instanceof Error ? err.message : String(err),
          });
      }
    })();
    return () => {
      alive = false;
    };
  }, []);

  const conns = load.kind === "ready" ? load.data.connections : [];
  const skipped = load.kind === "ready" ? load.data.skipped : [];
  const chosen = conns.filter((c) => selected[c.id]);
  const allSelected = conns.length > 0 && chosen.length === conns.length;

  const toggleAll = () => {
    const next = !allSelected;
    setSelected(Object.fromEntries(conns.map((c) => [c.id, next])));
  };

  const onImport = async () => {
    setImportError(null);
    setImporting(true);
    try {
      for (const c of chosen) {
        const database = (databases[c.id] ?? c.database).trim() || c.database;
        await saveConnection({ ...c, database }, passwords[c.id] || null);
      }
      onClose();
    } catch (err) {
      setImportError(err instanceof Error ? err.message : String(err));
    } finally {
      setImporting(false);
    }
  };

  return (
    <Modal onClose={onClose} width={760}>
      <div className="flex h-[38px] shrink-0 items-center justify-between border-b border-border-default pl-3.5 pr-2">
        <div className="flex items-center gap-2">
          <span className="inline-flex text-accent">
            <Icon.database size={14} />
          </span>
          <span className="whitespace-nowrap text-sm font-semibold text-fg-0">
            Import from DataGrip
          </span>
        </div>
        <button className="icon-btn" onClick={onClose} title="Close">
          <Icon.close size={13} />
        </button>
      </div>

      {/* Cap the scroll area at a screen-relative height so the list grows with
          the window but the footer (Import button) never gets pushed off-screen.
          The parent Modal's max-h doesn't reliably clamp in the webview, so we
          bound the list directly. Reserve room for the 8vh top gap + header +
          footer AND a bottom margin big enough to clear the macOS dock (100vh
          includes the strip behind it). NB: calc needs spaces → underscores. */}
      <div className="max-h-[calc(90vh_-_200px)] min-h-0 flex-1 overflow-y-auto px-4 pt-3 pb-4">
        {load.kind === "loading" && (
          <div className="px-2 py-8 text-center text-sm text-fg-3">
            Scanning DataGrip…
          </div>
        )}

        {load.kind === "error" && (
          <div className="px-2 py-8 text-center text-sm text-warn">
            {load.message}
          </div>
        )}

        {load.kind === "ready" && conns.length === 0 && (
          <div className="px-2 py-8 text-center text-sm text-fg-3">
            No importable DataGrip connections found.
          </div>
        )}

        {load.kind === "ready" && conns.length > 0 && (
          <div className="flex flex-col gap-1.5">
            <div className="mb-1 flex items-center justify-between gap-3">
              <span className="text-[12px] text-fg-3">
                Passwords aren&apos;t stored by DataGrip — add them now or on
                first connect.
              </span>
              <button
                type="button"
                onClick={toggleAll}
                className="shrink-0 text-sm font-medium text-accent hover:underline"
              >
                {allSelected ? "Unselect all" : "Select all"}
              </button>
            </div>
            {conns.map((c) => {
              const exists = existing.has(c.id);
              const on = !!selected[c.id];
              return (
                <label
                  key={c.id}
                  className="flex items-center gap-2.5 rounded-[5px] border border-border-default bg-bg-2 px-2.5 py-1.5"
                >
                  <input
                    type="checkbox"
                    checked={on}
                    onChange={(e) =>
                      setSelected((s) => ({ ...s, [c.id]: e.target.checked }))
                    }
                  />
                  <EngineBadge engine={c.engine as Engine} size={16} />
                  <div className="flex min-w-0 flex-1 flex-col">
                    <span className="truncate text-sm font-medium text-fg-0">
                      {c.name}
                      {exists && (
                        <span className="ml-2 text-[11px] font-normal text-warn">
                          overwrites existing
                        </span>
                      )}
                    </span>
                    <span className="truncate font-mono text-[11px] text-fg-3">
                      {c.user ? `${c.user}@` : ""}
                      {c.host}:{c.port}
                    </span>
                  </div>
                  <input
                    className={ROW_INPUT + " w-[130px]"}
                    placeholder="database"
                    title="Database to connect to"
                    value={databases[c.id] ?? c.database}
                    disabled={!on}
                    onChange={(e) =>
                      setDatabases((d) => ({ ...d, [c.id]: e.target.value }))
                    }
                  />
                  <input
                    className={ROW_INPUT + " w-[150px]"}
                    type="password"
                    placeholder="password (optional)"
                    value={passwords[c.id] ?? ""}
                    disabled={!on}
                    autoComplete="new-password"
                    onChange={(e) =>
                      setPasswords((p) => ({ ...p, [c.id]: e.target.value }))
                    }
                  />
                </label>
              );
            })}

            {skipped.length > 0 && (
              <div className="mt-2 rounded-[5px] border border-border-default bg-bg-inset px-2.5 py-2 text-[11.5px] text-fg-3">
                <div className="mb-1 font-medium text-fg-2">
                  Skipped {skipped.length}
                </div>
                {skipped.map((s, i) => (
                  <div key={i} className="truncate font-mono">
                    {s}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>

      <div className="flex h-11 shrink-0 items-center justify-between gap-3 border-t border-border-default bg-bg-2 px-3">
        <span className="text-[12px] text-fg-3">
          {load.kind === "ready" && conns.length > 0
            ? `${chosen.length} of ${conns.length} selected`
            : ""}
        </span>
        <div className="flex items-center gap-2">
          {importError && (
            <span className="text-[12px] text-warn">{importError}</span>
          )}
          <button className={BTN_SUBTLE} onClick={onClose}>
            Cancel
          </button>
          <button
            className={BTN_PRIMARY}
            disabled={importing || chosen.length === 0}
            onClick={() => void onImport()}
          >
            <Icon.plus size={11} />
            <span>
              {importing ? "Importing…" : `Import ${chosen.length || ""}`}
            </span>
          </button>
        </div>
      </div>
    </Modal>
  );
}

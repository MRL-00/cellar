import { useEffect, useMemo, useState } from "react";
import type { PendingChange, PendingChanges } from "@cellar/data-grid";
import {
  commands,
  unwrap,
  type CellAssignment,
  type Table,
  type TableChangeRequest,
  type TableCommitPreview,
} from "@cellar/ipc";

import { Icon } from "../icons";
import { Modal } from "./Modal";
import { tokenizeSql, tokensToLines, renderTokens } from "../../lib/sqlTokens";
import { useConnections } from "../../state/connections";
import { useStatus } from "../../state/status";
import { useTabs, type TableTab } from "../../state/tabs";

type PreviewState =
  | { kind: "idle" | "loading"; preview: null; error: null }
  | { kind: "ready"; preview: TableCommitPreview; error: null }
  | { kind: "error"; preview: null; error: string };

const EMPTY_CHANGES: PendingChanges = {};

const ED_RUN_BASE =
  "inline-flex h-[26px] items-center gap-[5px] whitespace-nowrap rounded-[4px] border border-transparent px-2.5 text-[12.5px] font-medium text-fg-1 transition-[background,color,border-color,filter] duration-[120ms]";

const ED_RUN_SUBTLE =
  ED_RUN_BASE +
  " bg-transparent border-border-default hover:bg-bg-3 hover:border-border-strong hover:text-fg-0";

const ED_RUN_DANGER =
  ED_RUN_BASE +
  " bg-delete text-white hover:brightness-[1.07] disabled:cursor-not-allowed disabled:opacity-60";

export function CommitModal({ onClose }: { onClose: () => void }) {
  const activeId = useTabs((s) => s.activeId);
  const tabs = useTabs((s) => s.tabs);
  const tableChanges = useTabs((s) => s.tableChanges);
  const clearTableChanges = useTabs((s) => s.clearTableChanges);
  const refreshTable = useTabs((s) => s.refreshTable);
  const connections = useConnections((s) => s.connections);
  const byId = useConnections((s) => s.byId);

  const active = tabs.find((t) => t.id === activeId) ?? null;
  const activeTable = active?.kind === "table" ? active : null;
  const changes = activeTable
    ? tableChanges[activeTable.id] ?? EMPTY_CHANGES
    : EMPTY_CHANGES;
  const entries = Object.entries(changes);
  const activeConn = activeTable
    ? connections.find((c) => c.id === activeTable.connectionId) ?? null
    : null;
  const tableMeta = activeTable ? findTableMeta(activeTable) : null;

  const blockers = useMemo(
    () => blockersFor(activeTable, tableMeta, entries.length),
    [activeTable, tableMeta, entries.length],
  );
  const request = useMemo(
    () =>
      activeTable && tableMeta && blockers.length === 0
        ? buildRequest(activeTable, tableMeta, changes)
        : null,
    [activeTable, tableMeta, blockers.length, changes],
  );
  const [previewState, setPreviewState] = useState<PreviewState>({
    kind: "idle",
    preview: null,
    error: null,
  });
  const [commitError, setCommitError] = useState<string | null>(null);
  const [committing, setCommitting] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setCommitError(null);
    if (!request) {
      setPreviewState({ kind: "idle", preview: null, error: null });
      return;
    }
    setPreviewState({ kind: "loading", preview: null, error: null });
    void (async () => {
      try {
        const preview = await unwrap(commands.previewTableChanges(request));
        if (!cancelled) setPreviewState({ kind: "ready", preview, error: null });
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        if (!cancelled) {
          setPreviewState({ kind: "error", preview: null, error: message });
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [request]);

  const preview = previewState.preview;
  const sqlText = preview?.sql ?? "";
  const lines = preview ? tokensToLines(tokenizeSql(sqlText)) : [];
  const updates = entries.filter(([, c]) => c.kind === "update");
  const inserts = entries.filter(([, c]) => c.kind === "insert");
  const deletes = entries.filter(([, c]) => c.kind === "delete");
  const canCommit = !!activeTable && !!request && !!preview && !committing;

  useEffect(() => {
    if (!canCommit) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Enter" || event.metaKey || event.ctrlKey || event.altKey) {
        return;
      }
      const target = event.target;
      if (target instanceof HTMLElement && isTextInput(target)) {
        return;
      }
      event.preventDefault();
      void handleCommit();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [canCommit, activeTable, request, preview]);

  async function handleCommit() {
    if (!activeTable || !request || !preview) return;
    setCommitting(true);
    setCommitError(null);
    try {
      const result = await unwrap(
        commands.commitTableChanges(activeTable.connectionId, request, activeTable.id),
      );
      useStatus.getState().setLastQuery({
        connectionId: activeTable.connectionId,
        tabId: activeTable.id,
        rowCount: result.rows_affected,
        truncated: false,
        durationMs: result.duration_ms,
      });
      clearTableChanges(activeTable.id);
      refreshTable(activeTable.id);
      onClose();
    } catch (err) {
      setCommitError(err instanceof Error ? err.message : String(err));
    } finally {
      setCommitting(false);
    }
  }

  return (
    <Modal onClose={onClose} width={880}>
      <div className="flex h-[38px] shrink-0 items-center justify-between border-b border-border-default pl-3.5 pr-2">
        <div className="flex min-w-0 items-center gap-2">
          <span className="inline-flex text-accent">
            <Icon.commit size={14} />
          </span>
          <span className="whitespace-nowrap text-sm font-semibold text-fg-0">
            Review &amp; commit
          </span>
          <span className="ml-1 min-w-0 overflow-hidden text-ellipsis whitespace-nowrap border-l border-border-divider pl-1.5 font-mono text-[12px] text-fg-2">
            {activeTable ? `${activeTable.schema}.${activeTable.table}` : "no table"}{" "}
            <span style={{ color: "var(--fg-3)" }}>·</span>{" "}
            {activeConn?.name ?? "no connection"}
            {activeConn?.env_tag ? ` (${activeConn.env_tag})` : ""}
          </span>
        </div>
        <button type="button" className="icon-btn" onClick={onClose} title="Close">
          <Icon.close size={13} />
        </button>
      </div>

      <div className="flex shrink-0 items-center gap-4 border-b border-border-default bg-bg-2 px-4 py-2.5">
        <SummaryItem icon={<Icon.plus size={11} />} bg="var(--insert-bg)" color="var(--insert)" n={inserts.length} label={`insert${inserts.length === 1 ? "" : "s"}`} />
        <SummaryItem icon={<Icon.diff size={11} />} bg="var(--update-bg)" color="var(--update)" n={updates.length} label={`update${updates.length === 1 ? "" : "s"}`} />
        <SummaryItem icon={<Icon.close size={11} />} bg="var(--delete-bg)" color="var(--delete)" n={deletes.length} label={`delete${deletes.length === 1 ? "" : "s"}`} />
        <div className="flex-1" />
        <span className="inline-flex items-center gap-1.5 whitespace-nowrap rounded-[4px] bg-bg-inset px-2 py-1 font-mono text-[11.5px] text-fg-2">
          <Icon.bracket size={10} />
          <span>BEGIN ... COMMIT - atomic</span>
        </span>
      </div>

      <div className="grid min-h-0 flex-1 overflow-hidden grid-cols-[320px_1fr]">
        <div className="flex min-h-0 flex-col border-r border-border-default">
          <div className="flex h-[26px] shrink-0 items-center gap-1.5 border-b border-border-divider px-3 text-[11px] font-semibold uppercase tracking-[0.05em] text-fg-3">
            <span>Changes</span>
            <span className="rounded-[8px] bg-bg-2 px-1.5 py-px font-mono text-[11px] text-fg-2">
              {entries.length}
            </span>
          </div>
          <div className="flex-1 overflow-y-auto py-1.5">
            {entries.length === 0 ? (
              <div className="px-3 py-3 text-[12px] text-fg-3">
                No pending changes on the active table.
              </div>
            ) : (
              entries.map(([rowId, change]) => (
                <ChangePreview key={rowId} rowId={rowId} change={change} />
              ))
            )}
          </div>
        </div>

        <div className="flex min-h-0 flex-col bg-bg-inset">
          <div className="flex h-[26px] shrink-0 items-center justify-between border-b border-border-divider bg-bg-1 px-3 text-[11px] font-semibold uppercase tracking-[0.05em] text-fg-3">
            <span>Generated SQL</span>
            <button
              type="button"
              disabled={!preview}
              onClick={() => void navigator.clipboard?.writeText(sqlText)}
              className="inline-flex h-[26px] items-center gap-1 rounded-[4px] border border-border-default bg-bg-2 px-2 text-[12px] text-fg-1 hover:bg-bg-3 disabled:cursor-not-allowed disabled:opacity-60"
            >
              <Icon.copy size={11} />
              <span>Copy</span>
            </button>
          </div>
          <div className="flex-1 overflow-auto py-2 font-mono text-sm leading-[1.55]">
            {blockers.length > 0 ? (
              <MessageList messages={blockers} tone="warn" />
            ) : previewState.kind === "loading" ? (
              <div className="px-3 text-[12px] text-fg-3">Generating preview...</div>
            ) : previewState.kind === "error" ? (
              <MessageList messages={[previewState.error]} tone="warn" />
            ) : (
              lines.map((toks, i) => (
                <div key={i} className="flex px-3">
                  <span className="inline-flex w-7 shrink-0 select-none items-center justify-end pr-2.5 font-variant-numeric-tabular text-[11px] text-fg-3 font-mono">
                    {i + 1}
                  </span>
                  <span className="whitespace-pre font-mono">
                    {renderTokens(toks)}
                  </span>
                </div>
              ))
            )}
          </div>
        </div>
      </div>

      <div className="flex min-h-11 shrink-0 items-center justify-between gap-3 border-t border-border-default bg-bg-2 px-3 py-2">
        <div className="flex min-w-0 items-center gap-2 text-[11.5px]">
          <Icon.warn size={10} stroke={activeConn?.env_tag === "prod" ? "var(--warn)" : "var(--fg-3)"} />
          <span className={activeConn?.env_tag === "prod" ? "text-warn" : "text-fg-2"}>
            {activeConn?.env_tag === "prod" ? "prod" : "transaction"}
          </span>
          <span className="min-w-0 text-fg-2">
            {commitError ??
              "commits rollback on error and require the expected row count"}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <button type="button" className={ED_RUN_SUBTLE} onClick={onClose}>
            Cancel
          </button>
          <button
            type="button"
            disabled
            title="Migration export is not wired yet"
            className={ED_RUN_SUBTLE + " cursor-not-allowed opacity-60"}
          >
            <Icon.undo size={11} />
            <span>Save as migration</span>
          </button>
          <button
            type="button"
            disabled={!canCommit}
            onClick={() => void handleCommit()}
            className={ED_RUN_DANGER}
            style={{
              borderColor: "color-mix(in oklab, var(--delete) 40%, black)",
            }}
          >
            <Icon.commit size={11} />
            <span>{committing ? "Committing..." : "Commit transaction"}</span>
            <kbd className="kbd ml-1">Return</kbd>
          </button>
        </div>
      </div>
    </Modal>
  );

  function findTableMeta(tab: TableTab): Table | null {
    return (
      byId[tab.connectionId]?.databases
        .find((d) => d.name === tab.database)
        ?.schemas.find((s) => s.name === tab.schema)
        ?.tables.find((t) => t.name === tab.table) ?? null
    );
  }
}

function ChangePreview({
  rowId,
  change,
}: {
  rowId: string;
  change: PendingChange;
}) {
  const tag = change.kind.toUpperCase();
  const tagBg =
    change.kind === "insert"
      ? "var(--insert-bg)"
      : change.kind === "delete"
        ? "var(--delete-bg)"
        : "var(--update-bg)";
  const tagColor =
    change.kind === "insert"
      ? "var(--insert)"
      : change.kind === "delete"
        ? "var(--delete)"
        : "var(--update)";
  const label = rowLabel(rowId);

  return (
    <ChangeRow tag={tag} tagBg={tagBg} tagColor={tagColor}>
      <div className="flex items-center gap-1.5 text-sm">
        <span className="font-mono font-medium text-fg-0">{label}</span>
      </div>
      {Object.entries(change.edits).map(([col, e]) => (
        <div
          key={col}
          className="grid items-center gap-[5px] pl-1 font-mono text-sm grid-cols-[90px_auto_auto_auto]"
        >
          <span className="overflow-hidden text-ellipsis text-fg-2">{col}</span>
          <span
            className="overflow-hidden text-ellipsis rounded-[3px] px-1.5 py-px text-delete line-through"
            style={{
              background: "var(--delete-bg)",
              textDecorationColor: "rgba(255, 255, 255, 0.2)",
            }}
          >
            {formatVal(e.from)}
          </span>
          <Icon.chevronRight size={10} stroke="var(--fg-3)" />
          <span className="overflow-hidden text-ellipsis rounded-[3px] bg-accent-soft px-1.5 py-px text-accent">
            {formatVal(e.to)}
          </span>
        </div>
      ))}
    </ChangeRow>
  );
}

function buildRequest(
  tab: TableTab,
  table: Table,
  changes: PendingChanges,
): TableChangeRequest {
  return {
    database: tab.database,
    schema: tab.schema,
    table: tab.table,
    primary_key: table.primary_key,
    columns: table.columns.map((c) => ({
      name: c.name,
      data_type: c.data_type,
      nullable: c.nullable,
    })),
    changes: Object.entries(changes).map(([rowId, change]) => {
      if (change.kind === "insert") {
        return {
          kind: "insert",
          row_id: rowId,
          values: editsToAssignments(change),
        };
      }
      if (change.kind === "delete") {
        return {
          kind: "delete",
          row_id: rowId,
          keys: decodeRowKeys(rowId),
        };
      }
      return {
        kind: "update",
        row_id: rowId,
        keys: decodeRowKeys(rowId),
        edits: editsToAssignments(change),
      };
    }),
  };
}

function blockersFor(
  active: TableTab | null,
  table: Table | null,
  changeCount: number,
): string[] {
  if (!active) return ["Open a table before reviewing changes."];
  if (changeCount === 0) return ["There are no pending changes to commit."];
  if (!table) return ["Schema metadata is still unavailable for this table."];
  if (table.primary_key.length === 0) {
    return ["Cellar needs a primary key before it can safely commit grid edits."];
  }
  return [];
}

function editsToAssignments(change: PendingChange): CellAssignment[] {
  return Object.entries(change.edits).map(([column, edit]) => ({
    column,
    value: { value: primitiveToString(edit.to) },
  }));
}

function decodeRowKeys(rowId: string): CellAssignment[] {
  try {
    const parsed = JSON.parse(rowId) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed
      .map((entry) => {
        if (!entry || typeof entry !== "object") return null;
        const item = entry as { column?: unknown; value?: unknown };
        if (typeof item.column !== "string") return null;
        return {
          column: item.column,
          value: { value: primitiveToString(item.value) },
        };
      })
      .filter((v): v is CellAssignment => v !== null);
  } catch {
    return [];
  }
}

function rowLabel(rowId: string): string {
  const keys = decodeRowKeys(rowId);
  if (keys.length === 0) return rowId;
  return keys
    .map((k) => `${k.column}=${k.value.value ?? "NULL"}`)
    .join(", ");
}

function primitiveToString(value: unknown): string | null {
  if (value === null || value === undefined) return null;
  return String(value);
}

function isTextInput(target: HTMLElement): boolean {
  return (
    target.tagName === "INPUT" ||
    target.tagName === "TEXTAREA" ||
    target.isContentEditable
  );
}

function formatVal(v: unknown): string {
  if (v === null || v === undefined) return "NULL";
  if (typeof v === "number") return String(v);
  if (typeof v === "boolean") return v ? "TRUE" : "FALSE";
  return "'" + String(v).replaceAll("'", "''") + "'";
}

function MessageList({
  messages,
  tone,
}: {
  messages: string[];
  tone: "warn" | "muted";
}) {
  return (
    <div className={tone === "warn" ? "px-3 text-[12px] text-warn" : "px-3 text-[12px] text-fg-3"}>
      {messages.map((message) => (
        <div key={message}>{message}</div>
      ))}
    </div>
  );
}

function SummaryItem({
  icon,
  bg,
  color,
  n,
  label,
}: {
  icon: React.ReactNode;
  bg: string;
  color: string;
  n: number;
  label: string;
}) {
  return (
    <div className="flex items-center gap-1.5">
      <span
        className="inline-flex h-[22px] w-[22px] items-center justify-center rounded-[4px]"
        style={{ background: bg, color }}
      >
        {icon}
      </span>
      <span className="font-mono text-[15px] font-semibold text-fg-0 tabular-nums">
        {n}
      </span>
      <span className="text-sm text-fg-2">{label}</span>
    </div>
  );
}

function ChangeRow({
  tag,
  tagBg,
  tagColor,
  children,
}: {
  tag: string;
  tagBg: string;
  tagColor: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex gap-2 border-b border-dashed border-border-divider px-3 py-1.5">
      <span
        className="mt-px inline-flex h-[14px] items-center self-start rounded-[3px] px-1 py-px font-mono text-[10.5px] font-semibold tracking-[0.04em]"
        style={{ background: tagBg, color: tagColor }}
      >
        {tag}
      </span>
      <div className="flex min-w-0 flex-1 flex-col gap-[3px]">{children}</div>
    </div>
  );
}

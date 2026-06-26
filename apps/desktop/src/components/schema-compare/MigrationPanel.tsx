import { useMemo, useState } from "react";
import { SqlCodeEditor } from "@cellar/sql-editor";
import type { MigrationKind, MigrationStatement } from "@cellar/ipc";

import { Icon } from "../icons";
import { useConnections } from "../../state/connections";
import { useSchemaCompare } from "../../state/schemaCompare";

/** The live destination a migration can be applied to (the comparison source). */
export interface ApplyTarget {
  connectionId: string;
  database: string;
  envTag: string | null;
}

const ED_BTN =
  "inline-flex h-[26px] items-center gap-[5px] whitespace-nowrap rounded-[4px] border border-border-default bg-bg-2 px-2.5 text-[11.5px] font-medium text-fg-1 transition-colors duration-100 hover:bg-bg-3 hover:text-fg-0 disabled:cursor-not-allowed disabled:opacity-60";
const ED_DANGER =
  "inline-flex h-[26px] items-center gap-[5px] whitespace-nowrap rounded-[4px] border px-2.5 text-[11.5px] font-medium text-white bg-delete transition-[filter] duration-100 hover:brightness-[1.07] disabled:cursor-not-allowed disabled:opacity-60";

/**
 * Right-hand panel of the schema-compare view: pick which generated DDL
 * statements to include, regenerate the script, review/edit it, and apply it
 * to the live source — the same review-before-run shape as the grid's
 * Review & Commit.
 */
export function MigrationPanel({
  tabId,
  applyTarget,
}: {
  tabId: string;
  applyTarget: ApplyTarget | null;
}) {
  const state = useSchemaCompare((s) => s.byTab[tabId]);
  const toggle = useSchemaCompare((s) => s.toggle);
  const setMany = useSchemaCompare((s) => s.setMany);
  const setWrap = useSchemaCompare((s) => s.setWrap);
  const generateScript = useSchemaCompare((s) => s.generateScript);
  const setSql = useSchemaCompare((s) => s.setSql);
  const apply = useSchemaCompare((s) => s.apply);
  const refreshSchema = useConnections((s) => s.refreshSchema);
  const recompare = useSchemaCompare((s) => s.recompare);
  const [confirming, setConfirming] = useState(false);

  const statements = state?.comparison?.statements ?? [];
  const selectedIds = useMemo(
    () => statements.filter((s) => state?.selected[s.id]).map((s) => s.id),
    [statements, state?.selected],
  );
  const selectedDestructive = statements.filter(
    (s) => state?.selected[s.id] && s.destructive,
  ).length;

  if (!state) return null;

  const allIds = statements.map((s) => s.id);
  const changeIds = allIds; // every statement is a change; "select all" === all
  const isProd = applyTarget?.envTag === "prod";
  const canApply =
    !!applyTarget && selectedIds.length > 0 && !state.applying && state.sql.trim().length > 0;

  async function handleApply() {
    if (!applyTarget) return;
    if ((selectedDestructive > 0 || isProd) && !confirming) {
      setConfirming(true);
      return;
    }
    setConfirming(false);
    const ok = await apply(tabId, applyTarget.connectionId, applyTarget.database);
    if (ok) {
      // Reflect the applied DDL: refresh the live source schema, then recompare
      // so the diff collapses to whatever is left.
      await refreshSchema(applyTarget.connectionId);
      await recompare(tabId);
    }
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      {/* Statement checklist */}
      <div className="flex h-[26px] shrink-0 items-center justify-between border-b border-border-divider bg-bg-1 px-3 text-[10px] font-semibold uppercase tracking-[0.05em] text-fg-3">
        <span>
          Include in migration · {selectedIds.length}/{statements.length}
        </span>
        <div className="flex items-center gap-1.5">
          <button
            type="button"
            className="rounded-[3px] px-1.5 py-0.5 normal-case tracking-normal text-fg-2 hover:bg-bg-2 hover:text-fg-0"
            onClick={() => setMany(tabId, changeIds, true)}
          >
            all
          </button>
          <button
            type="button"
            className="rounded-[3px] px-1.5 py-0.5 normal-case tracking-normal text-fg-2 hover:bg-bg-2 hover:text-fg-0"
            onClick={() => setMany(tabId, allIds, false)}
          >
            none
          </button>
        </div>
      </div>

      <div className="max-h-[34%] min-h-[60px] overflow-y-auto py-1">
        {statements.length === 0 ? (
          <div className="px-3 py-3 text-[11px] text-fg-3">
            No DDL to generate — the schemas already match.
          </div>
        ) : (
          statements.map((statement) => (
            <StatementRow
              key={statement.id}
              statement={statement}
              checked={!!state.selected[statement.id]}
              onToggle={() => toggle(tabId, statement.id)}
            />
          ))
        )}
      </div>

      {/* Generated SQL */}
      <div className="flex h-[26px] shrink-0 items-center justify-between border-y border-border-divider bg-bg-1 px-3 text-[10px] font-semibold uppercase tracking-[0.05em] text-fg-3">
        <span>Migration script{state.sqlDirty ? " · edited" : ""}</span>
        <div className="flex items-center gap-2">
          <label className="flex cursor-pointer items-center gap-1 normal-case tracking-normal text-fg-2">
            <input
              type="checkbox"
              checked={state.wrapInTransaction}
              onChange={(e) => setWrap(tabId, e.target.checked)}
            />
            transaction
          </label>
          <button
            type="button"
            className="inline-flex items-center gap-1 rounded-[3px] border border-border-default bg-bg-2 px-1.5 py-0.5 normal-case tracking-normal text-fg-1 hover:bg-bg-3"
            onClick={() => void generateScript(tabId)}
            title="Rebuild the script from the selected statements (discards manual edits)"
          >
            <Icon.history size={10} />
            regenerate
          </button>
          <button
            type="button"
            className="inline-flex items-center gap-1 rounded-[3px] border border-border-default bg-bg-2 px-1.5 py-0.5 normal-case tracking-normal text-fg-1 hover:bg-bg-3"
            onClick={() => void navigator.clipboard?.writeText(state.sql)}
          >
            <Icon.copy size={10} />
            copy
          </button>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-hidden">
        <SqlCodeEditor
          value={state.sql}
          engine="postgres"
          onChange={(value) => setSql(tabId, value)}
          placeholder="Select statements above, then Regenerate to build the migration script."
        />
      </div>

      {/* Apply bar */}
      <div className="flex min-h-11 shrink-0 items-center justify-between gap-3 border-t border-border-default bg-bg-2 px-3 py-2">
        <div className="flex min-w-0 items-center gap-2 text-[10.5px]">
          <Icon.warn
            size={10}
            stroke={
              isProd || selectedDestructive > 0 ? "var(--warn)" : "var(--fg-3)"
            }
          />
          <span className="min-w-0 truncate text-fg-2">
            {state.applyError
              ? state.applyError
              : !applyTarget
                ? "apply needs a live source connection — snapshot sources are read-only"
                : confirming
                  ? `apply ${selectedDestructive} destructive change${selectedDestructive === 1 ? "" : "s"}${isProd ? " to a PROD connection" : ""}?`
                  : state.appliedAt
                    ? "migration applied — diff refreshed"
                    : "review the script above; it runs in a transaction where supported"}
          </span>
        </div>
        <div className="flex items-center gap-2">
          {confirming && (
            <button
              type="button"
              className={ED_BTN}
              onClick={() => setConfirming(false)}
            >
              Cancel
            </button>
          )}
          <button
            type="button"
            disabled={!canApply}
            onClick={() => void handleApply()}
            className={
              selectedDestructive > 0 || isProd || confirming ? ED_DANGER : ED_BTN
            }
            style={
              selectedDestructive > 0 || isProd || confirming
                ? { borderColor: "color-mix(in oklab, var(--delete) 40%, black)" }
                : undefined
            }
          >
            <Icon.commit size={11} />
            <span>
              {state.applying
                ? "Applying…"
                : confirming
                  ? "Confirm apply"
                  : "Apply migration"}
            </span>
          </button>
        </div>
      </div>
    </div>
  );
}

function StatementRow({
  statement,
  checked,
  onToggle,
}: {
  statement: MigrationStatement;
  checked: boolean;
  onToggle: () => void;
}) {
  return (
    <label className="flex cursor-pointer items-center gap-2 px-3 py-1 hover:bg-bg-2">
      <input type="checkbox" checked={checked} onChange={onToggle} />
      <KindChip kind={statement.kind} destructive={statement.destructive} />
      <span className="min-w-0 flex-1 truncate font-mono text-[11px] text-fg-1">
        {statement.object}
      </span>
      <span className="shrink-0 text-[10px] text-fg-3">
        {statement.description}
      </span>
    </label>
  );
}

function KindChip({
  kind,
  destructive,
}: {
  kind: MigrationKind;
  destructive: boolean;
}) {
  const color = destructive ? "var(--delete)" : "var(--insert)";
  const bg = destructive ? "var(--delete-bg)" : "var(--insert-bg)";
  return (
    <span
      className="inline-flex h-[14px] shrink-0 items-center rounded-[3px] px-1 font-mono text-[9px] font-semibold uppercase tracking-[0.03em]"
      style={{ background: bg, color }}
    >
      {kind.replace(/-/g, " ")}
    </span>
  );
}

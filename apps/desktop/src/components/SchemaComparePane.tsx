import { useConnections } from "../state/connections";
import { useSchemaCompare } from "../state/schemaCompare";
import type { SchemaCompareTab } from "../state/tabs";
import { DiffTree } from "./schema-compare/DiffTree";
import { MigrationPanel, type ApplyTarget } from "./schema-compare/MigrationPanel";
import { Icon } from "./icons";

/**
 * Workspace pane for a schema comparison. Top half is the side-by-side diff
 * tree; bottom half is the migration panel (statement selection + editable
 * script + apply). Comparison state is owned by `useSchemaCompare`, keyed by
 * the tab id.
 */
export function SchemaComparePane({ tab }: { tab: SchemaCompareTab }) {
  const state = useSchemaCompare((s) => s.byTab[tab.id]);
  const recompare = useSchemaCompare((s) => s.recompare);
  const sourceConn = useConnections((s) => {
    if (!state || state.config.source.kind !== "live") return null;
    const id = state.config.source.connection_id;
    return s.connections.find((c) => c.id === id) ?? null;
  });

  if (!state) {
    return <PaneMessage>Comparison not initialized.</PaneMessage>;
  }
  if (state.loading) {
    return (
      <PaneMessage>
        <span className="animate-sb-pulse">comparing schemas…</span>
      </PaneMessage>
    );
  }
  // Only treat a missing comparison as a failure worth replacing the whole
  // view: a later script-generation error (with a populated comparison) is
  // surfaced inside the migration panel instead of hiding the diff.
  if (!state.comparison) {
    return (
      <PaneMessage>
        <div className="flex flex-col items-center gap-2">
          <span className="text-warn">{state.error ?? "Comparison failed."}</span>
          <button
            type="button"
            className="inline-flex items-center gap-1.5 rounded-[4px] border border-border-default bg-bg-2 px-2.5 py-1 text-[11.5px] text-fg-1 hover:bg-bg-3"
            onClick={() => void recompare(tab.id)}
          >
            <Icon.history size={11} />
            Retry
          </button>
        </div>
      </PaneMessage>
    );
  }

  const { diff } = state.comparison;
  const sourceIsLive = state.config.source.kind === "live";
  // Apply executes DDL against the live source, which only the Postgres driver
  // supports today. Gate on the connection's real engine — not the DDL dialect,
  // which falls back to Postgres for engines without their own dialect (e.g.
  // Firestore) — so an unsupported engine can't pass review and then error.
  const applyableEngine = sourceConn?.engine === "postgres";
  const applyTarget: ApplyTarget | null =
    state.config.source.kind === "live" && applyableEngine
      ? {
          connectionId: state.config.source.connection_id,
          database: state.config.source.database,
          envTag: sourceConn?.env_tag ?? null,
        }
      : null;
  const applyUnsupportedReason = applyTarget
    ? null
    : !sourceIsLive
      ? "apply needs a live source connection — snapshot sources are read-only"
      : `apply is only supported for Postgres sources right now (source is ${sourceConn?.engine ?? "unknown"})`;

  const s = diff.summary;
  const total =
    s.tables_added +
    s.tables_removed +
    s.tables_modified +
    s.views_added +
    s.views_removed +
    s.views_modified;

  return (
    <div className="flex min-h-0 flex-1 flex-col bg-bg-0">
      <div className="flex h-[30px] shrink-0 items-center gap-2 border-b border-border-default bg-bg-1 px-3">
        <Icon.diff size={13} stroke="var(--accent)" />
        <span className="text-[12px] font-semibold text-fg-0">Schema compare</span>
        <span className="flex items-center gap-2 text-[11px] text-fg-3">
          <SummaryPill n={s.tables_added + s.views_added} label="added" color="var(--insert)" />
          <SummaryPill n={s.tables_removed + s.views_removed} label="dropped" color="var(--delete)" />
          <SummaryPill n={s.tables_modified + s.views_modified} label="changed" color="var(--update)" />
          {total === 0 && <span className="text-fg-3">schemas match</span>}
        </span>
        <div className="flex-1" />
        <button
          type="button"
          className="inline-flex items-center gap-1.5 rounded-[4px] border border-border-default bg-bg-2 px-2 py-1 text-[11px] text-fg-1 hover:bg-bg-3"
          onClick={() => void recompare(tab.id)}
          title="Re-run the comparison against the current live schema"
        >
          <Icon.history size={11} />
          Recompare
        </button>
      </div>

      <div className="flex min-h-0 flex-[2] flex-col overflow-hidden border-b border-border-default">
        <DiffTree diff={diff} />
      </div>
      <div className="flex min-h-0 flex-[3] flex-col overflow-hidden">
        <MigrationPanel
          tabId={tab.id}
          applyTarget={applyTarget}
          unsupportedReason={applyUnsupportedReason}
        />
      </div>
    </div>
  );
}

function SummaryPill({
  n,
  label,
  color,
}: {
  n: number;
  label: string;
  color: string;
}) {
  return (
    <span className="inline-flex items-center gap-1">
      <span className="font-mono font-semibold tabular-nums" style={{ color }}>
        {n}
      </span>
      <span>{label}</span>
    </span>
  );
}

function PaneMessage({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex flex-1 items-center justify-center bg-bg-inset text-[11.5px] text-fg-3">
      {children}
    </div>
  );
}

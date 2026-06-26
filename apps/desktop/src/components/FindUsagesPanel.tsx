import { useMemo, type ReactNode } from "react";
import type { UsageKind, UsageReference } from "@cellar/ipc";

import { useFindUsages, type UsageTarget } from "../state/findUsages";
import { useTabs } from "../state/tabs";
import { qualifiedName } from "../lib/sqlIdent";
import { Icon } from "./icons";

/** Display metadata per usage kind: label, icon, and a short pill text. */
const KIND_META: Record<
  UsageKind,
  { label: string; pill: string; icon: (size: number) => ReactNode }
> = {
  view: { label: "View", pill: "view", icon: (s) => <Icon.tree size={s} /> },
  materialized_view: {
    label: "Materialized view",
    pill: "matview",
    icon: (s) => <Icon.tree size={s} />,
  },
  function: { label: "Function", pill: "fn", icon: (s) => <Icon.fn size={s} /> },
  procedure: {
    label: "Procedure",
    pill: "proc",
    icon: (s) => <Icon.proc size={s} />,
  },
  trigger: { label: "Trigger", pill: "trigger", icon: (s) => <Icon.bolt size={s} /> },
  constraint: {
    label: "Constraint",
    pill: "constraint",
    icon: (s) => <Icon.bracket size={s} />,
  },
};

export function FindUsagesPanel() {
  const target = useFindUsages((s) => s.target);
  const status = useFindUsages((s) => s.status);
  const results = useFindUsages((s) => s.results);
  const error = useFindUsages((s) => s.error);
  const allSchemas = useFindUsages((s) => s.allSchemas);
  const setAllSchemas = useFindUsages((s) => s.setAllSchemas);
  const refresh = useFindUsages((s) => s.refresh);
  const openTable = useTabs((s) => s.openTable);
  const newQueryTab = useTabs((s) => s.newQueryTab);
  const setQuerySql = useTabs((s) => s.setQuerySql);

  const targetLabel = useMemo(() => {
    if (!target) return null;
    const base = qualifiedName(target.schema, target.table);
    return target.column ? `${base}.${target.column}` : base;
  }, [target]);

  const openUsage = (usage: UsageReference, t: UsageTarget) => {
    if (usage.kind === "view" || usage.kind === "materialized_view") {
      // Views are browsable like tables — jump straight to the object.
      openTable(t.connectionId, t.database, usage.schema, usage.name);
      return;
    }
    // Routines, triggers, and constraints open as their definition text in a
    // fresh SQL editor tab so the user can read (and adapt) the reference.
    const id = newQueryTab(t.connectionId, t.database);
    setQuerySql(id, usage.definition);
  };

  if (!target) {
    return (
      <EmptyState
        title="No usages searched yet"
        detail="Right-click a table in the sidebar — or a column header in the grid — and choose “Find Usages” to search views, functions, procedures, triggers, and constraints that reference it."
      />
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-col bg-bg-inset text-[11px]">
      <div className="flex h-8 shrink-0 items-center justify-between gap-3 border-b border-border-divider px-2">
        <div className="flex min-w-0 items-center gap-2">
          <span className="shrink-0 font-medium text-fg-1">Find Usages</span>
          <span className="min-w-0 truncate font-mono text-[10.5px] text-fg-2">
            {targetLabel}
          </span>
          {target.column && (
            <span className="shrink-0 rounded-[4px] bg-accent-soft px-1.5 py-px font-mono text-[9.5px] text-accent">
              column
            </span>
          )}
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <div className="inline-flex overflow-hidden rounded-[4px] border border-border-default">
            <ScopeButton
              active={!allSchemas}
              label="This schema"
              onClick={() => setAllSchemas(false)}
            />
            <ScopeButton
              active={allSchemas}
              label="All schemas"
              onClick={() => setAllSchemas(true)}
            />
          </div>
          <button
            className="icon-btn"
            title="Re-run search"
            onClick={refresh}
            disabled={status === "loading"}
          >
            <Icon.history size={11} />
          </button>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-auto">
        {status === "loading" ? (
          <EmptyState title="Searching…" detail={`Scanning catalog definitions for ${targetLabel}.`} />
        ) : status === "error" ? (
          <EmptyState title="Search failed" detail={error ?? "Unknown error."} tone="warn" />
        ) : results.length === 0 ? (
          <EmptyState
            title="No usages found"
            detail={
              allSchemas
                ? `No view, function, procedure, trigger, or constraint references ${targetLabel}.`
                : `No references in schema “${target.schema}”. Try “All schemas” to widen the search.`
            }
          />
        ) : (
          <div className="divide-y divide-border-divider">
            {results.map((usage, index) => (
              <UsageRow
                key={`${usage.kind}:${usage.schema}.${usage.name}:${index}`}
                usage={usage}
                onOpen={() => openUsage(usage, target)}
              />
            ))}
          </div>
        )}
      </div>

      {status === "ready" && results.length > 0 && (
        <div className="flex h-6 shrink-0 items-center border-t border-border-divider px-2 font-mono text-[10px] text-fg-3">
          {results.length} {results.length === 1 ? "usage" : "usages"}
          {allSchemas ? " across all schemas" : ` in ${target.schema}`}
        </div>
      )}
    </div>
  );
}

function ScopeButton({
  active,
  label,
  onClick,
}: {
  active: boolean;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={
        "h-[20px] px-2 text-[10px] " +
        (active
          ? "bg-accent-soft text-accent"
          : "text-fg-2 hover:bg-bg-2 hover:text-fg-0")
      }
    >
      {label}
    </button>
  );
}

function UsageRow({
  usage,
  onOpen,
}: {
  usage: UsageReference;
  onOpen: () => void;
}) {
  const meta = KIND_META[usage.kind];
  return (
    <button
      type="button"
      onClick={onOpen}
      title="Click to open"
      className="group flex w-full items-start gap-2 px-2.5 py-2 text-left hover:bg-bg-1"
    >
      <span className="mt-px inline-flex h-[14px] w-[14px] shrink-0 items-center justify-center text-fg-3">
        {meta.icon(12)}
      </span>
      <div className="min-w-0 flex-1">
        <div className="flex min-w-0 items-center gap-1.5">
          <span className="shrink-0 rounded-[3px] bg-bg-2 px-1 py-px font-mono text-[9px] text-fg-3">
            {meta.pill}
          </span>
          <span className="min-w-0 truncate font-mono text-[11px] text-fg-1">
            {qualifiedName(usage.schema, usage.name)}
          </span>
          {usage.on_table && (
            <span className="shrink-0 font-mono text-[10px] text-fg-3">
              on {usage.on_table}
            </span>
          )}
          {usage.matched_column && (
            <span className="shrink-0 rounded-[3px] bg-accent-soft px-1 py-px font-mono text-[9px] text-accent">
              {usage.matched_column}
            </span>
          )}
          <span className="ml-auto shrink-0 font-mono text-[9.5px] text-fg-4">
            L{usage.line}
          </span>
        </div>
        <pre className="m-0 mt-1 max-h-[40px] overflow-hidden whitespace-pre-wrap break-words font-mono text-[10.5px] leading-[1.45] text-fg-2">
          {usage.snippet}
        </pre>
      </div>
      <span className="mt-px shrink-0 text-fg-4 opacity-0 group-hover:opacity-100">
        <Icon.terminal size={11} />
      </span>
    </button>
  );
}

function EmptyState({
  title,
  detail,
  tone = "muted",
}: {
  title: string;
  detail: string;
  tone?: "muted" | "warn";
}) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-1.5 p-6 text-center">
      <div
        className={
          "text-[12px] font-medium " + (tone === "warn" ? "text-warn" : "text-fg-1")
        }
      >
        {title}
      </div>
      <div className="max-w-[460px] text-[10.5px] leading-[1.5] text-fg-3">
        {detail}
      </div>
    </div>
  );
}

import { useEffect, useMemo, useRef, useState } from "react";
import {
  commands,
  unwrap,
  type PlanMode,
  type PlanNode,
  type QueryPlan,
} from "@cellar/ipc";

import { useBottomPanel } from "../state/bottomPanel";
import { noteConnectionIssue, useConnections } from "../state/connections";
import { useTabs, type QueryTab } from "../state/tabs";
import { Icon } from "./icons";

type WorkspaceTab = ReturnType<typeof useTabs.getState>["tabs"][number];
type ConnectionConfig = ReturnType<typeof useConnections.getState>["connections"][number];

export function PlanPanel({ activeTab }: { activeTab: WorkspaceTab | null }) {
  const connections = useConnections((s) => s.connections);
  const byId = useConnections((s) => s.byId);
  const connection = activeTab
    ? connections.find((c) => c.id === activeTab.connectionId) ?? null
    : null;
  const connected = connection ? byId[connection.id]?.status === "connected" : false;
  const [plan, setPlan] = useState<QueryPlan | null>(null);
  const [mode, setMode] = useState<PlanMode>("estimate");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const copiedTimer = useRef<number | null>(null);
  const queryTab = activeTab?.kind === "query" ? activeTab : null;
  const sql = queryTab?.sql.trim() ?? "";
  const stale = Boolean(plan && sql && normalizeSql(plan.sql) !== normalizeSql(sql));
  const canCopyRawJson = Boolean(plan && clipboardAvailable());
  const unavailable = useMemo(
    () => unavailableReason(activeTab, queryTab, connection, connected, sql),
    [activeTab, queryTab, connection, connected, sql],
  );

  // The editor's "Explain plan" toolbar button bumps `explainNonce`; run a
  // fresh plan when that happens (and the statement can actually be explained).
  const explainNonce = useBottomPanel((s) => s.explainNonce);
  const seenNonce = useRef(explainNonce);
  useEffect(() => {
    if (explainNonce === seenNonce.current) return;
    seenNonce.current = explainNonce;
    if (!unavailable && !loading) void loadPlan(mode);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [explainNonce]);

  useEffect(() => {
    return () => {
      if (copiedTimer.current != null) clearTimeout(copiedTimer.current);
    };
  }, []);

  // Stale-guard: a plan resolved after the user switched tabs (or re-ran)
  // must not land on the panel. Bumping the token invalidates in-flight runs.
  const planToken = useRef(0);
  const queryTabId = queryTab?.id ?? null;
  useEffect(() => {
    planToken.current++;
    setLoading(false);
  }, [queryTabId]);

  async function loadPlan(nextMode = mode) {
    if (!queryTab || unavailable) return;
    if (nextMode === "analyze" && !confirmAnalyze()) return;
    const token = ++planToken.current;
    setLoading(true);
    setError(null);
    try {
      const next = await unwrap(
        commands.explainQuery(
          queryTab.connectionId,
          queryTab.sql,
          nextMode,
          queryTab.database,
        ),
      );
      if (token !== planToken.current) return;
      setPlan(next);
    } catch (err) {
      noteConnectionIssue(queryTab.connectionId, err);
      if (token !== planToken.current) return;
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      if (token === planToken.current) setLoading(false);
    }
  }

  function copyRawJson() {
    if (!plan || !clipboardAvailable()) return;
    void navigator.clipboard.writeText(JSON.stringify(plan.raw_json, null, 2)).then(() => {
      setCopied(true);
      if (copiedTimer.current != null) clearTimeout(copiedTimer.current);
      copiedTimer.current = window.setTimeout(() => setCopied(false), 1400);
    });
  }

  return (
    <div className="flex h-full min-h-0 flex-col bg-bg-inset">
      <div className="flex h-9 shrink-0 items-center justify-between border-b border-border-default px-2.5">
        <div className="flex min-w-0 items-center gap-2">
          <span className="inline-flex text-accent">
            <Icon.tree size={13} />
          </span>
          <span className="truncate font-mono text-[11px] text-fg-2">
            {queryTab ? queryTab.title : "no SQL query selected"}
          </span>
          {stale && (
            <span className="rounded-[3px] border border-warn/40 px-1.5 py-[1px] text-[10px] text-warn">
              stale
            </span>
          )}
        </div>
        <div className="flex items-center gap-1.5">
          <div className="inline-flex h-[23px] rounded-[4px] border border-border-default bg-bg-1 p-[1px]">
            <PlanModeButton
              active={mode === "estimate"}
              label="Estimate"
              title="Run EXPLAIN without executing the statement"
              onClick={() => setMode("estimate")}
            />
            <PlanModeButton
              active={mode === "analyze"}
              label="Analyze"
              title="Run EXPLAIN ANALYZE, which executes the statement"
              onClick={() => setMode("analyze")}
            />
          </div>
          <button
            onClick={copyRawJson}
            disabled={!canCopyRawJson}
            title="Copy raw Postgres EXPLAIN JSON"
            className="inline-flex h-[23px] items-center gap-1.5 rounded-[4px] border border-border-default bg-bg-2 px-2 text-[11px] text-fg-2 disabled:cursor-not-allowed disabled:opacity-50 hover:border-border-strong hover:bg-bg-3 hover:text-fg-0"
          >
            {copied ? <Icon.check size={10} /> : <Icon.copy size={10} />}
            {copied ? "Copied" : "JSON"}
          </button>
          <button
            onClick={() => void loadPlan()}
            disabled={Boolean(unavailable) || loading}
            title={
              unavailable ??
              (mode === "analyze"
                ? "Run Postgres EXPLAIN ANALYZE"
                : "Run Postgres EXPLAIN without executing the statement")
            }
            className="inline-flex h-[23px] items-center gap-1.5 rounded-[4px] border border-border-default bg-bg-2 px-2 text-[11px] text-fg-1 disabled:cursor-not-allowed disabled:opacity-50 hover:border-border-strong hover:bg-bg-3"
          >
            <Icon.bolt size={10} />
            {loading ? "Explaining..." : "Run"}
          </button>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-auto p-2.5">
        <div className="flex min-h-full flex-col gap-2">
          {mode === "analyze" && <AnalyzeWarning />}
          {unavailable ? (
            <PlanEmpty title={unavailable} detail={detailForUnavailable(unavailable)} />
          ) : loading ? (
            <PlanEmpty
              title="Loading execution plan"
              detail={
                mode === "analyze"
                  ? "Postgres is executing the selected statement and collecting timings."
                  : "Postgres is estimating the selected statement."
              }
            />
          ) : error ? (
            <PlanEmpty title="Plan failed" detail={error} warn />
          ) : plan ? (
            <>
              <PlanSummary plan={plan} stale={stale} />
              <PlanNodeView node={plan.root} maxCost={plan.root.total_cost ?? 0} />
            </>
          ) : (
            <PlanEmpty
              title="No plan yet"
              detail={
                mode === "analyze"
                  ? "Run Analyze only when executing the selected SQL is acceptable."
                  : "Run Explain to inspect the estimated Postgres plan before executing SQL."
              }
            />
          )}
        </div>
      </div>
    </div>
  );
}

function PlanModeButton({
  active,
  label,
  title,
  onClick,
}: {
  active: boolean;
  label: string;
  title: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      title={title}
      aria-pressed={active}
      onClick={onClick}
      className={
        "rounded-[3px] px-2 text-[10.5px] leading-[19px] transition-colors " +
        (active
          ? "bg-bg-3 text-fg-0 shadow-sm"
          : "text-fg-3 hover:bg-bg-2 hover:text-fg-1")
      }
    >
      {label}
    </button>
  );
}

function AnalyzeWarning() {
  return (
    <div className="flex items-start gap-2 rounded-[4px] border border-warn/30 bg-update-bg px-2.5 py-2 text-[10.5px] leading-[1.45] text-fg-2">
      <Icon.warn size={13} className="mt-[1px] text-warn" />
      <div>
        <span className="font-medium text-warn">EXPLAIN ANALYZE executes SQL.</span>{" "}
        INSERT, UPDATE, DELETE, and long-running SELECT statements can write data,
        take locks, or trigger side effects.
      </div>
    </div>
  );
}

function unavailableReason(
  activeTab: WorkspaceTab | null,
  queryTab: QueryTab | null,
  connection: ConnectionConfig | null,
  connected: boolean,
  sql: string,
): string | null {
  if (!activeTab) return "No query selected";
  if (!queryTab) return "Plans are available for SQL query tabs";
  if (!connection) return "No connection selected";
  if (connection.engine !== "postgres") return "Execution plans are Postgres-only for now";
  if (!connected) return "Connection is not open";
  if (!sql) return "No SQL statement selected";
  return null;
}

function detailForUnavailable(reason: string): string {
  switch (reason) {
    case "Plans are available for SQL query tabs":
      return "Table tabs use a generated read path and do not expose freeform SQL to explain.";
    case "Execution plans are Postgres-only for now":
      return "Other engines will need their own typed plan command and renderer.";
    case "Connection is not open":
      return "Connect the tab's database before requesting a plan.";
    default:
      return "Open a SQL query tab with a selected statement to inspect its plan.";
  }
}

function PlanSummary({ plan, stale }: { plan: QueryPlan; stale: boolean }) {
  return (
    <div className="grid grid-cols-2 gap-2 text-[10.5px] sm:grid-cols-4">
      <Metric
        label="mode"
        value={plan.mode === "analyze" ? "analyze" : "estimate"}
        tone={stale || plan.mode === "analyze" ? "warn" : undefined}
      />
      <Metric label="planning" value={formatMs(plan.planning_time_ms)} />
      <Metric label="execution" value={formatMs(plan.execution_time_ms)} />
      <Metric label="round trip" value={`${plan.duration_ms} ms`} />
    </div>
  );
}

function Metric({
  label,
  value,
  tone,
}: {
  label: string;
  value: string;
  tone?: "warn";
}) {
  return (
    <div className="rounded-[4px] border border-border-default bg-bg-1 px-2 py-1.5">
      <div className="text-[9.5px] uppercase tracking-[0.08em] text-fg-3">
        {label}
      </div>
      <div
        className={
          "font-mono text-[11px] " +
          (tone === "warn" ? "text-warn" : "text-fg-1")
        }
      >
        {value}
      </div>
    </div>
  );
}

function PlanNodeView({ node, maxCost }: { node: PlanNode; maxCost: number }) {
  const heat =
    maxCost > 0 && node.total_cost ? Math.min(1, node.total_cost / maxCost) : 0;
  const heatPercent = Math.round(heat * 100);
  const heatColor = costHeatColor(heat);
  return (
    <div className="rounded-[4px] border border-border-default bg-bg-1">
      <div className="flex flex-wrap items-center gap-x-2 gap-y-1 px-2 py-1.5">
        <span className="font-mono text-[11.5px] font-medium text-fg-0">
          {node.node_type}
        </span>
        {node.relation_name && (
          <span className="font-mono text-[10.5px] text-fg-2">
            {node.schema_name ? `${node.schema_name}.` : ""}
            {node.relation_name}
          </span>
        )}
        {node.index_name && (
          <span className="font-mono text-[10.5px] text-accent">
            {node.index_name}
          </span>
        )}
        <span className="ml-auto font-mono text-[10px] text-fg-3">
          cost {formatRange(node.startup_cost, node.total_cost)} · rows{" "}
          {node.plan_rows ?? "?"}
        </span>
        <div
          className="flex h-[14px] w-[92px] items-center gap-1"
          title={`${heatPercent}% of root total cost`}
        >
          <div className="h-[5px] min-w-0 flex-1 overflow-hidden rounded-[2px] bg-bg-3">
            <div
              className="h-full rounded-[2px]"
              style={{
                width: `${heatPercent}%`,
                backgroundColor: heatColor,
              }}
            />
          </div>
          <span className="w-[28px] text-right font-mono text-[9.5px] text-fg-3">
            {heatPercent}%
          </span>
        </div>
      </div>
      {(node.actual_total_time_ms != null || node.details.length > 0) && (
        <div className="border-t border-border-divider px-2 py-1.5 text-[10.5px] text-fg-2">
          {node.actual_total_time_ms != null && (
            <div className="font-mono text-fg-2">
              actual {formatRange(node.actual_startup_time_ms, node.actual_total_time_ms)} ms · rows{" "}
              {node.actual_rows ?? "?"} · loops {node.actual_loops ?? "?"}
            </div>
          )}
          {node.details.map((d) => (
            <div key={d.label} className="font-mono">
              <span className="text-fg-3">{d.label}: </span>
              <span>{d.value}</span>
            </div>
          ))}
        </div>
      )}
      {node.children.length > 0 && (
        <div className="space-y-1.5 border-t border-border-divider p-1.5 pl-4">
          {node.children.map((child, idx) => (
            <PlanNodeView
              key={`${child.node_type}-${idx}`}
              node={child}
              maxCost={maxCost}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function PlanEmpty({
  title,
  detail,
  warn,
}: {
  title: string;
  detail: string;
  warn?: boolean;
}) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-1.5 p-6 text-center text-[11.5px] text-fg-3">
      <div className={"text-[12px] font-medium " + (warn ? "text-warn" : "text-fg-1")}>
        {title}
      </div>
      <div className="max-w-[360px] text-[10.5px] leading-[1.5] text-fg-3">
        {detail}
      </div>
    </div>
  );
}

function formatMs(value: number | null): string {
  return value == null ? "n/a" : `${value.toFixed(value >= 10 ? 1 : 2)} ms`;
}

function formatRange(start: number | null, end: number | null): string {
  if (start == null && end == null) return "?";
  if (start == null) return `?..${end}`;
  if (end == null) return `${start}..?`;
  return `${start}..${end}`;
}

function costHeatColor(heat: number): string {
  if (heat >= 0.75) return "var(--delete)";
  if (heat >= 0.35) return "var(--update)";
  if (heat > 0) return "var(--insert)";
  return "var(--fg-4)";
}

function confirmAnalyze(): boolean {
  if (typeof window === "undefined") return false;
  return window.confirm(
    "Run EXPLAIN ANALYZE?\n\nPostgres will execute the SQL. Writes can change data, and long statements can take locks.",
  );
}

function clipboardAvailable(): boolean {
  return typeof navigator !== "undefined" && Boolean(navigator.clipboard);
}

function normalizeSql(sql: string): string {
  return sql.trim().replace(/;+\s*$/, "");
}

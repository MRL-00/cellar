import { useEffect, useMemo, useRef, useState } from "react";
import { commands, unwrap, type PlanNode, type QueryPlan } from "@cellar/ipc";

import { useBottomPanel } from "../state/bottomPanel";
import { useConnections } from "../state/connections";
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
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const queryTab = activeTab?.kind === "query" ? activeTab : null;
  const sql = queryTab?.sql.trim() ?? "";
  const stale = Boolean(plan && sql && normalizeSql(plan.sql) !== normalizeSql(sql));
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
    if (!unavailable && !loading) void loadPlan();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [explainNonce]);

  async function loadPlan() {
    if (!queryTab || unavailable) return;
    setLoading(true);
    setError(null);
    try {
      const next = await unwrap(
        commands.explainQuery(
          queryTab.connectionId,
          queryTab.sql,
          "estimate",
          queryTab.database,
        ),
      );
      setPlan(next);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
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
          <button
            onClick={loadPlan}
            disabled={Boolean(unavailable) || loading}
            title={
              unavailable ?? "Run Postgres EXPLAIN without executing the statement"
            }
            className="inline-flex h-[23px] items-center gap-1.5 rounded-[4px] border border-border-default bg-bg-2 px-2 text-[11px] text-fg-1 disabled:cursor-not-allowed disabled:opacity-50 hover:border-border-strong hover:bg-bg-3"
          >
            <Icon.bolt size={10} />
            {loading ? "Explaining..." : "Explain"}
          </button>
          <button
            disabled
            title="EXPLAIN ANALYZE executes SQL and will be exposed after destructive-query gating lands"
            className="inline-flex h-[23px] items-center gap-1.5 rounded-[4px] border border-border-default px-2 text-[11px] text-fg-3 opacity-50"
          >
            Analyze
          </button>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-auto p-2.5">
        {unavailable ? (
          <PlanEmpty title={unavailable} detail={detailForUnavailable(unavailable)} />
        ) : loading ? (
          <PlanEmpty
            title="Loading execution plan"
            detail="Postgres is estimating the selected statement."
          />
        ) : error ? (
          <PlanEmpty title="Plan failed" detail={error} warn />
        ) : plan ? (
          <div className="space-y-2">
            <PlanSummary plan={plan} stale={stale} />
            <PlanNodeView node={plan.root} maxCost={plan.root.total_cost ?? 0} />
          </div>
        ) : (
          <PlanEmpty
            title="No plan yet"
            detail="Run Explain to inspect the estimated Postgres plan before executing SQL."
          />
        )}
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
      <Metric label="mode" value={plan.mode} tone={stale ? "warn" : undefined} />
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
  return (
    <div
      className="rounded-[4px] border border-border-default bg-bg-1"
      style={{ borderLeftColor: `rgba(76, 201, 160, ${0.25 + heat * 0.75})` }}
    >
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

function normalizeSql(sql: string): string {
  return sql.trim().replace(/;+\s*$/, "");
}

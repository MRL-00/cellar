import { Icon } from "./icons";
import { useConnections } from "../state/connections";
import { useTabs } from "../state/tabs";
import { useStatus } from "../state/status";

export function StatusBar() {
  const activeTabId = useTabs((s) => s.activeId);
  const tabs = useTabs((s) => s.tabs);
  const connections = useConnections((s) => s.connections);
  const byId = useConnections((s) => s.byId);
  const lastQuery = useStatus((s) => s.lastQuery);

  const activeTab = tabs.find((t) => t.id === activeTabId) ?? null;
  const activeConn = activeTab
    ? connections.find((c) => c.id === activeTab.connectionId)
    : null;
  const activeState = activeConn ? byId[activeConn.id] : null;
  const connected = activeState?.status === "connected";

  return (
    <div className="flex h-[22px] shrink-0 items-center justify-between border-t border-border-default bg-bg-1 px-2.5 text-[11px] text-fg-2">
      <div className="flex items-center gap-3.5">
        <span className="inline-flex h-[18px] items-center gap-[5px] whitespace-nowrap text-[10.5px]">
          <span
            className="inline-block h-1.5 w-1.5 rounded-full"
            style={{
              background: connected ? "var(--insert)" : "var(--fg-4)",
              boxShadow: connected
                ? "0 0 0 2px var(--insert-bg)"
                : undefined,
            }}
          />
          <span style={{ color: "var(--fg-1)" }}>
            {activeConn?.name ?? "no connection"}
          </span>
          {activeConn && (
            <>
              <span style={{ color: "var(--fg-3)" }}>·</span>
              <span style={{ color: "var(--eng-postgres)" }}>
                {activeConn.engine.toUpperCase()}
              </span>
              {activeState?.driverInfo?.version && (
                <>
                  <span style={{ color: "var(--fg-3)" }}>·</span>
                  <span className="mono" style={{ color: "var(--fg-2)" }}>
                    {shortVersion(activeState.driverInfo.version)}
                  </span>
                </>
              )}
            </>
          )}
        </span>
        {activeConn && (
          <span className="inline-flex h-[18px] items-center gap-[5px] whitespace-nowrap text-[10.5px]">
            <Icon.user size={10} />
            <span className="mono">
              {activeConn.user}@{activeConn.host}
            </span>
          </span>
        )}
        {activeConn && activeConn.ssl_mode !== "disable" && (
          <span className="inline-flex h-[18px] items-center gap-[5px] whitespace-nowrap text-[10.5px]">
            <Icon.lock size={10} />
            <span>SSL · {activeConn.ssl_mode}</span>
          </span>
        )}
      </div>

      <div className="flex items-center gap-3.5">
        <span className="inline-flex h-[18px] items-center gap-[5px] whitespace-nowrap text-[10.5px]">
          {lastQuery ? (
            <>
              <Icon.check size={10} style={{ color: "var(--accent)" }} />
              <span className="mono">
                {lastQuery.rowCount}
                {lastQuery.truncated ? "+" : ""} rows ·{" "}
                {lastQuery.durationMs} ms
              </span>
            </>
          ) : (
            <span className="mono text-fg-3">— rows · — ms</span>
          )}
        </span>
        <span
          className="inline-flex h-[18px] items-center gap-[5px] whitespace-nowrap font-mono text-[10.5px]"
          style={{ color: "var(--fg-2)" }}
        >
          UTF-8 · LF
        </span>
      </div>
    </div>
  );
}

function shortVersion(v: string): string {
  // `PostgreSQL 16.2 on x86_64-linux-gnu, compiled by …` → `PostgreSQL 16.2`
  const match = v.match(/^(\S+\s+\d+(?:\.\d+)*)/);
  return match?.[1] ?? v.slice(0, 30);
}

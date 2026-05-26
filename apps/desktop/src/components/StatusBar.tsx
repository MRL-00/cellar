import { Icon } from "./icons";

export function StatusBar() {
  return (
    <div className="cellar-statusbar">
      <div className="cellar-statusbar-group">
        <span className="cellar-status-item">
          <span
            className="dot"
            style={{
              background: "var(--accent)",
              boxShadow: "0 0 0 2px var(--accent-soft)",
            }}
          />
          <span style={{ color: "var(--fg-1)" }}>shop-eu (prod)</span>
          <span style={{ color: "var(--fg-3)" }}>·</span>
          <span style={{ color: "var(--eng-postgres)" }}>PostgreSQL</span>
          <span style={{ color: "var(--fg-3)" }}>·</span>
          <span className="mono" style={{ color: "var(--fg-2)" }}>v16.2</span>
        </span>
        <span className="cellar-status-item">
          <Icon.user size={10} />
          <span className="mono">analytics_ro@prod-pg</span>
        </span>
        <span className="cellar-status-item">
          <Icon.lock size={10} />
          <span>SSL · SCRAM-SHA-256</span>
        </span>
      </div>

      <div className="cellar-statusbar-group">
        <span className="cellar-status-item">
          <Icon.check size={10} style={{ color: "var(--accent)" }} />
          <span className="mono">— rows · — ms</span>
        </span>
        <span className="cellar-status-item mono" style={{ color: "var(--fg-2)" }}>
          UTF-8 · LF
        </span>
        <span className="cellar-status-item mono" style={{ color: "var(--fg-2)" }}>
          Ln 1, Col 1
        </span>
      </div>
    </div>
  );
}

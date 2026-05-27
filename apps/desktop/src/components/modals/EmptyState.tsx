import { Icon } from "../icons";
import { ENGINE_META, type Engine } from "../EngineBadge";

const ENGINE_ORDER: Engine[] = ["postgres", "mssql", "azure", "mysql", "sqlite"];

const ENGINE_HEX: Record<Engine, string> = {
  postgres: "#4f8ff7",
  mysql: "#f6a44a",
  mssql: "#d97a5a",
  azure: "#5bb8e0",
  sqlite: "#a78bfa",
};

const SHORT: Record<Engine, string> = {
  postgres: "Postgres",
  mysql: "MySQL",
  mssql: "MSSQL",
  azure: "Azure",
  sqlite: "SQLite",
};

export function EmptyState({ onNew }: { onNew: () => void }) {
  return (
    <div className="empty-root">
      <div className="empty-card">
        <div className="empty-logo">
          <span className="empty-logo-mark" />
        </div>
        <h1 className="empty-title">Welcome to Cellar</h1>
        <p className="empty-sub">
          A fast, native database client with AI built in. Open-source, BYO key.
        </p>

        <div className="empty-actions">
          <button className="empty-action primary" onClick={onNew}>
            <Icon.plus size={12} />
            <span>New connection</span>
          </button>
          <button className="empty-action">
            <Icon.fileText size={12} />
            <span>Import from DataGrip / DBeaver</span>
          </button>
          <button className="empty-action">
            <Icon.cloud size={12} />
            <span>Connect to demo database</span>
          </button>
        </div>

        <div className="empty-engines-label">or pick an engine to start</div>
        <div className="empty-engines">
          {ENGINE_ORDER.map((e) => {
            const m = ENGINE_META[e];
            const hex = ENGINE_HEX[e];
            return (
              <button
                key={e}
                className="empty-engine"
                onClick={onNew}
                title={m.label}
              >
                <span
                  className="empty-engine-letter mono"
                  style={{
                    color: hex,
                    background: `color-mix(in oklab, ${hex} 12%, transparent)`,
                    borderColor: `color-mix(in oklab, ${hex} 30%, transparent)`,
                  }}
                >
                  {m.letter}
                </span>
                <span className="empty-engine-name">{SHORT[e]}</span>
              </button>
            );
          })}
        </div>

        <div className="empty-shortcut-row">
          <span className="empty-shortcut">
            <kbd className="kbd">⌘</kbd>
            <kbd className="kbd">K</kbd>
            <span>command palette</span>
          </span>
          <span className="empty-shortcut">
            <kbd className="kbd">⌘</kbd>
            <kbd className="kbd">N</kbd>
            <span>new connection</span>
          </span>
          <span className="empty-shortcut">
            <kbd className="kbd">⌘</kbd>
            <kbd className="kbd">,</kbd>
            <span>settings</span>
          </span>
        </div>

        <div className="empty-foot">
          <span>
            v0.1.0 · MIT licensed ·{" "}
            <button className="cd-link">docs</button> ·{" "}
            <button className="cd-link">github</button>
          </span>
        </div>
      </div>
    </div>
  );
}

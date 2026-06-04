import { Icon } from "../icons";
import { ENGINE_META, type Engine } from "../EngineBadge";

const ENGINE_ORDER: Engine[] = ["postgres", "firestore", "mssql", "azure", "mysql", "sqlite"];

const ENGINE_HEX: Record<Engine, string> = {
  postgres: "#4f8ff7",
  mysql: "#f6a44a",
  mssql: "#d97a5a",
  azure: "#5bb8e0",
  sqlite: "#a78bfa",
  firestore: "#f4c542",
};

const SHORT: Record<Engine, string> = {
  postgres: "Postgres",
  mysql: "MySQL",
  mssql: "MSSQL",
  azure: "Azure",
  sqlite: "SQLite",
  firestore: "Firestore",
};

export function EmptyState({ onNew }: { onNew: () => void }) {
  return (
    <div className="relative flex flex-1 items-center justify-center overflow-hidden bg-bg-0">
      <div
        className="pointer-events-none absolute inset-0 opacity-60"
        style={{
          backgroundImage:
            "radial-gradient(circle at 30% 20%, var(--accent-soft), transparent 40%), radial-gradient(circle at 70% 80%, color-mix(in oklab, var(--syn-kw) 10%, transparent), transparent 50%)",
        }}
      />
      <div className="relative w-[540px] rounded-xl border border-border-default bg-bg-1 px-9 pt-9 pb-7 text-center shadow-md">
        <div className="mb-[18px] flex justify-center">
          <span
            className="relative h-9 w-9 rounded-lg"
            style={{
              background:
                "linear-gradient(135deg, #c4b5fd 0%, var(--accent) 55%, #6d4ed1 100%)",
              boxShadow: "0 0 24px var(--accent-soft)",
            }}
          >
            <span
              className="absolute inset-[5px] rounded bg-bg-1"
              style={{
                clipPath:
                  "polygon(0 0, 100% 0, 100% 35%, 35% 35%, 35% 65%, 100% 65%, 100% 100%, 0 100%)",
              }}
            />
          </span>
        </div>
        <h1 className="m-0 mb-1 text-[20px] font-semibold tracking-[-0.01em] text-fg-0">
          Welcome to Cellar
        </h1>
        <p
          className="m-0 mb-[22px] text-[12.5px] text-fg-2"
          style={{ textWrap: "pretty" }}
        >
          A fast, native database client with AI built in. Open-source, BYO key.
        </p>

        <div className="mb-[22px] flex flex-col gap-1.5">
          <button
            onClick={onNew}
            className="flex h-8 items-center justify-center gap-2 whitespace-nowrap rounded-[6px] border px-3 text-xs font-medium text-accent-fg transition-[filter] duration-[120ms] hover:brightness-[1.07]"
            style={{
              background:
                "linear-gradient(135deg, #c4b5fd 0%, var(--accent) 55%, #6d4ed1 100%)",
              borderColor: "color-mix(in oklab, var(--accent) 40%, black)",
            }}
          >
            <Icon.plus size={12} />
            <span>New connection</span>
          </button>
          <button
            disabled
            title="Connection import is not wired yet"
            className="flex h-8 cursor-not-allowed items-center justify-center gap-2 whitespace-nowrap rounded-[6px] border border-border-default bg-bg-2 px-3 text-xs text-fg-2 opacity-55"
          >
            <Icon.fileText size={12} />
            <span>Import from DataGrip / DBeaver</span>
          </button>
          <button
            disabled
            title="Demo database provisioning is not wired yet"
            className="flex h-8 cursor-not-allowed items-center justify-center gap-2 whitespace-nowrap rounded-[6px] border border-border-default bg-bg-2 px-3 text-xs text-fg-2 opacity-55"
          >
            <Icon.cloud size={12} />
            <span>Connect to demo database</span>
          </button>
        </div>

        <div className="mb-2 text-[10px] uppercase tracking-[0.06em] text-fg-3">
          or pick an engine to start
        </div>
        <div className="mb-[22px] grid grid-cols-6 gap-1.5">
          {ENGINE_ORDER.map((e) => {
            const m = ENGINE_META[e];
            const hex = ENGINE_HEX[e];
            const available = e === "postgres";
            return (
              <button
                key={e}
                onClick={available ? onNew : undefined}
                disabled={!available}
                title={available ? m.label : `${m.label} support is coming soon`}
                className={
                  "flex flex-col items-center gap-1.5 rounded-[6px] border border-border-default bg-bg-2 px-1.5 pt-2.5 pb-2 transition-all duration-150 " +
                  (available
                    ? "hover:-translate-y-px hover:border-border-strong"
                    : "cursor-not-allowed opacity-45")
                }
              >
                <span
                  className="inline-flex h-[26px] w-[26px] items-center justify-center rounded-[5px] border font-mono text-xs font-semibold"
                  style={{
                    color: hex,
                    background: `color-mix(in oklab, ${hex} 12%, transparent)`,
                    borderColor: `color-mix(in oklab, ${hex} 30%, transparent)`,
                  }}
                >
                  {m.letter}
                </span>
                <span className="whitespace-nowrap text-[10px] text-fg-1">
                  {SHORT[e]}
                </span>
              </button>
            );
          })}
        </div>

        <div className="mb-[18px] flex justify-center gap-4">
          <span className="inline-flex items-center gap-1 whitespace-nowrap text-[10.5px] text-fg-3">
            <kbd className="kbd">⌘</kbd>
            <kbd className="kbd">K</kbd>
            <span>command palette</span>
          </span>
          <span className="inline-flex items-center gap-1 whitespace-nowrap text-[10.5px] text-fg-3">
            <kbd className="kbd">⌘</kbd>
            <kbd className="kbd">N</kbd>
            <span>new connection</span>
          </span>
          <span className="inline-flex items-center gap-1 whitespace-nowrap text-[10.5px] text-fg-3">
            <kbd className="kbd">⌘</kbd>
            <kbd className="kbd">,</kbd>
            <span>settings</span>
          </span>
        </div>

        <div className="border-t border-border-divider pt-4 text-[10.5px] text-fg-3">
          <span>
            v0.1.0 · MIT licensed ·{" "}
            <button
              disabled
              title="Documentation links are not wired in the desktop shell yet"
              className="cursor-not-allowed bg-transparent text-[10.5px] text-fg-3 underline underline-offset-2 opacity-70"
            >
              docs
            </button>{" "}
            ·{" "}
            <button
              disabled
              title="External links are not wired in the desktop shell yet"
              className="cursor-not-allowed bg-transparent text-[10.5px] text-fg-3 underline underline-offset-2 opacity-70"
            >
              github
            </button>
          </span>
        </div>
      </div>
    </div>
  );
}

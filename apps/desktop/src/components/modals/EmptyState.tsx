import { Icon } from "../icons";
import { ENGINE_META, type Engine } from "../EngineBadge";
import { EngineLogo } from "../EngineLogo";
import { CellarMark } from "../CellarMark";

const ENGINE_ORDER: Engine[] = [
  "postgres",
  "firestore",
  "convex",
  "cosmos",
  "mssql",
  "mysql",
  "sqlite",
];

const ENGINE_HEX: Record<Engine, string> = {
  postgres: "#a1a1a1",
  mysql: "#a1a1a1",
  mssql: "#a1a1a1",
  azure: "#a1a1a1",
  sqlite: "#a1a1a1",
  firestore: "#a1a1a1",
  convex: "#a1a1a1",
  cosmos: "#a1a1a1",
  supabase: "#a1a1a1",
  neon: "#a1a1a1",
  planetscale: "#a1a1a1",
};

const SHORT: Record<Engine, string> = {
  postgres: "Postgres",
  mysql: "MySQL",
  mssql: "MSSQL",
  azure: "Azure",
  sqlite: "SQLite",
  firestore: "Firestore",
  convex: "Convex",
  cosmos: "Cosmos",
  supabase: "Supabase",
  neon: "Neon",
  planetscale: "PlanetScale",
};

export function EmptyState({ onNew }: { onNew: () => void }) {
  return (
    <div className="relative flex flex-1 items-center justify-center overflow-hidden bg-bg-0">
      <div className="relative w-[540px] rounded-lg border border-border-default bg-bg-1 px-9 pt-9 pb-7 text-center shadow-md">
        <div className="mb-[18px] flex justify-center">
          <CellarMark
            accented
            className="h-12 w-12 drop-shadow-[0_0_14px_var(--accent-soft)]"
          />
        </div>
        <h1 className="m-0 mb-1 text-[21px] font-semibold tracking-[-0.01em] text-fg-0">
          Welcome to Cellar
        </h1>
        <p
          className="m-0 mb-[22px] text-sm text-fg-2"
          style={{ textWrap: "pretty" }}
        >
          Connect to Postgres, inspect schemas, run SQL, and browse table data.
        </p>

        <div className="mb-[22px] flex flex-col gap-1.5">
          <button
            onClick={onNew}
            className="flex h-8 items-center justify-center gap-2 whitespace-nowrap rounded-[6px] border px-3 text-sm font-medium text-accent-fg transition-[filter] duration-[120ms] hover:brightness-[1.07]"
            style={{
              background: "var(--accent)",
              borderColor: "var(--accent-line)",
            }}
          >
            <Icon.plus size={12} />
            <span>New connection</span>
          </button>
          <button
            disabled
            title="Connection import is not wired yet"
            className="flex h-8 cursor-not-allowed items-center justify-center gap-2 whitespace-nowrap rounded-[6px] border border-border-default bg-bg-2 px-3 text-sm text-fg-2 opacity-55"
          >
            <Icon.fileText size={12} />
            <span>Import from DataGrip / DBeaver</span>
          </button>
          <button
            disabled
            title="Demo database provisioning is not wired yet"
            className="flex h-8 cursor-not-allowed items-center justify-center gap-2 whitespace-nowrap rounded-[6px] border border-border-default bg-bg-2 px-3 text-sm text-fg-2 opacity-55"
          >
            <Icon.cloud size={12} />
            <span>Connect to demo database</span>
          </button>
        </div>

        <div className="mb-2 text-[11px] uppercase tracking-[0.06em] text-fg-3">
          or pick an engine to start
        </div>
        <div className="mb-[22px] grid grid-cols-[repeat(auto-fill,minmax(92px,1fr))] gap-1.5">
          {ENGINE_ORDER.map((e) => {
            const m = ENGINE_META[e];
            const hex = ENGINE_HEX[e];
            const available =
              e === "postgres" ||
              e === "mssql" ||
              e === "firestore" ||
              e === "convex" ||
              e === "cosmos" ||
              e === "mysql";
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
                  <EngineLogo engine={e} size={16} />
                </span>
                <span className="whitespace-nowrap text-sm text-fg-1">
                  {SHORT[e]}
                </span>
              </button>
            );
          })}
        </div>

        <div className="mb-[18px] flex justify-center gap-4">
          <span className="inline-flex items-center gap-1 whitespace-nowrap text-[11.5px] text-fg-3">
            <kbd className="kbd">⌘</kbd>
            <kbd className="kbd">K</kbd>
            <span>command palette</span>
          </span>
          <span className="inline-flex items-center gap-1 whitespace-nowrap text-[11.5px] text-fg-3">
            <kbd className="kbd">⌘</kbd>
            <kbd className="kbd">N</kbd>
            <span>new connection</span>
          </span>
          <span className="inline-flex items-center gap-1 whitespace-nowrap text-[11.5px] text-fg-3">
            <kbd className="kbd">⌘</kbd>
            <kbd className="kbd">,</kbd>
            <span>settings</span>
          </span>
        </div>

        <div className="border-t border-border-divider pt-4 text-[11.5px] text-fg-3">
          <span>
            v0.1.0 · MIT licensed ·{" "}
            <button
              disabled
              title="Documentation links are not wired in the desktop shell yet"
              className="cursor-not-allowed bg-transparent text-[11.5px] text-fg-3 underline underline-offset-2 opacity-70"
            >
              docs
            </button>{" "}
            ·{" "}
            <button
              disabled
              title="External links are not wired in the desktop shell yet"
              className="cursor-not-allowed bg-transparent text-[11.5px] text-fg-3 underline underline-offset-2 opacity-70"
            >
              github
            </button>
          </span>
        </div>
      </div>
    </div>
  );
}

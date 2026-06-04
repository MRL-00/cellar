import { Icon } from "../icons";
import { Row, Section, StaticSegment, Toggle } from "./settingsPrimitives";

export function SettingsPrivacy() {
  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section title="Telemetry">
        <Row
          label="Send anonymous usage stats"
          hint="counts of feature use, no query content"
        >
          <Toggle on={false} ariaLabel="Send anonymous usage stats" />
        </Row>
        <Row label="Send crash reports" hint="stack traces only, never DB contents">
          <Toggle on={false} ariaLabel="Send crash reports" />
        </Row>
      </Section>
      <Section
        title="Stored locally only"
        sub="Cellar never uploads any of these. Open ~/.cellar to inspect."
      >
        <div className="w-full overflow-hidden rounded-[5px] border border-border-default">
          {[
            { k: "Connections", v: "12 connections", path: "connections.toml" },
            {
              k: "Query history",
              v: "23,418 queries · 14.2 MB",
              path: "history.sqlite",
            },
            {
              k: "AI conversations",
              v: "20 conversations · 3.2 MB",
              path: "ai/",
            },
            { k: "Snapshots", v: "84 snapshots · 412 MB", path: "snapshots/" },
            { k: "Cached schemas", v: "12 dbs · 8.4 MB", path: "cache/" },
          ].map((x, i, arr) => (
            <div
              key={x.k}
              className={
                "grid grid-cols-[160px_1fr_auto_22px] items-center gap-2.5 bg-bg-2 px-2.5 py-1.5 text-[11px] hover:bg-bg-3 " +
                (i !== arr.length - 1 ? "border-b border-border-divider" : "")
              }
            >
              <span className="font-medium text-fg-0">{x.k}</span>
              <span className="text-fg-2">{x.v}</span>
              <span className="font-mono text-[10.5px] text-fg-3">
                ~/.cellar/{x.path}
              </span>
              <button
                type="button"
                disabled
                className="icon-btn cursor-not-allowed opacity-60"
                title="Local file browsing is not wired yet"
              >
                <Icon.chevronRight size={10} />
              </button>
            </div>
          ))}
        </div>
      </Section>
    </div>
  );
}

export function SettingsUpdates() {
  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section title="Updates">
        <div className="mb-2 flex items-center justify-between rounded-[5px] border border-border-default bg-bg-inset px-3 py-2.5">
          <div className="flex items-center gap-2.5">
            <span className="font-mono text-[13px] font-semibold text-fg-0">
              v0.1.0-alpha
            </span>
            <span className="inline-flex items-center gap-1 text-[11px]">
              <Icon.info size={11} stroke="var(--fg-2)" />
              <span className="text-fg-2">Updater not configured</span>
            </span>
            <span className="text-[11px] text-fg-3">last checked never</span>
          </div>
          <button
            type="button"
            disabled
            title="Updater checks are not wired yet"
            className="inline-flex h-[26px] cursor-not-allowed items-center gap-1 rounded-[4px] border border-border-default bg-bg-2 px-2 text-[11px] text-fg-2 opacity-70"
          >
            <Icon.power size={11} />
            <span>Check now</span>
          </button>
        </div>
        <Row label="Channel">
          <StaticSegment values={["stable", "beta", "nightly"]} activeIdx={0} />
        </Row>
        <Row label="Auto-install on quit">
          <Toggle on={false} ariaLabel="Auto-install on quit" />
        </Row>
      </Section>
    </div>
  );
}

export function SettingsAbout() {
  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section title="About">
        <div className="flex items-start gap-4">
          <span
            className="relative h-12 w-12 shrink-0 rounded-[10px]"
            style={{
              background:
                "linear-gradient(135deg, var(--accent), color-mix(in oklab, var(--accent) 50%, var(--syn-kw)))",
              boxShadow: "0 0 24px var(--accent-soft)",
            }}
          >
            <span
              className="absolute inset-2 rounded-[4px] bg-bg-1"
              style={{
                clipPath:
                  "polygon(0 0, 100% 0, 100% 35%, 35% 35%, 35% 65%, 100% 65%, 100% 100%, 0 100%)",
              }}
            />
          </span>
          <div>
            <div className="text-[18px] font-semibold tracking-[-0.01em] text-fg-0">
              Cellar
            </div>
            <div className="mb-2 text-[12px] text-fg-2">
              A fast, native database client with AI built in.
            </div>
            <div className="mb-2.5 flex gap-1.5 font-mono text-[10.5px] text-fg-2">
              <span>v0.1.0-alpha · development build</span>
              <span className="text-fg-3">·</span>
              <span>MIT licensed</span>
              <span className="text-fg-3">·</span>
              <span>commit unavailable</span>
            </div>
            <div className="flex gap-1.5 text-[11px]">
              <button
                type="button"
                disabled
                className="cursor-not-allowed text-fg-3 underline underline-offset-2 opacity-70"
                title="Documentation links are not wired in the desktop shell yet"
              >
                docs
              </button>
              <span className="text-fg-3">·</span>
              <button
                type="button"
                disabled
                className="cursor-not-allowed text-fg-3 underline underline-offset-2 opacity-70"
                title="External links are not wired in the desktop shell yet"
              >
                github
              </button>
              <span className="text-fg-3">·</span>
              <button
                type="button"
                disabled
                className="cursor-not-allowed text-fg-3 underline underline-offset-2 opacity-70"
                title="Changelog links are not wired in the desktop shell yet"
              >
                changelog
              </button>
              <span className="text-fg-3">·</span>
              <button
                type="button"
                disabled
                className="cursor-not-allowed text-fg-3 underline underline-offset-2 opacity-70"
                title="Acknowledgements links are not wired in the desktop shell yet"
              >
                acknowledgements
              </button>
            </div>
          </div>
        </div>
      </Section>
    </div>
  );
}

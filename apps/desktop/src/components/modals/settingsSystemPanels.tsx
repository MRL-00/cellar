import { Icon } from "../icons";
import { Row, Section, StaticSegment, Toggle } from "./settingsPrimitives";
import { useConnections } from "../../state/connections";
import { useUpdater } from "../../lib/updater";
import { openExternal } from "../../lib/openExternal";

export function SettingsPrivacy() {
  const connectionCount = useConnections((s) => s.connections.length);

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
            {
              k: "Connections",
              v: `${connectionCount} ${connectionCount === 1 ? "connection" : "connections"}`,
              path: "connections.toml",
            },
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
  const { appVersion, status, lastChecked, checkForUpdate, downloadAndInstall } = useUpdater();

  const versionLabel = appVersion ? `v${appVersion}` : "v0.0.0";
  const lastCheckedLabel = lastChecked
    ? `last checked ${new Date(lastChecked).toLocaleString()}`
    : "last checked never";

  const statusText = (() => {
    switch (status.kind) {
      case "idle":
        return "Ready";
      case "checking":
        return "Checking…";
      case "available":
        return `Update available: v${status.version}`;
      case "up-to-date":
        return "Up to date";
      case "downloading":
        return `Downloading… ${Math.round(status.fraction * 100)}%`;
      case "installing":
        return "Installing…";
      case "error":
        return `Error: ${status.message}`;
    }
  })();

  const isBusy =
    status.kind === "checking" ||
    status.kind === "downloading" ||
    status.kind === "installing";
  const canCheck = !isBusy && status.kind !== "available";
  const canInstall = status.kind === "available";

  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section title="Updates">
        <div className="mb-2 flex items-center justify-between rounded-[5px] border border-border-default bg-bg-inset px-3 py-2.5">
          <div className="flex items-center gap-2.5">
            <span className="font-mono text-[13px] font-semibold text-fg-0">
              {versionLabel}
            </span>
            <span className="inline-flex items-center gap-1 text-[11px]">
              <Icon.info size={11} stroke="var(--fg-2)" />
              <span className="text-fg-2">{statusText}</span>
            </span>
            <span className="text-[11px] text-fg-3">{lastCheckedLabel}</span>
          </div>
          <div className="flex items-center gap-1.5">
            {canInstall && (
              <button
                type="button"
                onClick={downloadAndInstall}
                disabled={isBusy}
                className="inline-flex h-[26px] items-center gap-1 rounded-[4px] border border-accent-line bg-accent-soft px-2 text-[11px] font-medium text-accent hover:bg-accent/20"
              >
                <Icon.download size={11} />
                <span>Download &amp; install</span>
              </button>
            )}
            <button
              type="button"
              onClick={checkForUpdate}
              disabled={!canCheck}
              title={canCheck ? "Check for updates" : "Update check in progress"}
              className={
                canCheck
                  ? "inline-flex h-[26px] items-center gap-1 rounded-[4px] border border-border-default bg-bg-2 px-2 text-[11px] text-fg-1 hover:bg-bg-3"
                  : "inline-flex h-[26px] cursor-not-allowed items-center gap-1 rounded-[4px] border border-border-default bg-bg-2 px-2 text-[11px] text-fg-2 opacity-70"
              }
            >
              <Icon.power size={11} />
              <span>Check now</span>
            </button>
          </div>
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
  const { appVersion } = useUpdater();
  const versionLabel = appVersion ? `v${appVersion}` : "v0.0.0";
  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section title="About">
        <div className="flex items-start gap-4">
          <span
            className="relative h-12 w-12 shrink-0 rounded-[10px]"
            style={{
              background:
                "linear-gradient(135deg, #c4b5fd 0%, #a78bfa 55%, #6d4ed1 100%)",
              boxShadow: "0 0 24px rgba(167, 139, 250, 0.14)",
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
              <span>
                {versionLabel}
                {import.meta.env.DEV ? " · development build" : ""}
              </span>
              <span className="text-fg-3">·</span>
              <span>MIT licensed</span>
              <span className="text-fg-3">·</span>
              <span>commit unavailable</span>
            </div>
            <div className="mb-2.5 text-[11px] text-fg-2">
              built by{" "}
              <button
                type="button"
                onClick={() => openExternal("https://x.com/codermatt")}
                className="text-fg-1 underline underline-offset-2 hover:text-fg-0"
              >
                Matt List
              </button>
            </div>
            <div className="flex gap-1.5 text-[11px]">
              <button
                type="button"
                disabled
                className="cursor-not-allowed text-fg-3 underline underline-offset-2 opacity-70"
                title="Documentation is coming soon"
              >
                docs
              </button>
              <span className="text-fg-3">·</span>
              <button
                type="button"
                onClick={() => openExternal("https://github.com/MRL-00/cellar")}
                className="text-fg-2 underline underline-offset-2 hover:text-fg-0"
              >
                github
              </button>
              <span className="text-fg-3">·</span>
              <button
                type="button"
                onClick={() =>
                  openExternal("https://github.com/MRL-00/cellar/releases")
                }
                className="text-fg-2 underline underline-offset-2 hover:text-fg-0"
              >
                changelog
              </button>
              <span className="text-fg-3">·</span>
              <button
                type="button"
                disabled
                className="cursor-not-allowed text-fg-3 underline underline-offset-2 opacity-70"
                title="Acknowledgements are coming soon"
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

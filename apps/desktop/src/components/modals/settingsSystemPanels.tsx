import { Icon } from "../icons";
import { CellarMark } from "../CellarMark";
import { Row, Section, StaticSegment, Toggle } from "./settingsPrimitives";
import { useConnections } from "../../state/connections";
import { useUpdater } from "../../lib/updater";
import { notesSince } from "../../lib/releaseNotes";
import { openExternal } from "../../lib/openExternal";
import changelogMd from "../../../../../CHANGELOG.md?raw";

// Minimal markdown renderer for release notes / changelog: headings, bullets,
// and **bold**. GitHub release bodies and our CHANGELOG only use this subset,
// so a dependency-free pass is enough. ponytail: extend if notes get richer.
function renderInline(text: string, keyBase: string) {
  return text.split(/(\*\*[^*]+\*\*)/g).map((part, i) =>
    part.startsWith("**") && part.endsWith("**") ? (
      <strong key={`${keyBase}-${i}`} className="font-semibold text-fg-0">
        {part.slice(2, -2)}
      </strong>
    ) : (
      part
    ),
  );
}

function Changelog({ source }: { source: string }) {
  const lines = source.replace(/\r\n/g, "\n").split("\n");
  return (
    <div className="max-h-[260px] overflow-y-auto rounded-[5px] border border-border-default bg-bg-inset px-3 py-2.5 text-[12.5px] leading-[1.55] text-fg-2">
      {lines.map((raw, i) => {
        const line = raw.trimEnd();
        if (!line.trim()) return <div key={i} className="h-1.5" />;
        const h = /^(#{1,6})\s+(.*)$/.exec(line);
        if (h) {
          const top = h[1]!.length <= 2;
          return (
            <div
              key={i}
              className={
                "mt-2 mb-1 font-semibold text-fg-0 " +
                (top ? "text-sm" : "text-sm")
              }
            >
              {renderInline(h[2]!, `h${i}`)}
            </div>
          );
        }
        const bullet = /^[-*]\s+(.*)$/.exec(line);
        if (bullet) {
          return (
            <div key={i} className="flex gap-1.5 pl-1">
              <span className="text-fg-3">•</span>
              <span>{renderInline(bullet[1]!, `b${i}`)}</span>
            </div>
          );
        }
        return <div key={i}>{renderInline(line, `p${i}`)}</div>;
      })}
    </div>
  );
}

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
                "grid grid-cols-[160px_1fr_auto_22px] items-center gap-2.5 bg-bg-2 px-2.5 py-1.5 text-sm hover:bg-bg-3 " +
                (i !== arr.length - 1 ? "border-b border-border-divider" : "")
              }
            >
              <span className="font-medium text-fg-0">{x.k}</span>
              <span className="text-fg-2">{x.v}</span>
              <span className="font-mono text-[11.5px] text-fg-3">
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
            <span className="font-mono text-sm font-semibold text-fg-0">
              {versionLabel}
            </span>
            <span className="inline-flex items-center gap-1 text-[12px]">
              <Icon.info size={11} stroke="var(--fg-2)" />
              <span className="text-fg-2">{statusText}</span>
            </span>
            <span className="text-sm text-fg-3">{lastCheckedLabel}</span>
          </div>
          <div className="flex items-center gap-1.5">
            {canInstall && (
              <button
                type="button"
                onClick={downloadAndInstall}
                disabled={isBusy}
                className="update-download-cta inline-flex h-[26px] items-center gap-1 rounded-[4px] px-2 text-[12px] font-medium text-accent"
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
                  ? "inline-flex h-[26px] items-center gap-1 rounded-[4px] border border-border-default bg-bg-2 px-2 text-[12px] text-fg-1 hover:bg-bg-3"
                  : "inline-flex h-[26px] cursor-not-allowed items-center gap-1 rounded-[4px] border border-border-default bg-bg-2 px-2 text-[12px] text-fg-2 opacity-70"
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
      <Section
        title="What's new"
        sub={
          status.kind === "available"
            ? `What changed since ${versionLabel}`
            : `Recent changes in ${versionLabel}`
        }
      >
        <Changelog
          source={
            status.kind === "available" && status.update.body
              ? notesSince(status.update.body, appVersion)
              : changelogMd
          }
        />
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
          <CellarMark
            accented
            className="h-12 w-12 shrink-0 drop-shadow-[0_0_14px_var(--accent-soft)]"
          />
          <div>
            <div className="text-[19px] font-semibold tracking-[-0.01em] text-fg-0">
              Cellar
            </div>
            <div className="mb-2 text-sm text-fg-2">
              A fast, native database client with AI built in.
            </div>
            <div className="mb-2.5 flex gap-1.5 font-mono text-[11.5px] text-fg-2">
              <span>
                {versionLabel}
                {import.meta.env.DEV ? " · development build" : ""}
              </span>
              <span className="text-fg-3">·</span>
              <span>MIT licensed</span>
              <span className="text-fg-3">·</span>
              <span>commit unavailable</span>
            </div>
            <div className="mb-2.5 text-sm text-fg-2">
              built by{" "}
              <button
                type="button"
                onClick={() => void openExternal("https://x.com/codermatt")}
                className="text-fg-1 underline underline-offset-2 hover:text-fg-0"
              >
                Matt List
              </button>
            </div>
            <div className="flex gap-1.5 text-[12px]">
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
                onClick={() => void openExternal("https://github.com/MRL-00/cellar")}
                className="text-fg-2 underline underline-offset-2 hover:text-fg-0"
              >
                github
              </button>
              <span className="text-fg-3">·</span>
              <button
                type="button"
                onClick={() =>
                  void openExternal("https://github.com/MRL-00/cellar/releases")
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

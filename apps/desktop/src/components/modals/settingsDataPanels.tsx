import { Icon } from "../icons";
import {
  CD_INPUT,
  Row,
  Section,
  StaticSegment,
  StubBanner,
  Toggle,
} from "./settingsPrimitives";

export function SettingsConnections() {
  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section
        title="Defaults for new connections"
        sub="Applied when you create a connection. Per-connection overrides win."
      >
        <Row label="Read-only by default">
          <Toggle on={true} ariaLabel="Read-only by default" />
        </Row>
        <Row label="Connection timeout">
          <input
            readOnly
            className={CD_INPUT + " cursor-not-allowed font-mono opacity-80"}
            defaultValue="10"
            style={{ width: 70, flex: "none" }}
          />
          <span className="text-[11px] text-fg-2">seconds</span>
        </Row>
        <Row label="Keep-alive interval">
          <input
            readOnly
            className={CD_INPUT + " cursor-not-allowed font-mono opacity-80"}
            defaultValue="30"
            style={{ width: 70, flex: "none" }}
          />
          <span className="text-[11px] text-fg-2">seconds</span>
        </Row>
        <Row label="Application name">
          <input
            readOnly
            className={CD_INPUT + " cursor-not-allowed font-mono opacity-80"}
            defaultValue="cellar (alice@laptop)"
            style={{ flex: 1 }}
          />
        </Row>
      </Section>
      <Section
        title="Production safety"
        sub="Cellar will require you to type the connection name before running these against any 'prod' connection."
      >
        <Row label="Confirm DML on prod">
          <Toggle on={true} locked ariaLabel="Confirm DML on prod" />
        </Row>
        <Row label="Confirm DROP / TRUNCATE on prod">
          <Toggle on={true} locked ariaLabel="Confirm DROP or TRUNCATE on prod" />
        </Row>
        <Row label="Block UPDATE without WHERE">
          <Toggle on={true} ariaLabel="Block UPDATE without WHERE" />
        </Row>
        <Row label="Block DELETE without WHERE">
          <Toggle on={true} ariaLabel="Block DELETE without WHERE" />
        </Row>
        <Row label="Max rows affected before warn">
          <input
            readOnly
            className={CD_INPUT + " cursor-not-allowed font-mono opacity-80"}
            defaultValue="100"
            style={{ width: 70, flex: "none" }}
          />
          <span className="text-[11px] text-fg-2">rows</span>
        </Row>
      </Section>
    </div>
  );
}

export function SettingsHistory() {
  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section title="Query history">
        <Row label="Retain history for">
          <StaticSegment
            values={["7 days", "30 days", "90 days", "forever"]}
            activeIdx={2}
          />
        </Row>
        <Row label="Store query results">
          <Toggle on={false} ariaLabel="Store query results" />
        </Row>
      </Section>
      <StubBanner>23,418 queries · 14.2 MB · last cleared 12 days ago</StubBanner>
    </div>
  );
}

export function SettingsBackups() {
  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section title="Backups">
        <Row
          label="Auto-snapshot before commits"
          hint="pg_dump --schema-only + affected rows"
        >
          <Toggle on={true} ariaLabel="Auto-snapshot before commits" />
        </Row>
        <Row label="Snapshot location">
          <input
            readOnly
            className={CD_INPUT + " cursor-not-allowed font-mono opacity-80"}
            defaultValue="~/.cellar/snapshots"
            style={{ flex: 1 }}
          />
          <button
            type="button"
            disabled
            title="Snapshot location browsing is not wired yet"
            className="inline-flex h-[26px] cursor-not-allowed items-center gap-1 rounded-[4px] border border-border-default bg-bg-2 px-2 text-[11px] text-fg-2 opacity-70"
          >
            <Icon.fileText size={11} />
            <span>Browse</span>
          </button>
        </Row>
        <Row label="Retain snapshots for">
          <input
            readOnly
            className={CD_INPUT + " cursor-not-allowed font-mono opacity-80"}
            defaultValue="30"
            style={{ width: 70, flex: "none" }}
          />
          <span className="text-[11px] text-fg-2">days</span>
        </Row>
      </Section>
      <Section title="Export defaults">
        <Row label="Format">
          <StaticSegment
            values={["CSV", "JSON", "Parquet", "SQL INSERT"]}
            activeIdx={0}
          />
        </Row>
        <Row label="NULL as">
          <input
            readOnly
            className={CD_INPUT + " cursor-not-allowed font-mono opacity-80"}
            defaultValue="\\N"
            style={{ width: 120, flex: "none" }}
          />
        </Row>
        <Row label="Include headers">
          <Toggle on={true} ariaLabel="Include headers" />
        </Row>
      </Section>
    </div>
  );
}

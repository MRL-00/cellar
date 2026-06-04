import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { ConnectionConfig, DriverInfo, EnvTag, SslMode } from "@cellar/ipc";
import { commands, unwrap } from "@cellar/ipc";

import { Icon } from "../icons";
import { ENGINE_META, type Engine } from "../EngineBadge";
import { Modal } from "./Modal";
import { useConnections } from "../../state/connections";

const ENGINE_ORDER: Engine[] = ["postgres", "firestore", "mssql", "azure", "mysql", "sqlite"];

const ENGINE_HEX: Record<Engine, string> = {
  postgres: "#4f8ff7",
  mysql: "#f6a44a",
  mssql: "#d97a5a",
  azure: "#5bb8e0",
  sqlite: "#a78bfa",
  firestore: "#f4c542",
};

const SWATCH_COLORS = ["#4f8ff7", "#f6a44a", "#d97a5a", "#5bb8e0", "#a78bfa", "#4ade80", "#f87171"];

const DEFAULT_PORT: Record<Engine, number> = {
  postgres: 5432,
  mysql: 3306,
  mssql: 1433,
  azure: 1433,
  sqlite: 0,
  firestore: 443,
};

type Tab = "general" | "ssh" | "ssl" | "options";
type TestStatus =
  | { kind: "idle" }
  | { kind: "running" }
  | { kind: "ok"; info: DriverInfo; durationMs: number }
  | { kind: "error"; message: string };

const ENV_TAGS: EnvTag[] = ["prod", "staging", "dev", "local"];
const SSL_MODES: SslMode[] = [
  "disable",
  "prefer",
  "require",
  "verify-ca",
  "verify-full",
];

const ED_RUN_BASE =
  "inline-flex h-[26px] items-center gap-[5px] whitespace-nowrap rounded-[4px] border border-transparent px-2.5 text-[11.5px] font-medium transition-[background,color,border-color,filter] duration-[120ms]";

const ED_RUN_SUBTLE =
  ED_RUN_BASE +
  " text-fg-1 bg-transparent border-border-default hover:bg-bg-3 hover:border-border-strong hover:text-fg-0 disabled:opacity-40 disabled:cursor-not-allowed";

const ED_RUN_PRIMARY =
  ED_RUN_BASE +
  " bg-accent text-accent-fg hover:brightness-[1.07] disabled:opacity-40 disabled:cursor-not-allowed";

const CD_INPUT =
  "h-[26px] min-w-0 flex-1 rounded-[4px] border border-border-default bg-bg-inset px-2 text-[11.5px] text-fg-0 outline-none font-sans focus:border-accent-line focus:bg-bg-2";

interface ConnectionDialogProps {
  onClose: () => void;
  /** "edit" keeps the existing id; "new" derives a fresh one from the name. */
  mode?: "new" | "edit";
  /** Prefill values — an existing connection (edit) or a copy seed (duplicate). */
  initial?: ConnectionConfig;
}

export function ConnectionDialog({
  onClose,
  mode = "new",
  initial,
}: ConnectionDialogProps) {
  const saveConnection = useConnections((s) => s.saveConnection);
  const isEdit = mode === "edit" && !!initial;

  const [engine, setEngine] = useState<Engine>(
    (initial?.engine as Engine) ?? "postgres",
  );
  const [tab, setTab] = useState<Tab>("general");
  const [ssh, setSsh] = useState(false);
  const [ssl, setSsl] = useState(
    initial ? initial.ssl_mode !== "disable" : true,
  );
  const [sslMode, setSslMode] = useState<SslMode>(
    initial && initial.ssl_mode !== "disable" ? initial.ssl_mode : "prefer",
  );
  const [swatch, setSwatch] = useState<string>(
    initial?.color ?? ENGINE_HEX[(initial?.engine as Engine | undefined) ?? "postgres"],
  );
  const [envTag, setEnvTag] = useState<EnvTag>(initial?.env_tag ?? "local");

  const [name, setName] = useState(initial?.name ?? "");
  const [host, setHost] = useState(initial?.host ?? "localhost");
  const [port, setPort] = useState<number>(initial?.port ?? DEFAULT_PORT.postgres);
  const [database, setDatabase] = useState(initial?.database ?? "postgres");
  const [user, setUser] = useState(initial?.user ?? "");
  const [password, setPassword] = useState("");
  const [appName, setAppName] = useState(initial?.application_name ?? "cellar");

  const [test, setTest] = useState<TestStatus>({ kind: "idle" });
  const [savingError, setSavingError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  // Only snap the port/swatch to engine defaults when the user picks an engine
  // for a *new* connection — never clobber values loaded for editing.
  const userPickedEngine = useRef(false);
  useEffect(() => {
    if (!userPickedEngine.current) return;
    setPort(DEFAULT_PORT[engine] || 5432);
    setSwatch(ENGINE_HEX[engine]);
    if (engine === "firestore") {
      setHost("firestore.googleapis.com");
      setDatabase("");
      setUser("(default)");
      setSsl(true);
      setSslMode("require");
    }
  }, [engine]);

  const derivedId = useMemo(
    () => slugify(name || `${host}-${database}`),
    [name, host, database],
  );
  const id = isEdit && initial ? initial.id : derivedId;

  const buildConfig = (): ConnectionConfig => ({
    id,
    name: name || `${user || "user"}@${host}/${database}`,
    engine: engine as ConnectionConfig["engine"],
    host,
    port,
    database,
    user,
    ssl_mode: ssl ? sslMode : "disable",
    env_tag: envTag,
    application_name: appName || null,
    color: swatch,
  });

  const onTest = async () => {
    setTest({ kind: "running" });
    const started = performance.now();
    try {
      const info = await unwrap(
        commands.testConnection(buildConfig(), password || null),
      );
      const durationMs = Math.round(performance.now() - started);
      setTest({ kind: "ok", info, durationMs });
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setTest({ kind: "error", message });
    }
  };

  const onSave = async () => {
    setSavingError(null);
    setSaving(true);
    try {
      await saveConnection(buildConfig(), password || null);
      onClose();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setSavingError(message);
    } finally {
      setSaving(false);
    }
  };

  const sqliteOnly = engine === "sqlite";
  const isFirestore = engine === "firestore";
  const hostLabel = isFirestore ? "API host" : "Host";
  const databaseLabel = isFirestore ? "Project ID" : "Database";
  const userLabel = isFirestore ? "Database ID" : "User";
  const passwordLabel = isFirestore ? "Credentials" : "Password";
  const passwordHint = isFirestore
    ? isEdit
      ? "Leave blank to keep saved JSON/token"
      : "Leave blank for emulator; JSON/token is stored in OS keychain"
    : isEdit
      ? "Leave blank to keep the saved password"
      : "Stored in OS keychain";
  const canSave = Boolean(host && database && (isFirestore || user));

  return (
    <Modal onClose={onClose} width={760}>
      <div className="flex h-[38px] shrink-0 items-center justify-between border-b border-border-default pl-3.5 pr-2">
        <div className="flex items-center gap-2">
          <span className="inline-flex text-accent">
            <Icon.database size={14} />
          </span>
          <span className="whitespace-nowrap text-[12.5px] font-semibold text-fg-0">
            {isEdit ? "Edit connection" : "New connection"}
          </span>
        </div>
        <button className="icon-btn" onClick={onClose} title="Close">
          <Icon.close size={13} />
        </button>
      </div>

      <div className="flex-1 overflow-y-auto px-4 pt-3 pb-4">
        <div className="mb-3.5 grid grid-cols-6 gap-1.5">
          {ENGINE_ORDER.map((e) => {
            const m = ENGINE_META[e];
            const hex = ENGINE_HEX[e];
            const active = engine === e;
            const disabled = e !== "postgres" && e !== "firestore";
            return (
              <button
                key={e}
                onClick={() => {
                  if (disabled) return;
                  userPickedEngine.current = true;
                  setEngine(e);
                }}
                disabled={disabled}
                title={disabled ? "coming soon" : m.label}
                className={
                  "flex flex-col items-center gap-1.5 rounded-[6px] border border-border-default bg-bg-2 px-1.5 pt-2.5 pb-[9px] transition-all duration-150 hover:border-border-strong " +
                  (active ? "shadow-[inset_0_0_0_1px_var(--accent)]" : "") +
                  (disabled ? " opacity-40 cursor-not-allowed" : "")
                }
              >
                <span
                  className="inline-flex h-7 w-7 items-center justify-center rounded-[6px] border font-mono text-[13px] font-semibold"
                  style={{
                    color: hex,
                    background: `color-mix(in oklab, ${hex} 14%, transparent)`,
                    borderColor: `color-mix(in oklab, ${hex} 30%, transparent)`,
                  }}
                >
                  {m.letter}
                </span>
                <span
                  className={
                    "text-[10.5px] " +
                    (active ? "font-medium text-fg-0" : "text-fg-1")
                  }
                >
                  {m.label}
                </span>
              </button>
            );
          })}
        </div>

        <div className="mb-3.5 flex gap-0.5 border-b border-border-default">
          {(["general", "ssh", "ssl", "options"] as Tab[]).map((t) => {
            const isActive = tab === t;
            return (
              <button
                key={t}
                onClick={() => setTab(t)}
                className={
                  "relative -mb-px inline-flex h-[26px] items-center gap-1.5 px-2.5 text-[11.5px] capitalize border-b-[1.5px] " +
                  (isActive
                    ? "border-accent text-accent"
                    : "border-transparent text-fg-2 hover:text-fg-0")
                }
              >
                {t === "general" && <Icon.database size={11} />}
                {t === "ssh" && <Icon.ssh size={11} />}
                {t === "ssl" && <Icon.lock size={11} />}
                {t === "options" && <Icon.settings size={11} />}
                <span>
                  {t === "ssh"
                    ? "SSH tunnel"
                    : t === "ssl"
                      ? "SSL / TLS"
                      : t}
                </span>
                {t === "ssh" && ssh && (
                  <span className="ml-0.5 h-[5px] w-[5px] rounded-full bg-accent" />
                )}
                {t === "ssl" && ssl && (
                  <span className="ml-0.5 h-[5px] w-[5px] rounded-full bg-accent" />
                )}
              </button>
            );
          })}
        </div>

        {tab === "general" && (
          <div className="flex flex-col gap-2.5">
            <FormRow label="Name" hint="Shown in the sidebar">
              <input
                className={CD_INPUT}
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder={isFirestore ? "prod-firestore" : "local-postgres"}
              />
            </FormRow>

            <FormRow label={hostLabel}>
              <input
                className={CD_INPUT + " font-mono"}
                value={host}
                onChange={(e) => setHost(e.target.value)}
                style={{ flex: 1 }}
              />
              <span className="text-fg-3">:</span>
              <input
                className={CD_INPUT + " font-mono w-[70px] flex-none"}
                value={port}
                inputMode="numeric"
                onChange={(e) => setPort(Number(e.target.value) || 0)}
              />
            </FormRow>

            {!sqliteOnly && (
              <FormRow label={databaseLabel}>
                <input
                  className={CD_INPUT + " font-mono"}
                  value={database}
                  onChange={(e) => setDatabase(e.target.value)}
                  placeholder={isFirestore ? "my-gcp-project" : undefined}
                />
              </FormRow>
            )}

            <FormRow label={userLabel}>
              <input
                className={CD_INPUT + " font-mono"}
                value={user}
                onChange={(e) => setUser(e.target.value)}
                autoComplete="off"
                placeholder={isFirestore ? "(default)" : undefined}
              />
            </FormRow>

            <FormRow
              label={passwordLabel}
              hint={passwordHint}
            >
              <input
                className={CD_INPUT + " font-mono"}
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder={
                  isEdit
                    ? "•••••••• (unchanged)"
                    : isFirestore
                      ? "{ service_account_json }"
                      : ""
                }
                style={{ flex: 1 }}
                autoComplete="new-password"
              />
            </FormRow>

            <div className="my-1 h-px bg-border-divider" />

            <FormRow label="Accent" hint="Visual marker — protects against running on prod by mistake">
              <div className="flex gap-1">
                {SWATCH_COLORS.map((c) => (
                  <button
                    key={c}
                    onClick={() => setSwatch(c)}
                    title={c}
                    className={
                      "h-[18px] w-[18px] rounded-[4px] border border-white/10 p-0 transition-transform hover:scale-110 " +
                      (c === swatch
                        ? "shadow-[0_0_0_2px_var(--bg-1),0_0_0_3px_var(--fg-0)]"
                        : "")
                    }
                    style={{ background: c }}
                  />
                ))}
              </div>
            </FormRow>

            <FormRow label="Environment">
              <Segment>
                {ENV_TAGS.map((t) => (
                  <Seg key={t} active={envTag === t} onClick={() => setEnvTag(t)}>
                    {t}
                  </Seg>
                ))}
              </Segment>
              {envTag === "prod" && (
                <span className="ml-2 inline-flex items-center gap-1 text-[10.5px] text-warn">
                  <Icon.warn size={10} />
                  <span>prod requires confirmation for DML</span>
                </span>
              )}
            </FormRow>
          </div>
        )}

        {tab === "ssh" && (
          <div className="flex flex-col gap-2.5">
            <FormRow label="Use SSH tunnel">
              <Toggle on={ssh} onChange={setSsh} />
            </FormRow>
            <div className="text-[11px] text-fg-3">
              SSH tunneling lands in a follow-up slice. Connect directly for now.
            </div>
          </div>
        )}

        {tab === "ssl" && (
          <div className="flex flex-col gap-2.5">
            <FormRow label="Use SSL / TLS">
              <Toggle on={ssl} onChange={setSsl} />
            </FormRow>
            {ssl && (
              <FormRow label="SSL mode">
                <Segment>
                  {SSL_MODES.map((m) => (
                    <Seg
                      key={m}
                      active={sslMode === m}
                      onClick={() => setSslMode(m)}
                    >
                      {m}
                    </Seg>
                  ))}
                </Segment>
              </FormRow>
            )}
          </div>
        )}

        {tab === "options" && (
          <div className="flex flex-col gap-2.5">
            <FormRow label="Application name">
              <input
                className={CD_INPUT + " font-mono"}
                value={appName}
                onChange={(e) => setAppName(e.target.value)}
              />
            </FormRow>
          </div>
        )}
      </div>

      <div className="flex h-11 shrink-0 items-center justify-between gap-3 border-t border-border-default bg-bg-2 px-3">
        <div className="flex items-center gap-2">
          <button
            className={ED_RUN_SUBTLE}
            onClick={() => void onTest()}
            disabled={test.kind === "running"}
          >
            <Icon.power size={11} />
            <span>{test.kind === "running" ? "Testing…" : "Test connection"}</span>
          </button>
          <TestPill status={test} />
        </div>
        <div className="flex items-center gap-2">
          {savingError && (
            <span className="text-[11px] text-warn">{savingError}</span>
          )}
          <button className={ED_RUN_SUBTLE} onClick={onClose}>
            Cancel
          </button>
          <button
            className={ED_RUN_PRIMARY}
            disabled={saving || !canSave}
            onClick={() => void onSave()}
            style={{
              borderColor: "color-mix(in oklab, var(--accent) 30%, black)",
            }}
          >
            <Icon.plus size={11} />
            <span>{saving ? "Saving…" : isEdit ? "Save changes" : "Save"}</span>
          </button>
        </div>
      </div>
    </Modal>
  );
}

function TestPill({ status }: { status: TestStatus }) {
  if (status.kind === "idle") return null;
  if (status.kind === "running") {
    return (
      <span className="inline-flex items-center gap-1 text-[11px] text-fg-3">
        <span className="h-1.5 w-1.5 animate-sb-pulse rounded-full bg-accent" />
        contacting…
      </span>
    );
  }
  if (status.kind === "ok") {
    return (
      <span className="inline-flex items-center gap-1.5 text-[11px]">
        <span
          className="inline-flex h-[15px] items-center gap-1 rounded-[3px] px-1.5 font-medium"
          style={{
            color: "var(--accent)",
            background: "var(--accent-soft)",
          }}
        >
          <Icon.check size={10} stroke="var(--accent)" />
          Connection successful
        </span>
        <span className="font-mono" style={{ color: "var(--fg-2)" }}>
          {status.durationMs} ms
        </span>
        <span style={{ color: "var(--fg-3)" }}>·</span>
        <span
          className="font-mono"
          style={{ color: "var(--fg-2)" }}
          title={status.info.version}
        >
          {shortVersion(status.info.version)}
        </span>
      </span>
    );
  }
  return (
    <span className="inline-flex max-w-[420px] items-center gap-1 truncate text-[11px] text-warn">
      <Icon.warn size={10} />
      <span title={status.message}>{truncate(status.message, 80)}</span>
    </span>
  );
}

function truncate(s: string, n: number): string {
  return s.length <= n ? s : s.slice(0, n - 1) + "…";
}

function shortVersion(v: string): string {
  // `PostgreSQL 16.12 on x86_64-pc-linux-gnu, compiled by …` → `PostgreSQL 16.12`
  const match = v.match(/^(\S+\s+\d+(?:\.\d+)*)/);
  return match?.[1] ?? truncate(v, 40);
}

function slugify(s: string): string {
  return (
    s
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 64) || "connection"
  );
}

function FormRow({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <div className="grid min-h-6 items-center gap-3 grid-cols-[110px_1fr]">
      <div className="flex flex-col gap-px pt-0.5 text-[11.5px] font-medium text-fg-1">
        <span>{label}</span>
        {hint && (
          <span className="text-[10px] font-normal text-fg-3">{hint}</span>
        )}
      </div>
      <div className="flex min-w-0 items-center gap-1.5 text-[11.5px]">
        {children}
      </div>
    </div>
  );
}

function Toggle({
  on,
  onChange,
}: {
  on: boolean;
  onChange?: (v: boolean) => void;
}) {
  return (
    <button
      onClick={() => onChange?.(!on)}
      type="button"
      className={
        "relative h-4 w-7 shrink-0 rounded-[10px] transition-[background] duration-150 " +
        (on ? "bg-accent" : "bg-bg-3")
      }
    >
      <span
        className={
          "absolute top-0.5 h-3 w-3 rounded-full bg-white transition-[left] duration-150 " +
          (on ? "left-3.5" : "left-0.5")
        }
      />
    </button>
  );
}

function Segment({ children }: { children: ReactNode }) {
  return (
    <div className="inline-flex gap-px rounded-[4px] border border-border-default bg-bg-inset p-0.5">
      {children}
    </div>
  );
}

function Seg({
  active,
  onClick,
  children,
}: {
  active?: boolean;
  onClick?: () => void;
  children: ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={
        "h-5 rounded-[3px] px-2.5 text-[11px] " +
        (active
          ? "bg-bg-3 font-medium text-fg-0"
          : "text-fg-2 hover:text-fg-0")
      }
    >
      {children}
    </button>
  );
}

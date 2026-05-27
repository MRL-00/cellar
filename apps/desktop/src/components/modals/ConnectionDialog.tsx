import { useEffect, useState, type ReactNode } from "react";
import { Icon } from "../icons";
import { ENGINE_META, type Engine } from "../EngineBadge";
import { Modal } from "./Modal";

const ENGINE_ORDER: Engine[] = ["postgres", "mssql", "azure", "mysql", "sqlite"];

const ENGINE_HEX: Record<Engine, string> = {
  postgres: "#4f8ff7",
  mysql: "#f6a44a",
  mssql: "#d97a5a",
  azure: "#5bb8e0",
  sqlite: "#a78bfa",
};

const SWATCH_COLORS = ["#4f8ff7", "#f6a44a", "#d97a5a", "#5bb8e0", "#a78bfa", "#4ade80", "#f87171"];

const DEFAULT_PORT: Record<Engine, string> = {
  postgres: "5432",
  mysql: "3306",
  mssql: "1433",
  azure: "1433",
  sqlite: "—",
};

type Auth = "password" | "kerberos" | "azure-ad" | "managed-id" | "windows";
type Tab = "general" | "ssh" | "ssl" | "options";

const ED_RUN_BASE =
  "inline-flex h-[26px] items-center gap-[5px] whitespace-nowrap rounded-[4px] border border-transparent px-2.5 text-[11.5px] font-medium text-fg-1 transition-[background,color,border-color,filter] duration-[120ms]";

const ED_RUN_SUBTLE =
  ED_RUN_BASE +
  " bg-transparent border-border-default hover:bg-bg-3 hover:border-border-strong hover:text-fg-0 disabled:opacity-40 disabled:cursor-not-allowed";

const ED_RUN_PRIMARY =
  ED_RUN_BASE +
  " bg-accent text-accent-fg hover:brightness-[1.07] disabled:opacity-40 disabled:cursor-not-allowed";

const CD_INPUT =
  "h-[26px] min-w-0 flex-1 rounded-[4px] border border-border-default bg-bg-inset px-2 text-[11.5px] text-fg-0 outline-none font-sans focus:border-accent-line focus:bg-bg-2";

export function ConnectionDialog({ onClose }: { onClose: () => void }) {
  const [engine, setEngine] = useState<Engine>("postgres");
  const [tab, setTab] = useState<Tab>("general");
  const [auth, setAuth] = useState<Auth>("password");
  const [ssh, setSsh] = useState(false);
  const [ssl, setSsl] = useState(true);
  const [swatch, setSwatch] = useState<string>(ENGINE_HEX.postgres);

  useEffect(() => {
    if (engine === "azure") setAuth("azure-ad");
    else if (auth === "azure-ad" && engine !== "mssql") setAuth("password");
    setSwatch(ENGINE_HEX[engine]);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [engine]);

  return (
    <Modal onClose={onClose} width={760}>
      <div className="flex h-[38px] shrink-0 items-center justify-between border-b border-border-default pl-3.5 pr-2">
        <div className="flex items-center gap-2">
          <span className="inline-flex text-accent">
            <Icon.database size={14} />
          </span>
          <span className="whitespace-nowrap text-[12.5px] font-semibold text-fg-0">
            New connection
          </span>
        </div>
        <button className="icon-btn" onClick={onClose} title="Close">
          <Icon.close size={13} />
        </button>
      </div>

      <div className="flex-1 overflow-y-auto px-4 pt-3 pb-4">
        <div className="mb-3.5 grid grid-cols-5 gap-1.5">
          {ENGINE_ORDER.map((e) => {
            const m = ENGINE_META[e];
            const hex = ENGINE_HEX[e];
            const active = engine === e;
            return (
              <button
                key={e}
                onClick={() => setEngine(e)}
                className={
                  "flex flex-col items-center gap-1.5 rounded-[6px] border border-border-default bg-bg-2 px-1.5 pt-2.5 pb-[9px] transition-all duration-150 hover:border-border-strong " +
                  (active ? "shadow-[inset_0_0_0_1px_var(--accent)]" : "")
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
              <input className={CD_INPUT} defaultValue="shop-eu (prod)" />
            </FormRow>

            <FormRow label="Host">
              <input
                className={CD_INPUT + " font-mono"}
                defaultValue="prod-pg.internal.shop.eu"
                style={{ flex: 1 }}
              />
              <span className="text-fg-3">:</span>
              <input
                className={CD_INPUT + " font-mono w-[70px] flex-none"}
                defaultValue={DEFAULT_PORT[engine]}
              />
            </FormRow>

            {engine !== "sqlite" && (
              <FormRow label="Database">
                <input className={CD_INPUT + " font-mono"} defaultValue="shop_eu" />
              </FormRow>
            )}

            {engine === "sqlite" && (
              <FormRow label="File">
                <input
                  className={CD_INPUT + " font-mono"}
                  defaultValue="~/projects/shop/local.db"
                  style={{ flex: 1 }}
                />
                <PickButton>
                  <Icon.fileText size={11} />
                  <span>Browse</span>
                </PickButton>
              </FormRow>
            )}

            <FormRow label="Auth">
              <Segment>
                <Seg active={auth === "password"} onClick={() => setAuth("password")}>
                  Password
                </Seg>
                {engine === "postgres" && (
                  <Seg active={auth === "kerberos"} onClick={() => setAuth("kerberos")}>
                    Kerberos
                  </Seg>
                )}
                {(engine === "azure" || engine === "mssql") && (
                  <Seg active={auth === "azure-ad"} onClick={() => setAuth("azure-ad")}>
                    Azure AD
                  </Seg>
                )}
                {engine === "azure" && (
                  <Seg active={auth === "managed-id"} onClick={() => setAuth("managed-id")}>
                    Managed identity
                  </Seg>
                )}
                {(engine === "mssql" || engine === "azure") && (
                  <Seg active={auth === "windows"} onClick={() => setAuth("windows")}>
                    Windows
                  </Seg>
                )}
              </Segment>
            </FormRow>

            {auth === "password" && (
              <>
                <FormRow label="User">
                  <input className={CD_INPUT + " font-mono"} defaultValue="analytics_ro" />
                </FormRow>
                <FormRow label="Password" hint="Stored in OS keychain">
                  <input
                    className={CD_INPUT + " font-mono"}
                    type="password"
                    defaultValue="••••••••••••••"
                    style={{ flex: 1 }}
                  />
                  <label className="inline-flex cursor-pointer items-center gap-1 text-[11px] text-fg-2">
                    <input
                      type="checkbox"
                      className="h-3 w-3"
                      style={{ accentColor: "var(--accent)" }}
                    />
                    save
                  </label>
                </FormRow>
              </>
            )}

            {auth === "azure-ad" && (
              <>
                <FormRow label="Tenant" hint="leave empty for default">
                  <input
                    className={CD_INPUT + " font-mono"}
                    placeholder="contoso.onmicrosoft.com"
                  />
                </FormRow>
                <FormRow label="Account">
                  <div className="inline-flex items-center gap-1.5 rounded-[4px] border border-border-default bg-bg-inset py-0.5 pl-0.5 pr-2 text-[11px]">
                    <span className="inline-flex h-[18px] w-[18px] items-center justify-center rounded-[3px] bg-eng-azure text-[10px] font-semibold text-white">
                      A
                    </span>
                    <span>alice@contoso.com</span>
                    <button className="bg-transparent text-[11px] text-accent underline underline-offset-2">
                      change
                    </button>
                  </div>
                </FormRow>
              </>
            )}

            {auth === "managed-id" && (
              <FormRow label="Client ID" hint="user-assigned identity">
                <input className={CD_INPUT + " font-mono"} placeholder="(system-assigned)" />
              </FormRow>
            )}

            <div className="my-1 h-px bg-border-divider" />

            <FormRow
              label="Accent"
              hint="visual marker — protects against running on prod by mistake"
            >
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
                <Seg active>prod</Seg>
                <Seg>staging</Seg>
                <Seg>dev</Seg>
                <Seg>local</Seg>
              </Segment>
              <span className="ml-2 inline-flex items-center gap-1 text-[10.5px] text-warn">
                <Icon.warn size={10} />
                <span>prod requires confirmation for DML</span>
              </span>
            </FormRow>
          </div>
        )}

        {tab === "ssh" && (
          <div className="flex flex-col gap-2.5">
            <FormRow label="Use SSH tunnel">
              <Toggle on={ssh} onChange={setSsh} />
            </FormRow>
            {ssh && (
              <>
                <FormRow label="SSH host">
                  <input
                    className={CD_INPUT + " font-mono"}
                    defaultValue="bastion.shop.eu"
                    style={{ flex: 1 }}
                  />
                  <span className="text-fg-3">:</span>
                  <input
                    className={CD_INPUT + " font-mono w-[70px] flex-none"}
                    defaultValue="22"
                  />
                </FormRow>
                <FormRow label="SSH user">
                  <input className={CD_INPUT + " font-mono"} defaultValue="alice" />
                </FormRow>
                <FormRow label="Auth">
                  <Segment>
                    <Seg active>Key pair</Seg>
                    <Seg>Password</Seg>
                    <Seg>Agent</Seg>
                  </Segment>
                </FormRow>
                <FormRow label="Private key">
                  <input
                    className={CD_INPUT + " font-mono"}
                    defaultValue="~/.ssh/id_ed25519"
                    style={{ flex: 1 }}
                  />
                  <PickButton>
                    <Icon.fileText size={11} />
                    <span>Browse</span>
                  </PickButton>
                </FormRow>
              </>
            )}
          </div>
        )}

        {tab === "ssl" && (
          <div className="flex flex-col gap-2.5">
            <FormRow label="Use SSL / TLS">
              <Toggle on={ssl} onChange={setSsl} />
            </FormRow>
            {ssl && (
              <>
                <FormRow label="SSL mode">
                  <Segment>
                    <Seg>disable</Seg>
                    <Seg>prefer</Seg>
                    <Seg>require</Seg>
                    <Seg active>verify-full</Seg>
                  </Segment>
                </FormRow>
                <FormRow label="Server CA">
                  <input
                    className={CD_INPUT + " font-mono"}
                    defaultValue="~/.cellar/certs/shop-eu-ca.pem"
                    style={{ flex: 1 }}
                  />
                </FormRow>
                <FormRow label="Client cert" hint="optional, for mTLS">
                  <input
                    className={CD_INPUT + " font-mono"}
                    placeholder="…"
                    style={{ flex: 1 }}
                  />
                </FormRow>
              </>
            )}
          </div>
        )}

        {tab === "options" && (
          <div className="flex flex-col gap-2.5">
            <FormRow label="Read-only by default">
              <Toggle on={true} />
            </FormRow>
            <FormRow label="Connection timeout">
              <input
                className={CD_INPUT + " font-mono w-[70px] flex-none"}
                defaultValue="10"
              />
              <span style={{ color: "var(--fg-3)" }}>seconds</span>
            </FormRow>
            <FormRow label="Application name">
              <input className={CD_INPUT + " font-mono"} defaultValue="cellar (alice@laptop)" />
            </FormRow>
            <FormRow label="Schema search path">
              <input
                className={CD_INPUT + " font-mono"}
                defaultValue="public, audit, analytics"
              />
            </FormRow>
          </div>
        )}
      </div>

      <div className="flex h-11 shrink-0 items-center justify-between gap-3 border-t border-border-default bg-bg-2 px-3">
        <div className="flex items-center gap-2">
          <button className={ED_RUN_SUBTLE}>
            <Icon.power size={11} />
            <span>Test connection</span>
          </button>
          <span className="inline-flex items-center gap-1.5 text-[11px]">
            <Icon.check size={10} stroke="var(--accent)" />
            <span style={{ color: "var(--accent)" }}>Connected</span>
            <span style={{ color: "var(--fg-3)" }}>·</span>
            <span style={{ color: "var(--fg-2)" }} className="font-mono">
              214 ms
            </span>
            <span style={{ color: "var(--fg-3)" }}>·</span>
            <span style={{ color: "var(--fg-2)" }} className="font-mono">
              PostgreSQL 16.2 on x86_64-linux-gnu
            </span>
          </span>
        </div>
        <div className="flex items-center gap-2">
          <button className={ED_RUN_SUBTLE} onClick={onClose}>
            Cancel
          </button>
          <button
            className={ED_RUN_PRIMARY}
            style={{
              borderColor: "color-mix(in oklab, var(--accent) 30%, black)",
            }}
          >
            <Icon.plus size={11} />
            <span>Save &amp; connect</span>
          </button>
        </div>
      </div>
    </Modal>
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

function PickButton({ children }: { children: ReactNode }) {
  return (
    <button className="inline-flex h-[26px] items-center gap-1 rounded-[4px] border border-border-default bg-bg-2 px-2 text-[11px] text-fg-1 hover:bg-bg-3">
      {children}
    </button>
  );
}

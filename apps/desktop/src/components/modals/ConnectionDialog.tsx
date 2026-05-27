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
      <div className="cd-head">
        <div className="cd-head-left">
          <span className="cd-head-icon">
            <Icon.database size={14} />
          </span>
          <span className="cd-head-title">New connection</span>
        </div>
        <button className="icon-btn" onClick={onClose} title="Close">
          <Icon.close size={13} />
        </button>
      </div>

      <div className="cd-body">
        <div className="cd-engines">
          {ENGINE_ORDER.map((e) => {
            const m = ENGINE_META[e];
            const hex = ENGINE_HEX[e];
            const active = engine === e;
            return (
              <button
                key={e}
                className={"cd-engine" + (active ? " active" : "")}
                onClick={() => setEngine(e)}
              >
                <span
                  className="cd-engine-letter mono"
                  style={{
                    color: hex,
                    background: `color-mix(in oklab, ${hex} 14%, transparent)`,
                    borderColor: `color-mix(in oklab, ${hex} 30%, transparent)`,
                  }}
                >
                  {m.letter}
                </span>
                <span className="cd-engine-name">{m.label}</span>
              </button>
            );
          })}
        </div>

        <div className="cd-tabs">
          {(["general", "ssh", "ssl", "options"] as Tab[]).map((t) => (
            <button
              key={t}
              className={"cd-tab" + (tab === t ? " active" : "")}
              onClick={() => setTab(t)}
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
              {t === "ssh" && ssh && <span className="cd-tab-dot" />}
              {t === "ssl" && ssl && <span className="cd-tab-dot" />}
            </button>
          ))}
        </div>

        {tab === "general" && (
          <div className="cd-form">
            <FormRow label="Name" hint="Shown in the sidebar">
              <input className="cd-input" defaultValue="shop-eu (prod)" />
            </FormRow>

            <FormRow label="Host">
              <input
                className="cd-input mono"
                defaultValue="prod-pg.internal.shop.eu"
                style={{ flex: 1 }}
              />
              <span className="cd-form-sep">:</span>
              <input
                className="cd-input mono cd-input-port"
                defaultValue={DEFAULT_PORT[engine]}
              />
            </FormRow>

            {engine !== "sqlite" && (
              <FormRow label="Database">
                <input className="cd-input mono" defaultValue="shop_eu" />
              </FormRow>
            )}

            {engine === "sqlite" && (
              <FormRow label="File">
                <input
                  className="cd-input mono"
                  defaultValue="~/projects/shop/local.db"
                  style={{ flex: 1 }}
                />
                <button className="cd-pick">
                  <Icon.fileText size={11} />
                  <span>Browse</span>
                </button>
              </FormRow>
            )}

            <FormRow label="Auth">
              <div className="cd-segment">
                <button
                  className={"cd-seg" + (auth === "password" ? " active" : "")}
                  onClick={() => setAuth("password")}
                >
                  Password
                </button>
                {engine === "postgres" && (
                  <button
                    className={"cd-seg" + (auth === "kerberos" ? " active" : "")}
                    onClick={() => setAuth("kerberos")}
                  >
                    Kerberos
                  </button>
                )}
                {(engine === "azure" || engine === "mssql") && (
                  <button
                    className={"cd-seg" + (auth === "azure-ad" ? " active" : "")}
                    onClick={() => setAuth("azure-ad")}
                  >
                    Azure AD
                  </button>
                )}
                {engine === "azure" && (
                  <button
                    className={
                      "cd-seg" + (auth === "managed-id" ? " active" : "")
                    }
                    onClick={() => setAuth("managed-id")}
                  >
                    Managed identity
                  </button>
                )}
                {(engine === "mssql" || engine === "azure") && (
                  <button
                    className={"cd-seg" + (auth === "windows" ? " active" : "")}
                    onClick={() => setAuth("windows")}
                  >
                    Windows
                  </button>
                )}
              </div>
            </FormRow>

            {auth === "password" && (
              <>
                <FormRow label="User">
                  <input className="cd-input mono" defaultValue="analytics_ro" />
                </FormRow>
                <FormRow label="Password" hint="Stored in OS keychain">
                  <input
                    className="cd-input mono"
                    type="password"
                    defaultValue="••••••••••••••"
                    style={{ flex: 1 }}
                  />
                  <label className="cd-check">
                    <input type="checkbox" /> save
                  </label>
                </FormRow>
              </>
            )}

            {auth === "azure-ad" && (
              <>
                <FormRow label="Tenant" hint="leave empty for default">
                  <input
                    className="cd-input mono"
                    placeholder="contoso.onmicrosoft.com"
                  />
                </FormRow>
                <FormRow label="Account">
                  <div className="cd-azure-account">
                    <span className="cd-azure-avatar">A</span>
                    <span>alice@contoso.com</span>
                    <button className="cd-link">change</button>
                  </div>
                </FormRow>
              </>
            )}

            {auth === "managed-id" && (
              <FormRow label="Client ID" hint="user-assigned identity">
                <input
                  className="cd-input mono"
                  placeholder="(system-assigned)"
                />
              </FormRow>
            )}

            <div className="cd-divider" />

            <FormRow
              label="Accent"
              hint="visual marker — protects against running on prod by mistake"
            >
              <div className="cd-swatches">
                {SWATCH_COLORS.map((c) => (
                  <button
                    key={c}
                    className={"cd-swatch" + (c === swatch ? " active" : "")}
                    style={{ background: c }}
                    onClick={() => setSwatch(c)}
                    title={c}
                  />
                ))}
              </div>
            </FormRow>

            <FormRow label="Environment">
              <div className="cd-segment">
                <button className="cd-seg active">prod</button>
                <button className="cd-seg">staging</button>
                <button className="cd-seg">dev</button>
                <button className="cd-seg">local</button>
              </div>
              <span className="cd-warn">
                <Icon.warn size={10} />
                <span>prod requires confirmation for DML</span>
              </span>
            </FormRow>
          </div>
        )}

        {tab === "ssh" && (
          <div className="cd-form">
            <FormRow label="Use SSH tunnel">
              <Toggle on={ssh} onChange={setSsh} />
            </FormRow>
            {ssh && (
              <>
                <FormRow label="SSH host">
                  <input
                    className="cd-input mono"
                    defaultValue="bastion.shop.eu"
                    style={{ flex: 1 }}
                  />
                  <span className="cd-form-sep">:</span>
                  <input
                    className="cd-input mono cd-input-port"
                    defaultValue="22"
                  />
                </FormRow>
                <FormRow label="SSH user">
                  <input className="cd-input mono" defaultValue="alice" />
                </FormRow>
                <FormRow label="Auth">
                  <div className="cd-segment">
                    <button className="cd-seg active">Key pair</button>
                    <button className="cd-seg">Password</button>
                    <button className="cd-seg">Agent</button>
                  </div>
                </FormRow>
                <FormRow label="Private key">
                  <input
                    className="cd-input mono"
                    defaultValue="~/.ssh/id_ed25519"
                    style={{ flex: 1 }}
                  />
                  <button className="cd-pick">
                    <Icon.fileText size={11} />
                    <span>Browse</span>
                  </button>
                </FormRow>
              </>
            )}
          </div>
        )}

        {tab === "ssl" && (
          <div className="cd-form">
            <FormRow label="Use SSL / TLS">
              <Toggle on={ssl} onChange={setSsl} />
            </FormRow>
            {ssl && (
              <>
                <FormRow label="SSL mode">
                  <div className="cd-segment">
                    <button className="cd-seg">disable</button>
                    <button className="cd-seg">prefer</button>
                    <button className="cd-seg">require</button>
                    <button className="cd-seg active">verify-full</button>
                  </div>
                </FormRow>
                <FormRow label="Server CA">
                  <input
                    className="cd-input mono"
                    defaultValue="~/.cellar/certs/shop-eu-ca.pem"
                    style={{ flex: 1 }}
                  />
                </FormRow>
                <FormRow label="Client cert" hint="optional, for mTLS">
                  <input
                    className="cd-input mono"
                    placeholder="…"
                    style={{ flex: 1 }}
                  />
                </FormRow>
              </>
            )}
          </div>
        )}

        {tab === "options" && (
          <div className="cd-form">
            <FormRow label="Read-only by default">
              <Toggle on={true} />
            </FormRow>
            <FormRow label="Connection timeout">
              <input
                className="cd-input mono cd-input-port"
                defaultValue="10"
              />
              <span style={{ color: "var(--fg-3)" }}>seconds</span>
            </FormRow>
            <FormRow label="Application name">
              <input
                className="cd-input mono"
                defaultValue="cellar (alice@laptop)"
              />
            </FormRow>
            <FormRow label="Schema search path">
              <input
                className="cd-input mono"
                defaultValue="public, audit, analytics"
              />
            </FormRow>
          </div>
        )}
      </div>

      <div className="cd-foot">
        <div className="cd-foot-left">
          <button className="ed-run subtle">
            <Icon.power size={11} />
            <span>Test connection</span>
          </button>
          <span className="cd-foot-status">
            <Icon.check size={10} stroke="var(--accent)" />
            <span style={{ color: "var(--accent)" }}>Connected</span>
            <span style={{ color: "var(--fg-3)" }}>·</span>
            <span style={{ color: "var(--fg-2)" }} className="mono">
              214 ms
            </span>
            <span style={{ color: "var(--fg-3)" }}>·</span>
            <span style={{ color: "var(--fg-2)" }} className="mono">
              PostgreSQL 16.2 on x86_64-linux-gnu
            </span>
          </span>
        </div>
        <div className="cd-foot-right">
          <button className="ed-run subtle" onClick={onClose}>
            Cancel
          </button>
          <button className="ed-run primary">
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
    <div className="cd-row">
      <div className="cd-row-label">
        <span>{label}</span>
        {hint && <span className="cd-row-hint">{hint}</span>}
      </div>
      <div className="cd-row-content">{children}</div>
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
      className={"cd-toggle" + (on ? " on" : "")}
      onClick={() => onChange?.(!on)}
      type="button"
    >
      <span className="cd-toggle-knob" />
    </button>
  );
}

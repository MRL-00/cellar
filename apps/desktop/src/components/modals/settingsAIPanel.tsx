import { useState } from "react";
import { Icon } from "../icons";
import { CD_INPUT, ED_RUN_DANGER, Row, Section } from "./settingsPrimitives";

type AiProviderId = "anthropic" | "openai" | "google" | "local" | "custom";
type AiModelTag = "balanced" | "max" | "fast" | "local";

type AiModel = {
  id: string;
  ctx: string;
  tag: AiModelTag;
  def?: boolean;
};

const PROVIDERS: Record<
  AiProviderId,
  {
    label: string;
    sub: string;
    keyPrefix: string;
    endpoint: string;
    models: [AiModel, ...AiModel[]];
  }
> = {
  anthropic: {
    label: "Anthropic",
    sub: "claude-* family",
    keyPrefix: "sk-ant-",
    endpoint: "https://api.anthropic.com/v1",
    models: [
      { id: "claude-sonnet-4.5", ctx: "200k", tag: "balanced", def: true },
      { id: "claude-opus-4", ctx: "200k", tag: "max" },
      { id: "claude-haiku-4.5", ctx: "200k", tag: "fast" },
    ],
  },
  openai: {
    label: "OpenAI",
    sub: "gpt-* family",
    keyPrefix: "sk-",
    endpoint: "https://api.openai.com/v1",
    models: [
      { id: "gpt-5.1", ctx: "256k", tag: "balanced", def: true },
      { id: "gpt-5.1-mini", ctx: "256k", tag: "fast" },
    ],
  },
  google: {
    label: "Google",
    sub: "gemini-* family",
    keyPrefix: "key",
    endpoint: "https://generativelanguage.googleapis.com/v1beta",
    models: [
      { id: "gemini-2.5-pro", ctx: "1m", tag: "max", def: true },
      { id: "gemini-2.5-flash", ctx: "1m", tag: "fast" },
    ],
  },
  local: {
    label: "Local",
    sub: "Ollama, LM Studio",
    keyPrefix: "none",
    endpoint: "http://localhost:11434/v1",
    models: [
      { id: "local-default", ctx: "model", tag: "local", def: true },
    ],
  },
  custom: {
    label: "Custom",
    sub: "OpenAI-compatible URL",
    keyPrefix: "key",
    endpoint: "https://example.invalid/v1",
    models: [
      { id: "custom-model", ctx: "provider", tag: "balanced", def: true },
    ],
  },
};

const TAG_CLASS: Record<AiModelTag, string> = {
  balanced: "text-accent bg-accent-soft",
  max: "text-update bg-update-bg",
  fast: "text-insert bg-insert-bg",
  local: "text-fg-1 bg-bg-3",
};

export function SettingsAI() {
  const [provider, setProvider] = useState<AiProviderId>("anthropic");
  const [model, setModel] = useState(PROVIDERS.anthropic.models[0].id);
  const [reveal, setReveal] = useState(false);
  const current = PROVIDERS[provider];

  const selectProvider = (id: AiProviderId) => {
    setProvider(id);
    setModel(PROVIDERS[id].models[0].id);
  };

  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section
        title="AI Assistant"
        sub="Cellar's AI runs entirely on your key. Your queries, schemas and results never touch our servers."
      >
        <div className="flex gap-2.5 rounded-[6px] border border-accent-line bg-accent-soft px-3.5 py-3">
          <span className="inline-flex h-[22px] w-[22px] shrink-0 items-center justify-center rounded-[5px] border border-accent-line bg-bg-1">
            <Icon.sparkles size={12} stroke="var(--accent)" />
          </span>
          <div className="text-pretty">
            <div className="mb-0.5 text-[12px] font-semibold text-fg-0">
              Bring-your-own-key, by design
            </div>
            <div className="text-[11.5px] leading-[1.45] text-fg-1">
              All AI requests go directly from your machine to the provider. We
              see nothing. Cellar is open-source; verify the network path in the
              AI package before enabling providers.
            </div>
          </div>
        </div>
      </Section>

      <Section title="Provider">
        <Row label="Provider">
          <div className="grid w-full grid-cols-5 gap-1.5">
            {Object.entries(PROVIDERS).map(([id, p]) => {
              const providerId = id as AiProviderId;
              const active = provider === providerId;
              return (
                <button
                  type="button"
                  key={id}
                  onClick={() => selectProvider(providerId)}
                  className={
                    "relative flex flex-col items-start gap-0.5 rounded-[5px] border px-[9px] py-2 text-left " +
                    (active
                      ? "border-accent bg-accent-soft shadow-[inset_0_0_0_1px_var(--accent)]"
                      : "border-border-default bg-bg-2 hover:border-border-strong")
                  }
                >
                  <span className="text-[11.5px] font-medium text-fg-0">
                    {p.label}
                  </span>
                  <span
                    className={
                      "font-mono text-[9.5px] " +
                      (active ? "text-accent opacity-85" : "text-fg-3")
                    }
                  >
                    {p.sub}
                  </span>
                  {active && (
                    <span className="absolute right-[5px] top-[5px] inline-flex h-3 w-3 items-center justify-center rounded-full bg-accent text-accent-fg">
                      <Icon.check size={9} />
                    </span>
                  )}
                </button>
              );
            })}
          </div>
        </Row>

        <Row label="Model" hint="used for chat, generation, ghost text">
          <div className="flex w-full flex-col gap-[3px]">
            {current.models.map((m) => {
              const active = model === m.id;
              return (
                <label
                  key={m.id}
                  className={
                    "grid cursor-pointer grid-cols-[14px_1fr_auto_auto_auto] items-center gap-2.5 rounded-[4px] border px-2.5 py-1.5 " +
                    (active
                      ? "border-accent-line bg-accent-soft"
                      : "border-border-default bg-bg-2 hover:border-border-strong")
                  }
                >
                  <span
                    className={
                      "relative inline-block h-3 w-3 rounded-full border " +
                      (active ? "border-accent" : "border-border-strong")
                    }
                  >
                    {active && (
                      <span
                        className="absolute inset-[2px] rounded-full"
                        style={{ background: "var(--accent)" }}
                      />
                    )}
                  </span>
                  <input
                    type="radio"
                    className="hidden"
                    checked={active}
                    onChange={() => setModel(m.id)}
                  />
                  <span className="font-mono text-[11.5px] text-fg-0">
                    {m.id}
                  </span>
                  <span className="font-mono text-[10px] text-fg-3">
                    {m.ctx} ctx
                  </span>
                  <span
                    className={
                      "rounded-[3px] px-1.5 py-px font-mono text-[9.5px] uppercase tracking-[0.04em] " +
                      TAG_CLASS[m.tag]
                    }
                  >
                    {m.tag}
                  </span>
                  {m.def && (
                    <span className="text-[9.5px] italic text-fg-2">
                      recommended
                    </span>
                  )}
                </label>
              );
            })}
          </div>
        </Row>

        <Row label="API key" hint="stored in OS keychain, never written to disk">
          <div className="inline-flex min-w-0 flex-1 items-center gap-1">
            <span className="inline-flex h-[26px] items-center rounded-l-[4px] border border-r-0 border-border-default bg-bg-inset px-1.5 font-mono text-[11px] text-fg-2">
              {current.keyPrefix}
            </span>
            <input
              className="-ml-px h-[26px] min-w-0 flex-1 border border-border-default bg-bg-inset px-2 font-mono text-[11.5px] text-fg-0 outline-none focus:border-accent-line focus:bg-bg-2"
              type={reveal ? "text" : "password"}
              value=""
              placeholder={reveal ? "No key configured" : "stored in keychain"}
              readOnly
              spellCheck={false}
            />
            <button
              type="button"
              onClick={() => setReveal(!reveal)}
              className="inline-flex h-[26px] items-center gap-1 rounded-[4px] border border-border-default bg-bg-2 px-2 text-[10.5px] text-fg-1 hover:bg-bg-3"
            >
              {reveal ? "hide" : "reveal"}
            </button>
            <button
              type="button"
              disabled
              title="Keychain editing is not wired yet"
              className="inline-flex h-[26px] cursor-not-allowed items-center gap-1 rounded-[4px] border border-border-default bg-bg-2 px-2 text-[11px] text-fg-2 opacity-70"
            >
              <Icon.edit size={11} />
              <span>change</span>
            </button>
          </div>
        </Row>

        <Row label="Endpoint" hint="override for proxies, custom routers">
          <input
            className={CD_INPUT + " cursor-not-allowed font-mono opacity-80"}
            value={current.endpoint}
            readOnly
            style={{ flex: 1 }}
          />
        </Row>
      </Section>

      <Section title="Danger zone">
        <Row
          label="Clear AI conversation history"
          hint="20 conversations, 3.2 MB locally"
        >
          <button
            type="button"
            disabled
            title="AI history deletion is not wired yet"
            className={ED_RUN_DANGER + " cursor-not-allowed opacity-60"}
          >
            <Icon.trash size={11} />
            <span>Clear all</span>
          </button>
        </Row>
        <Row label="Revoke API key" hint="remove from keychain — does not affect provider">
          <button
            type="button"
            disabled
            title="Key revocation is not wired yet"
            className={ED_RUN_DANGER + " cursor-not-allowed opacity-60"}
          >
            <Icon.close size={11} />
            <span>Revoke</span>
          </button>
        </Row>
      </Section>
    </div>
  );
}

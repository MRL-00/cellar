import { useEffect, useState } from "react";
import { PROVIDERS, getProvider, type AiProviderId } from "@cellar/ai";
import { Icon } from "../icons";
import { CD_INPUT, ED_RUN_DANGER, Row, Section } from "./settingsPrimitives";
import { useAi } from "../../state/ai";
import { openExternal } from "../../lib/openExternal";

export function SettingsAI() {
  const providerId = useAi((s) => s.providerId);
  const modelId = useAi((s) => s.modelId);
  const models = useAi((s) => s.models);
  const modelsLoading = useAi((s) => s.modelsLoading);
  const modelsError = useAi((s) => s.modelsError);
  const keyConfigured = useAi((s) => s.keyConfigured);
  const configured = useAi((s) => s.configured);
  const openAiAuthMode = useAi((s) => s.openAiAuthMode);
  const oauthStatus = useAi((s) => s.oauthStatus);
  const login = useAi((s) => s.login);
  const authLoading = useAi((s) => s.authLoading);
  const messages = useAi((s) => s.messages);

  const init = useAi((s) => s.init);
  const setProvider = useAi((s) => s.setProvider);
  const setOpenAiAuthMode = useAi((s) => s.setOpenAiAuthMode);
  const setModel = useAi((s) => s.setModel);
  const saveKey = useAi((s) => s.saveKey);
  const clearKey = useAi((s) => s.clearKey);
  const refreshModels = useAi((s) => s.refreshModels);
  const refreshOAuthStatus = useAi((s) => s.refreshOAuthStatus);
  const startOpenAiLogin = useAi((s) => s.startOpenAiLogin);
  const cancelOpenAiLogin = useAi((s) => s.cancelOpenAiLogin);
  const logoutOpenAi = useAi((s) => s.logoutOpenAi);
  const newThread = useAi((s) => s.newThread);

  const [keyDraft, setKeyDraft] = useState("");
  const [saving, setSaving] = useState(false);
  const [reveal, setReveal] = useState(false);
  const [browserError, setBrowserError] = useState<string | null>(null);

  const current = getProvider(providerId);
  const isOpenAi = providerId === "openai";
  const isChatGpt = isOpenAi && openAiAuthMode === "chatgpt";

  useEffect(() => {
    void init();
  }, [init]);

  useEffect(() => {
    if (!isChatGpt || !login || oauthStatus?.signed_in) return;
    const timer = window.setInterval(() => {
      void refreshOAuthStatus().then((status) => {
        if (status.signed_in) void refreshModels();
      }).catch(() => undefined);
    }, 1500);
    return () => window.clearInterval(timer);
  }, [isChatGpt, login, oauthStatus?.signed_in, refreshModels, refreshOAuthStatus]);

  const onSaveKey = async () => {
    const key = keyDraft.trim();
    if (!key || saving) return;
    setSaving(true);
    try {
      await saveKey(key);
      setKeyDraft("");
      setReveal(false);
    } finally {
      setSaving(false);
    }
  };

  const onStartLogin = async (method: "browser" | "deviceCode") => {
    try {
      setBrowserError(null);
      const started = await startOpenAiLogin(method);
      await openExternal(started.auth_url);
    } catch (error) {
      if (useAi.getState().login) {
        setBrowserError(
          error instanceof Error ? error.message : "Cellar could not open your default browser.",
        );
      }
      // The store exposes the typed IPC error in the settings panel.
    }
  };

  const onOpenLogin = async () => {
    if (!login) return;
    try {
      setBrowserError(null);
      await openExternal(login.auth_url);
    } catch (error) {
      setBrowserError(
        error instanceof Error ? error.message : "Cellar could not open your default browser.",
      );
    }
  };

  const onRefreshLogin = async () => {
    try {
      const status = await refreshOAuthStatus();
      if (status.signed_in) await refreshModels();
    } catch {
      // The store exposes the typed IPC error in the settings panel.
    }
  };

  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section
        title="AI Assistant"
        sub="Connect directly with a provider key or use your ChatGPT subscription. Cellar never proxies AI traffic through a hosted service."
      >
        <div className="flex gap-2.5 rounded-[6px] border border-accent-line bg-accent-soft px-3.5 py-3">
          <span className="inline-flex h-[22px] w-[22px] shrink-0 items-center justify-center rounded-[5px] border border-accent-line bg-bg-1">
            <Icon.sparkles size={12} stroke="var(--accent)" />
          </span>
          <div className="text-pretty">
            <div className="mb-0.5 text-sm font-semibold text-fg-0">
              Local credentials, by design
            </div>
            <div className="text-sm leading-[1.45] text-fg-1">
              Provider keys stay in the OS keychain. OpenAI requests run in the
              Rust backend, and ChatGPT OAuth tokens are owned by a local Codex
              app-server process rather than the webview.
            </div>
          </div>
        </div>
      </Section>

      <Section title="Provider">
        <Row label="Provider">
          <div className="grid w-full grid-cols-5 gap-1.5">
            {PROVIDERS.map((p) => {
              const active = providerId === p.id;
              const disabled = !p.enabled;
              return (
                <button
                  type="button"
                  key={p.id}
                  disabled={disabled}
                  aria-pressed={active}
                  onClick={() => !disabled && setProvider(p.id as AiProviderId)}
                  title={disabled ? "Coming soon" : p.label}
                  className={
                    "relative flex flex-col items-start gap-0.5 rounded-[5px] border px-[9px] py-2 text-left " +
                    (disabled
                      ? "cursor-not-allowed border-border-default bg-bg-2 opacity-55"
                      : active
                        ? "border-accent bg-accent-soft shadow-[inset_0_0_0_1px_var(--accent)]"
                        : "border-border-default bg-bg-2 hover:border-border-strong")
                  }
                >
                  <span className="text-sm font-medium text-fg-0">
                    {p.label}
                  </span>
                  <span
                    className={
                      "font-mono text-[10.5px] " +
                      (active ? "text-accent opacity-85" : "text-fg-3")
                    }
                  >
                    {p.sub}
                  </span>
                  {disabled && (
                    <span className="absolute right-[5px] top-[5px] rounded-[3px] bg-bg-3 px-1 py-px text-[9.5px] uppercase tracking-[0.04em] text-fg-3">
                      soon
                    </span>
                  )}
                  {!disabled && active && (
                    <span className="absolute right-[5px] top-[5px] inline-flex h-3 w-3 items-center justify-center rounded-full bg-accent text-accent-fg">
                      <Icon.check size={9} />
                    </span>
                  )}
                </button>
              );
            })}
          </div>
        </Row>

        {isOpenAi && (
          <Row label="Authentication" hint="choose usage-based API billing or subscription access">
            <div className="grid w-full grid-cols-2 gap-1.5">
              {(
                [
                  ["apiKey", "API key", "OpenAI Platform billing"],
                  ["chatgpt", "ChatGPT", "Subscription access"],
                ] as const
              ).map(([mode, label, hint]) => {
                const active = openAiAuthMode === mode;
                return (
                  <button
                    key={mode}
                    type="button"
                    aria-pressed={active}
                    onClick={() => setOpenAiAuthMode(mode)}
                    className={
                      "rounded-[5px] border px-2.5 py-2 text-left " +
                      (active
                        ? "border-accent bg-accent-soft"
                        : "border-border-default bg-bg-2 hover:border-border-strong")
                    }
                  >
                    <span className="block text-sm font-medium text-fg-0">{label}</span>
                    <span className="block text-[11px] text-fg-3">{hint}</span>
                  </button>
                );
              })}
            </div>
          </Row>
        )}

        {!isChatGpt && <Row label="API key" hint="stored in OS keychain, never written to disk">
          <div className="flex min-w-0 flex-1 flex-col gap-1.5">
            <div className="inline-flex min-w-0 items-center gap-1">
              <span className="inline-flex h-[26px] items-center rounded-l-[4px] border border-r-0 border-border-default bg-bg-inset px-1.5 font-mono text-sm text-fg-2">
                {current.keyPrefix}
              </span>
              <input
                className="-ml-px h-[26px] min-w-0 flex-1 border border-border-default bg-bg-inset px-2 font-mono text-sm text-fg-0 outline-none focus:border-accent-line focus:bg-bg-2"
                type={reveal ? "text" : "password"}
                aria-label={`${current.label} API key`}
                value={keyDraft}
                onChange={(e) => setKeyDraft(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void onSaveKey();
                }}
                placeholder={
                  keyConfigured ? "key stored, paste to replace" : "paste your API key"
                }
                spellCheck={false}
                autoComplete="off"
              />
              <button
                type="button"
                onClick={() => setReveal((v) => !v)}
                className="inline-flex h-[26px] items-center gap-1 rounded-[4px] border border-border-default bg-bg-2 px-2 text-sm text-fg-1 hover:bg-bg-3"
              >
                {reveal ? "hide" : "reveal"}
              </button>
              <button
                type="button"
                disabled={!keyDraft.trim() || saving}
                onClick={() => void onSaveKey()}
                className="inline-flex h-[26px] items-center gap-1 rounded-[4px] bg-accent px-2.5 text-sm font-medium text-accent-fg hover:brightness-[1.07] disabled:opacity-40"
              >
                {saving ? "saving…" : "save"}
              </button>
            </div>
            <div className="flex items-center gap-1.5 text-[11.5px]">
              {keyConfigured ? (
                <span className="inline-flex items-center gap-1 text-insert">
                  <Icon.check size={10} />
                  <span>key configured</span>
                </span>
              ) : (
                <span className="text-fg-3">no key configured</span>
              )}
            </div>
          </div>
        </Row>}

        {isChatGpt && (
          <Row label="ChatGPT account" hint="OAuth tokens stay in Cellar's local Codex runtime">
            <div className="flex min-w-0 flex-1 flex-col gap-2">
              {oauthStatus?.signed_in ? (
                <div
                  className="flex items-center justify-between rounded-[5px] border border-insert-line bg-insert-bg px-2.5 py-2"
                  aria-live="polite"
                >
                  <div className="min-w-0">
                    <div className="inline-flex items-center gap-1 text-sm font-medium text-insert">
                      <Icon.check size={11} /> Signed in
                    </div>
                    <div className="truncate text-[11px] text-fg-2">
                      {oauthStatus.email ?? "ChatGPT account"}
                      {oauthStatus.plan_type ? ` · ${oauthStatus.plan_type}` : ""}
                    </div>
                  </div>
                  <button
                    type="button"
                    disabled={authLoading}
                    onClick={() => void onRefreshLogin()}
                    className="h-[24px] rounded-[4px] border border-border-default bg-bg-2 px-2 text-[11px] text-fg-1 disabled:opacity-40"
                  >
                    refresh
                  </button>
                </div>
              ) : (
                <>
                  <div className="flex flex-wrap gap-1.5">
                    <button
                      type="button"
                      disabled={authLoading || !!login}
                      onClick={() => void onStartLogin("browser")}
                      className="h-[28px] rounded-[4px] bg-accent px-2.5 text-sm font-medium text-accent-fg disabled:opacity-40"
                    >
                      {authLoading ? "starting…" : "Sign in with ChatGPT"}
                    </button>
                    <button
                      type="button"
                      disabled={authLoading || !!login}
                      onClick={() => void onStartLogin("deviceCode")}
                      className="h-[28px] rounded-[4px] border border-border-default bg-bg-2 px-2.5 text-sm text-fg-1 disabled:opacity-40"
                    >
                      device code
                    </button>
                    <button
                      type="button"
                      disabled={authLoading}
                      onClick={() => void onRefreshLogin()}
                      className="h-[28px] rounded-[4px] border border-border-default bg-bg-2 px-2.5 text-sm text-fg-1 disabled:opacity-40"
                    >
                      check status
                    </button>
                  </div>
                  {login && (
                    <div className="rounded-[4px] border border-accent-line bg-accent-soft px-2.5 py-2 text-[11.5px] text-fg-1">
                      <div>Complete sign-in in your browser.</div>
                      {browserError && (
                        <div className="mt-1 text-delete" role="alert">
                          Could not open the browser: {browserError}
                        </div>
                      )}
                      {login.user_code && (
                        <div className="mt-1 font-mono text-sm text-accent">
                          code: {login.user_code}
                        </div>
                      )}
                      <div className="mt-1 flex gap-2">
                        <button
                          type="button"
                          onClick={() => void onOpenLogin()}
                          className="text-accent hover:underline"
                        >
                          open again
                        </button>
                        <button
                          type="button"
                          onClick={() => void cancelOpenAiLogin()}
                          className="text-delete hover:underline"
                        >
                          cancel
                        </button>
                      </div>
                    </div>
                  )}
                  <div className="text-[11px] leading-[1.4] text-fg-3">
                    Requires the current Codex CLI in your PATH. Cellar starts
                    it locally with an isolated data directory, read-only
                    sandbox, and approvals disabled.
                  </div>
                </>
              )}
            </div>
          </Row>
        )}

        <Row label="Model" hint={isChatGpt ? "available to your ChatGPT account" : "discovered from your API key"}>
          <div className="flex w-full flex-col gap-[3px]">
            <div className="mb-1 flex items-center justify-between">
              <span className="text-[11.5px] text-fg-3">
                {modelsLoading
                  ? "loading models…"
                  : modelsError
                    ? ""
                    : `${models.length} model${models.length === 1 ? "" : "s"} available`}
              </span>
              <button
                type="button"
                disabled={!configured || modelsLoading}
                onClick={() => void refreshModels()}
                className="inline-flex h-[22px] items-center gap-1 rounded-[4px] border border-border-default bg-bg-2 px-2 text-[11.5px] text-fg-1 hover:bg-bg-3 disabled:opacity-40"
              >
                <Icon.undo size={10} />
                <span>refresh</span>
              </button>
            </div>

            {modelsError && (
              <div
                className="mb-1 flex items-start gap-1.5 rounded-[4px] border border-delete-line bg-delete-bg px-2 py-1.5 text-[11.5px] text-delete"
                role="alert"
              >
                <Icon.warn size={11} />
                <span>{modelsError}</span>
              </div>
            )}

            {!configured ? (
              <div className="rounded-[4px] border border-dashed border-border-default px-2.5 py-2 text-[11.5px] text-fg-3">
                {isChatGpt
                  ? "Sign in with ChatGPT above to discover available models."
                  : "Add an API key above to discover available models."}
              </div>
            ) : models.length === 0 && !modelsLoading && !modelsError ? (
              <div className="rounded-[4px] border border-dashed border-border-default px-2.5 py-2 text-[11.5px] text-fg-3">
                No chat-capable models found for this key.
              </div>
            ) : (
              models.map((m) => {
                const active = modelId === m.id;
                return (
                  <label
                    key={m.id}
                    className={
                      "grid cursor-pointer grid-cols-[14px_1fr_auto] items-center gap-2.5 rounded-[4px] border px-2.5 py-1.5 " +
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
                    <span className="min-w-0 truncate font-mono text-sm text-fg-0">
                      {m.id}
                    </span>
                    {m.ctx && (
                      <span className="font-mono text-[11px] text-fg-3">
                        {m.ctx} ctx
                      </span>
                    )}
                  </label>
                );
              })
            )}
          </div>
        </Row>

        <Row label="Endpoint" hint="direct, not proxied through Cellar">
          <input
            className={CD_INPUT + " cursor-not-allowed font-mono opacity-80"}
            value={isChatGpt ? "local Codex app-server → OpenAI" : current.endpoint}
            readOnly
            style={{ flex: 1 }}
          />
        </Row>
      </Section>

      <Section title="Danger zone">
        <Row
          label="Clear AI conversation"
          hint={
            messages.length
              ? `${messages.length} message${messages.length === 1 ? "" : "s"} in the current thread`
              : "no active conversation"
          }
        >
          <button
            type="button"
            disabled={!messages.length}
            onClick={newThread}
            className={ED_RUN_DANGER + (messages.length ? "" : " cursor-not-allowed opacity-60")}
          >
            <Icon.trash size={11} />
            <span>Clear</span>
          </button>
        </Row>
        <Row
          label={isChatGpt ? "Sign out of ChatGPT" : "Revoke API key"}
          hint={isChatGpt ? "remove Cellar's local OAuth session" : "remove from keychain; does not affect provider"}
        >
          <button
            type="button"
            disabled={isChatGpt ? !oauthStatus?.signed_in : !keyConfigured}
            onClick={() => void (isChatGpt ? logoutOpenAi() : clearKey())}
            className={
              ED_RUN_DANGER +
              ((isChatGpt ? !!oauthStatus?.signed_in : keyConfigured)
                ? ""
                : " cursor-not-allowed opacity-60")
            }
          >
            <Icon.close size={11} />
            <span>{isChatGpt ? "Sign out" : "Revoke"}</span>
          </button>
        </Row>
      </Section>
    </div>
  );
}

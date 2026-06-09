import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { ORDERED_TOPICS, TOPICS, type AiTopic } from "@cellar/ai";
import { Icon } from "./icons";
import { useAi } from "../state/ai";
import { useTabs } from "../state/tabs";
import { useConnections } from "../state/connections";
import { buildActiveContext, type AiContextChip } from "../lib/aiContext";
import { AIMessage } from "./AIMessage";

const chipMeta: Record<AiContextChip["kind"], { icon: ReactNode; color: string }> = {
  schema: { icon: <Icon.schema size={9} />, color: "var(--fg-1)" },
  table: { icon: <Icon.table size={9} />, color: "var(--syn-tbl)" },
  query: { icon: <Icon.terminal size={9} />, color: "var(--syn-fn)" },
};

function ContextChip({ chip }: { chip: AiContextChip }) {
  const m = chipMeta[chip.kind];
  return (
    <span className="inline-flex h-5 items-center gap-1 whitespace-nowrap rounded-[4px] border border-border-default bg-bg-1 px-1.5 pl-[5px] text-[10.5px] text-fg-1">
      <span
        className="inline-flex items-center gap-[3px] text-[9.5px] lowercase opacity-85"
        style={{ color: m.color }}
      >
        {m.icon}
        <span>{chip.kind}</span>
      </span>
      <span className="text-fg-3">:</span>
      <span className="font-mono text-[10.5px] text-fg-0">{chip.value}</span>
    </span>
  );
}

const DISABLED_ICON =
  "icon-btn cursor-not-allowed opacity-45 hover:bg-transparent hover:text-fg-2";

const TOPIC_BUTTONS: AiTopic[] = ORDERED_TOPICS.filter((t) => t !== "ask");

export function AIPanel({
  onClose,
  onOpenSettings,
}: {
  onClose: () => void;
  onOpenSettings?: () => void;
}) {
  const [draft, setDraft] = useState("");
  const [topic, setTopic] = useState<AiTopic>("ask");
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const init = useAi((s) => s.init);
  const modelId = useAi((s) => s.modelId);
  const keyConfigured = useAi((s) => s.keyConfigured);
  const messages = useAi((s) => s.messages);
  const sending = useAi((s) => s.sending);
  const send = useAi((s) => s.send);
  const newThread = useAi((s) => s.newThread);

  // Recompute context whenever the active tab or its schema changes.
  const activeId = useTabs((s) => s.activeId);
  const byId = useConnections((s) => s.byId);
  const context = useMemo(
    () => buildActiveContext(),
    // byId carries introspected schema; activeId switches scope.
    [activeId, byId],
  );

  useEffect(() => {
    void init();
  }, [init]);

  const ready = keyConfigured && !!modelId;
  const canSend = ready && draft.trim().length > 0 && !sending;

  const submit = () => {
    if (!canSend) return;
    const text = draft;
    setDraft("");
    void send(topic, text, context.text);
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      e.preventDefault();
      submit();
    }
  };

  const pickTopic = (t: AiTopic) => {
    setTopic((cur) => (cur === t ? "ask" : t));
    textareaRef.current?.focus();
  };

  return (
    <div className="flex h-full flex-col bg-bg-1 text-[12.5px]">
      <div className="flex h-8 shrink-0 items-center justify-between border-b border-border-default pl-2.5 pr-2">
        <div className="flex min-w-0 flex-1 items-center gap-1.5 text-[11.5px]">
          <span className="inline-flex text-accent">
            <Icon.sparkles size={12} />
          </span>
          <span className="whitespace-nowrap font-semibold text-fg-0">
            AI Assistant
          </span>
          <span className="min-w-0 overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[10.5px] text-fg-2">
            {modelId ? `· ${modelId}` : "· not configured"}
          </span>
        </div>
        <div className="flex gap-px">
          <button
            className={DISABLED_ICON}
            disabled
            title="AI history is not wired yet"
          >
            <Icon.history size={12} />
          </button>
          <button
            className={
              messages.length ? "icon-btn" : DISABLED_ICON
            }
            disabled={!messages.length}
            onClick={newThread}
            title="New thread"
          >
            <Icon.plus size={12} />
          </button>
          <button
            className="icon-btn"
            onClick={onOpenSettings}
            title="AI settings"
          >
            <Icon.settings size={12} />
          </button>
          <button className="icon-btn" onClick={onClose} title="Close">
            <Icon.close size={12} />
          </button>
        </div>
      </div>

      <div className="flex shrink-0 flex-wrap items-start gap-1.5 border-b border-border-default bg-bg-2 px-2 pt-1.5 pb-[7px]">
        <div className="inline-flex h-[18px] shrink-0 items-center gap-1 pt-0.5 text-[10px] font-semibold uppercase tracking-[0.05em] text-fg-3">
          <Icon.context size={10} />
          <span>context</span>
        </div>
        <div className="flex flex-1 flex-wrap gap-1">
          {context.chips.length ? (
            context.chips.map((chip, i) => (
              <ContextChip key={`${chip.kind}-${i}`} chip={chip} />
            ))
          ) : (
            <span className="inline-flex h-5 items-center text-[10.5px] text-fg-3">
              no active connection
            </span>
          )}
        </div>
      </div>

      <div className="flex flex-1 flex-col gap-3 overflow-y-auto px-2.5 pt-3 pb-4">
        {messages.length === 0 ? (
          <div className="flex flex-1 flex-col items-center justify-center gap-1.5 p-6 text-center text-[11.5px] text-fg-3">
            <span className="mb-1 text-accent">
              <Icon.sparkles size={22} />
            </span>
            <div className="text-[12px] font-medium text-fg-1">Ask Cellar AI</div>
            <div className="max-w-[280px] text-[10.5px] leading-[1.5] text-fg-3">
              Generate SQL with full schema context, explain a result, or have it
              review a slow query. Bring your own API key; Cellar never proxies.
            </div>
            {!ready && (
              <button
                onClick={onOpenSettings}
                className="mt-2 inline-flex h-[24px] items-center gap-1.5 rounded-[5px] border border-accent-line bg-accent-soft px-2.5 text-[11px] font-medium text-accent hover:brightness-110"
              >
                <Icon.settings size={11} />
                <span>
                  {keyConfigured ? "Select a model" : "Configure a provider"}
                </span>
              </button>
            )}
          </div>
        ) : (
          messages.map((m) => <AIMessage key={m.id} entry={m} />)
        )}
        {sending && (
          <div className="flex items-center gap-2 px-1 text-[11px] text-fg-3">
            <span className="inline-block h-2 w-2 animate-pulse rounded-full bg-accent" />
            <span>thinking…</span>
          </div>
        )}
      </div>

      <div className="shrink-0 border-t border-border-default bg-bg-1">
        <div className="flex items-center gap-0.5 border-b border-border-divider px-1.5 py-[5px]">
          {TOPIC_BUTTONS.map((t) => {
            const active = topic === t;
            return (
              <button
                key={t}
                onClick={() => pickTopic(t)}
                title={TOPICS[t].hint}
                className={
                  "inline-flex h-[22px] items-center gap-1 rounded-[4px] px-2 text-[11px] transition-colors " +
                  (active
                    ? "bg-accent-soft text-accent"
                    : "text-fg-2 hover:bg-bg-2 hover:text-fg-1")
                }
              >
                {t === "generate" && <Icon.sparkles size={10} />}
                <span>{TOPICS[t].label}</span>
              </button>
            );
          })}
          <div className="flex-1" />
          <span className="inline-flex h-[22px] items-center gap-1 rounded-[4px] px-2 text-[10.5px] text-fg-3">
            <span style={{ color: topic === "ask" ? "var(--accent)" : "var(--fg-2)" }}>
              ask
            </span>
            <span style={{ color: "var(--fg-3)" }}> · read-only</span>
          </span>
        </div>
        <div className="px-1.5 py-1.5">
          <textarea
            ref={textareaRef}
            placeholder={
              ready
                ? "Ask, generate, or paste an error…  ⌘⏎ to send"
                : "Configure a provider in AI settings to start…"
            }
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={onKeyDown}
            rows={2}
            disabled={!ready}
            className="w-full resize-none rounded-[5px] border border-border-default bg-bg-inset px-2 py-[7px] text-[12px] leading-[1.45] text-fg-0 outline-none placeholder:text-fg-3 focus:border-accent-line font-sans min-h-[50px] disabled:opacity-60"
          />
          <div className="mt-1.5 flex items-center justify-between">
            <div className="flex items-center gap-1">
              <button
                className={DISABLED_ICON}
                disabled
                title="Query attachments are not wired yet"
              >
                <Icon.paperclip size={11} />
              </button>
              <span className="ml-1 inline-flex items-center gap-[3px] text-[10.5px]">
                {ready ? (
                  <span style={{ color: "var(--fg-3)" }}>
                    {topic === "ask" ? "ask" : TOPICS[topic].label} · {modelId}
                  </span>
                ) : (
                  <button
                    onClick={onOpenSettings}
                    className="text-fg-3 underline decoration-dotted underline-offset-2 hover:text-fg-1"
                  >
                    provider not configured
                  </button>
                )}
              </span>
            </div>
            <button
              onClick={submit}
              disabled={!canSend}
              className="inline-flex h-[22px] items-center gap-1.5 rounded-[4px] bg-accent px-2 text-[11px] font-medium text-accent-fg hover:brightness-[1.07] disabled:opacity-40 disabled:hover:brightness-100"
            >
              <span>Send</span>
              <Icon.send size={11} />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

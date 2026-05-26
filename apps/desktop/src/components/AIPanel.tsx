import { useState, type ReactNode } from "react";
import { Icon } from "./icons";

type ChipKind =
  | "schema"
  | "table"
  | "column"
  | "query"
  | "selection"
  | "plan"
  | "file";

const chipMeta: Record<ChipKind, { icon: ReactNode; color: string }> = {
  schema: { icon: <Icon.schema size={9} />, color: "var(--fg-1)" },
  table: { icon: <Icon.table size={9} />, color: "var(--syn-tbl)" },
  column: { icon: <Icon.fileText size={9} />, color: "var(--fg-1)" },
  query: { icon: <Icon.terminal size={9} />, color: "var(--syn-fn)" },
  selection: { icon: <Icon.fileText size={9} />, color: "var(--accent)" },
  plan: { icon: <Icon.tree size={9} />, color: "var(--fg-1)" },
  file: { icon: <Icon.fileText size={9} />, color: "var(--fg-1)" },
};

function ContextChip({ kind, value }: { kind: ChipKind; value: string }) {
  const m = chipMeta[kind];
  return (
    <span className="ai-chip">
      <span className="ai-chip-kind" style={{ color: m.color }}>
        {m.icon}
        <span>{kind}</span>
      </span>
      <span className="ai-chip-divider">:</span>
      <span className="ai-chip-value mono">{value}</span>
    </span>
  );
}

export function AIPanel({ onClose }: { onClose: () => void }) {
  const [draft, setDraft] = useState("");
  return (
    <div className="ai-root">
      <div className="ai-head">
        <div className="ai-head-left">
          <span className="ai-head-icon">
            <Icon.sparkles size={12} />
          </span>
          <span className="ai-head-title">AI Assistant</span>
          <span className="ai-head-thread">· new thread</span>
        </div>
        <div className="ai-head-right">
          <button className="icon-btn" title="History">
            <Icon.history size={12} />
          </button>
          <button className="icon-btn" title="New thread">
            <Icon.plus size={12} />
          </button>
          <button className="icon-btn" title="Settings">
            <Icon.settings size={12} />
          </button>
          <button className="icon-btn" onClick={onClose} title="Close">
            <Icon.close size={12} />
          </button>
        </div>
      </div>

      <div className="ai-context">
        <div className="ai-context-label">
          <Icon.context size={10} />
          <span>context</span>
        </div>
        <div className="ai-context-chips">
          <ContextChip kind="schema" value="public" />
          <ContextChip kind="table" value="orders" />
          <button className="ai-context-add">
            <Icon.plus size={9} />
            <span>add</span>
          </button>
        </div>
      </div>

      <div className="ai-msgs">
        <div className="ai-msgs-empty">
          <span className="ai-msgs-empty-icon">
            <Icon.sparkles size={22} />
          </span>
          <div className="ai-msgs-empty-title">Ask Cellar AI</div>
          <div className="ai-msgs-empty-sub">
            Generate SQL with full schema context, explain a result, or have it
            review a slow query. Bring your own API key — Cellar never proxies.
          </div>
        </div>
      </div>

      <div className="ai-composer">
        <div className="ai-composer-bar">
          <button className="ai-composer-mode active">
            <Icon.sparkles size={10} />
            <span>generate</span>
          </button>
          <button className="ai-composer-mode">explain</button>
          <button className="ai-composer-mode">optimize</button>
          <button className="ai-composer-mode">migrate</button>
          <div style={{ flex: 1 }} />
          <button className="ai-composer-mode subtle">
            <span style={{ color: "var(--fg-2)" }}>ask</span>
            <span style={{ color: "var(--fg-3)" }}> · read-only</span>
          </button>
        </div>
        <div className="ai-composer-input">
          <textarea
            placeholder="Ask, generate, or paste an error…  ⌘⏎ to send"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            rows={2}
          />
          <div className="ai-composer-foot">
            <div className="ai-composer-foot-left">
              <button className="icon-btn" title="Attach query">
                <Icon.paperclip size={11} />
              </button>
              <button className="icon-btn" title="Add table">
                <Icon.table size={11} />
              </button>
              <span className="ai-composer-foot-hint">
                <span style={{ color: "var(--fg-3)" }}>provider not configured</span>
              </span>
            </div>
            <button className="ai-send" disabled>
              <span>Send</span>
              <Icon.send size={11} />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

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
    <span className="inline-flex h-5 items-center gap-1 whitespace-nowrap rounded-[4px] border border-border-default bg-bg-1 px-1.5 pl-[5px] text-[10.5px] text-fg-1">
      <span
        className="inline-flex items-center gap-[3px] text-[9.5px] lowercase opacity-85"
        style={{ color: m.color }}
      >
        {m.icon}
        <span>{kind}</span>
      </span>
      <span className="text-fg-3">:</span>
      <span className="font-mono text-[10.5px] text-fg-0">{value}</span>
    </span>
  );
}

export function AIPanel({ onClose }: { onClose: () => void }) {
  const [draft, setDraft] = useState("");
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
          <span className="min-w-0 overflow-hidden text-ellipsis whitespace-nowrap text-[11px] text-fg-2">
            · new thread
          </span>
        </div>
        <div className="flex gap-px">
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

      <div className="flex shrink-0 flex-wrap items-start gap-1.5 border-b border-border-default bg-bg-2 px-2 pt-1.5 pb-[7px]">
        <div className="inline-flex h-[18px] shrink-0 items-center gap-1 pt-0.5 text-[10px] font-semibold uppercase tracking-[0.05em] text-fg-3">
          <Icon.context size={10} />
          <span>context</span>
        </div>
        <div className="flex flex-1 flex-wrap gap-1">
          <ContextChip kind="schema" value="public" />
          <ContextChip kind="table" value="orders" />
          <button className="inline-flex h-5 items-center gap-[3px] rounded-[4px] border border-dashed border-border-default px-1.5 text-[10px] text-fg-3 hover:border-border-strong hover:text-fg-0">
            <Icon.plus size={9} />
            <span>add</span>
          </button>
        </div>
      </div>

      <div className="flex flex-1 flex-col gap-[18px] overflow-y-auto px-2.5 pt-3 pb-4">
        <div className="flex flex-1 flex-col items-center justify-center gap-1.5 p-6 text-center text-[11.5px] text-fg-3">
          <span className="mb-1 text-accent">
            <Icon.sparkles size={22} />
          </span>
          <div className="text-[12px] font-medium text-fg-1">
            Ask Cellar AI
          </div>
          <div className="max-w-[280px] text-[10.5px] leading-[1.5] text-fg-3">
            Generate SQL with full schema context, explain a result, or have it
            review a slow query. Bring your own API key — Cellar never proxies.
          </div>
        </div>
      </div>

      <div className="shrink-0 border-t border-border-default bg-bg-1">
        <div className="flex items-center gap-0.5 border-b border-border-divider px-1.5 py-[5px]">
          <button className="inline-flex h-[22px] items-center gap-1 rounded-[4px] bg-accent-soft px-2 text-[11px] text-accent">
            <Icon.sparkles size={10} />
            <span>generate</span>
          </button>
          <button className="inline-flex h-[22px] items-center gap-1 rounded-[4px] px-2 text-[11px] text-fg-2 hover:bg-bg-2 hover:text-fg-0">
            explain
          </button>
          <button className="inline-flex h-[22px] items-center gap-1 rounded-[4px] px-2 text-[11px] text-fg-2 hover:bg-bg-2 hover:text-fg-0">
            optimize
          </button>
          <button className="inline-flex h-[22px] items-center gap-1 rounded-[4px] px-2 text-[11px] text-fg-2 hover:bg-bg-2 hover:text-fg-0">
            migrate
          </button>
          <div className="flex-1" />
          <button className="inline-flex h-[22px] items-center gap-1 rounded-[4px] px-2 text-[10.5px] text-fg-3 hover:bg-bg-2">
            <span style={{ color: "var(--fg-2)" }}>ask</span>
            <span style={{ color: "var(--fg-3)" }}> · read-only</span>
          </button>
        </div>
        <div className="px-1.5 py-1.5">
          <textarea
            placeholder="Ask, generate, or paste an error…  ⌘⏎ to send"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            rows={2}
            className="w-full resize-none rounded-[5px] border border-border-default bg-bg-inset px-2 py-[7px] text-[12px] leading-[1.45] text-fg-0 outline-none placeholder:text-fg-3 focus:border-accent-line font-sans min-h-[50px]"
          />
          <div className="mt-1.5 flex items-center justify-between">
            <div className="flex items-center gap-1">
              <button className="icon-btn" title="Attach query">
                <Icon.paperclip size={11} />
              </button>
              <button className="icon-btn" title="Add table">
                <Icon.table size={11} />
              </button>
              <span className="ml-1.5 inline-flex items-center gap-[3px] text-[10.5px]">
                <span style={{ color: "var(--fg-3)" }}>
                  provider not configured
                </span>
              </span>
            </div>
            <button
              disabled
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

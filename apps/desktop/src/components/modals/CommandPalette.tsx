import { useEffect, useState, type ReactNode } from "react";
import { Icon } from "../icons";

type Group = "Recent" | "Actions" | "Navigate" | "AI" | "View";

type Entry = {
  grp: Group;
  label: string;
  hint?: string;
  kbd?: string[];
};

const ENTRIES: Entry[] = [
  { grp: "Recent", label: "public.orders", hint: "table" },
  { grp: "Recent", label: "revenue_by_country.sql", hint: "query tab" },
  { grp: "Recent", label: "shop-eu (prod)", hint: "connection" },
  { grp: "Actions", label: "Run current statement", hint: "executes statement under cursor", kbd: ["⌘", "⏎"] },
  { grp: "Actions", label: "Run all", hint: "executes the full editor", kbd: ["⌘", "⇧", "⏎"] },
  { grp: "Actions", label: "Commit pending changes", hint: "review SQL diff before commit", kbd: ["⌘", "S"] },
  { grp: "Actions", label: "Revert pending changes", hint: "discard 4 staged edits", kbd: ["⌘", "⇧", "Z"] },
  { grp: "Navigate", label: "Go to table…", hint: "search across all schemas", kbd: ["⌘", "O"] },
  { grp: "Navigate", label: "Go to column…", hint: "fuzzy across catalog", kbd: ["⌘", "⇧", "O"] },
  { grp: "Navigate", label: "Switch connection…", hint: "6 connections", kbd: ["⌘", "K", "C"] },
  { grp: "AI", label: "Ask AI about selected text", hint: "starts a new thread", kbd: ["⌘", "L"] },
  { grp: "AI", label: "Generate query from prompt…", hint: "context: public.orders", kbd: ["⌘", "I"] },
  { grp: "AI", label: "Explain selected SQL", hint: "shows plan + line notes" },
  { grp: "View", label: "Split editor horizontally", kbd: ["⌘", "\\"] },
  { grp: "View", label: "Split editor vertically", kbd: ["⌘", "⇧", "\\"] },
  { grp: "View", label: "Toggle AI panel", kbd: ["⌘", "J"] },
];

function groupIcon(grp: Group): ReactNode {
  switch (grp) {
    case "AI":
      return <Icon.sparkles size={11} stroke="var(--accent)" />;
    case "Actions":
      return <Icon.bolt size={11} stroke="var(--update)" />;
    case "Navigate":
      return <Icon.chevronRight size={11} stroke="var(--fg-2)" />;
    case "View":
      return <Icon.layout size={11} stroke="var(--fg-2)" />;
    case "Recent":
      return <Icon.history size={11} stroke="var(--fg-2)" />;
  }
}

export function CommandPalette({ onClose }: { onClose: () => void }) {
  const [q, setQ] = useState("");

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const filtered = q
    ? ENTRIES.filter((c) => c.label.toLowerCase().includes(q.toLowerCase()))
    : ENTRIES;

  const grouped: Record<string, Entry[]> = {};
  for (const c of filtered) (grouped[c.grp] ||= []).push(c);

  return (
    <div
      className="fixed inset-0 z-[100] flex items-start justify-center bg-bg-overlay pt-[14vh] backdrop-blur-[4px] animate-scrim-in"
      onClick={onClose}
    >
      <div
        className="flex w-[580px] flex-col overflow-hidden rounded-lg border border-border-default bg-bg-1 shadow-lg animate-modal-in-fast"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex h-[38px] items-center gap-[9px] border-b border-border-default px-3">
          <Icon.search size={13} stroke="var(--fg-3)" />
          <input
            placeholder="Search tables, columns, commands, AI prompts…"
            value={q}
            onChange={(e) => setQ(e.target.value)}
            autoFocus
            className="flex-1 border-none bg-transparent text-[13px] text-fg-0 outline-none placeholder:text-fg-3"
          />
          <span className="kbd">esc</span>
        </div>

        <div className="max-h-[420px] overflow-y-auto pt-1 pb-2">
          {Object.entries(grouped).map(([grp, items]) => (
            <div key={grp} className="pt-1.5 pb-1">
              <div className="px-3.5 py-0.5 text-[10px] font-semibold uppercase tracking-[0.06em] text-fg-3">
                {grp}
              </div>
              {items.map((c, i) => {
                const isActive = grp === "Recent" && i === 0;
                return (
                  <button
                    key={c.label}
                    className={
                      "flex w-full items-center gap-2.5 px-3.5 py-1.5 text-left text-[12px] " +
                      (isActive
                        ? "bg-accent-soft text-accent"
                        : "text-fg-1 hover:bg-bg-2 hover:text-fg-0")
                    }
                  >
                    <span className="inline-flex w-[18px] shrink-0 items-center justify-center">
                      {groupIcon(grp as Group)}
                    </span>
                    <span className="shrink-0 whitespace-nowrap font-medium">
                      {c.label}
                    </span>
                    {c.hint && (
                      <span className="ml-auto min-w-0 overflow-hidden text-ellipsis whitespace-nowrap pr-1.5 text-[11px] text-fg-3">
                        {c.hint}
                      </span>
                    )}
                    {c.kbd && (
                      <span className="inline-flex shrink-0 gap-0.5">
                        {c.kbd.map((k, j) => (
                          <kbd key={j} className="kbd">
                            {k}
                          </kbd>
                        ))}
                      </span>
                    )}
                  </button>
                );
              })}
            </div>
          ))}
          {filtered.length === 0 && (
            <div className="px-3.5 py-5 text-center text-[11.5px] text-fg-3">
              No matches for &ldquo;{q}&rdquo;
            </div>
          )}
        </div>

        <div className="flex items-center gap-3 border-t border-border-default bg-bg-2 px-3 py-1.5 text-[10.5px] text-fg-3">
          <span className="inline-flex items-center gap-1">
            <kbd className="kbd">↑↓</kbd>
            <span>navigate</span>
          </span>
          <span className="inline-flex items-center gap-1">
            <kbd className="kbd">⏎</kbd>
            <span>select</span>
          </span>
          <span className="inline-flex items-center gap-1">
            <kbd className="kbd">⌘⏎</kbd>
            <span>open in new tab</span>
          </span>
          <div className="flex-1" />
          <span className="inline-flex items-center gap-1">
            <Icon.sparkles size={10} stroke="var(--accent)" />
            <span style={{ color: "var(--accent)" }}>type / to ask AI</span>
          </span>
        </div>
      </div>
    </div>
  );
}

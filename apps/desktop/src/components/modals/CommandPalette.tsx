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
    <div className="modal-scrim cmd-scrim" onClick={onClose}>
      <div className="cmd-shell" onClick={(e) => e.stopPropagation()}>
        <div className="cmd-input">
          <Icon.search size={13} stroke="var(--fg-3)" />
          <input
            placeholder="Search tables, columns, commands, AI prompts…"
            value={q}
            onChange={(e) => setQ(e.target.value)}
            autoFocus
          />
          <span className="kbd">esc</span>
        </div>

        <div className="cmd-list">
          {Object.entries(grouped).map(([grp, items]) => (
            <div key={grp} className="cmd-group">
              <div className="cmd-group-head">{grp}</div>
              {items.map((c, i) => (
                <button
                  key={c.label}
                  className={
                    "cmd-item" + (grp === "Recent" && i === 0 ? " active" : "")
                  }
                >
                  <span className="cmd-item-icon">{groupIcon(grp as Group)}</span>
                  <span className="cmd-item-label">{c.label}</span>
                  {c.hint && <span className="cmd-item-hint">{c.hint}</span>}
                  {c.kbd && (
                    <span className="cmd-item-kbd">
                      {c.kbd.map((k, j) => (
                        <kbd key={j} className="kbd">
                          {k}
                        </kbd>
                      ))}
                    </span>
                  )}
                </button>
              ))}
            </div>
          ))}
          {filtered.length === 0 && (
            <div
              style={{
                padding: "20px 14px",
                fontSize: 11.5,
                color: "var(--fg-3)",
                textAlign: "center",
              }}
            >
              No matches for &ldquo;{q}&rdquo;
            </div>
          )}
        </div>

        <div className="cmd-foot">
          <span>
            <kbd className="kbd">↑↓</kbd>
            <span>navigate</span>
          </span>
          <span>
            <kbd className="kbd">⏎</kbd>
            <span>select</span>
          </span>
          <span>
            <kbd className="kbd">⌘⏎</kbd>
            <span>open in new tab</span>
          </span>
          <div style={{ flex: 1 }} />
          <span>
            <Icon.sparkles size={10} stroke="var(--accent)" />
            <span style={{ color: "var(--accent)" }}>type / to ask AI</span>
          </span>
        </div>
      </div>
    </div>
  );
}

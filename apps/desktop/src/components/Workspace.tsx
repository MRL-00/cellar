import { Icon } from "./icons";

export function Workspace({ onCommit }: { onCommit?: () => void } = {}) {
  return (
    <div className="flex flex-1 flex-col min-h-0 overflow-hidden">
      <div className="flex flex-1 flex-col items-center justify-center gap-[14px] bg-bg-inset px-10 py-10 text-center text-[12.5px] text-fg-2">
        <span
          className="relative h-9 w-9 rounded-lg"
          style={{
            background:
              "linear-gradient(135deg, #c4b5fd 0%, var(--accent) 55%, #6d4ed1 100%)",
            boxShadow: "0 0 24px var(--accent-soft)",
          }}
        >
          <span
            className="absolute inset-[5px] rounded bg-bg-inset"
            style={{
              clipPath:
                "polygon(0 0, 100% 0, 100% 35%, 35% 35%, 35% 65%, 100% 65%, 100% 100%, 0 100%)",
            }}
          />
        </span>
        <div>
          <div className="text-[14px] font-semibold text-fg-0">
            SQL editor coming soon
          </div>
          <div className="max-w-[360px] text-[11.5px] leading-[1.5] text-fg-3">
            CodeMirror 6 with schema-aware autocomplete and ghost-text AI
            completion will live here. The data grid for table tabs ships in the
            same release.
          </div>
        </div>
        <div className="flex gap-3 text-[10.5px] text-fg-3">
          <span className="inline-flex items-center gap-1">
            <kbd className="kbd">⌘</kbd>
            <kbd className="kbd">⏎</kbd>&nbsp;run statement
          </span>
          <span className="inline-flex items-center gap-1">
            <kbd className="kbd">⌘</kbd>
            <kbd className="kbd">K</kbd>&nbsp;command palette
          </span>
          <span className="inline-flex items-center gap-1">
            <Icon.sparkles size={11} style={{ color: "var(--accent)" }} />
            <kbd className="kbd">⌘</kbd>
            <kbd className="kbd">I</kbd>&nbsp;ask AI
          </span>
        </div>
        {onCommit && (
          <button
            onClick={onCommit}
            title="Open the commit review modal"
            className="mt-1 inline-flex h-[26px] items-center gap-[5px] whitespace-nowrap rounded-[4px] border border-border-default bg-transparent px-2.5 text-[11.5px] font-medium text-fg-1 transition-[background,color,border-color] duration-[120ms] hover:border-border-strong hover:bg-bg-3 hover:text-fg-0"
          >
            <Icon.commit size={11} />
            <span>Review &amp; commit (4 pending)</span>
            <kbd className="kbd ml-1">⌘S</kbd>
          </button>
        )}
      </div>
    </div>
  );
}

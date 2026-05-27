import { Icon } from "./icons";

export function Workspace({ onCommit }: { onCommit?: () => void } = {}) {
  return (
    <div className="cellar-workspace">
      <div className="ws-placeholder">
        <span className="ws-placeholder-mark" />
        <div>
          <div className="ws-placeholder-title">SQL editor coming soon</div>
          <div className="ws-placeholder-sub">
            CodeMirror 6 with schema-aware autocomplete and ghost-text AI
            completion will live here. The data grid for table tabs ships in the
            same release.
          </div>
        </div>
        <div className="ws-placeholder-kbds">
          <span>
            <kbd className="kbd">⌘</kbd>
            <kbd className="kbd">⏎</kbd>&nbsp;run statement
          </span>
          <span>
            <kbd className="kbd">⌘</kbd>
            <kbd className="kbd">K</kbd>&nbsp;command palette
          </span>
          <span>
            <Icon.sparkles size={11} style={{ color: "var(--accent)" }} />
            <kbd className="kbd">⌘</kbd>
            <kbd className="kbd">I</kbd>&nbsp;ask AI
          </span>
        </div>
        {onCommit && (
          <button
            className="ed-run subtle"
            onClick={onCommit}
            style={{ marginTop: 4 }}
            title="Open the commit review modal"
          >
            <Icon.commit size={11} />
            <span>Review &amp; commit (4 pending)</span>
            <kbd className="kbd" style={{ marginLeft: 4 }}>
              ⌘S
            </kbd>
          </button>
        )}
      </div>
    </div>
  );
}

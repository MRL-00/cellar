import { Icon } from "./icons";

export function Workspace() {
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
      </div>
    </div>
  );
}

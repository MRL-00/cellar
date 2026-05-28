import { GridIcon } from "./icons";
import { countChanges } from "./status";
import type { PendingChanges } from "./types";

export type PendingBarProps = {
  changes: PendingChanges;
  onCommit?: () => void;
  onRevert?: () => void;
};

export function PendingBar({ changes, onCommit, onRevert }: PendingBarProps) {
  const { total, inserts, updates, deletes } = countChanges(changes);

  return (
    <div className={"grid-pending" + (total === 0 ? " empty" : "")}>
      <div className="grid-pending-left">
        <GridIcon.diff
          size={11}
          style={{ color: total > 0 ? "var(--update)" : "var(--fg-3)" }}
        />
        {total === 0 ? (
          <span style={{ color: "var(--fg-2)" }}>No pending changes</span>
        ) : (
          <span className="grid-pending-summary">
            <span>{total} pending</span>
            <span className="grid-pending-divider">·</span>
            {inserts > 0 && (
              <span className="grid-pending-chip grid-pending-chip-insert">
                <span className="dot" style={{ background: "var(--insert)" }} />
                <span>
                  {inserts} insert{inserts === 1 ? "" : "s"}
                </span>
              </span>
            )}
            {updates > 0 && (
              <span className="grid-pending-chip grid-pending-chip-update">
                <span className="dot" style={{ background: "var(--update)" }} />
                <span>
                  {updates} update{updates === 1 ? "" : "s"}
                </span>
              </span>
            )}
            {deletes > 0 && (
              <span className="grid-pending-chip grid-pending-chip-delete">
                <span className="dot" style={{ background: "var(--delete)" }} />
                <span>
                  {deletes} delete{deletes === 1 ? "" : "s"}
                </span>
              </span>
            )}
            <span className="grid-pending-divider">·</span>
            <span style={{ color: "var(--fg-3)" }}>wrapped in transaction</span>
          </span>
        )}
      </div>
      <div className="grid-pending-right">
        <button
          className="grid-pending-btn subtle"
          onClick={onRevert}
          disabled={total === 0}
        >
          <GridIcon.undo size={11} />
          <span>Revert</span>
        </button>
        <button
          className="grid-pending-btn primary"
          onClick={onCommit}
          disabled={total === 0}
        >
          <GridIcon.commit size={11} />
          <span>Review &amp; Commit</span>
          <span className="kbd">⌘S</span>
        </button>
      </div>
    </div>
  );
}

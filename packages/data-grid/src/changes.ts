import type { CellChange, PendingChange, PendingChanges } from "./types";

/**
 * Merge a single cell edit into pending changes. Preserves insert/update/delete
 * kind, anchors `from` to the original value, and drops the edit (or the whole
 * update row) when the value returns to original.
 */
export function applyCellChange(
  changes: PendingChanges,
  rowId: string,
  colKey: string,
  prev: CellChange["from"],
  next: CellChange["to"],
): PendingChanges {
  if (prev === next) return changes;

  const existing: PendingChange = changes[rowId] ?? { kind: "update", edits: {} };
  const baseEdit = existing.edits[colKey];
  const fromValue = baseEdit ? baseEdit.from : prev;

  const nextEdits = { ...existing.edits };
  if (fromValue === next) {
    delete nextEdits[colKey];
  } else {
    nextEdits[colKey] = { from: fromValue, to: next };
  }

  const updated: PendingChanges = { ...changes };
  const editKeys = Object.keys(nextEdits);
  if (editKeys.length === 0 && existing.kind === "update") {
    delete updated[rowId];
  } else {
    updated[rowId] = { kind: existing.kind, edits: nextEdits };
  }
  return updated;
}

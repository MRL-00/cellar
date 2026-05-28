import type { PendingChanges, GridStatusCounts } from "./types";

const STATUS_DOT_VAR: Record<string, string> = {
  pending: "var(--warn)",
  paid: "var(--accent)",
  fulfilled: "#60a5fa",
  shipped: "#a78bfa",
  delivered: "#34d399",
  cancelled: "#f87171",
  refunded: "#fb7185",
};

const STATUS_TEXT_VAR: Record<string, string> = {
  pending: "#fbbf24",
  paid: "var(--accent)",
  fulfilled: "#60a5fa",
  shipped: "#a78bfa",
  delivered: "#34d399",
  cancelled: "#f87171",
  refunded: "#fb7185",
};

/** Color for the dot rendered next to a status enum value. */
export function statusDotColor(value: string): string {
  return STATUS_DOT_VAR[value] ?? "var(--fg-2)";
}

/** Color for the status enum text/label. */
export function statusTextColor(value: string): string {
  return STATUS_TEXT_VAR[value] ?? "var(--fg-1)";
}

/** Tally pending changes by kind. */
export function countChanges(changes: PendingChanges): GridStatusCounts {
  const values = Object.values(changes);
  let inserts = 0;
  let updates = 0;
  let deletes = 0;
  for (const c of values) {
    if (c.kind === "insert") inserts += 1;
    else if (c.kind === "update") updates += 1;
    else if (c.kind === "delete") deletes += 1;
  }
  return { total: values.length, inserts, updates, deletes };
}

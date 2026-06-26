import type { PendingChanges, GridStatusCounts } from "./types";

// ponytail: two near-identical maps collapsed; pending has intentionally different dot/text values
const STATUS_COLORS: Record<string, { dot: string; text: string }> = {
  pending:   { dot: "var(--warn)", text: "#fbbf24" },
  paid:      { dot: "var(--accent)", text: "var(--accent)" },
  fulfilled: { dot: "#60a5fa", text: "#60a5fa" },
  shipped:   { dot: "#a78bfa", text: "#a78bfa" },
  delivered: { dot: "#34d399", text: "#34d399" },
  cancelled: { dot: "#f87171", text: "#f87171" },
  refunded:  { dot: "#fb7185", text: "#fb7185" },
};

/** Color for the dot rendered next to a status enum value. */
export function statusDotColor(value: string): string {
  return STATUS_COLORS[value]?.dot ?? "var(--fg-2)";
}

/** Color for the status enum text/label. */
export function statusTextColor(value: string): string {
  return STATUS_COLORS[value]?.text ?? "var(--fg-1)";
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

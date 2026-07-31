/** Shared accent markers for connections and sidebar folders. */
export const MARKER_SWATCHES = [
  { color: "#a1a1a1", label: "Neutral" },
  { color: "#4f8ff7", label: "Blue" },
  { color: "#f6a44a", label: "Orange" },
  { color: "#d97a5a", label: "Coral" },
  { color: "#5bb8e0", label: "Cyan" },
  { color: "#a78bfa", label: "Purple" },
  { color: "#4ade80", label: "Green" },
  { color: "#f87171", label: "Red" },
] as const;

const HEX = /^#[0-9a-fA-F]{6}$/;

export function isMarkerColor(value: unknown): value is string {
  return typeof value === "string" && HEX.test(value);
}

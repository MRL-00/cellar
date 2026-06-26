/**
 * Pluggable cell-renderer contracts for the Cellar data grid.
 *
 * Renderers let complex SQL types (JSON/JSONB, arrays, bytea/blob, PostGIS
 * geometry, …) display meaningfully in the grid instead of a raw stringified
 * value, while leaving the plain editable-cell path untouched for simple types.
 *
 * The interface is intentionally small so community plugins can ship their own
 * renderers (aligns with SPEC §9 — exporters and renderers are pluggable). A
 * renderer never owns editing or selection; it only decides how a *display*
 * cell looks. Double-click still opens the existing inline editor.
 *
 * Performance contract: `renderInline` is called for every visible cell on each
 * render, so it must be cheap and synchronous. The richer `renderExpanded` view
 * is mounted lazily — only when the user opens the popover — so heavy work (full
 * JSON trees, hex dumps, image decoding) never runs for off-screen or
 * un-interacted cells.
 */
import type { ReactNode } from "react";
import type { GridColumn, GridValue } from "../types";

/** A cell value that is known to be non-null (renderers never see NULL). */
export type NonNullGridValue = Exclude<GridValue, null>;

/** Where a renderer is currently being asked to draw. */
export type CellRenderMode = "inline" | "expanded";

/**
 * Persist a binary payload to disk. Defaults to a user-initiated browser
 * download (no new Tauri capability, no network). Hosts can inject a Tauri save
 * dialog implementation instead.
 */
export type SaveBlob = (data: Uint8Array, filename: string, mime: string) => void;

export type CellRenderContext = {
  column: GridColumn;
  /** The scalar value held by the grid for this cell. */
  value: NonNullGridValue;
  /** `String(value)` — precomputed for the common parse-from-text case. */
  text: string;
  mode: CellRenderMode;
  /** Text shown for SQL NULL, mirrored from the grid for nested nulls. */
  nullDisplay: string;
  /** Open the expanded popover for this cell (no-op in expanded mode). */
  requestExpand: () => void;
  /** Close the expanded popover (no-op in inline mode). */
  close: () => void;
  saveBlob: SaveBlob;
};

export type CellRenderer = {
  /** Stable id; doubles as a React key and lets a host override a built-in. */
  id: string;
  /** Higher wins when multiple renderers match the same cell. Default 0. */
  priority?: number;
  /** Whether this renderer claims the given column + value. */
  appliesTo: (column: GridColumn, value: NonNullGridValue) => boolean;
  /** Compact, cheap inline view rendered inside the virtualized cell. */
  renderInline: (ctx: CellRenderContext) => ReactNode;
  /**
   * Optional richer view shown in a popover on interaction. Presence of this
   * function is what makes the inline cell offer an expand affordance.
   */
  renderExpanded?: (ctx: CellRenderContext) => ReactNode;
  /** Optional popover heading. */
  title?: (ctx: CellRenderContext) => string;
};

export type RendererRegistry = {
  /** Renderers in resolution order (highest priority first). */
  readonly renderers: readonly CellRenderer[];
  /** First renderer whose `appliesTo` matches, or null for the default path. */
  resolve: (column: GridColumn, value: NonNullGridValue) => CellRenderer | null;
};

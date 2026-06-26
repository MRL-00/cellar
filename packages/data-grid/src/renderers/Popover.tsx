/**
 * Minimal popover used by rich cell renderers for their expanded view.
 *
 * It deliberately avoids `react-dom`'s `createPortal` so the data-grid package
 * keeps a single `react` dependency and can ship to plugin authors unchanged.
 * Instead it renders a `position: fixed` panel anchored to the cell — fixed
 * positioning escapes the grid's `overflow: hidden` cells because no grid
 * ancestor establishes a transformed containing block.
 *
 * The panel mounts only while open (the caller guards on its own `open` state),
 * so the heavy expanded content is never built for un-interacted cells.
 */
import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
  type RefObject,
} from "react";

const PANEL_WIDTH = 360;
const VIEWPORT_MARGIN = 8;
// Sit flush against the cell vertically: no dead space for a hover to fall
// through, and no visual gap between the cell you are on and its detail panel.
const ANCHOR_GAP = 0;
// Nudge the panel right of the cell's left edge so the column underneath stays
// uncovered — you can run the mouse straight down it to preview other rows.
const ANCHOR_OFFSET_X = 48;

export type PopoverProps = {
  anchorRef: RefObject<HTMLElement | null>;
  onClose: () => void;
  title?: string;
  children: ReactNode;
  /** Hover bridge: cancel a pending close when the pointer enters the panel. */
  onMouseEnter?: () => void;
  /** Hover bridge: schedule a close when the pointer leaves the panel. */
  onMouseLeave?: () => void;
};

export function Popover({
  anchorRef,
  onClose,
  title,
  children,
  onMouseEnter,
  onMouseLeave,
}: PopoverProps) {
  const panelRef = useRef<HTMLDivElement | null>(null);
  // The viewport position we *want* the panel to occupy.
  const targetRef = useRef<{ top: number; left: number } | null>(null);
  const [pos, setPos] = useState<{ top: number; left: number } | null>(null);
  const [settled, setSettled] = useState(false);

  // Pass 1 — compute the intended viewport-space position from the cell box.
  useLayoutEffect(() => {
    const anchor = anchorRef.current;
    if (!anchor || typeof window === "undefined") return;
    // Anchor to the whole cell box, not the vertically-centered inline content,
    // so the panel lands flush with the cell edge the user is actually on.
    const cell = (anchor.closest?.(".grid-cell") as HTMLElement | null) ?? anchor;
    const rect = cell.getBoundingClientRect();
    const panelHeight = panelRef.current?.offsetHeight ?? 0;

    const left = Math.max(
      VIEWPORT_MARGIN,
      Math.min(rect.left + ANCHOR_OFFSET_X, window.innerWidth - PANEL_WIDTH - VIEWPORT_MARGIN),
    );
    // Prefer directly below the cell; flip above when it would overflow, rather
    // than letting the clamp shove it down and away from the cell.
    const below = rect.bottom + ANCHOR_GAP;
    const fitsBelow = below + panelHeight <= window.innerHeight - VIEWPORT_MARGIN;
    const top = fitsBelow
      ? below
      : Math.max(VIEWPORT_MARGIN, rect.top - ANCHOR_GAP - panelHeight);
    targetRef.current = { top, left };
    setSettled(false);
    setPos({ top, left });
    // Position only when the popover opens against a given anchor — NOT on
    // `children`, or inner state changes (JSON expand, "Copied", image toggle)
    // would re-run this, reset `settled`, and flash the panel hidden.
  }, [anchorRef]);

  // Pass 2 — `position: fixed` is resolved against the nearest transformed or
  // `contain`-ed ancestor, which in this app's panel layout is not the viewport.
  // Measure where the panel actually landed and nudge by the delta so it ends
  // up where we intended in viewport space. Converges in one step, then no-ops.
  useLayoutEffect(() => {
    const panel = panelRef.current;
    const target = targetRef.current;
    if (!panel || !target || !pos) return;
    const actual = panel.getBoundingClientRect();
    const dx = target.left - actual.left;
    const dy = target.top - actual.top;
    if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) {
      if (!settled) setSettled(true);
      return;
    }
    setPos({ top: pos.top + dy, left: pos.left + dx });
  }, [pos, settled]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        onClose();
      }
    };
    // Capture phase so a click anywhere outside closes before other handlers,
    // while clicks inside the panel are excluded via `contains`.
    const onPointerDown = (event: MouseEvent) => {
      if (!panelRef.current?.contains(event.target as Node | null)) onClose();
    };
    // Capture phase catches scrolls on the grid's inner `.grid-scroll` (which
    // never reach `window` via bubbling) so the panel doesn't float away from
    // its anchor. But scrolling *inside* the panel (long JSON tree, hex dump)
    // must not dismiss it, so ignore scrolls originating within it.
    const onScroll = (event: Event) => {
      if (panelRef.current?.contains(event.target as Node | null)) return;
      onClose();
    };
    const onResize = () => onClose();
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("mousedown", onPointerDown, true);
    window.addEventListener("scroll", onScroll, true);
    window.addEventListener("resize", onResize);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("mousedown", onPointerDown, true);
      window.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("resize", onResize);
    };
  }, [onClose]);

  // Swallow grid-cell interactions (select / edit / context-menu) that would
  // otherwise bubble through the React tree from inside the panel.
  const stop = (event: { stopPropagation: () => void }) => event.stopPropagation();

  return (
    <div
      ref={panelRef}
      className="cell-popover mono"
      role="dialog"
      aria-label={title ?? "Cell detail"}
      style={
        pos
          ? {
              top: pos.top,
              left: pos.left,
              width: PANEL_WIDTH,
              visibility: settled ? "visible" : "hidden",
            }
          : { visibility: "hidden", width: PANEL_WIDTH }
      }
      onClick={stop}
      onDoubleClick={stop}
      onContextMenu={stop}
      onMouseDown={stop}
      onKeyDown={stop}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
    >
      {title && (
        <div className="cell-popover-head">
          <span className="cell-popover-title">{title}</span>
          <button
            type="button"
            className="cell-popover-close"
            onClick={onClose}
            aria-label="Close"
            title="Close"
          >
            ✕
          </button>
        </div>
      )}
      <div className="cell-popover-body">{children}</div>
    </div>
  );
}

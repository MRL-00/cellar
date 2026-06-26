/**
 * Wraps a resolved {@link CellRenderer} for use inside a grid cell: renders the
 * cheap inline view always, and lazily mounts the renderer's expanded view in a
 * popover only after the user interacts. Keeps the virtualized grid smooth — no
 * heavy expanded content is built until needed.
 *
 * The expanded view opens on hover with a short grace period: leaving the cell
 * schedules a close, but moving into the popover (across the small anchor gap)
 * cancels it, so the JSON tree, Copy button, image preview, etc. stay usable.
 * The focusable expand affordance keeps the feature reachable by keyboard.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Popover } from "./Popover";
import { defaultSaveBlob } from "./shared";
import type {
  CellRenderContext,
  CellRenderer,
  NonNullGridValue,
  SaveBlob,
} from "./types";
import type { GridColumn } from "../types";

const OPEN_DELAY_MS = 90;
const CLOSE_DELAY_MS = 180;

export type RichCellProps = {
  renderer: CellRenderer;
  column: GridColumn;
  value: NonNullGridValue;
  nullDisplay: string;
  saveBlob?: SaveBlob;
};

export function RichCell({
  renderer,
  column,
  value,
  nullDisplay,
  saveBlob,
}: RichCellProps) {
  const [open, setOpen] = useState(false);
  const anchorRef = useRef<HTMLSpanElement | null>(null);
  const openTimer = useRef<number | null>(null);
  const closeTimer = useRef<number | null>(null);

  const clearOpenTimer = useCallback(() => {
    if (openTimer.current !== null) {
      window.clearTimeout(openTimer.current);
      openTimer.current = null;
    }
  }, []);
  const clearCloseTimer = useCallback(() => {
    if (closeTimer.current !== null) {
      window.clearTimeout(closeTimer.current);
      closeTimer.current = null;
    }
  }, []);

  useEffect(
    () => () => {
      clearOpenTimer();
      clearCloseTimer();
    },
    [clearOpenTimer, clearCloseTimer],
  );

  // Hover in → open after a short delay (avoids flicker while scanning rows).
  const scheduleOpen = useCallback(() => {
    clearCloseTimer();
    if (openTimer.current !== null) return;
    openTimer.current = window.setTimeout(() => {
      openTimer.current = null;
      setOpen(true);
    }, OPEN_DELAY_MS);
  }, [clearCloseTimer]);

  // Hover out → close after a grace period; re-entering cancels it.
  const scheduleClose = useCallback(() => {
    clearOpenTimer();
    if (closeTimer.current !== null) return;
    closeTimer.current = window.setTimeout(() => {
      closeTimer.current = null;
      setOpen(false);
    }, CLOSE_DELAY_MS);
  }, [clearOpenTimer]);

  const openNow = useCallback(() => {
    clearOpenTimer();
    clearCloseTimer();
    setOpen(true);
  }, [clearOpenTimer, clearCloseTimer]);
  const close = useCallback(() => {
    clearOpenTimer();
    clearCloseTimer();
    setOpen(false);
  }, [clearOpenTimer, clearCloseTimer]);

  const base = useMemo(
    () => ({
      column,
      value,
      text: String(value),
      nullDisplay,
      saveBlob: saveBlob ?? defaultSaveBlob,
      requestExpand: openNow,
      close,
    }),
    [column, value, nullDisplay, saveBlob, openNow, close],
  );

  const inlineCtx: CellRenderContext = { ...base, mode: "inline" };
  const canExpand = renderer.renderExpanded !== undefined;

  return (
    <span
      ref={anchorRef}
      className="cell-rich"
      onMouseEnter={canExpand ? scheduleOpen : undefined}
      onMouseLeave={canExpand ? scheduleClose : undefined}
    >
      <span className="cell-rich-inline">{renderer.renderInline(inlineCtx)}</span>
      {canExpand && (
        <button
          type="button"
          className="cell-rich-expand"
          aria-label="Expand cell"
          aria-expanded={open}
          onClick={(event) => {
            event.stopPropagation();
            openNow();
          }}
          onFocus={openNow}
          // Prevent the cell's double-click-to-edit from firing on the toggle.
          onDoubleClick={(event) => event.stopPropagation()}
        >
          ⤢
        </button>
      )}
      {open && canExpand && (
        <Popover
          anchorRef={anchorRef}
          onClose={close}
          onMouseEnter={clearCloseTimer}
          onMouseLeave={scheduleClose}
          title={renderer.title?.({ ...base, mode: "expanded" })}
        >
          {renderer.renderExpanded?.({ ...base, mode: "expanded" })}
        </Popover>
      )}
    </span>
  );
}

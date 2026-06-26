/**
 * A calendar-based inline editor for date / datetime grid cells, injected into
 * the data-grid via its `renderEditor` prop. The grid package is deliberately
 * dependency-free, so the heavier UI (react-day-picker — the library shadcn/ui
 * wraps) lives here in the desktop app where Tailwind and the design tokens are.
 *
 * It only claims date and timestamp columns; time/number/text fall back to the
 * grid's built-in editor (`renderGridEditor` returns null for those).
 */
import { nativeControl, type CellEditorProps } from "@cellar/data-grid";
import { useLayoutEffect, useRef, useState } from "react";
import { DayPicker } from "react-day-picker";

const pad = (n: number) => String(n).padStart(2, "0");
// Format from local components, NOT toISOString(), so a date picked in the
// user's timezone doesn't shift a day when serialised.
export const ymd = (d: Date) =>
  `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;

/** Best-effort parse of a stored cell value into a Date for the calendar. */
export function parseDate(raw: string): Date | undefined {
  const ms = Date.parse(raw);
  return Number.isNaN(ms) ? undefined : new Date(ms);
}

/** Pull an HH:MM:SS time out of a stored timestamp string, defaulting to 00s. */
export function parseTime(raw: string): string {
  // No \b before the hour: in "...T05:00:31" there's no word boundary between
  // the separator letter and the digits, which would otherwise skip to "00:31".
  const m = /(\d{2}:\d{2}(?::\d{2})?)/.exec(raw);
  const hms = m?.[1];
  if (!hms) return "00:00:00";
  return hms.length === 5 ? `${hms}:00` : hms;
}

type Kind = "date" | "datetime" | "time";

/**
 * Returns the editor element when this column is a date/datetime/time, else
 * null so the grid uses its built-in editor. Wire as
 * `<DataGrid renderEditor={renderGridEditor} />`.
 */
export function renderGridEditor(props: CellEditorProps) {
  const initial = props.value == null ? "" : String(props.value);
  const control = nativeControl(props.col, initial);
  if (control?.type === "date") return <GridDateEditor {...props} kind="date" />;
  if (control?.type === "datetime-local")
    return <GridDateEditor {...props} kind="datetime" />;
  if (control?.type === "time") return <GridDateEditor {...props} kind="time" />;
  return null;
}

function GridDateEditor({
  value,
  onCommit,
  onCancel,
  kind,
}: CellEditorProps & { kind: Kind }) {
  const initial = value == null ? "" : String(value);
  const [date, setDate] = useState<Date | undefined>(() => parseDate(initial));
  const [time, setTime] = useState(() => parseTime(initial));
  const anchorRef = useRef<HTMLDivElement | null>(null);

  const commit = () => {
    if (kind === "time") {
      onCommit(time);
      return;
    }
    if (!date) {
      // Nothing chosen — leave the cell unchanged.
      onCancel();
      return;
    }
    onCommit(kind === "datetime" ? `${ymd(date)}T${time}` : ymd(date));
  };

  return (
    <>
      <div ref={anchorRef} className="grid-date-anchor mono">
        {initial || "—"}
      </div>
      <CalendarPopover anchorRef={anchorRef} onOutside={commit} onEscape={onCancel}>
        {kind !== "time" && (
          <DayPicker
            mode="single"
            selected={date}
            onSelect={setDate}
            defaultMonth={date}
            showOutsideDays
            autoFocus
          />
        )}
        {kind !== "date" && (
          <label className="grid-date-popover-time">
            Time
            <input
              type="time"
              step={1}
              value={time}
              autoFocus={kind === "time"}
              onChange={(e) => setTime(e.target.value || "00:00:00")}
            />
          </label>
        )}
        <div className="grid-date-popover-actions">
          <button type="button" className="gdp-cancel" onClick={onCancel}>
            Cancel
          </button>
          <button type="button" className="gdp-apply" onClick={commit}>
            Apply
          </button>
        </div>
      </CalendarPopover>
    </>
  );
}

/**
 * A fixed-position popover anchored under the editing cell. Escape cancels;
 * clicking away or scrolling commits (the usual "click-off applies" behaviour
 * of a picker). Uses `position: fixed` with a one-pass correction so it escapes
 * the grid cell's `overflow: hidden` and any transformed layout ancestor.
 */
function CalendarPopover({
  anchorRef,
  onOutside,
  onEscape,
  children,
}: {
  anchorRef: React.RefObject<HTMLElement | null>;
  onOutside: () => void;
  onEscape: () => void;
  children: React.ReactNode;
}) {
  const panelRef = useRef<HTMLDivElement | null>(null);
  const targetRef = useRef<{ top: number; left: number } | null>(null);
  const [pos, setPos] = useState<{ top: number; left: number } | null>(null);
  const [settled, setSettled] = useState(false);

  // Pass 1 — intended viewport position from the cell box.
  useLayoutEffect(() => {
    const cell =
      (anchorRef.current?.closest(".grid-cell") as HTMLElement | null) ??
      anchorRef.current;
    if (!cell) return;
    const r = cell.getBoundingClientRect();
    const h = panelRef.current?.offsetHeight ?? 0;
    const w = panelRef.current?.offsetWidth ?? 300;
    const left = Math.max(8, Math.min(r.left, window.innerWidth - w - 8));
    const below = r.bottom;
    const top =
      below + h <= window.innerHeight - 8 ? below : Math.max(8, r.top - h);
    targetRef.current = { top, left };
    setSettled(false);
    setPos({ top, left });
  }, [anchorRef]);

  // Pass 2 — fixed is resolved against the nearest transformed ancestor (not
  // the viewport here), so nudge by the delta between intended and actual.
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

  useLayoutEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onEscape();
      } else if (e.key === "Enter") {
        e.stopPropagation();
        onOutside();
      }
    };
    const onPointerDown = (e: MouseEvent) => {
      if (!panelRef.current?.contains(e.target as Node | null)) onOutside();
    };
    const onScroll = (e: Event) => {
      if (panelRef.current?.contains(e.target as Node | null)) return;
      onOutside();
    };
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("mousedown", onPointerDown, true);
    window.addEventListener("scroll", onScroll, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("mousedown", onPointerDown, true);
      window.removeEventListener("scroll", onScroll, true);
    };
  }, [onOutside, onEscape]);

  return (
    <div
      ref={panelRef}
      className="grid-date-popover"
      role="dialog"
      style={
        pos
          ? { top: pos.top, left: pos.left, visibility: settled ? "visible" : "hidden" }
          : { visibility: "hidden" }
      }
    >
      {children}
    </div>
  );
}

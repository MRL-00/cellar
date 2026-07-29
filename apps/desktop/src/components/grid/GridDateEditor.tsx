/**
 * A calendar-based picker for date / datetime / time values, injected into the
 * data-grid via `renderEditor` (cells) and `renderFilterValue` (where-clause).
 * The grid package is deliberately dependency-free, so the heavier UI
 * (react-day-picker — the library shadcn/ui wraps) lives here in the desktop
 * app where Tailwind and the design tokens are.
 *
 * It only claims date and timestamp columns; time/number/text fall back to the
 * grid's built-in controls (`renderGridEditor` / `renderFilterValue` return
 * null for those).
 */
import {
  nativeControl,
  type CellEditorProps,
  type FilterValueInputProps,
  type GridColumn,
} from "@cellar/data-grid";
import {
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
  type Ref,
} from "react";
import { createPortal } from "react-dom";
import { DayPicker } from "react-day-picker";

const VIEWPORT_MARGIN = 8;

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

function kindForColumn(column: GridColumn): Kind | null {
  // Probe with an empty value so a stale/unparseable draft still gets the
  // calendar — same idea as the filter bar's native fallback.
  const control = nativeControl(column, "");
  if (control?.type === "date") return "date";
  if (control?.type === "datetime-local") return "datetime";
  if (control?.type === "time") return "time";
  return null;
}

function formatCommitted(kind: Kind, date: Date | undefined, time: string): string | null {
  if (kind === "time") return time;
  if (!date) return null;
  return kind === "datetime" ? `${ymd(date)}T${time}` : ymd(date);
}

/**
 * Returns the editor element when this column is a date/datetime/time, else
 * null so the grid uses its built-in editor. Wire as
 * `<DataGrid renderEditor={renderGridEditor} />`.
 */
export function renderGridEditor(props: CellEditorProps) {
  const kind = kindForColumn(props.col);
  if (!kind) return null;
  return <GridDateEditor {...props} kind={kind} />;
}

/**
 * Same calendar for the filter-bar value slot. Wire as
 * `<DataGrid renderFilterValue={renderFilterValue} />`.
 */
export function renderFilterValue(props: FilterValueInputProps): ReactNode {
  const kind = kindForColumn(props.column);
  if (!kind) return null;
  // Key on column so switching date fields remounts a fresh picker state.
  return <FilterDateValueInput key={props.column.key} {...props} kind={kind} />;
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
    const next = formatCommitted(kind, date, time);
    if (next == null) {
      // Nothing chosen — leave the cell unchanged.
      onCancel();
      return;
    }
    onCommit(next);
  };

  return (
    <>
      <div ref={anchorRef} className="cell-edit-input grid-date-anchor mono">
        {initial || "—"}
      </div>
      <CalendarPopover anchorRef={anchorRef} onOutside={commit} onEscape={onCancel}>
        <DatePickerBody
          kind={kind}
          date={date}
          time={time}
          onDateChange={setDate}
          onTimeChange={setTime}
          onCommit={commit}
          onCancel={onCancel}
        />
      </CalendarPopover>
    </>
  );
}

function FilterDateValueInput({
  value,
  onChange,
  anchorRef: externalAnchorRef,
  kind,
}: FilterValueInputProps & { kind: Kind }) {
  const [open, setOpen] = useState(true);
  const [date, setDate] = useState<Date | undefined>(() => parseDate(value));
  const [time, setTime] = useState(() => parseTime(value));
  const localAnchorRef = useRef<HTMLButtonElement | null>(null);

  const setAnchorRefs = (node: HTMLButtonElement | null) => {
    localAnchorRef.current = node;
    assignRef(externalAnchorRef, node);
  };

  const openPicker = () => {
    setDate(parseDate(value));
    setTime(parseTime(value));
    setOpen(true);
  };

  const commit = () => {
    const next = formatCommitted(kind, date, time);
    if (next != null) onChange(next);
    setOpen(false);
  };

  const cancel = () => setOpen(false);

  const placeholder =
    kind === "time" ? "time" : kind === "datetime" ? "date & time" : "date";

  return (
    <>
      <button
        ref={setAnchorRefs}
        type="button"
        className={`grid-filter-input mono grid-filter-date-trigger grid-filter-input-${
          kind === "datetime" ? "datetime-local" : kind
        }`}
        onClick={openPicker}
        aria-label="Filter value"
        aria-haspopup="dialog"
        aria-expanded={open}
      >
        <span className={value ? undefined : "grid-filter-date-placeholder"}>
          {value || placeholder}
        </span>
      </button>
      {open && (
        <CalendarPopover
          anchorRef={localAnchorRef}
          onOutside={commit}
          onEscape={cancel}
        >
          <DatePickerBody
            kind={kind}
            date={date}
            time={time}
            onDateChange={setDate}
            onTimeChange={setTime}
            onCommit={commit}
            onCancel={cancel}
          />
        </CalendarPopover>
      )}
    </>
  );
}

function DatePickerBody({
  kind,
  date,
  time,
  onDateChange,
  onTimeChange,
  onCommit,
  onCancel,
}: {
  kind: Kind;
  date: Date | undefined;
  time: string;
  onDateChange: (next: Date | undefined) => void;
  onTimeChange: (next: string) => void;
  onCommit: () => void;
  onCancel: () => void;
}) {
  return (
    <>
      {kind !== "time" && (
        <DayPicker
          mode="single"
          selected={date}
          onSelect={onDateChange}
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
            onChange={(e) => onTimeChange(e.target.value || "00:00:00")}
          />
        </label>
      )}
      <div className="grid-date-popover-actions">
        <button type="button" className="gdp-cancel" onClick={onCancel}>
          Cancel
        </button>
        <button type="button" className="gdp-apply" onClick={onCommit}>
          Apply
        </button>
      </div>
    </>
  );
}

function assignRef<T>(ref: Ref<T> | undefined, value: T) {
  if (!ref) return;
  if (typeof ref === "function") {
    ref(value);
    return;
  }
  (ref as { current: T }).current = value;
}

/**
 * Collision bounds for the date popover. Prefer the grid's scroll viewport so
 * we flip above the cell when the pending/pagination bars (or an open bottom
 * panel) would otherwise clip a downward open. Fall back to the window.
 */
function collisionBounds(anchor: HTMLElement): {
  top: number;
  bottom: number;
  left: number;
  right: number;
} {
  const scroll = anchor.closest(".grid-scroll") as HTMLElement | null;
  if (scroll) {
    const r = scroll.getBoundingClientRect();
    return {
      top: r.top + VIEWPORT_MARGIN,
      bottom: r.bottom - VIEWPORT_MARGIN,
      left: VIEWPORT_MARGIN,
      right: window.innerWidth - VIEWPORT_MARGIN,
    };
  }
  return {
    top: VIEWPORT_MARGIN,
    bottom: window.innerHeight - VIEWPORT_MARGIN,
    left: VIEWPORT_MARGIN,
    right: window.innerWidth - VIEWPORT_MARGIN,
  };
}

/** Pure placement math — exported for unit tests. */
export function placeCalendarPopover(
  cell: { top: number; left: number; bottom: number },
  panel: { width: number; height: number },
  bounds: { top: number; bottom: number; left: number; right: number },
): { top: number; left: number } {
  const left = Math.max(
    bounds.left,
    Math.min(cell.left, bounds.right - panel.width),
  );
  // Prefer below; flip above when a downward open would hit the scroll
  // viewport bottom (pending bar / pagination / bottom panel edge).
  const fitsBelow = cell.bottom + panel.height <= bounds.bottom;
  const top = fitsBelow
    ? cell.bottom
    : Math.max(bounds.top, cell.top - panel.height);
  return { top, left };
}

/**
 * A fixed-position popover anchored under the editing cell. Escape cancels;
 * clicking away or scrolling commits (the usual "click-off applies" behaviour
 * of a picker). Portaled to `document.body` so `.grid-scroll`'s overflow and
 * the pending bar can't clip it; flips above the cell when there isn't room
 * below inside the grid viewport.
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
  const [pos, setPos] = useState<{ top: number; left: number } | null>(null);
  const [settled, setSettled] = useState(false);

  useLayoutEffect(() => {
    const cell =
      (anchorRef.current?.closest(".grid-cell") as HTMLElement | null) ??
      anchorRef.current;
    const panel = panelRef.current;
    if (!cell || !panel) return;

    const place = (markSettled: boolean) => {
      const r = cell.getBoundingClientRect();
      setPos(
        placeCalendarPopover(
          { top: r.top, left: r.left, bottom: r.bottom },
          { width: panel.offsetWidth || 300, height: panel.offsetHeight },
          collisionBounds(cell),
        ),
      );
      if (markSettled) setSettled(true);
    };

    // First pass positions with whatever height is available; a second pass on
    // the next frame re-flips once DayPicker's month grid has real dimensions.
    place(false);
    const raf = requestAnimationFrame(() => place(true));
    return () => cancelAnimationFrame(raf);
  }, [anchorRef]);

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

  return createPortal(
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
    </div>,
    document.body,
  );
}

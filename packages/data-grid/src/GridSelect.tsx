import {
  forwardRef,
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import { createPortal } from "react-dom";
import { GridIcon } from "./icons";

export type GridSelectOption = { value: string; label: string };

type GridSelectProps = {
  value: string;
  options: readonly GridSelectOption[];
  onChange: (value: string) => void;
  className?: string;
  "aria-label": string;
  title?: string;
};

/**
 * Styled replacement for a native `<select>` so the dropdown matches the app
 * theme instead of the OS menu. Renders the menu in a portal with fixed
 * positioning because the filter bar clips overflow.
 */
export const GridSelect = forwardRef<HTMLButtonElement, GridSelectProps>(
  function GridSelect({ value, options, onChange, className, title, ...aria }, ref) {
    const buttonRef = useRef<HTMLButtonElement | null>(null);
    useImperativeHandle(ref, () => buttonRef.current!, []);
    const menuRef = useRef<HTMLDivElement | null>(null);
    const [open, setOpen] = useState(false);
    const [active, setActive] = useState(0);
    const [pos, setPos] = useState({ left: 0, top: 0, minWidth: 0 });

    const openMenu = () => {
      const rect = buttonRef.current?.getBoundingClientRect();
      if (!rect) return;
      // The app scales the UI with CSS `zoom` on <body> (font-size setting).
      // The portaled menu lives inside that zoomed body, so its fixed coords
      // are in zoomed space while getBoundingClientRect() is in viewport
      // pixels — divide by the scale to line them up. Read the app's
      // --ui-scale variable (set alongside the zoom) rather than the computed
      // `zoom` value, which some webview engines don't report.
      const zoom =
        Number(
          getComputedStyle(document.documentElement).getPropertyValue("--ui-scale"),
        ) || 1;
      setPos({
        left: rect.left / zoom,
        top: (rect.bottom + 2) / zoom,
        minWidth: rect.width / zoom,
      });
      const current = options.findIndex((option) => option.value === value);
      setActive(current >= 0 ? current : 0);
      setOpen(true);
    };

    useEffect(() => {
      if (!open) return;
      const onPointerDown = (e: MouseEvent) => {
        const target = e.target as Node;
        if (menuRef.current?.contains(target)) return;
        if (buttonRef.current?.contains(target)) return;
        setOpen(false);
      };
      const close = () => setOpen(false);
      window.addEventListener("mousedown", onPointerDown);
      window.addEventListener("scroll", close, true);
      window.addEventListener("resize", close);
      return () => {
        window.removeEventListener("mousedown", onPointerDown);
        window.removeEventListener("scroll", close, true);
        window.removeEventListener("resize", close);
      };
    }, [open]);

    const select = (option: GridSelectOption) => {
      setOpen(false);
      if (option.value !== value) onChange(option.value);
      buttonRef.current?.focus();
    };

    const onKeyDown = (e: KeyboardEvent) => {
      if (!open) {
        if (e.key === "ArrowDown" || e.key === "ArrowUp" || e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          e.stopPropagation();
          openMenu();
        }
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        setOpen(false);
        return;
      }
      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        const delta = e.key === "ArrowDown" ? 1 : -1;
        setActive((prev) => (prev + delta + options.length) % options.length);
        return;
      }
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        e.stopPropagation();
        const option = options[active];
        if (option) select(option);
      }
    };

    const selected = options.find((option) => option.value === value);

    return (
      <>
        <button
          ref={buttonRef}
          type="button"
          className={`grid-select ${className ?? ""}`}
          title={title}
          aria-haspopup="listbox"
          aria-expanded={open}
          {...aria}
          onClick={() => (open ? setOpen(false) : openMenu())}
          onKeyDown={onKeyDown}
        >
          <span className="grid-select-label">{selected?.label ?? value}</span>
          <GridIcon.chevronDown size={8} />
        </button>
        {open &&
          createPortal(
            <div
              ref={menuRef}
              className="grid-select-menu"
              role="listbox"
              style={{ left: pos.left, top: pos.top, minWidth: pos.minWidth }}
            >
              {options.map((option, index) => (
                <div
                  key={option.value}
                  role="option"
                  aria-selected={option.value === value}
                  className={`grid-select-option${index === active ? " is-active" : ""}${
                    option.value === value ? " is-selected" : ""
                  }`}
                  onMouseEnter={() => setActive(index)}
                  onClick={() => select(option)}
                >
                  {option.label}
                </div>
              ))}
            </div>,
            document.body,
          )}
      </>
    );
  },
);

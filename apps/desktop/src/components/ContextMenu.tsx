import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";

export interface MenuItem {
  label: string;
  icon?: ReactNode;
  onClick: () => void;
  danger?: boolean;
  disabled?: boolean;
}

export interface ContextMenuState {
  x: number;
  y: number;
  items: MenuItem[];
}

export interface ContextMenuPositionInput {
  x: number;
  y: number;
  viewportWidth: number;
  viewportHeight: number;
  menuWidth: number;
  menuHeight: number;
  scale: number;
  margin?: number;
}

export function contextMenuPosition({
  x,
  y,
  viewportWidth,
  viewportHeight,
  menuWidth,
  menuHeight,
  scale,
  margin = 8,
}: ContextMenuPositionInput): { left: number; top: number } {
  const safeScale = Number.isFinite(scale) && scale > 0 ? scale : 1;
  const maxVisualLeft = Math.max(margin, viewportWidth - menuWidth - margin);
  const maxVisualTop = Math.max(margin, viewportHeight - menuHeight - margin);
  const visualLeft = Math.min(Math.max(x, margin), maxVisualLeft);
  const visualTop = Math.min(Math.max(y, margin), maxVisualTop);

  return {
    left: visualLeft / safeScale,
    top: visualTop / safeScale,
  };
}

function uiScale(): number {
  const scale = Number.parseFloat(
    getComputedStyle(document.documentElement).getPropertyValue("--ui-scale"),
  );
  return Number.isFinite(scale) && scale > 0 ? scale : 1;
}

/**
 * Lightweight right-click menu. Renders at a fixed viewport position, closes
 * on outside click, Escape, scroll, or window blur. Items are flat (no
 * submenus yet) which covers the connection actions in SPEC §6.1.
 */
export function ContextMenu({
  state,
  onClose,
}: {
  state: ContextMenuState | null;
  onClose: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState<{ left: number; top: number } | null>(
    null,
  );

  useEffect(() => {
    if (!state) return;
    const onDocClick = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("mousedown", onDocClick);
    window.addEventListener("keydown", onKey);
    window.addEventListener("scroll", onClose, true);
    window.addEventListener("blur", onClose);
    window.addEventListener("resize", onClose);
    return () => {
      window.removeEventListener("mousedown", onDocClick);
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("scroll", onClose, true);
      window.removeEventListener("blur", onClose);
      window.removeEventListener("resize", onClose);
    };
  }, [state, onClose]);

  useLayoutEffect(() => {
    if (!state || !ref.current) {
      setPosition(null);
      return;
    }

    const scale = uiScale();
    const rect = ref.current.getBoundingClientRect();
    setPosition(
      contextMenuPosition({
        x: state.x,
        y: state.y,
        viewportWidth: window.innerWidth,
        viewportHeight: window.innerHeight,
        menuWidth: rect.width,
        menuHeight: rect.height,
        scale,
      }),
    );
  }, [state]);

  if (!state) return null;

  const scale = uiScale();
  const fallback = {
    left: state.x / scale,
    top: state.y / scale,
  };
  const { left, top } = position ?? fallback;

  return (
    <div
      ref={ref}
      className="fixed z-[1000] min-w-[176px] rounded-[6px] border border-border-strong bg-bg-2 py-1 shadow-[0_8px_24px_rgba(0,0,0,0.4)]"
      style={{ left, top, opacity: position ? undefined : 0 }}
      role="menu"
    >
      {state.items.map((item, i) => (
        <button
          key={i}
          role="menuitem"
          disabled={item.disabled}
          onClick={() => {
            if (item.disabled) return;
            onClose();
            item.onClick();
          }}
          className={
            "flex w-full items-center gap-2 px-2.5 py-[5px] text-left text-sm transition-colors disabled:opacity-40 disabled:cursor-not-allowed " +
            (item.danger
              ? "text-warn hover:bg-warn/10"
              : "text-fg-1 hover:bg-accent-soft hover:text-accent")
          }
        >
          {item.icon && (
            <span className="inline-flex h-[14px] w-[14px] items-center justify-center text-fg-3">
              {item.icon}
            </span>
          )}
          <span>{item.label}</span>
        </button>
      ))}
    </div>
  );
}

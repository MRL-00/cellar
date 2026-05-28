import { useEffect, useRef, type ReactNode } from "react";

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
    window.addEventListener("blur", onClose);
    window.addEventListener("resize", onClose);
    return () => {
      window.removeEventListener("mousedown", onDocClick);
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("blur", onClose);
      window.removeEventListener("resize", onClose);
    };
  }, [state, onClose]);

  if (!state) return null;

  // Keep the menu on-screen if it would overflow the right/bottom edge.
  const maxX = window.innerWidth - 200;
  const maxY = window.innerHeight - state.items.length * 28 - 12;
  const left = Math.min(state.x, Math.max(8, maxX));
  const top = Math.min(state.y, Math.max(8, maxY));

  return (
    <div
      ref={ref}
      className="fixed z-[1000] min-w-[176px] rounded-[6px] border border-border-strong bg-bg-2 py-1 shadow-[0_8px_24px_rgba(0,0,0,0.4)]"
      style={{ left, top }}
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
            "flex w-full items-center gap-2 px-2.5 py-[5px] text-left text-[11.5px] transition-colors disabled:opacity-40 disabled:cursor-not-allowed " +
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

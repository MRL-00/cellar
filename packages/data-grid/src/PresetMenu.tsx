import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { GridIcon } from "./icons";

export type PresetMenuProps = {
  names: readonly string[];
  /** Preset the current toolbar state matches, if any — gets the checkmark. */
  activeName: string | null;
  onApply: (name: string) => void;
  onDelete: (name: string) => void;
  /** Reset the toolbar (unselect the active preset). */
  onClear: () => void;
  /** Open the host's "name this preset" input. */
  onSaveRequest: () => void;
};

/**
 * Dropdown menu for saved filter presets: preset items (check on the active
 * one, hover “×” to delete), then save/clear actions. Portal + fixed
 * positioning for the same reason as GridSelect — the filter bar clips
 * overflow.
 */
export function PresetMenu({
  names,
  activeName,
  onApply,
  onDelete,
  onClear,
  onSaveRequest,
}: PresetMenuProps) {
  const buttonRef = useRef<HTMLButtonElement | null>(null);
  const menuRef = useRef<HTMLDivElement | null>(null);
  const [open, setOpen] = useState(false);
  const [pos, setPos] = useState({ left: 0, top: 0, minWidth: 0 });

  const openMenu = () => {
    const rect = buttonRef.current?.getBoundingClientRect();
    if (!rect) return;
    // See GridSelect: the app zooms <body>, so viewport coords must be
    // divided by --ui-scale to land the fixed-position menu correctly.
    const zoom =
      Number(
        getComputedStyle(document.documentElement).getPropertyValue(
          "--ui-scale",
        ),
      ) || 1;
    setPos({
      left: rect.left / zoom,
      top: (rect.bottom + 4) / zoom,
      minWidth: rect.width / zoom,
    });
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
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    const close = () => setOpen(false);
    window.addEventListener("mousedown", onPointerDown);
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("scroll", close, true);
    window.addEventListener("resize", close);
    return () => {
      window.removeEventListener("mousedown", onPointerDown);
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("scroll", close, true);
      window.removeEventListener("resize", close);
    };
  }, [open]);

  const act = (action: () => void) => {
    setOpen(false);
    action();
  };

  return (
    <>
      <button
        ref={buttonRef}
        type="button"
        className={"grid-preset-trigger" + (activeName ? " is-active" : "")}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label="Saved filter presets"
        title="Saved filter presets"
        onClick={() => (open ? setOpen(false) : openMenu())}
      >
        <GridIcon.bookmark size={11} />
        <span className="grid-preset-trigger-label">
          {activeName ?? "Presets"}
        </span>
        <GridIcon.chevronDown size={8} />
      </button>
      {open &&
        createPortal(
          <div
            ref={menuRef}
            className="grid-menu"
            role="menu"
            style={{ left: pos.left, top: pos.top, minWidth: pos.minWidth }}
          >
            {names.length > 0 && (
              <>
                {names.map((name) => {
                  const isActive = name === activeName;
                  return (
                    <div
                      key={name}
                      className={
                        "grid-menu-item" + (isActive ? " is-selected" : "")
                      }
                    >
                      <button
                        type="button"
                        role="menuitemradio"
                        aria-checked={isActive}
                        className="grid-menu-item-main"
                        // Clicking the active preset again unselects it.
                        onClick={() =>
                          act(isActive ? onClear : () => onApply(name))
                        }
                      >
                        <span className="grid-menu-check">
                          {isActive && <GridIcon.check size={10} />}
                        </span>
                        <span className="grid-menu-item-label">{name}</span>
                      </button>
                      {/* The active preset can't be deleted — clear it first,
                          otherwise the toolbar is left on a view that no
                          longer exists anywhere. */}
                      {!isActive && (
                        <button
                          type="button"
                          className="grid-filter-remove grid-menu-delete"
                          onClick={() => onDelete(name)}
                          aria-label={`Delete preset ${name}`}
                          title="Delete preset"
                        >
                          <GridIcon.close size={9} />
                        </button>
                      )}
                    </div>
                  );
                })}
                <div className="grid-menu-separator" role="separator" />
              </>
            )}
            <div className="grid-menu-item">
              <button
                type="button"
                role="menuitem"
                className="grid-menu-item-main"
                onClick={() => act(onSaveRequest)}
              >
                <span className="grid-menu-check">
                  <GridIcon.plus size={10} />
                </span>
                <span className="grid-menu-item-label">
                  Save current as preset…
                </span>
              </button>
            </div>
            {activeName && (
              <div className="grid-menu-item">
                <button
                  type="button"
                  role="menuitem"
                  className="grid-menu-item-main"
                  onClick={() => act(onClear)}
                >
                  <span className="grid-menu-check">
                    <GridIcon.close size={9} />
                  </span>
                  <span className="grid-menu-item-label">Clear preset</span>
                </button>
              </div>
            )}
          </div>,
          document.body,
        )}
    </>
  );
}

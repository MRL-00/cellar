import { useEffect, type ReactNode } from "react";

export function Modal({
  children,
  onClose,
  width = 720,
  height,
}: {
  children: ReactNode;
  onClose: () => void;
  width?: number;
  height?: number;
}) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="modal-scrim" onClick={onClose}>
      <div
        className="modal-shell"
        style={{ width, height }}
        onClick={(e) => e.stopPropagation()}
      >
        {children}
      </div>
    </div>
  );
}

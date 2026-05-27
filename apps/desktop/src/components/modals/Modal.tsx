import { useEffect, type ReactNode } from "react";

export function Modal({
  children,
  onClose,
  width = 720,
  height,
  className = "",
}: {
  children: ReactNode;
  onClose: () => void;
  width?: number;
  height?: number;
  className?: string;
}) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div
      className="fixed inset-0 z-[100] flex items-start justify-center bg-bg-overlay pt-[8vh] backdrop-blur-[4px] animate-scrim-in"
      onClick={onClose}
    >
      <div
        className={
          "flex max-h-[84vh] flex-col overflow-hidden rounded-lg border border-border-default bg-bg-1 shadow-lg animate-modal-in " +
          className
        }
        style={{ width, height }}
        onClick={(e) => e.stopPropagation()}
      >
        {children}
      </div>
    </div>
  );
}

/** Small UI + IO helpers shared across the built-in renderers. */
import { useState, type ReactNode } from "react";
import type { SaveBlob } from "./types";

/**
 * Default `SaveBlob`: a user-initiated browser download. This adds no Tauri
 * capability and makes no network call — the bytes are already in memory. Hosts
 * can override with a native save dialog via the grid's `saveBlob` prop.
 */
export const defaultSaveBlob: SaveBlob = (data, filename, mime) => {
  if (typeof document === "undefined" || typeof URL === "undefined") return;
  const blob = new Blob([data as BlobPart], { type: mime });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
};

function writeClipboard(text: string): void {
  if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
    void navigator.clipboard.writeText(text);
  }
}

/** A compact copy-to-clipboard button with a transient "Copied" state. */
export function CopyButton({
  value,
  label = "Copy",
  className,
}: {
  value: string;
  label?: string;
  className?: string;
}) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      className={"cell-rich-action" + (className ? " " + className : "")}
      onClick={() => {
        writeClipboard(value);
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1200);
      }}
      title={`${label} to clipboard`}
    >
      {copied ? "Copied" : label}
    </button>
  );
}

/** A non-interactive pill, e.g. the byte-size or type badge. */
export function Badge({ children }: { children: ReactNode }) {
  return <span className="cell-rich-badge">{children}</span>;
}

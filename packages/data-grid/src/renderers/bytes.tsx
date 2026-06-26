/**
 * bytea / blob renderer: inline hex preview + size badge; expanded popover with
 * a hex dump, an image preview when the bytes look like an image (magic-byte
 * sniff), and save-to-file.
 *
 * The grid carries bytea as a full `\x…` hex string, so the renderer can
 * reconstruct the exact bytes for sniffing, preview, and save without any
 * network call. The hex *dump* is display-capped to keep the popover light.
 */
import { useEffect, useMemo, useState } from "react";
import { Badge } from "./shared";
import { isByteaType, isGeometryType } from "./typeMatch";
import type { CellRenderer } from "./types";

/** Decode a `\x`-prefixed hex string into bytes, tolerating a truncation tail. */
export function parseHexBytes(text: string): Uint8Array {
  const body = text.startsWith("\\x") || text.startsWith("\\X") ? text.slice(2) : text;
  const match = /^[0-9a-fA-F]*/.exec(body);
  let hex = match ? match[0] : "";
  if (hex.length % 2 !== 0) hex = hex.slice(0, -1);
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i++) {
    out[i] = Number.parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return out;
}

export type ByteaInfo = {
  /** The bytes we were able to decode (the full blob unless truncated). */
  bytes: Uint8Array;
  /** True blob size, recovered from a `… (N bytes)` marker when present. */
  total: number;
  /** The grid only carried a prefix, so save/image previews are incomplete. */
  truncated: boolean;
};

/**
 * Decode a bytea cell and recover the true size. The app maps bytea as full hex
 * so `truncated` is normally false, but a legacy/foreign `\x… (N bytes)` cell
 * only carries a prefix — we surface that rather than silently acting on a
 * partial buffer (which would mis-size, mis-sniff, and save incomplete data).
 */
export function byteaInfo(text: string): ByteaInfo {
  const bytes = parseHexBytes(text);
  const marker = /…\s*\((\d+)\s*bytes?\)/i.exec(text);
  const total = marker ? Number(marker[1]) : bytes.length;
  return { bytes, total, truncated: total > bytes.length };
}

/** Identify common image formats from their leading magic bytes. */
export function sniffImageMime(bytes: Uint8Array): string | null {
  const at = (i: number) => bytes[i] ?? -1;
  if (
    bytes.length >= 8 &&
    at(0) === 0x89 && at(1) === 0x50 && at(2) === 0x4e && at(3) === 0x47 &&
    at(4) === 0x0d && at(5) === 0x0a && at(6) === 0x1a && at(7) === 0x0a
  ) {
    return "image/png";
  }
  if (bytes.length >= 3 && at(0) === 0xff && at(1) === 0xd8 && at(2) === 0xff) {
    return "image/jpeg";
  }
  if (
    bytes.length >= 6 &&
    at(0) === 0x47 && at(1) === 0x49 && at(2) === 0x46 && at(3) === 0x38 &&
    (at(4) === 0x37 || at(4) === 0x39) && at(5) === 0x61
  ) {
    return "image/gif";
  }
  if (
    bytes.length >= 12 &&
    at(0) === 0x52 && at(1) === 0x49 && at(2) === 0x46 && at(3) === 0x46 &&
    at(8) === 0x57 && at(9) === 0x45 && at(10) === 0x42 && at(11) === 0x50
  ) {
    return "image/webp";
  }
  if (bytes.length >= 2 && at(0) === 0x42 && at(1) === 0x4d) return "image/bmp";
  return null;
}

const EXTENSION: Record<string, string> = {
  "image/png": "png",
  "image/jpeg": "jpg",
  "image/gif": "gif",
  "image/webp": "webp",
  "image/bmp": "bmp",
};

/** Human-readable byte count, e.g. `937 B`, `1.2 KB`, `3.4 MB`. */
export function formatByteSize(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

const HEX_DUMP_BYTES = 256;
const HEX_DUMP_COLS = 16;

export type HexDumpRow = { offset: string; hex: string; ascii: string };

/** Build classic `offset  hex…  ascii` dump rows for the first N bytes. */
export function hexDump(bytes: Uint8Array, limit = HEX_DUMP_BYTES): HexDumpRow[] {
  const rows: HexDumpRow[] = [];
  const end = Math.min(bytes.length, limit);
  for (let i = 0; i < end; i += HEX_DUMP_COLS) {
    const slice = bytes.subarray(i, Math.min(i + HEX_DUMP_COLS, end));
    let hex = "";
    let ascii = "";
    for (let j = 0; j < HEX_DUMP_COLS; j++) {
      const byte = slice[j];
      if (byte === undefined) {
        hex += "   ";
        continue;
      }
      hex += byte.toString(16).padStart(2, "0") + " ";
      ascii += byte >= 0x20 && byte < 0x7f ? String.fromCharCode(byte) : ".";
    }
    rows.push({ offset: i.toString(16).padStart(8, "0"), hex: hex.trimEnd(), ascii });
  }
  return rows;
}

function ImagePreview({ bytes, mime }: { bytes: Uint8Array; mime: string }) {
  const [url, setUrl] = useState<string | null>(null);
  useEffect(() => {
    if (typeof URL === "undefined") return;
    const objectUrl = URL.createObjectURL(new Blob([bytes as BlobPart], { type: mime }));
    setUrl(objectUrl);
    return () => URL.revokeObjectURL(objectUrl);
  }, [bytes, mime]);
  if (!url) return null;
  return (
    <div className="cell-bytes-image">
      <img src={url} alt="blob preview" />
    </div>
  );
}

function BytesExpanded({
  text,
  saveBlob,
  columnName,
}: {
  text: string;
  saveBlob: (data: Uint8Array, filename: string, mime: string) => void;
  columnName: string;
}) {
  // Parse once per distinct value. `renderExpanded` runs on every grid render
  // while the popover is open; without this the bytes ref would change each
  // time and ImagePreview would revoke/recreate its object URL (flicker).
  const info = useMemo(() => byteaInfo(text), [text]);
  const bytes = info.bytes;
  // Only the complete blob can be sniffed, previewed, or saved truthfully.
  const mime = useMemo(() => (info.truncated ? null : sniffImageMime(bytes)), [bytes, info.truncated]);
  const [showImage, setShowImage] = useState(false);
  const rows = useMemo(() => hexDump(bytes), [bytes]);
  const dumpClipped = bytes.length > HEX_DUMP_BYTES;
  const extension = mime ? EXTENSION[mime] ?? "bin" : "bin";

  return (
    <div className="cell-bytes-expanded">
      <div className="cell-rich-toolbar">
        <Badge>{formatByteSize(info.total)}{info.truncated ? " (preview)" : ""}</Badge>
        {mime && <Badge>{mime}</Badge>}
        <button
          type="button"
          className="cell-rich-action"
          onClick={() => saveBlob(bytes, `${columnName}.${extension}`, mime ?? "application/octet-stream")}
          disabled={info.truncated}
          title={
            info.truncated
              ? "Only a preview of this blob is loaded; full bytes are not available to save"
              : "Save bytes to a file"
          }
        >
          Save to file
        </button>
        {mime && (
          <button
            type="button"
            className="cell-rich-action"
            onClick={() => setShowImage((value) => !value)}
            aria-pressed={showImage}
          >
            {showImage ? "Hide image" : "View as image"}
          </button>
        )}
      </div>
      {showImage && mime && <ImagePreview bytes={bytes} mime={mime} />}
      <pre className="cell-bytes-dump">
        {rows.map((row) => (
          <div key={row.offset} className="cell-bytes-dump-row">
            <span className="cell-bytes-offset">{row.offset}</span>
            <span className="cell-bytes-hex">{row.hex}</span>
            <span className="cell-bytes-ascii">{row.ascii}</span>
          </div>
        ))}
        {(dumpClipped || info.truncated) && (
          <div className="cell-bytes-dump-more">
            {info.truncated
              ? `preview only — ${formatByteSize(info.total)} total`
              : `… ${formatByteSize(bytes.length - HEX_DUMP_BYTES)} more`}
          </div>
        )}
      </pre>
    </div>
  );
}

const INLINE_HEX_BYTES = 8;

export const byteaRenderer: CellRenderer = {
  id: "builtin:bytea",
  priority: 10,
  // Claim known binary column types, plus geometry columns whose value arrived
  // as raw bytes (`\x…`) — those engines (e.g. MySQL) decode GEOMETRY to bytes,
  // and the bytea hex/image/save view is the right home for them.
  appliesTo: (column, value) =>
    typeof value === "string" &&
    (isByteaType(column.type) ||
      (isGeometryType(column.type) && value.startsWith("\\x"))),
  renderInline: ({ text }) => {
    const { bytes, total, truncated } = byteaInfo(text);
    const head = Array.from(bytes.subarray(0, INLINE_HEX_BYTES))
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("");
    const mime = truncated ? null : sniffImageMime(bytes);
    return (
      <span className="cell-bytes-inline">
        {mime && <span className="cell-bytes-imgdot" title={mime} />}
        <span className="cell-bytes-hexpeek">
          \x{head}
          {bytes.length > INLINE_HEX_BYTES ? "…" : ""}
        </span>
        <span className="cell-bytes-size">{formatByteSize(total)}</span>
      </span>
    );
  },
  renderExpanded: ({ text, column, saveBlob }) => (
    <BytesExpanded text={text} saveBlob={saveBlob} columnName={column.name} />
  ),
  title: ({ column }) => `${column.name} · ${column.type}`,
};

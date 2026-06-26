/**
 * JSON / JSONB renderer: compact inline preview plus a collapsible,
 * syntax-highlighted tree in the popover with a copy-raw button.
 */
import { useState } from "react";
import { CopyButton } from "./shared";
import { isJsonType } from "./typeMatch";
import type { CellRenderer } from "./types";

export type JsonParse =
  | { ok: true; value: unknown }
  | { ok: false };

/** Parse the grid's stringified JSON back into a value, safely. */
export function safeParseJson(text: string): JsonParse {
  try {
    return { ok: true, value: JSON.parse(text) as unknown };
  } catch {
    return { ok: false };
  }
}

type JsonKind = "object" | "array" | "string" | "number" | "boolean" | "null";

export function jsonKind(value: unknown): JsonKind {
  if (value === null) return "null";
  if (Array.isArray(value)) return "array";
  const t = typeof value;
  if (t === "object") return "object";
  if (t === "number") return "number";
  if (t === "boolean") return "boolean";
  return "string";
}

/** One-line summary used for the collapsed/inline view. */
export function jsonSummary(value: unknown): string {
  switch (jsonKind(value)) {
    case "array": {
      const length = (value as unknown[]).length;
      return length === 0 ? "[ ]" : `[ ${length} ${length === 1 ? "item" : "items"} ]`;
    }
    case "object": {
      const keys = Object.keys(value as Record<string, unknown>).length;
      return keys === 0 ? "{ }" : `{ ${keys} ${keys === 1 ? "key" : "keys"} }`;
    }
    case "string":
      return JSON.stringify(value);
    case "null":
      return "null";
    default:
      return String(value);
  }
}

function Scalar({ value }: { value: unknown }) {
  const kind = jsonKind(value);
  return (
    <span className={`cell-json-${kind}`}>
      {kind === "string" ? JSON.stringify(value) : String(value)}
    </span>
  );
}

const DEFAULT_OPEN_DEPTH = 1;

function JsonNode({
  name,
  value,
  depth,
}: {
  name?: string;
  value: unknown;
  depth: number;
}) {
  const kind = jsonKind(value);
  const branch = kind === "object" || kind === "array";
  const [open, setOpen] = useState(depth < DEFAULT_OPEN_DEPTH);

  const keyLabel =
    name === undefined ? null : (
      <span className="cell-json-key">{kind === "array" || isIndex(name) ? name : JSON.stringify(name)}</span>
    );

  if (!branch) {
    return (
      <div className="cell-json-row" style={{ paddingLeft: depth * 12 }}>
        {keyLabel}
        {keyLabel && <span className="cell-json-punct">: </span>}
        <Scalar value={value} />
      </div>
    );
  }

  const entries =
    kind === "array"
      ? (value as unknown[]).map((item, index) => [String(index), item] as const)
      : Object.entries(value as Record<string, unknown>);
  const open_ = kind === "array" ? "[" : "{";
  const close = kind === "array" ? "]" : "}";

  return (
    <div className="cell-json-branch">
      <button
        type="button"
        className="cell-json-row cell-json-toggle"
        style={{ paddingLeft: depth * 12 }}
        onClick={() => setOpen((value_) => !value_)}
        aria-expanded={open}
      >
        <span className="cell-json-caret">{open ? "▾" : "▸"}</span>
        {keyLabel}
        {keyLabel && <span className="cell-json-punct">: </span>}
        {open ? (
          <span className="cell-json-punct">{open_}</span>
        ) : (
          <span className="cell-json-collapsed">{jsonSummary(value)}</span>
        )}
      </button>
      {open && (
        <>
          {entries.map(([childName, childValue]) => (
            <JsonNode
              key={childName}
              name={kind === "array" ? undefined : childName}
              value={childValue}
              depth={depth + 1}
            />
          ))}
          <div className="cell-json-row" style={{ paddingLeft: depth * 12 }}>
            <span className="cell-json-punct">{close}</span>
          </div>
        </>
      )}
    </div>
  );
}

function isIndex(name: string): boolean {
  return /^\d+$/.test(name);
}

function JsonExpanded({ text }: { text: string }) {
  const parsed = safeParseJson(text);
  if (!parsed.ok) {
    return <div className="cell-rich-error">Invalid JSON — showing raw text:<pre className="cell-rich-pre">{text}</pre></div>;
  }
  const pretty = JSON.stringify(parsed.value, null, 2);
  return (
    <div className="cell-json-expanded">
      <div className="cell-rich-toolbar">
        <CopyButton value={pretty} label="Copy JSON" />
      </div>
      <div className="cell-json-tree">
        <JsonNode value={parsed.value} depth={0} />
      </div>
    </div>
  );
}

export const jsonRenderer: CellRenderer = {
  id: "builtin:json",
  priority: 10,
  appliesTo: (column, value) => isJsonType(column.type) && typeof value === "string",
  renderInline: ({ text }) => {
    const parsed = safeParseJson(text);
    return (
      <span className="cell-json-inline">
        {parsed.ok ? jsonSummary(parsed.value) : text}
      </span>
    );
  },
  renderExpanded: ({ text }) => <JsonExpanded text={text} />,
  title: ({ column }) => `${column.name} · ${column.type}`,
};

/**
 * Array renderer: native SQL arrays (`int4[]`, `text[]`, …) shown as a chip
 * list with per-element type awareness, expandable for large arrays.
 *
 * Values arrive as the engine's array literal text (e.g. `{1,2,3}` or
 * `{"a","b,c",NULL}`); `parsePgArray` splits the top-level elements while
 * respecting quoting and nested braces.
 */
import { CopyButton } from "./shared";
import {
  arrayElementType,
  isArrayType,
  isBooleanElement,
  isNumericElement,
} from "./typeMatch";
import type { CellRenderer } from "./types";

export type ArrayElementKind = "number" | "boolean" | "null" | "string";

export type ArrayElement = {
  /** Display text for the element (unquoted). */
  text: string;
  kind: ArrayElementKind;
};

/**
 * Split a Postgres array literal into its top-level element tokens. Handles
 * double-quoted elements (with escaped quotes/backslashes) and nested arrays
 * (kept as a single `{...}` token). Returns raw tokens — quotes preserved — so
 * callers can distinguish an unquoted `NULL` from the string `"NULL"`.
 */
export function parsePgArray(literal: string): string[] {
  const trimmed = literal.trim();
  if (!trimmed.startsWith("{") || !trimmed.endsWith("}")) return [];
  const inner = trimmed.slice(1, -1);
  if (inner.length === 0) return [];

  const tokens: string[] = [];
  let current = "";
  let depth = 0;
  let inQuotes = false;
  for (let i = 0; i < inner.length; i++) {
    const ch = inner[i];
    if (inQuotes) {
      if (ch === "\\" && i + 1 < inner.length) {
        current += ch + inner[i + 1];
        i++;
        continue;
      }
      if (ch === '"') inQuotes = false;
      current += ch;
      continue;
    }
    if (ch === '"') {
      inQuotes = true;
      current += ch;
      continue;
    }
    if (ch === "{") depth++;
    if (ch === "}") depth--;
    if (ch === "," && depth === 0) {
      tokens.push(current);
      current = "";
      continue;
    }
    current += ch;
  }
  tokens.push(current);
  return tokens.map((token) => token.trim());
}

/** Classify and unquote a single array element token. */
export function elementOf(token: string, elementType: string): ArrayElement {
  if (token.startsWith('"') && token.endsWith('"') && token.length >= 2) {
    const unquoted = token
      .slice(1, -1)
      .replace(/\\(.)/g, "$1");
    return { text: unquoted, kind: "string" };
  }
  if (token === "NULL") return { text: "NULL", kind: "null" };
  if (isNumericElement(elementType)) return { text: token, kind: "number" };
  if (isBooleanElement(elementType)) {
    return { text: token === "t" ? "true" : token === "f" ? "false" : token, kind: "boolean" };
  }
  return { text: token, kind: "string" };
}

function Chip({ element }: { element: ArrayElement }) {
  return (
    <span className={`cell-array-chip kind-${element.kind}`}>
      {element.kind === "null" ? "NULL" : element.text}
    </span>
  );
}

const INLINE_LIMIT = 8;

export const arrayRenderer: CellRenderer = {
  id: "builtin:array",
  priority: 10,
  appliesTo: (column, value) =>
    isArrayType(column.type) &&
    typeof value === "string" &&
    value.trim().startsWith("{") &&
    value.trim().endsWith("}"),
  renderInline: ({ text, column }) => {
    const elementType = arrayElementType(column.type);
    const tokens = parsePgArray(text);
    const shown = tokens.slice(0, INLINE_LIMIT);
    const overflow = tokens.length - shown.length;
    if (tokens.length === 0) {
      // Distinguish a genuinely empty array from a literal we could not parse,
      // so malformed/truncated data does not masquerade as `{ }`.
      return text.trim() === "{}" ? (
        <span className="cell-array-empty">{"{ }"}</span>
      ) : (
        <span className="cell-array-empty">{text}</span>
      );
    }
    return (
      <span className="cell-array-inline">
        {shown.map((token, index) => (
          <Chip key={index} element={elementOf(token, elementType)} />
        ))}
        {overflow > 0 && <span className="cell-array-more">+{overflow}</span>}
      </span>
    );
  },
  renderExpanded: ({ text, column }) => {
    const elementType = arrayElementType(column.type);
    const tokens = parsePgArray(text);
    return (
      <div className="cell-array-expanded">
        <div className="cell-rich-toolbar">
          <span className="cell-rich-badge">
            {tokens.length} × {elementType}
          </span>
          <CopyButton value={text} label="Copy" />
        </div>
        <ol className="cell-array-list">
          {tokens.map((token, index) => (
            <li key={index}>
              <span className="cell-array-index">{index}</span>
              <Chip element={elementOf(token, elementType)} />
            </li>
          ))}
        </ol>
      </div>
    );
  },
  title: ({ column }) => `${column.name} · ${column.type}`,
};

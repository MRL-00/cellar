import { useEffect, useRef, useState } from "react";
import { GridIcon } from "./icons";
import { defaultRendererRegistry } from "./renderers/registry";
import { RichCell } from "./renderers/RichCell";
import { isByteaType } from "./renderers/typeMatch";
import type { RendererRegistry, SaveBlob } from "./renderers/types";
import { statusDotColor, statusTextColor } from "./status";
import type { GridColumn, GridValue } from "./types";

type Value = GridValue | undefined;

// ---------------------------------------------------------------------------
// Pure decision helpers — exported so that Cell.test.ts can test them
// directly without a DOM environment.
// ---------------------------------------------------------------------------

/**
 * Decides what to do when the Enter key is pressed.
 * - If the value was modified (dirty) → commit the new value.
 * - Otherwise → cancel (no-op; avoids phantom pending edits on NULL cells).
 */
export function resolveEnterAction(dirty: boolean): "commit" | "cancel" {
  return dirty ? "commit" : "cancel";
}

/**
 * Decides what to do on blur.
 * - If the editor is already settled (Enter/Escape was handled) → noop.
 * - Otherwise follows the same dirty logic as Enter.
 */
export function resolveBlurAction(
  settled: boolean,
  dirty: boolean,
): "commit" | "cancel" | "noop" {
  if (settled) return "noop";
  return dirty ? "commit" : "cancel";
}

export type ParsedCellValue =
  | { ok: true; value: GridValue }
  | { ok: false; message: string };

export function parseCellInput(col: GridColumn, raw: string): ParsedCellValue {
  if (col.enum) {
    return col.enum.includes(raw)
      ? { ok: true, value: raw }
      : { ok: false, message: `Choose one of: ${col.enum.join(", ")}` };
  }

  const type = normalizeType(col.type);
  const trimmed = raw.trim();

  if (trimmed === "" && type !== "text") {
    if (col.nullable) return { ok: true, value: null };
    return { ok: false, message: `${col.name} cannot be NULL` };
  }

  if (type === "bool") {
    const lower = trimmed.toLowerCase();
    if (["true", "t", "1", "yes", "y"].includes(lower)) {
      return { ok: true, value: true };
    }
    if (["false", "f", "0", "no", "n"].includes(lower)) {
      return { ok: true, value: false };
    }
    return { ok: false, message: "Enter TRUE or FALSE" };
  }

  if (type === "integer") {
    if (!/^[+-]?\d+$/.test(trimmed)) {
      return { ok: false, message: "Enter a whole number" };
    }
    const value = Number(trimmed);
    return { ok: true, value: Number.isSafeInteger(value) ? value : trimmed };
  }

  if (type === "float") {
    const value = Number(trimmed);
    if (!Number.isFinite(value)) {
      return { ok: false, message: "Enter a valid number" };
    }
    return { ok: true, value };
  }

  if (type === "numeric") {
    if (!/^[+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?$/i.test(trimmed)) {
      return { ok: false, message: "Enter a valid numeric value" };
    }
    return { ok: true, value: trimmed };
  }

  if (type === "guid") {
    if (
      !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(
        trimmed,
      )
    ) {
      return { ok: false, message: "Enter a valid GUID" };
    }
    return { ok: true, value: trimmed };
  }

  if (type === "json") {
    try {
      JSON.parse(trimmed);
      return { ok: true, value: trimmed };
    } catch {
      return { ok: false, message: "Enter valid JSON" };
    }
  }

  if (type === "date") {
    if (!/^\d{4}-\d{2}-\d{2}$/.test(trimmed) || Number.isNaN(Date.parse(trimmed))) {
      return { ok: false, message: "Use YYYY-MM-DD" };
    }
    return { ok: true, value: trimmed };
  }

  if (type === "time") {
    if (!/^\d{2}:\d{2}(?::\d{2}(?:\.\d{1,6})?)?(?:[+-]\d{2}(?::?\d{2})?)?$/.test(trimmed)) {
      return { ok: false, message: "Use HH:MM[:SS]" };
    }
    return { ok: true, value: trimmed };
  }

  if (type === "timestamp") {
    if (Number.isNaN(Date.parse(trimmed))) {
      return { ok: false, message: "Enter a valid timestamp" };
    }
    return { ok: true, value: trimmed };
  }

  if (type === "bytea") {
    if (!/^\\x(?:[0-9a-f]{2})*$/i.test(trimmed)) {
      return { ok: false, message: "Use hex bytea format, e.g. \\x0a2b" };
    }
    return { ok: true, value: trimmed };
  }

  return { ok: true, value: raw };
}

function normalizeType(type: string):
  | "bool"
  | "bytea"
  | "date"
  | "float"
  | "integer"
  | "json"
  | "numeric"
  | "text"
  | "time"
  | "timestamp"
  | "guid"
  | "unknown" {
  const t = type.toLowerCase().replace(/\(.+\)$/, "").trim();
  // "bit" is SQL Server's boolean. (Postgres bit(n) strings also land here —
  // acceptable until someone edits a pg bitstring column in the grid.)
  if (["bool", "boolean", "bit"].includes(t)) return "bool";
  // Keep the hex-editing set in lockstep with the bytea renderer's detection so
  // every type shown as a hex blob is also validated as hex when edited.
  if (isByteaType(t)) return "bytea";
  if (t === "date") return "date";
  if (["float4", "float8", "real", "double precision", "float", "double"].includes(t)) {
    return "float";
  }
  if (
    // Postgres int2/int4/int8/serial; MSSQL int/tinyint; MySQL mediumint
    ["int2", "int4", "int8", "smallint", "integer", "int", "tinyint", "mediumint", "bigint", "serial", "bigserial", "oid"].includes(
      t,
    )
  ) {
    return "integer";
  }
  if (["json", "jsonb"].includes(t)) return "json";
  if (["numeric", "decimal", "money", "smallmoney"].includes(t)) return "numeric";
  if (
    // Postgres text/varchar/...; MSSQL nvarchar/nchar/ntext/sysname/xml; MySQL *text
    ["text", "varchar", "char", "bpchar", "citext", "name", "character varying", "character",
     "nvarchar", "nchar", "ntext", "sysname", "xml", "tinytext", "mediumtext", "longtext"].includes(
      t,
    )
  ) {
    return "text";
  }
  if (["time", "timetz", "time without time zone", "time with time zone"].includes(t)) {
    return "time";
  }
  if (
    // Postgres timestamp/timestamptz; MSSQL datetime/datetime2/smalldatetime/datetimeoffset
    ["timestamp", "timestamptz", "timestamp without time zone", "timestamp with time zone",
     "datetime", "datetime2", "smalldatetime", "datetimeoffset"].includes(
      t,
    )
  ) {
    return "timestamp";
  }
  if (["uuid", "guid", "uniqueidentifier"].includes(t)) return "guid";
  return "unknown";
}

/**
 * Picks a native HTML input control for the editor so date/time/number columns
 * get the browser's built-in picker and keypad-level rejection of bad keys.
 * Returns null to keep the plain text editor — for unknown types, or values the
 * native control can't represent (those stay freely editable as raw text).
 *
 * `numeric`/`decimal` deliberately stay text: their value is a lossless string
 * and a number input would lose precision on large decimals. The existing
 * parseCellInput validation already rejects letters in those on commit.
 */
export function nativeControl(
  col: GridColumn,
  initial: string,
): { type: string; step?: string; value: string } | null {
  if (col.enum) return null;
  const t = normalizeType(col.type);
  if (t === "integer") return { type: "number", step: "1", value: initial };
  if (t === "float") return { type: "number", step: "any", value: initial };

  // For date/time/timestamp, an empty cell still gets a picker; a populated one
  // only does if its stored value parses into the control's required format.
  if (t === "date") {
    if (initial === "") return { type: "date", value: "" };
    return /^\d{4}-\d{2}-\d{2}/.test(initial)
      ? { type: "date", value: initial.slice(0, 10) }
      : null;
  }
  if (t === "time") {
    if (initial === "") return { type: "time", step: "1", value: "" };
    const m = /^(\d{2}:\d{2}(?::\d{2})?)/.exec(initial.trim());
    return m?.[1] ? { type: "time", step: "1", value: m[1] } : null;
  }
  if (t === "timestamp") {
    if (initial === "") return { type: "datetime-local", step: "1", value: "" };
    // "2023-04-03T05:00:31.15863" or "2023-04-03 05:00:31" → "...T05:00:31".
    // Sub-second precision is dropped — a picker can't express it — but only a
    // user who actually changes the value commits, so stored values are intact.
    const m = /^(\d{4}-\d{2}-\d{2})[T ](\d{2}:\d{2}(?::\d{2})?)/.exec(initial.trim());
    return m?.[1] && m[2]
      ? { type: "datetime-local", step: "1", value: `${m[1]}T${m[2]}` }
      : null;
  }
  return null;
}

export function CellValue({
  col,
  value,
  nullDisplay = "NULL",
  renderers = defaultRendererRegistry,
  saveBlob,
}: {
  col: GridColumn;
  value: Value;
  nullDisplay?: string;
  /** Rich-type renderer registry. Defaults to the built-in renderers. */
  renderers?: RendererRegistry | null;
  saveBlob?: SaveBlob;
}) {
  if (value === null || value === undefined) {
    return <span className="cell-null mono">{nullDisplay}</span>;
  }
  // Rich renderers take over the *display* of complex types (JSON, arrays,
  // bytea, geometry). Editing is unaffected — double-click still opens the
  // existing inline editor for these cells.
  if (renderers) {
    const renderer = renderers.resolve(col, value);
    if (renderer) {
      return (
        <RichCell
          renderer={renderer}
          column={col}
          value={value}
          nullDisplay={nullDisplay}
          saveBlob={saveBlob}
        />
      );
    }
  }
  if (col.enum && col.key === "status") {
    const v = String(value);
    return (
      <span className="cell-enum">
        <span className="dot" style={{ background: statusDotColor(v) }} />
        <span style={{ color: statusTextColor(v) }}>{v}</span>
      </span>
    );
  }
  if (col.fk) {
    return (
      <span className="cell-fk">
        <span className={col.mono ? "mono" : ""}>{String(value)}</span>
        <span className="cell-fk-jump" title={`Foreign key: ${col.fk}`}>
          <GridIcon.link2 size={9} />
        </span>
      </span>
    );
  }
  if (col.align === "right") {
    return (
      <span className="mono tnum" style={{ textAlign: "right", width: "100%" }}>
        {String(value)}
      </span>
    );
  }
  return <span className={col.mono ? "mono" : ""}>{String(value)}</span>;
}

export type CellEditorProps = {
  col: GridColumn;
  value: Value;
  onCommit: (next: Value) => void;
  onCancel: () => void;
};

export function CellEditor({ col, value, onCommit, onCancel }: CellEditorProps) {
  const initialValue = value == null ? "" : String(value);
  const control = nativeControl(col, initialValue);
  const [v, setV] = useState<string>(control ? control.value : initialValue);
  const [error, setError] = useState<string | null>(null);
  const ref = useRef<HTMLInputElement | null>(null);
  // Tracks whether Enter or Escape has already been handled so that the
  // subsequent blur event does not fire a second commit or override a cancel.
  const settledRef = useRef(false);
  // Tracks whether the user has actually changed the value from the initial
  // state.  An untouched editor that closes (via Enter, Escape, or blur) must
  // be a no-op — most importantly, opening a NULL cell and pressing Enter
  // without typing must NOT record a phantom pending edit of null→"".
  const dirtyRef = useRef(false);

  useEffect(() => {
    ref.current?.focus();
    // select() throws InvalidStateError on date/time/number inputs; only text
    // (and the like) supports it, so guard rather than branch on type.
    try {
      ref.current?.select();
    } catch {
      /* non-text native control — nothing to select */
    }
  }, []);

  const commitRawValue = () => {
    const parsed = parseCellInput(col, v);
    if (!parsed.ok) {
      settledRef.current = false;
      setError(parsed.message);
      window.setTimeout(() => {
        ref.current?.focus();
        ref.current?.select();
      }, 0);
      return;
    }
    onCommit(parsed.value);
  };

  // Boolean columns edit via the same single-select option UI as enums, so
  // the only committable values are TRUE / FALSE (and NULL when allowed).
  const boolEditor = !col.enum && normalizeType(col.type) === "bool";
  const options =
    col.enum ??
    (boolEditor ? ["TRUE", "FALSE", ...(col.nullable ? ["NULL"] : [])] : null);
  const selected = boolEditor
    ? value == null
      ? "NULL"
      : String(value) === "true"
        ? "TRUE"
        : "FALSE"
    : v;

  if (options) {
    return (
      <div
        className="cell-edit-enum"
        // Clicking outside the enum editor (blur on the container) cancels the
        // edit unless a button click has already settled the interaction.
        onBlur={(e) => {
          // Only fire when focus leaves the entire container (not when moving
          // between buttons within it).
          if (e.currentTarget.contains(e.relatedTarget as Node | null)) return;
          if (settledRef.current) return;
          settledRef.current = true;
          onCancel();
        }}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            settledRef.current = true;
            onCancel();
          }
        }}
        // tabIndex is required so the div can receive focus and fire blur
        // events, which lets the onBlur handler detect clicks outside.
        tabIndex={-1}
      >
        {options.map((opt) => (
          <button
            key={opt}
            className={"cell-edit-opt" + (opt === selected ? " active" : "")}
            onClick={() => {
              settledRef.current = true;
              onCommit(
                boolEditor ? (opt === "NULL" ? null : opt === "TRUE") : opt,
              );
            }}
          >
            {col.enum && col.key === "status" && (
              <span
                className="dot"
                style={{ background: statusDotColor(opt) }}
              />
            )}
            <span>{opt}</span>
          </button>
        ))}
      </div>
    );
  }

  return (
    <input
      ref={ref}
      className={"cell-edit-input" + (col.mono ? " mono" : "")}
      type={control?.type ?? "text"}
      step={control?.step}
      value={v}
      onChange={(e) => {
        dirtyRef.current = true;
        setError(null);
        setV(e.target.value);
      }}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          settledRef.current = true;
          const decision = resolveEnterAction(dirtyRef.current);
          if (decision === "commit") {
            commitRawValue();
          } else {
            onCancel();
          }
        }
        if (e.key === "Escape") {
          settledRef.current = true;
          onCancel();
        }
      }}
      onBlur={() => {
        const decision = resolveBlurAction(settledRef.current, dirtyRef.current);
        if (decision === "noop") return;
        settledRef.current = true;
        if (decision === "commit") {
          commitRawValue();
        } else {
          onCancel();
        }
      }}
      aria-invalid={error ? true : undefined}
      title={error ?? undefined}
    />
  );
}

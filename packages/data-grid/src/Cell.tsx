import { useEffect, useRef, useState } from "react";
import { GridIcon } from "./icons";
import { statusDotColor, statusTextColor } from "./status";
import type { GridColumn, GridValue } from "./types";

type Value = GridValue | undefined;

export function CellValue({ col, value }: { col: GridColumn; value: Value }) {
  if (value === null || value === undefined) {
    return <span className="cell-null mono">NULL</span>;
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
  const [v, setV] = useState<string>(initialValue);
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
    ref.current?.select();
  }, []);

  if (col.enum) {
    return (
      <div className="cell-edit-enum">
        {col.enum.map((opt) => (
          <button
            key={opt}
            className={"cell-edit-opt" + (opt === v ? " active" : "")}
            onClick={() => {
              settledRef.current = true;
              onCommit(opt);
            }}
          >
            {col.key === "status" && (
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
      value={v}
      onChange={(e) => {
        dirtyRef.current = true;
        setV(e.target.value);
      }}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          settledRef.current = true;
          if (dirtyRef.current) {
            onCommit(v);
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
        if (settledRef.current) return;
        settledRef.current = true;
        if (dirtyRef.current) {
          onCommit(v);
        } else {
          onCancel();
        }
      }}
    />
  );
}

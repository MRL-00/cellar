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
        <button className="cell-fk-jump" title={`Jump to ${col.fk}`}>
          <GridIcon.link2 size={9} />
        </button>
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
  const [v, setV] = useState<string>(value == null ? "" : String(value));
  const ref = useRef<HTMLInputElement | null>(null);

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
            onClick={() => onCommit(opt)}
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
      onChange={(e) => setV(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === "Enter") onCommit(v);
        if (e.key === "Escape") onCancel();
      }}
      onBlur={() => onCommit(v)}
    />
  );
}

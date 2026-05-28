import { GridIcon } from "./icons";
import type { ColumnFilters } from "./types";

export type FilterBarProps = {
  filters: ColumnFilters;
  setFilters: (next: ColumnFilters) => void;
  totalRows: number;
  filteredRows: number;
};

export function FilterBar({
  filters,
  setFilters,
  totalRows,
  filteredRows,
}: FilterBarProps) {
  return (
    <div className="grid-filterbar">
      <div className="grid-filterbar-label">
        <GridIcon.filter size={11} style={{ color: "var(--accent)" }} />
        <span>where</span>
      </div>
      <div className="grid-filterbar-chips">
        {Object.entries(filters).map(([k, v]) => (
          <span key={k} className="grid-filter-chip">
            <span className="mono">{k}</span>
            <span className="grid-filter-op">=</span>
            <span className="mono" style={{ color: "var(--syn-str)" }}>
              {"'"}
              {v}
              {"'"}
            </span>
            <button
              onClick={() => {
                const next = { ...filters };
                delete next[k];
                setFilters(next);
              }}
              aria-label={`Remove filter on ${k}`}
            >
              <GridIcon.close size={9} />
            </button>
          </span>
        ))}
        <button className="grid-filter-add">
          <GridIcon.plus size={10} />
          <span>add</span>
        </button>
      </div>
      <div className="grid-filterbar-summary mono">
        <span style={{ color: "var(--fg-1)" }}>
          {filteredRows.toLocaleString()}
        </span>
        <span style={{ color: "var(--fg-3)" }}>/</span>
        <span style={{ color: "var(--fg-2)" }}>{totalRows.toLocaleString()}</span>
        <span style={{ color: "var(--fg-3)" }}> rows</span>
      </div>
    </div>
  );
}

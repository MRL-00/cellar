/**
 * PostGIS geometry / geography renderer — interface stub.
 *
 * The mini-map preview is a deferred stretch goal (SPEC §6.5 rich types). For
 * now this renderer establishes the contract and surfaces the geometry text
 * (WKT / EWKT / GeoJSON) honestly: the expanded view shows the raw value and a
 * clearly-labelled "map preview not available yet" placeholder rather than a
 * fake control (per the repo rule against rendering stub controls as if they
 * work). A community or follow-up renderer can register a higher-priority
 * geometry renderer that draws a real map without touching the grid.
 */
import { CopyButton } from "./shared";
import { isGeometryType } from "./typeMatch";
import type { CellRenderer } from "./types";

/** Best-effort short label for a geometry value (the WKT type keyword). */
export function geometryLabel(text: string): string {
  const match = /^(SRID=\d+;)?\s*([A-Za-z]+)/.exec(text.trim());
  return match?.[2]?.toUpperCase() ?? "GEOMETRY";
}

export const geometryRenderer: CellRenderer = {
  id: "builtin:geometry",
  priority: 5,
  appliesTo: (column, value) => isGeometryType(column.type) && typeof value === "string",
  renderInline: ({ text }) => (
    <span className="cell-geometry-inline">
      <span className="cell-geometry-glyph" aria-hidden="true">◈</span>
      <span className="cell-geometry-kind">{geometryLabel(text)}</span>
    </span>
  ),
  renderExpanded: ({ text }) => (
    <div className="cell-geometry-expanded">
      <div className="cell-rich-toolbar">
        <span className="cell-rich-badge">{geometryLabel(text)}</span>
        <CopyButton value={text} label="Copy" />
      </div>
      <div className="cell-geometry-map" role="img" aria-label="Map preview placeholder">
        <span>Map preview not available yet</span>
      </div>
      <pre className="cell-rich-pre">{text}</pre>
    </div>
  ),
  title: ({ column }) => `${column.name} · ${column.type}`,
};

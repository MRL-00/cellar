import { GridIcon, type GridIconProps } from "./icons";
import type { GridColumn } from "./types";

/**
 * Renders the small monochrome glyph in a column header that signals what kind
 * of value the column holds. PKs and FKs win over data type — they're the
 * structural fact about the column. Otherwise the SQL type drives the icon.
 */
export function TypeIcon({ col }: { col: GridColumn }) {
  const type = col.type.toLowerCase();
  let Comp: (p: GridIconProps) => JSX.Element = GridIcon.text;
  if (col.pk) Comp = GridIcon.key;
  else if (col.fk) Comp = GridIcon.link;
  else if (type.startsWith("uuid")) Comp = GridIcon.hash;
  else if (type.startsWith("text") || type.startsWith("char")) Comp = GridIcon.text;
  else if (
    type.includes("int") ||
    type.includes("numeric") ||
    type.includes("decimal")
  )
    Comp = GridIcon.hash;
  else if (
    type.includes("timestamp") ||
    type.includes("date") ||
    type.includes("time")
  )
    Comp = GridIcon.cal;
  else if (type.includes("bool")) Comp = GridIcon.bool;
  else if (
    type.includes("json") ||
    type === "object" ||
    type === "array" ||
    type === "map"
  )
    Comp = GridIcon.json;
  else if (col.enum) Comp = GridIcon.enum;

  const color = col.pk ? "var(--update)" : col.fk ? "var(--accent)" : "var(--fg-3)";

  return (
    <Comp
      size={10}
      stroke={color}
      fill={col.pk || col.fk ? color : "none"}
    />
  );
}

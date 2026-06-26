import { memo, type PointerEvent as ReactPointerEvent } from "react";
import type { ErNode } from "@cellar/ipc";

import {
  NODE_FOOTER_H,
  NODE_HEADER_H,
  NODE_ROW_H,
  visibleColumns,
  type NodeBox,
} from "./layout";

const CHAR_PX = 6.6;

function truncate(text: string, widthPx: number): string {
  const max = Math.max(2, Math.floor(widthPx / CHAR_PX));
  return text.length > max ? `${text.slice(0, max - 1)}…` : text;
}

function rowCount(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

export interface ErNodeCardProps {
  node: ErNode;
  box: NodeBox;
  compact: boolean;
  expanded: boolean;
  focused: boolean;
  onPointerDown: (event: ReactPointerEvent, id: string) => void;
  onToggleExpand: (id: string) => void;
}

/**
 * One table rendered as pure SVG — keeps 100+ nodes cheap and lets the whole
 * card theme from the same CSS tokens as the rest of the app. Columns are
 * monospace per the spec's "monospace for identifiers and data" rule.
 */
function ErNodeCardImpl({
  node,
  box,
  compact,
  expanded,
  focused,
  onPointerDown,
  onToggleExpand,
}: ErNodeCardProps) {
  const { columns, hidden, overflow } = visibleColumns(node, compact, expanded);
  const nameWidth = box.width - 16;

  return (
    <g
      transform={`translate(${box.x},${box.y})`}
      style={{ cursor: "pointer" }}
      onPointerDown={(e) => onPointerDown(e, node.id)}
    >
      <title>
        {node.schema}.{node.name}
        {node.row_count != null ? ` · ${node.row_count} rows` : ""}
      </title>
      <rect
        width={box.width}
        height={box.height}
        rx={4}
        fill="var(--bg-1)"
        stroke={focused ? "var(--accent)" : "var(--border-default)"}
        strokeWidth={focused ? 1.5 : 1}
      />
      <rect width={box.width} height={NODE_HEADER_H} rx={4} fill="var(--bg-2)" />
      <rect y={NODE_HEADER_H - 6} width={box.width} height={6} fill="var(--bg-2)" />
      <line
        x1={0}
        y1={NODE_HEADER_H}
        x2={box.width}
        y2={NODE_HEADER_H}
        stroke="var(--border-default)"
      />
      <text
        className="font-mono"
        x={8}
        y={NODE_HEADER_H / 2}
        dominantBaseline="central"
        fontSize={11}
        fontWeight={600}
        fill="var(--fg-0)"
      >
        {truncate(node.name, nameWidth - 36)}
      </text>
      {node.row_count != null && (
        <text
          className="font-mono"
          x={box.width - 8}
          y={NODE_HEADER_H / 2}
          textAnchor="end"
          dominantBaseline="central"
          fontSize={9}
          fill="var(--fg-3)"
        >
          {rowCount(node.row_count)}
        </text>
      )}

      {columns.map((col, i) => {
        const cy = NODE_HEADER_H + i * NODE_ROW_H + NODE_ROW_H / 2;
        const badge = col.is_primary_key ? "PK" : col.is_foreign_key ? "FK" : "";
        const badgeColor = col.is_primary_key
          ? "var(--accent)"
          : "var(--warn)";
        const typeText = truncate(col.data_type, 60);
        const nameMax = nameWidth - typeText.length * CHAR_PX - 22;
        return (
          <g key={col.name}>
            {badge && (
              <text
                className="font-mono"
                x={8}
                y={cy}
                dominantBaseline="central"
                fontSize={8}
                fontWeight={700}
                fill={badgeColor}
              >
                {badge}
              </text>
            )}
            <text
              className="font-mono"
              x={badge ? 26 : 10}
              y={cy}
              dominantBaseline="central"
              fontSize={10.5}
              fill={col.is_primary_key ? "var(--fg-0)" : "var(--fg-1)"}
            >
              {truncate(col.name, nameMax)}
            </text>
            <text
              className="font-mono"
              x={box.width - 8}
              y={cy}
              textAnchor="end"
              dominantBaseline="central"
              fontSize={9.5}
              fill="var(--fg-3)"
            >
              {typeText}
            </text>
          </g>
        );
      })}

      {overflow && (
        <g
          style={{ cursor: "pointer" }}
          onPointerDown={(e) => e.stopPropagation()}
          onClick={(e) => {
            e.stopPropagation();
            onToggleExpand(node.id);
          }}
        >
          {/* Full-width hit area so the whole footer toggles expansion. */}
          <rect
            y={box.height - NODE_FOOTER_H}
            width={box.width}
            height={NODE_FOOTER_H}
            fill="transparent"
          />
          <text
            className="font-mono"
            x={10}
            y={box.height - NODE_FOOTER_H / 2}
            dominantBaseline="central"
            fontSize={9}
            fontWeight={600}
            fill="var(--accent)"
          >
            {expanded ? "▾ show less" : `▸ +${hidden} more`}
          </text>
        </g>
      )}
    </g>
  );
}

// Memoized so panning/zooming (which only changes the root transform) doesn't
// re-render every table's full SVG subtree — props are stable between frames.
export const ErNodeCard = memo(ErNodeCardImpl);

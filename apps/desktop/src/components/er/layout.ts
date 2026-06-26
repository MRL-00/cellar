// Deterministic, dependency-free layout for the ER diagram. The spec's
// philosophy is to own the rendering surface (the data grid is custom for the
// same reason), so rather than pull in reactflow/dagre/elkjs — which bring
// their own styling and, in elkjs' case, a non-MIT (EPL) license — we lay the
// graph out ourselves. A degree-ordered BFS clusters related tables, then they
// pack into a grid. Nodes are plain SVG, so 100+ tables stay cheap and the
// whole surface themes from the same CSS tokens as the rest of the app.

import type { ErColumn, ErEdge, ErGraph, ErNode } from "@cellar/ipc";

export const NODE_HEADER_H = 26;
export const NODE_ROW_H = 18;
export const NODE_FOOTER_H = 16;
export const NODE_MIN_WIDTH = 184;
export const NODE_MAX_WIDTH = 300;

/** Cap rows so a wide table can't produce an unusably tall node. */
const MAX_ROWS = 14;
/** Approximate monospace advance at the node font size, in px. */
const CHAR_PX = 6.6;

export interface NodeBox {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

export type NodePositions = Record<string, NodeBox>;

export interface VisibleColumns {
  columns: ErColumn[];
  /** How many columns are hidden behind the row cap (0 when fully shown). */
  hidden: number;
  /** `true` when this node has more columns than the cap — i.e. it is
   *  expandable, regardless of whether it is currently expanded. */
  overflow: boolean;
}

/**
 * Which columns to render for a node. Compact mode keeps only key columns
 * (primary or foreign), which is what matters for reading relationships in a
 * dense graph; full mode shows everything. Either way the visible rows are
 * capped at [`MAX_ROWS`] so a wide table stays readable — unless `expanded`,
 * in which case every column in the set is shown.
 */
export function visibleColumns(
  node: ErNode,
  compact: boolean,
  expanded = false,
): VisibleColumns {
  const cols = compact
    ? node.columns.filter((c) => c.is_primary_key || c.is_foreign_key)
    : node.columns;
  const overflow = cols.length > MAX_ROWS;
  if (expanded || !overflow) return { columns: cols, hidden: 0, overflow };
  return {
    columns: cols.slice(0, MAX_ROWS),
    hidden: cols.length - MAX_ROWS,
    overflow,
  };
}

function measure(
  node: ErNode,
  compact: boolean,
  expanded: boolean,
): { width: number; height: number } {
  const { columns, overflow } = visibleColumns(node, compact, expanded);
  const longest = Math.max(
    node.name.length + 6,
    ...columns.map((c) => c.name.length + c.data_type.length + 4),
    10,
  );
  const width = Math.min(
    NODE_MAX_WIDTH,
    Math.max(NODE_MIN_WIDTH, Math.round(longest * CHAR_PX) + 24),
  );
  const height =
    NODE_HEADER_H +
    columns.length * NODE_ROW_H +
    (overflow ? NODE_FOOTER_H : 0);
  return { width, height };
}

/**
 * Order nodes so connected tables land near each other: degree-ordered BFS
 * across components. Deterministic — ties break on id — so re-running
 * auto-arrange always produces the same layout.
 */
function orderNodes(graph: ErGraph): ErNode[] {
  const byId = new Map(graph.nodes.map((n) => [n.id, n] as const));
  const adj = new Map<string, Set<string>>();
  for (const n of graph.nodes) adj.set(n.id, new Set());
  for (const e of graph.edges) {
    if (e.source === e.target) continue;
    adj.get(e.source)?.add(e.target);
    adj.get(e.target)?.add(e.source);
  }
  const degree = (id: string) => adj.get(id)?.size ?? 0;
  const cmp = (a: string, b: string) =>
    degree(b) - degree(a) || a.localeCompare(b);

  const seeds = [...graph.nodes].sort((a, b) => cmp(a.id, b.id));
  const visited = new Set<string>();
  const order: ErNode[] = [];
  for (const seed of seeds) {
    if (visited.has(seed.id)) continue;
    const queue = [seed.id];
    visited.add(seed.id);
    while (queue.length > 0) {
      const id = queue.shift() as string;
      const node = byId.get(id);
      if (node) order.push(node);
      const neighbors = [...(adj.get(id) ?? [])].sort(cmp);
      for (const nb of neighbors) {
        if (!visited.has(nb)) {
          visited.add(nb);
          queue.push(nb);
        }
      }
    }
  }
  return order;
}

export interface LayoutOptions {
  compact: boolean;
  /** Ids of nodes that are expanded to show all columns. */
  expanded?: Set<string>;
  gapX?: number;
  gapY?: number;
}

/** Pack the ordered nodes into a grid sized to roughly square the diagram. */
export function layoutGraph(graph: ErGraph, options: LayoutOptions): NodePositions {
  const gapX = options.gapX ?? 48;
  const gapY = options.gapY ?? 36;
  const expanded = options.expanded ?? new Set<string>();
  const dims = new Map(
    graph.nodes.map(
      (n) => [n.id, measure(n, options.compact, expanded.has(n.id))] as const,
    ),
  );
  const order = orderNodes(graph);
  const count = order.length;
  if (count === 0) return {};

  const cols = Math.max(1, Math.ceil(Math.sqrt(count)));
  const cellWidth = Math.max(
    NODE_MIN_WIDTH,
    ...[...dims.values()].map((d) => d.width),
  );

  const positions: NodePositions = {};
  let y = 0;
  let rowHeight = 0;
  order.forEach((node, i) => {
    const col = i % cols;
    if (col === 0 && i > 0) {
      y += rowHeight + gapY;
      rowHeight = 0;
    }
    const d = dims.get(node.id) ?? { width: NODE_MIN_WIDTH, height: NODE_HEADER_H };
    positions[node.id] = {
      id: node.id,
      x: col * (cellWidth + gapX),
      y,
      width: d.width,
      height: d.height,
    };
    rowHeight = Math.max(rowHeight, d.height);
  });
  return positions;
}

export interface Bounds {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
  width: number;
  height: number;
}

export function graphBounds(positions: NodePositions): Bounds {
  const boxes = Object.values(positions);
  if (boxes.length === 0) {
    return { minX: 0, minY: 0, maxX: 0, maxY: 0, width: 0, height: 0 };
  }
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const b of boxes) {
    minX = Math.min(minX, b.x);
    minY = Math.min(minY, b.y);
    maxX = Math.max(maxX, b.x + b.width);
    maxY = Math.max(maxY, b.y + b.height);
  }
  return { minX, minY, maxX, maxY, width: maxX - minX, height: maxY - minY };
}

export interface EdgeGeometry {
  id: string;
  edge: ErEdge;
  sx: number;
  sy: number;
  tx: number;
  ty: number;
  labelX: number;
  labelY: number;
  selfLoop: boolean;
}

/**
 * Vertical offset (relative to a node's top-left) of each visible column row's
 * centre. Lets edges anchor at the exact FK/PK field rather than the box edge.
 */
export function columnRowOffsets(
  node: ErNode,
  compact: boolean,
  expanded: boolean,
): Map<string, number> {
  const { columns } = visibleColumns(node, compact, expanded);
  const map = new Map<string, number>();
  columns.forEach((c, i) => {
    map.set(c.name, NODE_HEADER_H + i * NODE_ROW_H + NODE_ROW_H / 2);
  });
  return map;
}

/** Resolve a column's row-centre offset within a node, if it is visible. */
export type RowOffsetLookup = (
  nodeId: string,
  column: string,
) => number | undefined;

export function edgeGeometry(
  edge: ErEdge,
  positions: NodePositions,
  rowOffset?: RowOffsetLookup,
): EdgeGeometry | null {
  const s = positions[edge.source];
  const t = positions[edge.target];
  if (!s || !t) return null;

  // Anchor at the first FK column on the source and its referenced column on
  // the target; fall back to the box centre if the column is hidden.
  const sCol = edge.source_columns[0];
  const tCol = edge.target_columns[0];
  const sRowY = (sCol ? rowOffset?.(edge.source, sCol) : undefined) ?? s.height / 2;
  const tRowY = (tCol ? rowOffset?.(edge.target, tCol) : undefined) ?? t.height / 2;

  if (edge.source === edge.target) {
    const x = s.x + s.width;
    const sy = s.y + sRowY;
    const ty = s.y + (tRowY === sRowY ? tRowY + NODE_ROW_H : tRowY);
    return {
      id: edge.id,
      edge,
      sx: x,
      sy,
      tx: x,
      ty,
      labelX: x + 30,
      labelY: (sy + ty) / 2,
      selfLoop: true,
    };
  }

  // Leave/enter on whichever side faces the other node so the line connects to
  // the row horizontally.
  const targetIsRight = t.x + t.width / 2 >= s.x + s.width / 2;
  const sx = targetIsRight ? s.x + s.width : s.x;
  const tx = targetIsRight ? t.x : t.x + t.width;
  const sy = s.y + sRowY;
  const ty = t.y + tRowY;
  return {
    id: edge.id,
    edge,
    sx,
    sy,
    tx,
    ty,
    labelX: (sx + tx) / 2,
    labelY: (sy + ty) / 2,
    selfLoop: false,
  };
}

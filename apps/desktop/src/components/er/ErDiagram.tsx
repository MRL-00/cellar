import { commands, unwrap, type ErGraph } from "@cellar/ipc";
import {
  memo,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
} from "react";

import { Icon } from "../icons";
import { useTabs, type ErDiagramTab } from "../../state/tabs";
import { ErNodeCard } from "./ErNodeCard";
import {
  columnRowOffsets,
  edgeGeometry,
  graphBounds,
  layoutGraph,
  type EdgeGeometry,
  type NodePositions,
} from "./layout";

type Status = "loading" | "ready" | "error";

const MIN_ZOOM = 0.1;
const MAX_ZOOM = 2.5;
const CLICK_THRESHOLD = 4;
/** Above this many tables, default to compact mode so the graph stays legible. */
const COMPACT_THRESHOLD = 30;

function clamp(v: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, v));
}

interface Viewport {
  tx: number;
  ty: number;
  k: number;
}

interface DragState {
  mode: "pan" | "node";
  id?: string;
  pointerId: number;
  startX: number;
  startY: number;
  origTx: number;
  origTy: number;
  origX: number;
  origY: number;
  moved: boolean;
}

/** Per-tab view state, kept across remounts. The diagram component unmounts
 *  when you switch to another tab (e.g. clicking a node opens its grid), so
 *  zoom/pan, dragged positions, expanded tables, and the column/schema filters
 *  live here keyed by tab id instead of being lost on remount. */
interface ErViewState {
  compact: boolean;
  hidden: Set<string>;
  expanded: Set<string>;
  overrides: Record<string, { x: number; y: number }>;
  view: Viewport;
  focusedId: string | null;
}

const erViewCache = new Map<string, ErViewState>();

export function ErDiagram({ tab }: { tab: ErDiagramTab }) {
  const openTable = useTabs((s) => s.openTable);

  const cached = erViewCache.get(tab.id);
  const [graph, setGraph] = useState<ErGraph | null>(null);
  const [status, setStatus] = useState<Status>("loading");
  const [error, setError] = useState<string | null>(null);
  const [compact, setCompact] = useState(cached?.compact ?? false);
  const [hidden, setHidden] = useState<Set<string>>(cached?.hidden ?? new Set());
  const [expanded, setExpanded] = useState<Set<string>>(
    cached?.expanded ?? new Set(),
  );
  const [overrides, setOverrides] = useState<Record<string, { x: number; y: number }>>(
    cached?.overrides ?? {},
  );
  const [view, setView] = useState<Viewport>(cached?.view ?? { tx: 0, ty: 0, k: 1 });
  const [focusedId, setFocusedId] = useState<string | null>(
    cached?.focusedId ?? null,
  );
  const [schemaMenuOpen, setSchemaMenuOpen] = useState(false);
  // Container size drives viewport culling — only on-screen nodes are rendered.
  const [size, setSize] = useState({ w: 0, h: 0 });

  const containerRef = useRef<HTMLDivElement | null>(null);
  const dragRef = useRef<DragState | null>(null);
  // When restoring a cached view, treat the diagram as already fitted so we
  // don't auto-fit over the user's saved zoom/pan.
  const restoredRef = useRef(Boolean(cached));
  const fittedRef = useRef(Boolean(cached));
  // Mirror live state into refs so pointer handlers can stay referentially
  // stable (deps `[]`). That keeps `beginNodeDrag`/`focusNode` from changing
  // every frame, which is what lets the memoized nodes/edges skip re-rendering
  // during a pan or zoom.
  const viewRef = useRef(view);
  viewRef.current = view;
  const graphRef = useRef(graph);
  graphRef.current = graph;

  const load = useCallback(() => {
    let cancelled = false;
    setStatus("loading");
    setError(null);
    unwrap(commands.erGraph(tab.connectionId, tab.database, tab.schemas))
      .then((g) => {
        if (cancelled) return;
        setGraph(g);
        // Only pick the default density / fit for a fresh diagram; a restored
        // one keeps whatever the user last had.
        if (!restoredRef.current) {
          setCompact(g.nodes.length > COMPACT_THRESHOLD);
          fittedRef.current = false;
        }
        setStatus("ready");
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setError(err instanceof Error ? err.message : String(err));
        setStatus("error");
      });
    return () => {
      cancelled = true;
    };
  }, [tab.connectionId, tab.database, tab.schemas]);

  useEffect(() => load(), [load]);

  // Persist the live view state so switching tabs and back restores it.
  useEffect(() => {
    erViewCache.set(tab.id, {
      compact,
      hidden,
      expanded,
      overrides,
      view,
      focusedId,
    });
  }, [tab.id, compact, hidden, expanded, overrides, view, focusedId]);

  const filtered = useMemo<ErGraph | null>(() => {
    if (!graph) return null;
    const nodes = graph.nodes.filter((n) => !hidden.has(n.schema));
    const ids = new Set(nodes.map((n) => n.id));
    const edges = graph.edges.filter(
      (e) => ids.has(e.source) && ids.has(e.target),
    );
    return { ...graph, nodes, edges };
  }, [graph, hidden]);

  const basePositions = useMemo<NodePositions>(
    () => (filtered ? layoutGraph(filtered, { compact, expanded }) : {}),
    [filtered, compact, expanded],
  );

  const positions = useMemo<NodePositions>(() => {
    const out: NodePositions = {};
    for (const [id, box] of Object.entries(basePositions)) {
      const o = overrides[id];
      out[id] = o ? { ...box, x: o.x, y: o.y } : box;
    }
    return out;
  }, [basePositions, overrides]);
  const positionsRef = useRef(positions);
  positionsRef.current = positions;

  // Row-centre offset of every visible column, so edges anchor at the actual
  // FK/PK field instead of the box edge.
  const rowOffsets = useMemo(() => {
    const m = new Map<string, Map<string, number>>();
    if (filtered) {
      for (const n of filtered.nodes) {
        m.set(n.id, columnRowOffsets(n, compact, expanded.has(n.id)));
      }
    }
    return m;
  }, [filtered, compact, expanded]);

  const edgeGeometries = useMemo<EdgeGeometry[]>(() => {
    if (!filtered) return [];
    const lookup = (id: string, col: string) => rowOffsets.get(id)?.get(col);
    const out: EdgeGeometry[] = [];
    for (const e of filtered.edges) {
      const geo = edgeGeometry(e, positions, lookup);
      if (geo) out.push(geo);
    }
    return out;
  }, [filtered, positions, rowOffsets]);

  // Graph-space rectangle currently in view (plus a margin so panning doesn't
  // pop nodes in at the edge). Used to cull off-screen content from the DOM —
  // the big win when zoomed in on a large/expanded table.
  const viewRect = useMemo(() => {
    const margin = 400;
    return {
      x0: -view.tx / view.k - margin,
      y0: -view.ty / view.k - margin,
      x1: (size.w - view.tx) / view.k + margin,
      y1: (size.h - view.ty) / view.k + margin,
    };
  }, [view, size]);

  const visibleNodes = useMemo(() => {
    if (!filtered) return [];
    if (size.w === 0) return filtered.nodes;
    return filtered.nodes.filter((n) => {
      const b = positions[n.id];
      return (
        b &&
        b.x <= viewRect.x1 &&
        b.x + b.width >= viewRect.x0 &&
        b.y <= viewRect.y1 &&
        b.y + b.height >= viewRect.y0
      );
    });
  }, [filtered, positions, viewRect, size.w]);

  const visibleEdges = useMemo(() => {
    if (size.w === 0) return edgeGeometries;
    const visIds = new Set(visibleNodes.map((n) => n.id));
    return edgeGeometries.filter((g) => {
      if (visIds.has(g.edge.source) || visIds.has(g.edge.target)) return true;
      // Keep long edges that cross the viewport even when both ends are off-screen.
      return (
        Math.min(g.sx, g.tx) <= viewRect.x1 &&
        Math.max(g.sx, g.tx) >= viewRect.x0 &&
        Math.min(g.sy, g.ty) <= viewRect.y1 &&
        Math.max(g.sy, g.ty) >= viewRect.y0
      );
    });
  }, [edgeGeometries, visibleNodes, viewRect, size.w]);

  const fit = useCallback(() => {
    const el = containerRef.current;
    if (!el) return;
    const b = graphBounds(positions);
    const w = el.clientWidth;
    const h = el.clientHeight;
    if (b.width === 0 || b.height === 0) {
      setView({ tx: w / 2, ty: h / 2, k: 1 });
      return;
    }
    const pad = 48;
    const k = clamp(
      Math.min((w - pad * 2) / b.width, (h - pad * 2) / b.height),
      MIN_ZOOM,
      1.2,
    );
    setView({
      tx: (w - b.width * k) / 2 - b.minX * k,
      ty: (h - b.height * k) / 2 - b.minY * k,
      k,
    });
  }, [positions]);

  // Fit once when the graph first becomes ready.
  useEffect(() => {
    if (status === "ready" && !fittedRef.current) {
      fittedRef.current = true;
      fit();
    }
  }, [status, fit]);

  // Track the viewport size for culling.
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const update = () => setSize({ w: el.clientWidth, h: el.clientHeight });
    update();
    const ro = new ResizeObserver(update);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  // Native non-passive wheel listener so we can preventDefault on zoom.
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const rect = el.getBoundingClientRect();
      const px = e.clientX - rect.left;
      const py = e.clientY - rect.top;
      setView((v) => {
        const k = clamp(v.k * Math.exp(-e.deltaY * 0.0015), MIN_ZOOM, MAX_ZOOM);
        const ratio = k / v.k;
        return {
          k,
          tx: px - (px - v.tx) * ratio,
          ty: py - (py - v.ty) * ratio,
        };
      });
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, []);

  const onPointerMove = useCallback((e: PointerEvent) => {
    const drag = dragRef.current;
    if (!drag) return;
    const dx = e.clientX - drag.startX;
    const dy = e.clientY - drag.startY;
    if (!drag.moved && Math.hypot(dx, dy) > CLICK_THRESHOLD) {
      drag.moved = true;
    }
    if (!drag.moved) return;
    if (drag.mode === "pan") {
      setView((v) => ({ ...v, tx: drag.origTx + dx, ty: drag.origTy + dy }));
    } else if (drag.id) {
      const k = viewRef.current.k;
      const id = drag.id;
      setOverrides((o) => ({
        ...o,
        [id]: { x: drag.origX + dx / k, y: drag.origY + dy / k },
      }));
    }
  }, []);

  const endDrag = useCallback(() => {
    const drag = dragRef.current;
    if (!drag) return;
    dragRef.current = null;
    window.removeEventListener("pointermove", onPointerMove);
    window.removeEventListener("pointerup", endDrag);
    // A node press that never moved is a click → open the table grid.
    if (drag.mode === "node" && drag.id && !drag.moved) {
      const node = graphRef.current?.nodes.find((n) => n.id === drag.id);
      if (node) {
        setFocusedId(node.id);
        openTable(tab.connectionId, tab.database, node.schema, node.name);
      }
    }
  }, [openTable, tab.connectionId, tab.database, onPointerMove]);

  const beginPan = useCallback(
    (e: ReactPointerEvent) => {
      if (e.button !== 0) return;
      // Stop the browser from starting a text/range selection on the SVG.
      e.preventDefault();
      const v = viewRef.current;
      dragRef.current = {
        mode: "pan",
        pointerId: e.pointerId,
        startX: e.clientX,
        startY: e.clientY,
        origTx: v.tx,
        origTy: v.ty,
        origX: 0,
        origY: 0,
        moved: false,
      };
      window.addEventListener("pointermove", onPointerMove);
      window.addEventListener("pointerup", endDrag);
    },
    [onPointerMove, endDrag],
  );

  const beginNodeDrag = useCallback(
    (e: ReactPointerEvent, id: string) => {
      if (e.button !== 0) return;
      e.stopPropagation();
      e.preventDefault();
      const box = positionsRef.current[id];
      if (!box) return;
      const v = viewRef.current;
      dragRef.current = {
        mode: "node",
        id,
        pointerId: e.pointerId,
        startX: e.clientX,
        startY: e.clientY,
        origTx: v.tx,
        origTy: v.ty,
        origX: box.x,
        origY: box.y,
        moved: false,
      };
      window.addEventListener("pointermove", onPointerMove);
      window.addEventListener("pointerup", endDrag);
    },
    [onPointerMove, endDrag],
  );

  const focusNode = useCallback((id: string) => {
    const box = positionsRef.current[id];
    const el = containerRef.current;
    if (!box || !el) return;
    setFocusedId(id);
    setView((v) => ({
      ...v,
      tx: el.clientWidth / 2 - (box.x + box.width / 2) * v.k,
      ty: el.clientHeight / 2 - (box.y + box.height / 2) * v.k,
    }));
  }, []);

  const autoArrange = useCallback(() => {
    setOverrides({});
    fittedRef.current = false;
    // Re-fit on the next frame once positions recompute.
    requestAnimationFrame(() => {
      fittedRef.current = true;
      fit();
    });
  }, [fit]);

  const zoomBy = useCallback((factor: number) => {
    const el = containerRef.current;
    const cx = (el?.clientWidth ?? 0) / 2;
    const cy = (el?.clientHeight ?? 0) / 2;
    setView((v) => {
      const k = clamp(v.k * factor, MIN_ZOOM, MAX_ZOOM);
      const ratio = k / v.k;
      return { k, tx: cx - (cx - v.tx) * ratio, ty: cy - (cy - v.ty) * ratio };
    });
  }, []);

  const toggleExpand = useCallback((id: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  const toggleSchema = useCallback((schema: string) => {
    setHidden((prev) => {
      const next = new Set(prev);
      if (next.has(schema)) next.delete(schema);
      else next.add(schema);
      return next;
    });
  }, []);

  const allSchemas = graph?.schemas ?? [];
  const visibleCount = filtered?.nodes.length ?? 0;
  const edgeCount = filtered?.edges.length ?? 0;

  return (
    <div className="flex min-h-0 flex-1 flex-col bg-bg-inset">
      <Toolbar
        zoom={view.k}
        compact={compact}
        schemas={allSchemas}
        hidden={hidden}
        schemaMenuOpen={schemaMenuOpen}
        nodeCount={visibleCount}
        edgeCount={edgeCount}
        onZoomIn={() => zoomBy(1.2)}
        onZoomOut={() => zoomBy(1 / 1.2)}
        onFit={fit}
        onAutoArrange={autoArrange}
        onToggleCompact={() => setCompact((c) => !c)}
        onToggleSchemaMenu={() => setSchemaMenuOpen((o) => !o)}
        onToggleSchema={toggleSchema}
        onRefresh={load}
      />

      <div
        ref={containerRef}
        className="relative min-h-0 flex-1 overflow-hidden"
        style={{ cursor: "grab", userSelect: "none", WebkitUserSelect: "none" }}
        onPointerDown={beginPan}
      >
        {status === "loading" && (
          <Centered>
            <span className="animate-sb-pulse">building diagram…</span>
          </Centered>
        )}
        {status === "error" && (
          <Centered>
            <span className="text-warn">
              Could not build diagram. {error}
            </span>
          </Centered>
        )}
        {status === "ready" && visibleCount === 0 && (
          <Centered>
            <span>
              No tables to show
              {hidden.size > 0 ? " (every schema is hidden)" : ""}.
            </span>
          </Centered>
        )}
        {status === "ready" && visibleCount > 0 && (
          <svg className="h-full w-full" style={{ display: "block" }}>
            <defs>
              <marker
                id="er-arrow"
                viewBox="0 0 10 10"
                refX={9}
                refY={5}
                markerWidth={7}
                markerHeight={7}
                orient="auto-start-reverse"
              >
                <path d="M0 0L10 5L0 10z" fill="var(--fg-3)" />
              </marker>
            </defs>
            <g transform={`translate(${view.tx},${view.ty}) scale(${view.k})`}>
              {visibleEdges.map((geo) => (
                <EdgeLine key={geo.id} geo={geo} onFocus={focusNode} />
              ))}
              {visibleNodes.map((node) => {
                const box = positions[node.id];
                if (!box) return null;
                return (
                  <ErNodeCard
                    key={node.id}
                    node={node}
                    box={box}
                    compact={compact}
                    expanded={expanded.has(node.id)}
                    focused={focusedId === node.id}
                    onPointerDown={beginNodeDrag}
                    onToggleExpand={toggleExpand}
                  />
                );
              })}
            </g>
          </svg>
        )}
      </div>
    </div>
  );
}

const EdgeLine = memo(function EdgeLine({
  geo,
  onFocus,
}: {
  geo: EdgeGeometry;
  onFocus: (target: string) => void;
}) {
  const label = geo.edge.source_columns.join(", ");
  // Control points sit at the horizontal midpoint so the line leaves and meets
  // each field row horizontally — making the connection point unambiguous.
  const midX = (geo.sx + geo.tx) / 2;
  const path = geo.selfLoop
    ? `M ${geo.sx} ${geo.sy} C ${geo.sx + 44} ${geo.sy} ${geo.tx + 44} ${geo.ty} ${geo.tx} ${geo.ty}`
    : `M ${geo.sx} ${geo.sy} C ${midX} ${geo.sy} ${midX} ${geo.ty} ${geo.tx} ${geo.ty}`;
  return (
    <g
      style={{ cursor: "pointer" }}
      onClick={(e) => {
        e.stopPropagation();
        onFocus(geo.edge.target);
      }}
    >
      <title>
        {geo.edge.constraint_name}: {label} → {geo.edge.target_columns.join(", ")}
      </title>
      {/* Fat transparent hit area so thin edges are easy to click. */}
      <path d={path} fill="none" stroke="transparent" strokeWidth={10} />
      <path
        d={path}
        fill="none"
        stroke="var(--fg-3)"
        strokeWidth={1}
        markerEnd="url(#er-arrow)"
      />
      {label && (
        <text
          className="font-mono"
          x={geo.labelX}
          y={geo.labelY}
          textAnchor="middle"
          dominantBaseline="central"
          fontSize={9}
          fill="var(--fg-2)"
          style={{ paintOrder: "stroke" }}
          stroke="var(--bg-inset)"
          strokeWidth={3}
        >
          {label}
        </text>
      )}
    </g>
  );
});

function Toolbar(props: {
  zoom: number;
  compact: boolean;
  schemas: string[];
  hidden: Set<string>;
  schemaMenuOpen: boolean;
  nodeCount: number;
  edgeCount: number;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onFit: () => void;
  onAutoArrange: () => void;
  onToggleCompact: () => void;
  onToggleSchemaMenu: () => void;
  onToggleSchema: (schema: string) => void;
  onRefresh: () => void;
}) {
  return (
    <div className="relative flex h-7 shrink-0 items-center gap-1 border-b border-border-default bg-bg-1 px-2 text-[11px] text-fg-2">
      <button type="button" className="icon-btn" title="Zoom out" onClick={props.onZoomOut}>
        <Icon.minus size={12} />
      </button>
      <span className="w-9 text-center font-mono text-[10px] text-fg-3">
        {Math.round(props.zoom * 100)}%
      </span>
      <button type="button" className="icon-btn" title="Zoom in" onClick={props.onZoomIn}>
        <Icon.plus size={12} />
      </button>
      <button type="button" className="icon-btn" title="Fit to view" onClick={props.onFit}>
        <Icon.expand size={12} />
      </button>
      <button
        type="button"
        className="icon-btn"
        title="Auto-arrange"
        onClick={props.onAutoArrange}
      >
        <Icon.layout size={12} />
      </button>

      <span className="mx-1 h-3.5 w-px bg-border-default" />

      <button
        type="button"
        className={
          "inline-flex h-[20px] items-center gap-1 rounded-[4px] border px-1.5 text-[10.5px] " +
          (props.compact
            ? "border-border-default bg-bg-2 text-fg-2 hover:text-fg-0"
            : "border-accent-line bg-accent-soft text-accent")
        }
        aria-pressed={!props.compact}
        title={
          props.compact
            ? "Showing key columns only — click to show all fields"
            : "Showing all fields — click to show key columns only"
        }
        onClick={props.onToggleCompact}
      >
        <Icon.format size={11} />
        {props.compact ? "Keys only" : "All fields"}
      </button>

      {props.schemas.length > 0 && (
        <button
          type="button"
          className={"icon-btn" + (props.hidden.size > 0 ? " active" : "")}
          title="Show/hide schemas"
          onClick={props.onToggleSchemaMenu}
        >
          <Icon.eye size={12} />
        </button>
      )}

      <button type="button" className="icon-btn" title="Refresh" onClick={props.onRefresh}>
        <Icon.history size={12} />
      </button>

      <span className="ml-auto font-mono text-[10px] text-fg-3">
        {props.nodeCount} tables · {props.edgeCount} refs
      </span>

      {props.schemaMenuOpen && props.schemas.length > 0 && (
        <div className="absolute left-2 top-[30px] z-20 max-h-[260px] w-52 overflow-y-auto rounded-[6px] border border-border-default bg-bg-1 py-1 shadow-xl">
          {props.schemas.map((schema) => {
            const checked = !props.hidden.has(schema);
            return (
              <label
                key={schema}
                className="flex h-6 cursor-pointer items-center gap-2 px-2 text-[11px] text-fg-1 hover:bg-bg-2"
              >
                <input
                  type="checkbox"
                  checked={checked}
                  onChange={() => props.onToggleSchema(schema)}
                  className="h-3 w-3 accent-[var(--accent)]"
                />
                <span className="min-w-0 flex-1 overflow-hidden text-ellipsis whitespace-nowrap font-mono">
                  {schema}
                </span>
              </label>
            );
          })}
        </div>
      )}
    </div>
  );
}

function Centered({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex h-full items-center justify-center text-[11.5px] text-fg-3">
      {children}
    </div>
  );
}

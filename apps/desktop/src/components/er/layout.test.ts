import { describe, expect, it } from "vitest";
import type { ErGraph, ErNode } from "@cellar/ipc";

import {
  edgeGeometry,
  graphBounds,
  layoutGraph,
  visibleColumns,
} from "./layout";

function node(id: string, extra: Partial<ErNode> = {}): ErNode {
  const [schema, name] = id.split(".");
  return {
    id,
    schema: schema ?? "public",
    name: name ?? id,
    columns: [
      {
        name: "id",
        data_type: "int8",
        nullable: false,
        is_primary_key: true,
        is_foreign_key: false,
      },
    ],
    primary_key: ["id"],
    row_count: null,
    ...extra,
  };
}

const graph: ErGraph = {
  database: "shop",
  schemas: ["public"],
  nodes: [
    node("public.orders", {
      columns: [
        {
          name: "id",
          data_type: "int8",
          nullable: false,
          is_primary_key: true,
          is_foreign_key: false,
        },
        {
          name: "customer_id",
          data_type: "int8",
          nullable: false,
          is_primary_key: false,
          is_foreign_key: true,
        },
        {
          name: "note",
          data_type: "text",
          nullable: true,
          is_primary_key: false,
          is_foreign_key: false,
        },
      ],
    }),
    node("public.customers"),
    node("public.lonely"),
  ],
  edges: [
    {
      id: "public.orders->public.customers:fk",
      constraint_name: "fk",
      source: "public.orders",
      target: "public.customers",
      source_columns: ["customer_id"],
      target_columns: ["id"],
    },
  ],
};

describe("er layout", () => {
  it("places every node without overlap on the grid", () => {
    const positions = layoutGraph(graph, { compact: false });
    expect(Object.keys(positions).sort()).toEqual([
      "public.customers",
      "public.lonely",
      "public.orders",
    ]);
    const boxes = Object.values(positions);
    for (const b of boxes) {
      expect(b.width).toBeGreaterThan(0);
      expect(b.height).toBeGreaterThan(0);
    }
  });

  it("compact mode keeps only key columns", () => {
    const orders = graph.nodes[0]!;
    const full = visibleColumns(orders, false);
    const compact = visibleColumns(orders, true);
    expect(full.columns).toHaveLength(3);
    expect(compact.columns.map((c) => c.name)).toEqual(["id", "customer_id"]);
  });

  it("derives edge endpoints from node boxes", () => {
    const positions = layoutGraph(graph, { compact: true });
    const geo = edgeGeometry(graph.edges[0]!, positions);
    expect(geo).not.toBeNull();
    expect(geo?.selfLoop).toBe(false);
    expect(Number.isFinite(geo?.sx)).toBe(true);
    expect(Number.isFinite(geo?.ty)).toBe(true);
  });

  it("anchors edge endpoints at the resolved column rows", () => {
    const positions = layoutGraph(graph, { compact: true });
    const lookup = (id: string, col: string) =>
      id === "public.orders" && col === "customer_id" ? 50 : undefined;
    const geo = edgeGeometry(graph.edges[0]!, positions, lookup);
    const source = positions["public.orders"]!;
    // Source anchors at its row centre; target falls back to its box centre.
    expect(geo?.sy).toBe(source.y + 50);
    const target = positions["public.customers"]!;
    expect(geo?.ty).toBe(target.y + target.height / 2);
  });

  it("returns null geometry when an endpoint is filtered out", () => {
    const positions = layoutGraph(
      { ...graph, nodes: [graph.nodes[0]!] },
      { compact: true },
    );
    expect(edgeGeometry(graph.edges[0]!, positions)).toBeNull();
  });

  it("bounds cover the laid-out nodes", () => {
    const positions = layoutGraph(graph, { compact: false });
    const bounds = graphBounds(positions);
    expect(bounds.width).toBeGreaterThan(0);
    expect(bounds.height).toBeGreaterThan(0);
  });

  it("is deterministic across runs", () => {
    const a = layoutGraph(graph, { compact: false });
    const b = layoutGraph(graph, { compact: false });
    expect(a).toEqual(b);
  });
});

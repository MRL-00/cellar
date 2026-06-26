/**
 * Renderer registry: resolves the highest-priority renderer that claims a cell.
 * The built-in set covers JSON/JSONB, arrays, bytea/blob, and a geometry stub;
 * hosts and plugins compose their own renderers via `createRendererRegistry` or
 * `extendRegistry`.
 */
import { arrayRenderer } from "./array";
import { byteaRenderer } from "./bytes";
import { geometryRenderer } from "./geometry";
import { jsonRenderer } from "./json";
import type { CellRenderer, RendererRegistry } from "./types";

export function createRendererRegistry(
  renderers: readonly CellRenderer[],
): RendererRegistry {
  const sorted = [...renderers].sort(
    (a, b) => (b.priority ?? 0) - (a.priority ?? 0),
  );
  return {
    renderers: sorted,
    resolve(column, value) {
      for (const renderer of sorted) {
        if (renderer.appliesTo(column, value)) return renderer;
      }
      return null;
    },
  };
}

/** The built-in renderers, in declaration order (priority decides ties). */
export const builtinRenderers: readonly CellRenderer[] = [
  jsonRenderer,
  arrayRenderer,
  byteaRenderer,
  geometryRenderer,
];

/** Default registry used by the grid when a host does not supply its own. */
export const defaultRendererRegistry: RendererRegistry =
  createRendererRegistry(builtinRenderers);

/** Prepend custom renderers (they win ties at equal priority) to a base set. */
export function extendRegistry(
  base: RendererRegistry,
  extra: readonly CellRenderer[],
): RendererRegistry {
  return createRendererRegistry([...extra, ...base.renderers]);
}

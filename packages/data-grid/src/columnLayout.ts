import type { GridColumn, GridColumnLayout } from "./types";

export const MIN_COLUMN_WIDTH = 56;

export function emptyColumnLayout(): GridColumnLayout {
  return { order: [], widths: {} };
}

export function layoutForColumns(
  columns: readonly GridColumn[],
  layout: GridColumnLayout,
): GridColumn[] {
  const byKey = new Map(columns.map((column) => [column.key, column]));
  const orderedKeys = [
    ...layout.order.filter((key) => byKey.has(key)),
    ...columns
      .map((column) => column.key)
      .filter((key) => !layout.order.includes(key)),
  ];
  return orderedKeys.flatMap((key) => {
    const column = byKey.get(key);
    if (!column) return [];
    return [{ ...column, width: layout.widths[key] ?? column.width }];
  });
}

export function pruneColumnLayout(
  columns: readonly GridColumn[],
  layout: GridColumnLayout,
): GridColumnLayout {
  const keys = new Set(columns.map((column) => column.key));
  return {
    order: layout.order.filter((key) => keys.has(key)),
    widths: Object.fromEntries(
      Object.entries(layout.widths).filter(([key]) => keys.has(key)),
    ),
  };
}

export function sameColumnLayout(
  left: GridColumnLayout,
  right: GridColumnLayout,
): boolean {
  if (left.order.length !== right.order.length) return false;
  if (left.order.some((key, index) => key !== right.order[index])) return false;
  const leftWidths = Object.entries(left.widths);
  if (leftWidths.length !== Object.keys(right.widths).length) return false;
  return leftWidths.every(([key, width]) => right.widths[key] === width);
}

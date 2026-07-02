const MIN_ROW_OVERSCAN = 24;
const OVERSCAN_VIEWPORTS = 3;

export const HEADER_HEIGHT = 26;
export const DEFAULT_ROW_HEIGHT = 22;
// Below this, full row flow is cheap enough; above it we window the rows so a
// wide table never puts thousands of cells in the DOM (which makes every
// interaction — typing in the filter included — janky). The default page size
// is 500, so this must sit under it or ordinary pages never virtualize.
export const VIRTUAL_ROW_THRESHOLD = 100;

export function calculateVirtualRows({
  rowCount,
  viewportHeight,
  scrollTop,
  rowHeight,
}: {
  rowCount: number;
  viewportHeight: number;
  scrollTop: number;
  rowHeight: number;
}) {
  const measuredRowHeight = rowHeight || DEFAULT_ROW_HEIGHT;
  const bodyScrollTop = Math.max(0, scrollTop - HEADER_HEIGHT);
  const measuredViewportHeight = viewportHeight || measuredRowHeight * 30;
  const visibleCount = Math.ceil(
    (measuredViewportHeight + HEADER_HEIGHT) / measuredRowHeight,
  );
  const overscan = Math.max(
    MIN_ROW_OVERSCAN,
    visibleCount * OVERSCAN_VIEWPORTS,
  );
  const first = Math.max(
    0,
    Math.floor(bodyScrollTop / measuredRowHeight) - overscan,
  );
  const last = Math.min(rowCount, first + visibleCount + overscan * 2);

  return {
    first,
    last,
    totalHeight: rowCount * measuredRowHeight,
  };
}

export function shouldVirtualizeRows(rowCount: number): boolean {
  return rowCount > VIRTUAL_ROW_THRESHOLD;
}

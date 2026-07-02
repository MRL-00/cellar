import { GridIcon } from "./icons";
import type { GridPagination } from "./types";

export function PaginationBar({
  pagination,
  rowCount,
}: {
  pagination: GridPagination;
  rowCount: number;
}) {
  const options = pagination.pageSizeOptions ?? [100, 250, 500];
  const firstRow = rowCount === 0 ? pagination.offset : pagination.offset + 1;
  const lastRow = pagination.offset + rowCount;
  const totalRows = pagination.totalRows ?? null;

  const range =
    rowCount === 0
      ? `No rows at offset ${pagination.offset}`
      : `Rows ${formatNumber(firstRow)}–${formatNumber(lastRow)}${
          totalRows !== null ? ` of ${formatNumber(totalRows)}` : ""
        }`;
  const nextRangeStart = pagination.offset + pagination.limit + 1;
  const nextRangeEnd = pagination.offset + pagination.limit * 2;

  return (
    <div className="grid-pagination">
      <div className="grid-pagination-range">
        <span className="tnum">{range}</span>
        {pagination.hasNext && totalRows === null && (
          <span className="grid-pagination-more">more available</span>
        )}
      </div>
      <div className="grid-pagination-controls">
        {pagination.onPageSizeChange && (
          <label className="grid-pagination-size">
            <span>Page size</span>
            <select
              value={pagination.limit}
              onChange={(e) => pagination.onPageSizeChange?.(Number(e.target.value))}
              disabled={pagination.loading}
              aria-label="Page size"
            >
              {options.map((option) => (
                <option key={option} value={option}>
                  {option}
                </option>
              ))}
            </select>
          </label>
        )}
        <button
          className="grid-pagination-btn"
          type="button"
          onClick={pagination.onPrevious}
          disabled={!pagination.hasPrevious || pagination.loading}
          title="Previous page"
          aria-label="Previous page"
        >
          <GridIcon.chevronLeft size={12} />
        </button>
        <button
          className="grid-pagination-btn"
          type="button"
          onClick={pagination.onNext}
          disabled={!pagination.hasNext || pagination.loading}
          title={
            pagination.hasNext
              ? `Next page: rows ${formatNumber(nextRangeStart)}–${formatNumber(nextRangeEnd)}`
              : "Next page"
          }
          aria-label="Next page"
        >
          <GridIcon.chevronRight size={12} />
        </button>
      </div>
    </div>
  );
}

function formatNumber(value: number): string {
  return new Intl.NumberFormat("en-US").format(value);
}

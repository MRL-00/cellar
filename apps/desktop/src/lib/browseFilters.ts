import { normalizeLikePattern, operatorsForColumn } from "@cellar/data-grid";
import type {
  FilterClause,
  FilterOperator,
  GridColumn,
  SortState,
} from "@cellar/data-grid";
import type {
  TableFilterClause,
  TableFilterOperator,
  TableSortClause,
} from "@cellar/ipc";

/**
 * Maps the grid's advanced filter chips and the toolbar quick filter into the
 * server-side `TableBrowseRequest` filter/sort model so browse applies them to
 * the whole table instead of just the loaded page.
 */

const ADVANCED_OPERATOR_MAP: Partial<Record<FilterOperator, TableFilterOperator>> = {
  equals: "equals",
  notEquals: "not_equals",
  contains: "contains",
  notContains: "not_contains",
  startsWith: "starts_with",
  endsWith: "ends_with",
  like: "like",
  greaterThan: "greater_than",
  greaterThanOrEqual: "greater_than_or_equal",
  lessThan: "less_than",
  lessThanOrEqual: "less_than_or_equal",
  isNull: "is_null",
  isNotNull: "is_not_null",
};

function operatorNeedsValue(operator: TableFilterOperator): boolean {
  return operator !== "is_null" && operator !== "is_not_null";
}

export function advancedFiltersToClauses(
  filters: readonly FilterClause[],
): TableFilterClause[] {
  // The drivers AND every clause together with no grouping. If the chips use
  // OR we can't express it server-side, so push nothing and let the grid keep
  // filtering the loaded page locally (the pre-existing behavior).
  // ponytail: OR-across-chips needs a backend grouping model — deferred (W3).
  if (filters.some((clause, index) => index > 0 && clause.logic === "or")) {
    return [];
  }

  const clauses: TableFilterClause[] = [];
  for (const filter of filters) {
    const operator = ADVANCED_OPERATOR_MAP[filter.operator];
    if (!operator) continue;
    const needsValue = operatorNeedsValue(operator);
    let value = (filter.value ?? "").trim();
    if (needsValue && value.length === 0) continue;
    // Match the grid's local `like` semantics: bare text means %text%.
    if (operator === "like") value = normalizeLikePattern(value);
    clauses.push({
      column: filter.columnKey,
      operator,
      value: needsValue ? value : null,
    });
  }
  return clauses;
}

const ID_VALUE = /^-?\d+$/;

/** A text-ish column accepts `contains` (ILIKE) — reuse the grid's own rule. */
export function isTextColumn(column: GridColumn): boolean {
  return operatorsForColumn(column).includes("contains");
}

export function firstTextColumnKey(columns: readonly GridColumn[]): string | null {
  return columns.find(isTextColumn)?.key ?? null;
}

function idLikeColumnKey(columns: readonly GridColumn[]): string | null {
  return (
    columns.find((column) => column.pk)?.key ??
    columns.find((column) => column.key.toLowerCase() === "id")?.key ??
    null
  );
}

export function quickFilterToClause(
  quickFilter: string,
  columns: readonly GridColumn[],
  targetColumnKey: string | null,
): TableFilterClause | null {
  const value = quickFilter.trim();
  if (value.length === 0) return null;

  // Numeric value + an id/PK column → exact match on the id.
  if (ID_VALUE.test(value)) {
    const idColumn = idLikeColumnKey(columns);
    if (idColumn) return { column: idColumn, operator: "equals", value };
  }

  // Otherwise a contains search on the chosen (or first) text column.
  const target = targetColumnKey ?? firstTextColumnKey(columns);
  if (!target) return null;
  return { column: target, operator: "contains", value };
}

export function buildBrowseFilters(
  columns: readonly GridColumn[],
  advancedFilters: readonly FilterClause[],
  quickFilter: string,
  quickFilterColumn: string | null,
): TableFilterClause[] {
  const clauses = advancedFiltersToClauses(advancedFilters);
  const quick = quickFilterToClause(quickFilter, columns, quickFilterColumn);
  return quick ? [...clauses, quick] : clauses;
}

export function sortToClauses(sort: SortState): TableSortClause[] {
  if (!sort) return [];
  return [{ column: sort.columnKey, direction: sort.direction }];
}

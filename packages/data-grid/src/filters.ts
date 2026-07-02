import type {
  FilterClause,
  FilterOperator,
  GridColumn,
  GridRow,
  PendingChanges,
} from "./types";

export const FILTER_OPERATORS: readonly {
  value: FilterOperator;
  label: string;
  needsValue: boolean;
}[] = [
  { value: "equals", label: "=", needsValue: true },
  { value: "notEquals", label: "!=", needsValue: true },
  { value: "contains", label: "contains", needsValue: true },
  { value: "notContains", label: "not contains", needsValue: true },
  { value: "startsWith", label: "starts with", needsValue: true },
  { value: "endsWith", label: "ends with", needsValue: true },
  { value: "like", label: "like", needsValue: true },
  { value: "greaterThan", label: ">", needsValue: true },
  { value: "greaterThanOrEqual", label: ">=", needsValue: true },
  { value: "lessThan", label: "<", needsValue: true },
  { value: "lessThanOrEqual", label: "<=", needsValue: true },
  { value: "isNull", label: "is null", needsValue: false },
  { value: "isNotNull", label: "is not null", needsValue: false },
];

const NUMERIC_TYPES = [
  "int",
  "int2",
  "int4",
  "int8",
  "integer",
  "bigint",
  "smallint",
  "serial",
  "bigserial",
  "float",
  "float4",
  "float8",
  "real",
  "double",
  "numeric",
  "decimal",
  "money",
  "oid",
];

const DATE_TYPES = ["date", "time", "timestamp", "timestamptz", "timetz"];
const BOOL_TYPES = ["bool", "boolean"];
const TEXT_TYPES = [
  "text",
  "char",
  "varchar",
  "citext",
  "uuid",
  "guid",
  "uniqueidentifier",
  "json",
  "jsonb",
];

function operatorMeta(operator: FilterOperator) {
  return FILTER_OPERATORS.find((op) => op.value === operator) ?? FILTER_OPERATORS[0]!;
}

export function filterOperatorLabel(operator: FilterOperator): string {
  return operatorMeta(operator).label;
}

export function filterNeedsValue(operator: FilterOperator): boolean {
  return operatorMeta(operator).needsValue;
}

export function operatorsForColumn(column: GridColumn): FilterOperator[] {
  const base: FilterOperator[] = ["equals", "notEquals"];
  const category = columnCategory(column);

  if (category === "text") {
    base.push("contains", "notContains", "startsWith", "endsWith", "like");
  }

  if (category === "number" || category === "date") {
    base.push("greaterThan", "greaterThanOrEqual", "lessThan", "lessThanOrEqual");
  }

  if (column.nullable) {
    base.push("isNull", "isNotNull");
  }

  return base;
}

export function nextOperatorForColumn(
  column: GridColumn,
  operator: FilterOperator,
): FilterOperator {
  const operators = operatorsForColumn(column);
  return operators.includes(operator) ? operator : operators[0]!;
}

export function filterRows(
  rows: readonly GridRow[],
  columns: readonly GridColumn[],
  filters: readonly FilterClause[],
  changes: PendingChanges = {},
): GridRow[] {
  if (filters.length === 0) return [...rows];
  const columnsByKey = new Map(columns.map((column) => [column.key, column]));
  const validFilters = filters.filter((clause) => {
    const column = columnsByKey.get(clause.columnKey);
    return column && operatorsForColumn(column).includes(clause.operator);
  });

  if (validFilters.length === 0) return [...rows];
  return rows.filter((row) => rowMatchesFilters(row, columnsByKey, validFilters, changes));
}

export function rowMatchesFilters(
  row: GridRow,
  columnsByKey: ReadonlyMap<string, GridColumn>,
  filters: readonly FilterClause[],
  changes: PendingChanges = {},
): boolean {
  let matched = false;

  filters.forEach((clause, index) => {
    const column = columnsByKey.get(clause.columnKey);
    const clauseMatched = column
      ? evaluateFilterClause(displayValue(row, clause.columnKey, changes), column, clause)
      : true;
    if (index === 0) {
      matched = clauseMatched;
      return;
    }
    matched =
      clause.logic === "or"
        ? matched || clauseMatched
        : matched && clauseMatched;
  });

  return matched;
}

export function evaluateFilterClause(
  rawValue: GridRow[string],
  column: GridColumn,
  clause: FilterClause,
): boolean {
  if (!operatorsForColumn(column).includes(clause.operator)) return true;
  const value = rawValue ?? null;

  if (clause.operator === "isNull") return value === null;
  if (clause.operator === "isNotNull") return value !== null;

  const needle = clause.value ?? "";
  if (needle.length === 0) return true;

  const category = columnCategory(column);
  if (
    clause.operator === "greaterThan" ||
    clause.operator === "greaterThanOrEqual" ||
    clause.operator === "lessThan" ||
    clause.operator === "lessThanOrEqual"
  ) {
    return compareOrdered(value, needle, category, clause.operator);
  }

  const current = normalizeString(value);
  const expected = normalizeString(needle);

  if (clause.operator === "equals") return current === expected;
  if (clause.operator === "notEquals") return current !== expected;
  if (clause.operator === "contains") return current.includes(expected);
  if (clause.operator === "notContains") return !current.includes(expected);
  if (clause.operator === "startsWith") return current.startsWith(expected);
  if (clause.operator === "endsWith") return current.endsWith(expected);
  if (clause.operator === "like") return likeMatch(current, expected);
  return true;
}

export function filterValuePreview(clause: FilterClause): string {
  if (!filterNeedsValue(clause.operator)) return "";
  const value = clause.value ?? "";
  return `"${value.replaceAll('"', '\\"')}"`;
}

export function createFilterId(): string {
  return crypto.randomUUID(); // ponytail: Tauri target is chrome105+, always has crypto.randomUUID
}

function displayValue(
  row: GridRow,
  columnKey: string,
  changes: PendingChanges,
): GridRow[string] {
  return changes[row.id]?.edits[columnKey]?.to ?? row[columnKey];
}

function compareOrdered(
  value: GridRow[string],
  needle: string,
  category: ReturnType<typeof columnCategory>,
  operator: "greaterThan" | "greaterThanOrEqual" | "lessThan" | "lessThanOrEqual",
): boolean {
  if (value === null || value === undefined) return false;

  let left: number;
  let right: number;
  if (category === "number") {
    left = Number(value);
    right = Number(needle);
  } else if (category === "date") {
    left = Date.parse(String(value));
    right = Date.parse(needle);
  } else {
    return false;
  }

  if (!Number.isFinite(left) || !Number.isFinite(right)) return false;
  switch (operator) {
    case "greaterThan":
      return left > right;
    case "greaterThanOrEqual":
      return left >= right;
    case "lessThan":
      return left < right;
    case "lessThanOrEqual":
      return left <= right;
  }
}

/**
 * SQL LIKE semantics: `%` matches any run, `_` matches one char. Case-insensitive.
 * A pattern with no wildcards is treated as `%pattern%` so plain text "just works";
 * typing any `%`/`_` switches to exact SQL semantics.
 */
function likeMatch(value: string, pattern: string): boolean {
  if (!pattern.includes("%") && !pattern.includes("_")) {
    pattern = `%${pattern}%`;
  }
  const regex = pattern
    .replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
    .replaceAll("%", ".*")
    .replaceAll("_", ".");
  return new RegExp(`^${regex}$`).test(value);
}

function columnCategory(
  column: GridColumn,
): "text" | "number" | "date" | "bool" | "unknown" {
  if (column.enum) return "text";
  const type = column.type.toLowerCase();
  if (BOOL_TYPES.some((t) => type.includes(t))) return "bool";
  if (NUMERIC_TYPES.some((t) => type.includes(t))) return "number";
  if (DATE_TYPES.some((t) => type.includes(t))) return "date";
  if (TEXT_TYPES.some((t) => type.includes(t))) return "text";
  return "unknown";
}

function normalizeString(value: GridRow[string]): string {
  if (value === null || value === undefined) return "";
  return String(value).toLowerCase();
}


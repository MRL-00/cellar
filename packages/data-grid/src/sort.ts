import type {
  GridColumn,
  GridRow,
  GridValue,
  PendingChanges,
  SortState,
} from "./types";

export function cycleSortState(
  current: SortState,
  columnKey: string,
): SortState {
  // Database grids benefit from a reversible three-state cycle: first click
  // sorts ascending, second descending, third returns to the source/page order.
  if (!current || current.columnKey !== columnKey) {
    return { columnKey, direction: "asc" };
  }
  if (current.direction === "asc") {
    return { columnKey, direction: "desc" };
  }
  return null;
}

export function sortGridRows(
  rows: readonly GridRow[],
  columns: readonly GridColumn[],
  sort: SortState,
  changes: PendingChanges = {},
): GridRow[] {
  if (!sort) return [...rows];

  const column = columns.find((c) => c.key === sort.columnKey);
  if (!column) return [...rows];

  const direction = sort.direction === "asc" ? 1 : -1;
  return rows
    .map((row, index) => ({ row, index }))
    .sort((a, b) => {
      const compared =
        compareGridValues(
          sortValueFor(a.row, sort.columnKey, changes),
          sortValueFor(b.row, sort.columnKey, changes),
          column,
        ) * direction;
      return compared === 0 ? a.index - b.index : compared;
    })
    .map(({ row }) => row);
}

export function compareGridValues(
  left: GridValue | undefined,
  right: GridValue | undefined,
  column?: GridColumn,
): number {
  const a = normalizeNull(left);
  const b = normalizeNull(right);
  if (a === null && b === null) return 0;
  if (a === null) return 1;
  if (b === null) return -1;

  const boolA = booleanValue(a, column);
  const boolB = booleanValue(b, column);
  if (boolA !== null && boolB !== null) {
    return Number(boolA) - Number(boolB);
  }

  const numA = numericValue(a, column);
  const numB = numericValue(b, column);
  if (numA !== null && numB !== null) {
    return numA === numB ? 0 : numA < numB ? -1 : 1;
  }

  const timeA = temporalValue(a, column);
  const timeB = temporalValue(b, column);
  if (timeA !== null && timeB !== null) {
    return timeA === timeB ? 0 : timeA < timeB ? -1 : 1;
  }

  return stringCompare(String(a), String(b));
}

function sortValueFor(
  row: GridRow,
  columnKey: string,
  changes: PendingChanges,
): GridValue | undefined {
  return changes[row.id]?.edits[columnKey]?.to ?? row[columnKey];
}

function normalizeNull(value: GridValue | undefined): GridValue | null {
  return value === undefined ? null : value;
}

function booleanValue(
  value: Exclude<GridValue, null>,
  column?: GridColumn,
): boolean | null {
  if (typeof value === "boolean") return value;
  if (!column || !isBooleanType(column.type)) return null;
  if (value === "true") return true;
  if (value === "false") return false;
  return null;
}

function numericValue(
  value: Exclude<GridValue, null>,
  column?: GridColumn,
): number | null {
  if (typeof value === "number" && Number.isFinite(value)) return value;
  if (!column || !isNumericType(column.type)) return null;
  if (typeof value !== "string" || value.trim() === "") return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function temporalValue(
  value: Exclude<GridValue, null>,
  column?: GridColumn,
): number | null {
  if (typeof value !== "string") return null;
  const trimmed = value.trim();
  if (trimmed === "") return null;

  if (column && isTimeOnlyType(column.type)) {
    return timeOnlyValue(trimmed);
  }

  if (!looksDateLike(trimmed) && (!column || !isTemporalType(column.type))) {
    return null;
  }

  const normalized = normalizeDateString(trimmed);
  const parsed = Date.parse(normalized);
  return Number.isFinite(parsed) ? parsed : null;
}

function timeOnlyValue(value: string): number | null {
  const match = /^(\d{2}):(\d{2})(?::(\d{2})(?:\.(\d{1,6}))?)?/.exec(value);
  if (!match) return null;
  const hours = Number(match[1]);
  const minutes = Number(match[2]);
  const seconds = Number(match[3] ?? "0");
  if (hours > 23 || minutes > 59 || seconds > 59) return null;
  return ((hours * 60 + minutes) * 60 + seconds) * 1000;
}

function normalizeDateString(value: string): string {
  let out = value.includes("T") ? value : value.replace(" ", "T");
  out = out.replace(/([+-]\d{2})$/, "$1:00");
  return out;
}

function looksDateLike(value: string): boolean {
  return /^\d{4}-\d{2}-\d{2}(?:[ T]\d{2}:\d{2})?/.test(value);
}

function isBooleanType(type: string): boolean {
  const t = type.toLowerCase();
  return t === "bool" || t === "boolean";
}

function isNumericType(type: string): boolean {
  const t = type.toLowerCase();
  return (
    t === "int2" ||
    t === "int4" ||
    t === "int8" ||
    t === "integer" ||
    t === "bigint" ||
    t === "smallint" ||
    t === "oid" ||
    t === "float4" ||
    t === "float8" ||
    t === "real" ||
    t === "double precision" ||
    t === "numeric" ||
    t.startsWith("numeric(") ||
    t.startsWith("decimal(")
  );
}

function isTemporalType(type: string): boolean {
  const t = type.toLowerCase();
  return (
    t === "date" ||
    t === "time" ||
    t === "timetz" ||
    t === "timestamp" ||
    t === "timestamptz" ||
    t.startsWith("timestamp(") ||
    t.startsWith("time(")
  );
}

function isTimeOnlyType(type: string): boolean {
  const t = type.toLowerCase();
  return t === "time" || t === "timetz" || t.startsWith("time(");
}

function stringCompare(left: string, right: string): number {
  const primary = left.localeCompare(right, undefined, {
    numeric: true,
    sensitivity: "base",
  });
  if (primary !== 0) return primary;
  return left.localeCompare(right, undefined, {
    numeric: true,
    sensitivity: "variant",
  });
}

import type { CellValue, Database, DetectedParameter } from "@cellar/ipc";

/**
 * How a parameter input is interpreted before binding. Each maps to a typed
 * `CellValue` so the value is bound through the native protocol, never
 * interpolated into SQL.
 */
export type ParamInputType = "text" | "number" | "boolean" | "date" | "null";

export const PARAM_INPUT_TYPES: ParamInputType[] = [
  "text",
  "number",
  "boolean",
  "date",
  "null",
];

export const PARAM_INPUT_LABELS: Record<ParamInputType, string> = {
  text: "Text",
  number: "Number",
  boolean: "Boolean",
  date: "Date",
  null: "NULL",
};

export interface ParamValue {
  type: ParamInputType;
  /** Raw input string. For `boolean` it is "true"/"false"; for `null` unused. */
  value: string;
}

/** A fresh value for a parameter, typed from its column hint where possible. */
export function defaultParamValue(
  param: DetectedParameter,
  databases: readonly Database[],
  database: string | null = null,
): ParamValue {
  const type = inferParamType(param.column_hint, databases, database);
  return { type, value: type === "boolean" ? "false" : "" };
}

/**
 * Best-effort input type from the column a parameter is compared against.
 *
 * The hint is only a column *name* (we don't know the table), so this scopes to
 * the tab's current database and only commits to a type when every matching
 * column agrees on one. When the name is ambiguous (e.g. `id`/`status` appears
 * in several tables with different types) or unknown, it falls back to text so
 * we never bind a confidently-wrong type.
 */
export function inferParamType(
  columnHint: string | null,
  databases: readonly Database[],
  database: string | null = null,
): ParamInputType {
  if (!columnHint) return "text";
  const types = collectColumnInputTypes(columnHint, databases, database);
  return types.size === 1 ? [...types][0]! : "text";
}

function collectColumnInputTypes(
  name: string,
  databases: readonly Database[],
  database: string | null,
): Set<ParamInputType> {
  const target = name.toLowerCase();
  const collect = (dbs: readonly Database[]): Set<ParamInputType> => {
    const types = new Set<ParamInputType>();
    for (const db of dbs) {
      for (const schema of db.schemas) {
        for (const table of schema.tables) {
          for (const column of table.columns) {
            if (column.name.toLowerCase() === target) {
              types.add(inputTypeForDataType(column.data_type));
            }
          }
        }
      }
    }
    return types;
  };
  // Prefer the tab's database; only widen to all databases if it has no match.
  if (database) {
    const scoped = collect(databases.filter((d) => d.name === database));
    if (scoped.size > 0) return scoped;
  }
  return collect(databases);
}

/** Map an engine-native type name to an input type. */
export function inputTypeForDataType(dataType: string): ParamInputType {
  const t = dataType.toLowerCase();
  if (/^(bool|boolean)$/.test(t)) return "boolean";
  if (/^(date)$/.test(t)) return "date";
  if (
    /(int|serial|oid|float|double|real|numeric|decimal|money)/.test(t) &&
    !/interval/.test(t)
  ) {
    return "number";
  }
  return "text";
}

export type ParamConversion =
  | { ok: true; value: CellValue }
  | { ok: false; error: string };

/** Convert a raw input value to a typed `CellValue`, validating as it goes. */
export function toCellValue(pv: ParamValue): ParamConversion {
  switch (pv.type) {
    case "null":
      return { ok: true, value: { type: "Null" } };
    case "boolean":
      return { ok: true, value: { type: "Bool", value: pv.value === "true" } };
    case "number": {
      const raw = pv.value.trim();
      if (raw.length === 0) return { ok: false, error: "Enter a number" };
      if (/^-?\d+$/.test(raw)) {
        const n = Number(raw);
        if (!Number.isFinite(n)) return { ok: false, error: "Invalid number" };
        return { ok: true, value: { type: "Int", value: n } };
      }
      const n = Number(raw);
      if (!Number.isFinite(n)) return { ok: false, error: "Invalid number" };
      return { ok: true, value: { type: "Float", value: n } };
    }
    case "date": {
      const raw = pv.value.trim();
      if (!/^\d{4}-\d{2}-\d{2}$/.test(raw)) {
        return { ok: false, error: "Use YYYY-MM-DD" };
      }
      // Bind as a real date so it matches `date` columns; the driver binds a
      // NaiveDate rather than a string.
      return { ok: true, value: { type: "Date", value: raw } };
    }
    case "text":
    default:
      return { ok: true, value: { type: "Text", value: pv.value } };
  }
}

/** Whether a value is complete enough to run (NULL and boolean always are). */
export function isFilled(pv: ParamValue): boolean {
  if (pv.type === "null" || pv.type === "boolean") return true;
  return pv.value.trim().length > 0;
}

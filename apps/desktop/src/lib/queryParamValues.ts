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
): ParamValue {
  const type = inferParamType(param.column_hint, databases);
  return { type, value: type === "boolean" ? "false" : "" };
}

/**
 * Best-effort input type from the column a parameter is compared against. Falls
 * back to text when there is no hint or the column is not found in schema.
 */
export function inferParamType(
  columnHint: string | null,
  databases: readonly Database[],
): ParamInputType {
  if (!columnHint) return "text";
  const dataType = findColumnType(columnHint, databases);
  return dataType ? inputTypeForDataType(dataType) : "text";
}

function findColumnType(
  name: string,
  databases: readonly Database[],
): string | null {
  const target = name.toLowerCase();
  for (const db of databases) {
    for (const schema of db.schemas) {
      for (const table of schema.tables) {
        for (const column of table.columns) {
          if (column.name.toLowerCase() === target) {
            return column.data_type;
          }
        }
      }
    }
  }
  return null;
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

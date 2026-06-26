/**
 * Pure engine-type predicates shared by the built-in renderers. Kept separate
 * from the React components so they can be unit-tested without a DOM.
 */

/** Lowercase a type name and strip a trailing precision/size suffix like `(10,2)`. */
export function baseType(type: string): string {
  return type.toLowerCase().replace(/\(.*\)$/, "").trim();
}

export function isJsonType(type: string): boolean {
  const t = baseType(type);
  return t === "json" || t === "jsonb";
}

/**
 * True for native array types. Postgres reports these as `int4[]` / `text[]`
 * via the public name, or as the internal element type `_int4`; other engines
 * use a ` array` suffix. JSON arrays are intentionally excluded — those are
 * owned by the JSON renderer.
 */
export function isArrayType(type: string): boolean {
  const t = baseType(type);
  if (isJsonType(t)) return false;
  return t.endsWith("[]") || t.endsWith(" array") || (t.startsWith("_") && t.length > 1);
}

/** Element type name for an array column, best-effort (`int4[]` → `int4`). */
export function arrayElementType(type: string): string {
  const t = baseType(type);
  if (t.endsWith("[]")) return t.slice(0, -2);
  if (t.endsWith(" array")) return t.slice(0, -" array".length).trim();
  if (t.startsWith("_") && t.length > 1) return t.slice(1);
  return t;
}

export function isByteaType(type: string): boolean {
  const t = baseType(type);
  return (
    t === "bytea" ||
    t === "blob" ||
    t === "binary" ||
    t === "varbinary" ||
    t === "bytes" ||
    t === "longblob" ||
    t === "mediumblob" ||
    t === "tinyblob" ||
    t === "image"
  );
}

export function isGeometryType(type: string): boolean {
  const t = baseType(type);
  return (
    t === "geometry" ||
    t === "geography" ||
    t.startsWith("geometry") ||
    t.startsWith("geography")
  );
}

/** Whether an array element token names a numeric-ish element type. */
export function isNumericElement(elementType: string): boolean {
  const t = baseType(elementType);
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
    t === "decimal" ||
    t === "money"
  );
}

export function isBooleanElement(elementType: string): boolean {
  const t = baseType(elementType);
  return t === "bool" || t === "boolean";
}

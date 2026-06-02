// Identifier quoting for SQL drafted in the UI (context-menu "Query SELECT *",
// copy-qualified-name). These are starter drafts the user reviews before
// running — not an execution path — but per the repo rules any UI-built SQL
// must still quote identifiers safely. A typed builder in `cellar-sql` will
// own real execution paths later.

/** Double-quote a SQL identifier, escaping embedded double quotes. */
export function quoteIdent(name: string): string {
  return '"' + name.replace(/"/g, '""') + '"';
}

/** Quoted, dot-joined qualified name, e.g. `"public"."orders"`. */
export function qualifiedName(...parts: string[]): string {
  return parts.map(quoteIdent).join(".");
}

/** A safe starter `SELECT *` for browsing a relation in the SQL editor. */
export function selectAllStatement(
  schema: string,
  relation: string,
  limit = 100,
): string {
  return `SELECT *\nFROM ${qualifiedName(schema, relation)}\nLIMIT ${limit};`;
}

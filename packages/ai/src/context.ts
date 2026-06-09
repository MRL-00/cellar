// Builds the compact schema context string sent to the model. Kept structurally
// typed (not tied to @cellar/ipc) so the package stays standalone and testable;
// the desktop app maps its introspected `Database[]` onto these shapes.

export interface ContextColumn {
  name: string;
  data_type: string;
  nullable: boolean;
  is_primary_key: boolean;
}

export interface ContextTable {
  schema: string;
  name: string;
  columns: ContextColumn[];
}

export interface SchemaContextInput {
  connectionName?: string;
  engine?: string;
  database?: string;
  schema?: string;
  /** Tables to inline as lightweight DDL. Cap upstream — this renders all. */
  tables?: ContextTable[];
}

function renderColumn(c: ContextColumn): string {
  const flags: string[] = [];
  if (c.is_primary_key) flags.push("pk");
  if (!c.nullable) flags.push("not null");
  const suffix = flags.length ? ` [${flags.join(", ")}]` : "";
  return `  ${c.name} ${c.data_type}${suffix}`;
}

function renderTable(t: ContextTable): string {
  const head = `${t.schema}.${t.name} (`;
  const cols = t.columns.map(renderColumn).join(",\n");
  return `${head}\n${cols}\n)`;
}

/** Produce a terse, deterministic context block. Returns an empty string when
 * there is nothing useful to send, so callers can omit the section entirely. */
export function buildSchemaContext(input: SchemaContextInput): string {
  const header: string[] = [];
  if (input.engine) header.push(`Engine: ${input.engine}`);
  if (input.connectionName) header.push(`Connection: ${input.connectionName}`);
  if (input.database) header.push(`Database: ${input.database}`);
  if (input.schema) header.push(`Schema: ${input.schema}`);

  const tables = (input.tables ?? []).filter((t) => t.columns.length > 0);
  const body = tables.map(renderTable).join("\n\n");

  if (!header.length && !body) return "";
  if (!body) return header.join("\n");
  if (!header.length) return body;
  return `${header.join("\n")}\n\n${body}`;
}

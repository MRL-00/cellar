// Derives the schema context sent to the AI from whatever the user is looking
// at: the active tab's connection, database, schema, and tables. Returns both
// the compact text block for the model and the chips shown in the panel header.

import {
  buildSchemaContext,
  type ContextTable,
  type SchemaContextInput,
} from "@cellar/ai";
import type { Table } from "@cellar/ipc";
import { useConnections } from "../state/connections";
import { useTabs } from "../state/tabs";

/** Cap inlined tables for a query tab so the context stays small. */
const MAX_TABLES = 12;

export type AiContextChip = {
  kind: "schema" | "table" | "query";
  value: string;
};

export interface ActiveContext {
  /** Compact schema block for the provider, or "" when nothing is in scope. */
  text: string;
  chips: AiContextChip[];
}

function toContextTable(t: Table): ContextTable {
  return {
    schema: t.schema,
    name: t.name,
    columns: t.columns.map((c) => ({
      name: c.name,
      data_type: c.data_type,
      nullable: c.nullable,
      is_primary_key: c.is_primary_key,
    })),
  };
}

export function buildActiveContext(): ActiveContext {
  const tabs = useTabs.getState();
  const active = tabs.tabs.find((t) => t.id === tabs.activeId) ?? null;
  if (!active) return { text: "", chips: [] };

  const conns = useConnections.getState();
  const config = conns.connections.find((c) => c.id === active.connectionId);
  const connState = conns.byId[active.connectionId];
  const databases = connState?.databases ?? [];
  const db =
    databases.find((d) => d.name === active.database) ??
    databases.find((d) => d.is_default) ??
    databases[0];

  const chips: AiContextChip[] = [];
  let tables: ContextTable[] = [];
  let schemaName: string | undefined;

  if (active.kind === "table") {
    schemaName = active.schema;
    const schema = db?.schemas.find((s) => s.name === active.schema);
    const table = schema?.tables.find((t) => t.name === active.table);
    if (table) tables = [toContextTable(table)];
    chips.push({ kind: "schema", value: active.schema });
    chips.push({ kind: "table", value: active.table });
  } else {
    const schema =
      db?.schemas.find((s) => s.name === "public") ?? db?.schemas[0];
    if (schema) {
      schemaName = schema.name;
      tables = schema.tables.slice(0, MAX_TABLES).map(toContextTable);
      chips.push({ kind: "schema", value: schema.name });
      const shown = Math.min(schema.tables.length, MAX_TABLES);
      if (shown > 0) {
        chips.push({
          kind: "table",
          value: shown === 1 ? schema.tables[0]!.name : `${shown} tables`,
        });
      }
    }
    chips.push({ kind: "query", value: active.title });
  }

  const input: SchemaContextInput = {
    connectionName: config?.name,
    engine: config?.engine,
    database: active.database,
    schema: schemaName,
    tables,
  };

  return { text: buildSchemaContext(input), chips };
}

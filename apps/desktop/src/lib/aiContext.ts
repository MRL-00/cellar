// Derives the schema context sent to the AI from whatever the user is looking
// at: the active tab's connection, database, schema, tables, and open SQL.
// Returns both the compact text block for the model and the chips shown in the
// panel header.

import {
  buildSchemaContext,
  type ContextForeignKey,
  type ContextTable,
  type SchemaContextInput,
} from "@cellar/ai";
import type { Column, ForeignKey, Schema, Table } from "@cellar/ipc";
import { useConnections } from "../state/connections";
import { useTabs } from "../state/tabs";

/** Cap inlined tables for a query tab so the context stays small. */
const MAX_TABLES = 12;
/** Extra slots reserved for FK neighbors of the focused tables. */
const MAX_RELATED = 6;

/** SQL Server fixed-role / system schemas that must never be AI defaults. */
const MSSQL_SYSTEM_SCHEMAS = new Set([
  "sys",
  "information_schema",
  "guest",
  "db_accessadmin",
  "db_backupoperator",
  "db_datareader",
  "db_datawriter",
  "db_ddladmin",
  "db_denydatareader",
  "db_denydatawriter",
  "db_owner",
  "db_securityadmin",
]);

const PG_SYSTEM_SCHEMAS = new Set([
  "information_schema",
  "pg_catalog",
  "pg_toast",
]);

export type AiContextChip = {
  kind: "schema" | "table" | "query";
  value: string;
};

export interface ActiveContext {
  /** Compact schema block for the provider, or "" when nothing is in scope. */
  text: string;
  chips: AiContextChip[];
}

export type SqlTableRef = {
  schema: string | null;
  table: string;
};

type Relation = {
  schema: string;
  name: string;
  columns: Column[];
  foreign_keys: ForeignKey[];
};

function relationKey(schema: string, name: string): string {
  return `${schema.toLowerCase()}.${name.toLowerCase()}`;
}

function toRelation(table: Table): Relation {
  return {
    schema: table.schema,
    name: table.name,
    columns: table.columns,
    foreign_keys: table.foreign_keys,
  };
}

function toContextTable(t: Relation): ContextTable {
  const foreign_keys: ContextForeignKey[] = t.foreign_keys.map((fk) => ({
    columns: fk.columns,
    referenced_schema: fk.referenced_schema,
    referenced_table: fk.referenced_table,
    referenced_columns: fk.referenced_columns,
  }));
  return {
    schema: t.schema,
    name: t.name,
    columns: t.columns.map((c) => ({
      name: c.name,
      data_type: c.data_type,
      nullable: c.nullable,
      is_primary_key: c.is_primary_key,
    })),
    foreign_keys: foreign_keys.length ? foreign_keys : undefined,
  };
}

function schemaHasTables(schema: Schema): boolean {
  return schema.tables.length > 0 || schema.views.length > 0;
}

/** True for engine system / fixed-role schemas that should not be AI defaults. */
export function isNoiseSchema(name: string, engine?: string): boolean {
  const lower = name.toLowerCase();
  if (lower.startsWith("pg_temp_") || lower.startsWith("pg_toast_temp_")) {
    return true;
  }
  const family = engineFamily(engine);
  if (family === "mssql") return MSSQL_SYSTEM_SCHEMAS.has(lower);
  if (family === "postgres") return PG_SYSTEM_SCHEMAS.has(lower);
  return MSSQL_SYSTEM_SCHEMAS.has(lower) || PG_SYSTEM_SCHEMAS.has(lower);
}

function engineFamily(engine?: string): "postgres" | "mssql" | "mysql" | "other" {
  switch ((engine ?? "").toLowerCase()) {
    case "postgres":
    case "supabase":
    case "neon":
      return "postgres";
    case "mssql":
    case "azure":
      return "mssql";
    case "mysql":
    case "planetscale":
      return "mysql";
    default:
      return "other";
  }
}

function preferredSchemaNames(engine?: string): string[] {
  switch (engineFamily(engine)) {
    case "postgres":
      return ["public"];
    case "mssql":
      return ["dbo"];
    default:
      return [];
  }
}

/** Strip string literals and comments so table-ref scans ignore noise. */
export function stripSqlNoise(sql: string): string {
  let out = "";
  let i = 0;
  while (i < sql.length) {
    const c = sql[i]!;
    const next = sql[i + 1];
    if (c === "-" && next === "-") {
      i += 2;
      while (i < sql.length && sql[i] !== "\n") i++;
      continue;
    }
    if (c === "/" && next === "*") {
      i += 2;
      while (i < sql.length && !(sql[i] === "*" && sql[i + 1] === "/")) i++;
      i = Math.min(sql.length, i + 2);
      out += " ";
      continue;
    }
    // Only single-quoted strings are values; ", `, and [] are identifiers.
    if (c === "'") {
      i++;
      while (i < sql.length) {
        if (sql[i] === "'") {
          if (sql[i + 1] === "'") {
            i += 2;
            continue;
          }
          i++;
          break;
        }
        i++;
      }
      out += " ";
      continue;
    }
    out += c;
    i++;
  }
  return out;
}

function unquoteIdent(raw: string): string {
  if (
    (raw.startsWith("[") && raw.endsWith("]")) ||
    (raw.startsWith('"') && raw.endsWith('"')) ||
    (raw.startsWith("`") && raw.endsWith("`"))
  ) {
    return raw.slice(1, -1);
  }
  return raw;
}

const IDENT = '(?:\\[[^\\]]+\\]|"[^"]+"|`[^`]+`|[A-Za-z_][\\w$]*)';

/**
 * Pull schema.table and bare FROM/JOIN table names out of SQL. Best-effort —
 * enough to pin AI context to the relations the user is already editing.
 */
export function extractSqlTableRefs(sql: string): SqlTableRef[] {
  const clean = stripSqlNoise(sql);
  const found: SqlTableRef[] = [];
  const seen = new Set<string>();

  const push = (schema: string | null, table: string) => {
    const s = schema ? unquoteIdent(schema) : null;
    const t = unquoteIdent(table);
    if (!t || /^(select|where|and|or|group|order|set|values|on|as)$/i.test(t)) {
      return;
    }
    const key = `${(s ?? "").toLowerCase()}.${t.toLowerCase()}`;
    if (seen.has(key)) return;
    seen.add(key);
    found.push({ schema: s, table: t });
  };

  // Only match relation positions (FROM/JOIN/…) so `p.CustomerName` column
  // refs are not mistaken for schema.table pairs.
  const afterClause = new RegExp(
    `\\b(?:from|join|update|into|table|merge)\\s+(?:(${IDENT})\\s*\\.\\s*)?(${IDENT})`,
    "gi",
  );
  for (const m of clean.matchAll(afterClause)) {
    push(m[1] ?? null, m[2]!);
  }

  return found;
}

function findRelation(
  schemas: Schema[],
  schemaName: string | null,
  tableName: string,
): Relation | undefined {
  const tableLower = tableName.toLowerCase();

  if (schemaName) {
    const schemaLower = schemaName.toLowerCase();
    const schema = schemas.find((s) => s.name.toLowerCase() === schemaLower);
    if (!schema) return undefined;
    const table = schema.tables.find((t) => t.name.toLowerCase() === tableLower);
    if (table) return toRelation(table);
    const view = schema.views.find((v) => v.name.toLowerCase() === tableLower);
    if (view) {
      return {
        schema: view.schema,
        name: view.name,
        columns: view.columns,
        foreign_keys: [],
      };
    }
    return undefined;
  }

  for (const schema of schemas) {
    if (isNoiseSchema(schema.name)) continue;
    const table = schema.tables.find((t) => t.name.toLowerCase() === tableLower);
    if (table) return toRelation(table);
    const view = schema.views.find((v) => v.name.toLowerCase() === tableLower);
    if (view) {
      return {
        schema: view.schema,
        name: view.name,
        columns: view.columns,
        foreign_keys: [],
      };
    }
  }
  return undefined;
}

/** Dimension-ish table names worth pulling in when they share a join key. */
const LOOKUP_NAME =
  /compan|tenant|customer|client|account|product|vendor|merchant|org|user/i;

function isJoinKey(name: string): boolean {
  const lower = name.toLowerCase();
  return lower !== "id" && /id$|_id$/.test(lower);
}

/**
 * Pull in tables linked by foreign keys (both directions) so the model can
 * join lookup/dimension tables — e.g. Payments → Companies via TenantId.
 * Also pulls same-schema lookup tables that share an *Id column when the DB
 * has no formal FK (common for TenantId).
 */
export function expandWithRelatedTables(
  schemas: Schema[],
  seed: Relation[],
  maxTotal: number = MAX_TABLES,
): Relation[] {
  const out: Relation[] = [];
  const seen = new Set<string>();

  const add = (relation: Relation): boolean => {
    const key = relationKey(relation.schema, relation.name);
    if (seen.has(key)) return false;
    if (out.length >= maxTotal) return false;
    seen.add(key);
    out.push(relation);
    return true;
  };

  for (const relation of seed) add(relation);

  const seedKeys = new Set(seed.map((r) => relationKey(r.schema, r.name)));
  let relatedAdded = 0;

  // Outgoing FKs: Payments.TenantId → Companies.TenantId
  for (const relation of seed) {
    for (const fk of relation.foreign_keys) {
      if (relatedAdded >= MAX_RELATED || out.length >= maxTotal) break;
      const target = findRelation(
        schemas,
        fk.referenced_schema,
        fk.referenced_table,
      );
      if (target && add(target)) relatedAdded++;
    }
  }

  // Incoming FKs: other tables that reference a seed table
  for (const schema of schemas) {
    if (isNoiseSchema(schema.name)) continue;
    for (const table of schema.tables) {
      if (relatedAdded >= MAX_RELATED || out.length >= maxTotal) break;
      const pointsAtSeed = table.foreign_keys.some((fk) =>
        seedKeys.has(relationKey(fk.referenced_schema, fk.referenced_table)),
      );
      if (pointsAtSeed && add(toRelation(table))) relatedAdded++;
    }
  }

  // Shared join-key heuristic when FKs are missing from the catalog.
  for (const relation of seed) {
    const joinKeys = new Set(
      relation.columns.filter((c) => isJoinKey(c.name)).map((c) => c.name.toLowerCase()),
    );
    if (joinKeys.size === 0) continue;
    for (const schema of schemas) {
      if (isNoiseSchema(schema.name)) continue;
      // Prefer same-schema neighbors; still allow cross-schema if named clearly.
      for (const table of schema.tables) {
        if (relatedAdded >= MAX_RELATED || out.length >= maxTotal) break;
        if (!LOOKUP_NAME.test(table.name)) continue;
        const sharesKey = table.columns.some((c) => joinKeys.has(c.name.toLowerCase()));
        if (sharesKey && add(toRelation(table))) relatedAdded++;
      }
    }
  }

  return out;
}

/** Pick the best default schema when the SQL buffer has no usable refs. */
export function pickDefaultSchema(
  schemas: Schema[],
  engine?: string,
): Schema | undefined {
  const usable = schemas.filter(
    (s) => schemaHasTables(s) && !isNoiseSchema(s.name, engine),
  );
  if (usable.length === 0) {
    return schemas.find((s) => !isNoiseSchema(s.name, engine));
  }
  const preferred = new Set(
    preferredSchemaNames(engine).map((n) => n.toLowerCase()),
  );
  // Prefer the schema with the most relations so app schemas beat a sparse
  // `dbo`/`public`. Tie-break toward the engine's conventional default.
  return [...usable].sort((a, b) => {
    const countDiff =
      b.tables.length +
      b.views.length -
      (a.tables.length + a.views.length);
    if (countDiff !== 0) return countDiff;
    const aPref = preferred.has(a.name.toLowerCase()) ? 0 : 1;
    const bPref = preferred.has(b.name.toLowerCase()) ? 0 : 1;
    return aPref - bPref || a.name.localeCompare(b.name);
  })[0];
}

/**
 * Resolve which tables to send for a query tab: prefer relations referenced in
 * the open SQL, expand via foreign keys, then fill from the best matching schema.
 */
export function resolveQueryContextTables(
  schemas: Schema[],
  sql: string,
  engine?: string,
): { schemaName?: string; tables: ContextTable[] } {
  const refs = extractSqlTableRefs(sql);
  const seed: Relation[] = [];
  const seedKeys = new Set<string>();
  const schemaCounts = new Map<string, number>();

  const addSeed = (relation: Relation) => {
    const key = relationKey(relation.schema, relation.name);
    if (seedKeys.has(key)) return;
    seedKeys.add(key);
    seed.push(relation);
    schemaCounts.set(
      relation.schema,
      (schemaCounts.get(relation.schema) ?? 0) + 1,
    );
  };

  for (const ref of refs) {
    const relation = findRelation(schemas, ref.schema, ref.table);
    if (relation) addSeed(relation);
  }

  let focusSchema =
    [...schemaCounts.entries()].sort((a, b) => b[1] - a[1])[0]?.[0] ??
    pickDefaultSchema(schemas, engine)?.name;

  if (seed.length === 0) {
    const fallback = pickDefaultSchema(schemas, engine);
    if (fallback) {
      focusSchema = fallback.name;
      // Seed with a small head of the schema; FK expansion pulls lookup tables.
      for (const table of fallback.tables.slice(0, 4)) addSeed(toRelation(table));
    }
  }

  let selected = expandWithRelatedTables(schemas, seed, MAX_TABLES);

  if (focusSchema && selected.length < MAX_TABLES) {
    const schema = schemas.find(
      (s) => s.name.toLowerCase() === focusSchema!.toLowerCase(),
    );
    if (schema) {
      const selectedKeys = new Set(
        selected.map((r) => relationKey(r.schema, r.name)),
      );
      for (const table of schema.tables) {
        if (selected.length >= MAX_TABLES) break;
        const key = relationKey(table.schema, table.name);
        if (selectedKeys.has(key)) continue;
        selectedKeys.add(key);
        selected.push(toRelation(table));
      }
    }
  }

  return {
    schemaName: focusSchema,
    tables: selected.map(toContextTable),
  };
}

/** Context for a focused table tab: the table plus FK-related neighbors. */
export function resolveTableContextTables(
  schemas: Schema[],
  schemaName: string,
  tableName: string,
): { schemaName: string; tables: ContextTable[] } {
  const seed = findRelation(schemas, schemaName, tableName);
  const selected = seed
    ? expandWithRelatedTables(schemas, [seed], MAX_TABLES)
    : [];
  return {
    schemaName,
    tables: selected.map(toContextTable),
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
    const resolved = resolveTableContextTables(
      db?.schemas ?? [],
      active.schema,
      active.table,
    );
    schemaName = resolved.schemaName;
    tables = resolved.tables;
    chips.push({ kind: "schema", value: active.schema });
    chips.push({
      kind: "table",
      value:
        tables.length > 1
          ? `${active.table} +${tables.length - 1}`
          : active.table,
    });
  } else if (active.kind === "query") {
    const resolved = resolveQueryContextTables(
      db?.schemas ?? [],
      active.sql,
      config?.engine,
    );
    schemaName = resolved.schemaName;
    tables = resolved.tables;
    if (schemaName) chips.push({ kind: "schema", value: schemaName });
    if (tables.length === 1) {
      chips.push({ kind: "table", value: tables[0]!.name });
    } else if (tables.length > 1) {
      chips.push({ kind: "table", value: `${tables.length} tables` });
    }
    chips.push({ kind: "query", value: active.title });
  } else {
    const resolved = resolveQueryContextTables(
      db?.schemas ?? [],
      "",
      config?.engine,
    );
    schemaName = resolved.schemaName;
    tables = resolved.tables;
    if (schemaName) chips.push({ kind: "schema", value: schemaName });
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

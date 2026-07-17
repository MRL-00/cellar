// Pre-resolve named entities (company / tenant / customer) against lookup
// tables before asking the model. Injecting real TenantId values stops Gemini
// from guessing with LIKE '%Shotover%' on the wrong columns.

import type { ContextTable } from "@cellar/ai";
import { commands, unwrap, type QueryResult } from "@cellar/ipc";
import { useConnections } from "../state/connections";
import { useTabs } from "../state/tabs";
import {
  isNoiseSchema,
  resolveQueryContextTables,
  resolveTableContextTables,
} from "./aiContext";
import { cellValueToGrid } from "./gridMapping";

const LOOKUP_TABLE =
  /compan|tenant|customer|client|account|merchant|vendor|org/i;
const NAME_COLUMN =
  /^(name|companyname|tradingname|displayname|legalname|title|label|fullname|customername)$/i;
const KEY_COLUMN = /^(tenantid|companyid|customerid|clientid|accountid|id)$/i;
const STOP = new Set([
  "all",
  "the",
  "and",
  "for",
  "from",
  "with",
  "based",
  "baed",
  "on",
  "to",
  "in",
  "of",
  "a",
  "an",
  "my",
  "me",
  "give",
  "get",
  "show",
  "list",
  "find",
  "june",
  "july",
  "august",
  "september",
  "october",
  "november",
  "december",
  "january",
  "february",
  "march",
  "april",
  "may",
  "time",
  "nz",
  "utc",
  "company",
  "tenant",
  "customer",
  "customers",
  "id",
]);

export type LookupTarget = {
  schema: string;
  table: string;
  keyColumns: string[];
  nameColumns: string[];
};

function engineFamily(engine?: string): "mssql" | "postgres" | "other" {
  switch ((engine ?? "").toLowerCase()) {
    case "mssql":
    case "azure":
      return "mssql";
    case "postgres":
    case "supabase":
    case "neon":
      return "postgres";
    default:
      return "other";
  }
}

function quoteIdent(engine: string | undefined, name: string): string {
  if (engineFamily(engine) === "mssql") {
    return `[${name.replace(/]/g, "]]")}]`;
  }
  return `"${name.replace(/"/g, '""')}"`;
}

function escapeLike(value: string): string {
  return value.replace(/([%_\[\]\\])/g, "\\$1");
}

function sqlString(engine: string | undefined, value: string): string {
  const escaped = value.replace(/'/g, "''");
  return engineFamily(engine) === "mssql" ? `N'${escaped}'` : `'${escaped}'`;
}

/** Pull likely entity names out of a natural-language ask. */
export function extractEntityPhrases(text: string): string[] {
  const phrases: string[] = [];
  const seen = new Set<string>();
  const push = (raw: string) => {
    const phrase = raw.trim().replace(/\s+/g, " ");
    if (phrase.length < 2 || phrase.length > 80) return;
    const key = phrase.toLowerCase();
    if (seen.has(key) || STOP.has(key)) return;
    if (/^\d/.test(phrase)) return;
    seen.add(key);
    phrases.push(phrase);
  };

  for (const m of text.matchAll(/['"]([^'"]{2,80})['"]/g)) push(m[1]!);

  // "for shotover jet, based on…" / "company shotover jet"
  const anchored =
    /\b(?:for|company|tenant|customer|client|named)\s+([a-z0-9][a-z0-9&'.\-\s]{1,60}?)(?=,|;|\.|based|baed|\bfrom\b|\bbetween\b|\bin\b|\bon\b|\bfor\b|\bwith\b|$)/gi;
  for (const m of text.matchAll(anchored)) {
    const chunk = m[1]!
      .split(/\s+/)
      .filter((w) => !STOP.has(w.toLowerCase()) && !/^\d+(st|nd|rd|th)?$/i.test(w))
      .join(" ");
    push(chunk);
  }

  return phrases.slice(0, 3);
}

export function pickLookupTargets(tables: ContextTable[]): LookupTarget[] {
  const out: LookupTarget[] = [];
  for (const table of tables) {
    if (isNoiseSchema(table.schema)) continue;
    if (!LOOKUP_TABLE.test(table.name)) continue;
    const nameColumns = table.columns
      .map((c) => c.name)
      .filter((n) => NAME_COLUMN.test(n));
    if (nameColumns.length === 0) continue;
    const keyColumns = table.columns
      .map((c) => c.name)
      .filter((n) => KEY_COLUMN.test(n))
      .sort((a, b) => {
        // Prefer TenantId over bare Id.
        const score = (n: string) =>
          n.toLowerCase() === "tenantid" ? 0 : n.toLowerCase() === "id" ? 2 : 1;
        return score(a) - score(b);
      });
    if (keyColumns.length === 0) continue;
    out.push({
      schema: table.schema,
      table: table.name,
      keyColumns: keyColumns.slice(0, 2),
      nameColumns: nameColumns.slice(0, 3),
    });
  }
  // Prefer Companies/Tenants before Customers.
  return out.sort((a, b) => {
    const rank = (n: string) =>
      /compan/i.test(n) ? 0 : /tenant/i.test(n) ? 1 : 2;
    return rank(a.table) - rank(b.table) || a.table.localeCompare(b.table);
  });
}

/** Build a small dialect-aware lookup statement for one target + phrase. */
export function buildLookupSql(
  target: LookupTarget,
  phrase: string,
  engine?: string,
): string {
  const q = (name: string) => quoteIdent(engine, name);
  const cols = [...new Set([...target.keyColumns, ...target.nameColumns])]
    .map(q)
    .join(", ");
  const pattern = sqlString(engine, `%${escapeLike(phrase)}%`);
  const where = target.nameColumns
    .map((col) =>
      engineFamily(engine) === "postgres"
        ? `${q(col)} ILIKE ${pattern} ESCAPE '\\'`
        : `${q(col)} LIKE ${pattern} ESCAPE '\\'`,
    )
    .join(" OR ");
  const from = `${q(target.schema)}.${q(target.table)}`;
  if (engineFamily(engine) === "mssql") {
    return `SELECT TOP 5 ${cols}\nFROM ${from}\nWHERE ${where}`;
  }
  return `SELECT ${cols}\nFROM ${from}\nWHERE ${where}\nLIMIT 5`;
}

export function formatLookupHits(
  target: LookupTarget,
  phrase: string,
  result: QueryResult,
): string {
  if (result.rows.length === 0) {
    return `- ${target.schema}.${target.table} for "${phrase}": no rows`;
  }
  const lines = result.rows.map((row) => {
    const parts = result.columns.map((col, i) => {
      const raw = cellValueToGrid(row[i]!);
      const value = raw == null ? "NULL" : String(raw);
      return `${col.name}=${value}`;
    });
    return `  • ${parts.join(", ")}`;
  });
  return [
    `- ${target.schema}.${target.table} for "${phrase}" (use these keys with = / IN, not LIKE):`,
    ...lines,
  ].join("\n");
}

/**
 * Run best-effort entity lookups against the active connection. Failures are
 * swallowed — the model still gets schema context without hits.
 */
export async function runEntityLookups(userText: string): Promise<string> {
  const phrases = extractEntityPhrases(userText);
  if (phrases.length === 0) return "";

  const tabs = useTabs.getState();
  const active = tabs.tabs.find((t) => t.id === tabs.activeId);
  if (!active) return "";

  const conns = useConnections.getState();
  const config = conns.connections.find((c) => c.id === active.connectionId);
  const connState = conns.byId[active.connectionId];
  const databases = connState?.databases ?? [];
  const db =
    databases.find((d) => d.name === active.database) ??
    databases.find((d) => d.is_default) ??
    databases[0];
  if (!db) return "";

  const schemas = db.schemas;
  let tables: ContextTable[] = [];
  if (active.kind === "table") {
    tables = resolveTableContextTables(schemas, active.schema, active.table).tables;
  } else if (active.kind === "query") {
    tables = resolveQueryContextTables(schemas, active.sql, config?.engine).tables;
  } else {
    tables = resolveQueryContextTables(schemas, "", config?.engine).tables;
  }

  // Also consider every lookup-shaped table in the database so we don't miss
  // Companies when the editor is focused on Payments/Customers alone.
  const extras: ContextTable[] = schemas.flatMap((schema) =>
    schema.tables
      .filter((t) => LOOKUP_TABLE.test(t.name))
      .map((t) => ({
        schema: t.schema,
        name: t.name,
        columns: t.columns.map((c) => ({
          name: c.name,
          data_type: c.data_type,
          nullable: c.nullable,
          is_primary_key: c.is_primary_key,
        })),
      })),
  );
  const byKey = new Map<string, ContextTable>();
  for (const t of [...tables, ...extras]) {
    byKey.set(`${t.schema.toLowerCase()}.${t.name.toLowerCase()}`, t);
  }
  const targets = pickLookupTargets([...byKey.values()]).slice(0, 3);
  if (targets.length === 0) return "";

  const engine = config?.engine;
  const database = active.database || null;
  const blocks: string[] = [];

  for (const phrase of phrases) {
    for (const target of targets) {
      try {
        const sql = buildLookupSql(target, phrase, engine);
        const result = await unwrap(
          commands.runReadOnlyQuery(active.connectionId, sql, 5, database),
        );
        blocks.push(formatLookupHits(target, phrase, result));
        // First hit with rows is enough for this phrase.
        if (result.rows.length > 0) break;
      } catch {
        // ignore lookup failures — model still has schema context
      }
    }
  }

  if (blocks.length === 0) return "";
  return `Lookup hits (resolved from the live database; prefer these keys):\n${blocks.join("\n")}`;
}

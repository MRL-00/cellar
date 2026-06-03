import type {
  Completion,
  CompletionContext,
  CompletionResult,
} from "@codemirror/autocomplete";
import type {
  SqlColumnMeta,
  SqlDatabaseMeta,
  SqlRelationMeta,
} from "./types";

interface CompletionConfig {
  databases: readonly SqlDatabaseMeta[];
  database: string | null;
}

interface RelationRef {
  schema: string;
  name: string;
  columns: readonly SqlColumnMeta[];
  kind: "table" | "view";
}

interface ParsedSqlContext {
  relationContext: boolean;
  aliasOrQualifier: string | null;
  replacementFrom: number;
  text: string;
}

const RELATION_PRECEDERS = new Set([
  "from",
  "join",
  "into",
  "update",
  "table",
  "describe",
  "truncate",
]);

const COLUMN_PRECEDERS = new Set([
  "select",
  "where",
  "and",
  "or",
  "on",
  "by",
  "group",
  "order",
  "having",
  "set",
  "returning",
]);

const KEYWORD_COMPLETIONS: readonly Completion[] = [
  "SELECT",
  "FROM",
  "WHERE",
  "JOIN",
  "LEFT JOIN",
  "INNER JOIN",
  "GROUP BY",
  "ORDER BY",
  "HAVING",
  "LIMIT",
  "INSERT INTO",
  "UPDATE",
  "DELETE FROM",
  "CREATE TABLE",
  "ALTER TABLE",
  "DROP TABLE",
  "WITH",
  "VALUES",
  "RETURNING",
  "EXPLAIN",
].map((label) => ({
  label,
  type: "keyword",
  section: "keywords",
  boost: -5,
}));

const SNIPPET_COMPLETIONS: readonly Completion[] = [
  {
    label: "sel",
    detail: "select rows",
    apply: "SELECT *\nFROM \nLIMIT 100;",
    type: "keyword",
    section: "snippets",
    boost: 20,
  },
  {
    label: "ins",
    detail: "insert row",
    apply: "INSERT INTO \n  ()\nVALUES\n  ();",
    type: "keyword",
    section: "snippets",
    boost: 12,
  },
  {
    label: "upd",
    detail: "update rows",
    apply: "UPDATE \nSET \nWHERE ;",
    type: "keyword",
    section: "snippets",
    boost: 12,
  },
  {
    label: "del",
    detail: "delete rows",
    apply: "DELETE FROM \nWHERE ;",
    type: "keyword",
    section: "snippets",
    boost: 12,
  },
  {
    label: "jln",
    detail: "left join",
    apply: "LEFT JOIN  ON ",
    type: "keyword",
    section: "snippets",
    boost: 8,
  },
];

export function cellarCompletionSource(config: CompletionConfig) {
  return (context: CompletionContext): CompletionResult | null => {
    const parsed = parseCompletionContext(context);
    if (!parsed) return null;

    const relations = relationsFor(config);
    const aliases = aliasesFor(context.state.doc.toString(), context.pos, relations);
    const options = completionOptions(parsed, relations, aliases);

    return {
      from: parsed.replacementFrom,
      options,
      validFor: /^[\w$"]*$/,
    };
  };
}

function parseCompletionContext(context: CompletionContext): ParsedSqlContext | null {
  const match = context.matchBefore(/[A-Za-z_][\w$]*(?:\.[A-Za-z_][\w$]*)?|\./);
  const explicitEmpty = context.explicit && context.pos > 0;
  if (!match && !explicitEmpty) return null;

  const before = context.state.sliceDoc(0, context.pos);
  const text = match?.text ?? "";
  const tokenFrom = match?.from ?? context.pos;
  const lastDot = text.lastIndexOf(".");
  const aliasOrQualifier = lastDot >= 0 ? text.slice(0, lastDot) : null;
  const replacementFrom = lastDot >= 0 ? tokenFrom + lastDot + 1 : tokenFrom;
  const previous = previousWord(before.slice(0, tokenFrom)).toLowerCase();
  const relationContext = RELATION_PRECEDERS.has(previous);
  const columnContext = COLUMN_PRECEDERS.has(previous);

  if (!context.explicit && text.length === 0 && !relationContext && !columnContext) {
    return null;
  }

  return {
    relationContext,
    aliasOrQualifier,
    replacementFrom,
    text,
  };
}

function completionOptions(
  context: ParsedSqlContext,
  relations: readonly RelationRef[],
  aliases: ReadonlyMap<string, RelationRef>,
): Completion[] {
  if (context.aliasOrQualifier) {
    const qualifier = normalizeIdentifier(context.aliasOrQualifier);
    const aliasRelation = aliases.get(qualifier);
    if (aliasRelation) return columnOptions(aliasRelation.columns, aliasRelation.name, 30);

    const schemaRelations = relations.filter(
      (relation) => relation.schema.toLowerCase() === qualifier,
    );
    if (schemaRelations.length > 0) return relationOptions(schemaRelations, false);

    const relation = relations.find(
      (candidate) => candidate.name.toLowerCase() === qualifier,
    );
    if (relation) return columnOptions(relation.columns, relation.name, 24);
  }

  if (context.relationContext) {
    return [
      ...relationOptions(relations, true),
      ...relationOptions(relations, false),
      ...schemaOptions(relations),
      ...KEYWORD_COMPLETIONS,
    ];
  }

  const scopedColumns = columnsForAliases(aliases);
  const columnSource = scopedColumns.length > 0 ? scopedColumns : allColumns(relations);

  return [
    ...SNIPPET_COMPLETIONS,
    ...columnOptions(columnSource, "schema", scopedColumns.length > 0 ? 18 : 0),
    ...relationOptions(relations, true).map((option) => ({ ...option, boost: -8 })),
    ...KEYWORD_COMPLETIONS,
  ];
}

function relationsFor(config: CompletionConfig): RelationRef[] {
  const db = databaseFor(config.databases, config.database);
  if (!db) return [];

  return db.schemas.flatMap((schema) => [
    ...schema.tables.map((table) => relationRef(schema.name, table, "table")),
    ...schema.views.map((view) => relationRef(schema.name, view, "view")),
  ]);
}

function relationRef(
  schema: string,
  relation: SqlRelationMeta,
  kind: "table" | "view",
): RelationRef {
  return {
    schema,
    name: relation.name,
    columns: relation.columns,
    kind,
  };
}

function databaseFor(
  databases: readonly SqlDatabaseMeta[],
  database: string | null,
): SqlDatabaseMeta | null {
  if (database) {
    const match = databases.find((db) => db.name === database);
    if (match) return match;
  }
  return databases.find((db) => db.is_default) ?? databases[0] ?? null;
}

function relationOptions(
  relations: readonly RelationRef[],
  qualified: boolean,
): Completion[] {
  const seen = new Set<string>();
  return relations.flatMap((relation) => {
    const label = qualified ? `${relation.schema}.${relation.name}` : relation.name;
    const key = `${label}:${relation.kind}`;
    if (seen.has(key)) return [];
    seen.add(key);
    return [
      {
        label,
        detail: relation.kind,
        type: relation.kind === "view" ? "interface" : "type",
        section: relation.kind === "view" ? "views" : "tables",
        boost: relation.kind === "table" ? 15 : 8,
      },
    ];
  });
}

function schemaOptions(relations: readonly RelationRef[]): Completion[] {
  const names = new Set(relations.map((relation) => relation.schema));
  return [...names].map((label) => ({
    label,
    detail: "schema",
    type: "namespace",
    section: "schemas",
  }));
}

function columnOptions(
  columns: readonly SqlColumnMeta[],
  source: string,
  boost: number,
): Completion[] {
  const seen = new Set<string>();
  return columns.flatMap((column) => {
    const key = column.name.toLowerCase();
    if (seen.has(key)) return [];
    seen.add(key);
    return [
      {
        label: column.name,
        detail: column.data_type || source,
        type: "property",
        section: "columns",
        boost: column.is_primary_key ? boost + 4 : boost,
      },
    ];
  });
}

function columnsForAliases(aliases: ReadonlyMap<string, RelationRef>): SqlColumnMeta[] {
  const columns: SqlColumnMeta[] = [];
  for (const [alias, relation] of aliases) {
    for (const column of relation.columns) {
      columns.push({
        ...column,
        name: aliases.size > 1 ? `${alias}.${column.name}` : column.name,
      });
    }
  }
  return columns;
}

function allColumns(relations: readonly RelationRef[]): SqlColumnMeta[] {
  return relations.flatMap((relation) => relation.columns);
}

function aliasesFor(
  sqlText: string,
  position: number,
  relations: readonly RelationRef[],
): Map<string, RelationRef> {
  const statement = currentStatementText(sqlText, position);
  const aliases = new Map<string, RelationRef>();
  const pattern =
    /\b(?:from|join|update|into)\s+((?:"[^"]+"|[A-Za-z_][\w$]*)(?:\s*\.\s*(?:"[^"]+"|[A-Za-z_][\w$]*))?)(?:\s+(?:as\s+)?(?!(?:on|where|join|left|right|inner|outer|full|cross|group|order|limit|having|set|values|returning)\b)("?[\w$]+"?))?/gi;

  for (const match of statement.matchAll(pattern)) {
    const relationName = normalizeIdentifier((match[1] ?? "").split(".").pop() ?? "");
    const relationSchema = relationQualifier(match[1] ?? "");
    const relation = relations.find((candidate) => {
      if (candidate.name.toLowerCase() !== relationName) return false;
      return relationSchema ? candidate.schema.toLowerCase() === relationSchema : true;
    });
    if (!relation) continue;

    aliases.set(relation.name.toLowerCase(), relation);
    if (match[2]) aliases.set(normalizeIdentifier(match[2]), relation);
  }

  return aliases;
}

function currentStatementText(sqlText: string, position: number): string {
  const start = sqlText.lastIndexOf(";", Math.max(0, position - 1)) + 1;
  const end = sqlText.indexOf(";", position);
  return sqlText.slice(start, end === -1 ? sqlText.length : end);
}

function relationQualifier(raw: string): string | null {
  const parts = raw.split(".");
  if (parts.length < 2) return null;
  return normalizeIdentifier(parts[0] ?? "");
}

function normalizeIdentifier(value: string): string {
  return value.trim().replace(/^"|"$/g, "").toLowerCase();
}

function previousWord(text: string): string {
  const match = text.match(/([A-Za-z_][\w$]*)\s*$/);
  return match?.[1] ?? "";
}

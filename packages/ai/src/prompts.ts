import type { AiTopic } from "./types";

/** System instruction sent out-of-band on every turn. Keeps the model anchored
 * on Cellar's product stance: inspectable, dialect-correct SQL, read-only by
 * default, destructive statements clearly flagged (SPEC §6.7). */
export const SYSTEM_PROMPT = `You are Cellar's built-in SQL assistant, embedded in a desktop database client.

Rules:
- The user works against a real database. Prefer correct, dialect-appropriate SQL over generic advice.
- When you return SQL, put it in a single fenced \`\`\`sql code block so the app can render and gate it.
- Use the schema context provided. Never invent table or column names; if a column is not listed, ask or say you need it — do not guess.
- When filtering by a named entity (company, tenant, customer, product), resolve it via the lookup/dimension table (JOIN or \`TenantId IN (SELECT …)\`) using foreign keys in the schema context. Prefer equality (\`=\` / \`IN\`) on the real name column — do not use \`LIKE '%…%'\` when Lookup hits or an exact name column are available.
- If the prompt includes a "Lookup hits" section, treat those rows as ground truth: filter with the returned TenantId/key values directly. Do not re-derive the entity with LIKE.
- When the prompt includes "Today:", use that date for relative or year-less ranges (e.g. "16 June to 16 July", "last month", "this year"). Do not invent a different calendar year.
- For SQL Server / Azure, prefer inline expressions over DECLARE/@variables so Cellar can auto-run the query as a single batch.
- Cellar runs queries read-only by default. Destructive statements (DROP, TRUNCATE, DELETE/UPDATE without WHERE) must be called out explicitly with a one-line warning.
- Be concise and technical. The user understands SQL.`;

export interface TopicMeta {
  // ponytail: label dropped — it was identical to the AiTopic key; callers use the key directly
  /** Hover hint / one-liner. */
  hint: string;
  /** Instruction prepended to the user's text for this preset. */
  instruction: string;
}

export const TOPICS: Record<AiTopic, TopicMeta> = {
  generate: {
    hint: "Write SQL from a description",
    instruction:
      "Generate a SQL query for the request below. Return the query in a single ```sql block, then a one-sentence explanation of what it does. Use only tables/columns from the schema context. Prefer JOINs via foreign keys when filtering by a named entity. Avoid DECLARE/@variables so the query can run as one batch.",
  },
  explain: {
    hint: "Explain SQL or answer with a read-only query",
    instruction:
      "If the request is SQL, explain what it returns and any performance characteristics worth noting. If it is a natural-language question about the data, return one read-only SQL query in a single ```sql block so Cellar can run it, then add one short sentence describing what the result will show. Use only tables/columns from the schema context. Prefer JOINs via foreign keys when filtering by a named entity (company/tenant/customer). Avoid DECLARE/@variables; keep the query a single runnable statement.",
  },
  optimize: {
    hint: "Suggest a faster equivalent query",
    instruction:
      "Optimize the SQL below. Return an improved query in a ```sql block, then a short bulleted list of the changes and why they help. Preserve the original result set.",
  },
  migrate: {
    hint: "Draft a schema migration",
    instruction:
      "Draft the SQL migration described below. Return forward DDL in a ```sql block. Flag any destructive or irreversible step explicitly, and note whether it can run inside a transaction.",
  },
  ask: {
    hint: "Free-form question",
    instruction: "",
  },
};

export const ORDERED_TOPICS: AiTopic[] = [
  "generate",
  "explain",
  "optimize",
  "migrate",
  "ask",
];

/** Local calendar "Today:" line so year-less dates resolve correctly. */
export function formatTodayContext(now: Date = new Date()): string {
  // en-CA → YYYY-MM-DD; weekday helps "this week" / "yesterday" phrasing.
  const iso = new Intl.DateTimeFormat("en-CA", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(now);
  const weekday = new Intl.DateTimeFormat("en-US", { weekday: "long" }).format(now);
  const year = now.getFullYear();
  return `Today: ${iso} (${weekday}). Current year: ${year}. Use this year for dates that omit one.`;
}

/** Assemble the user-role turn for a topic: preset instruction, then schema
 * context, optional lookup hits, then the user's own text. Any part may be empty. */
export function buildUserPrompt(
  topic: AiTopic,
  userText: string,
  context?: string,
  lookupHits?: string,
  now: Date = new Date(),
): string {
  const sections: string[] = [];
  const instruction = TOPICS[topic].instruction.trim();
  if (instruction) sections.push(instruction);
  sections.push(formatTodayContext(now));
  const ctx = context?.trim();
  if (ctx) sections.push(`Schema context:\n${ctx}`);
  const hits = lookupHits?.trim();
  if (hits) sections.push(hits);
  const text = userText.trim();
  if (text) sections.push(`Request:\n${text}`);
  return sections.join("\n\n");
}

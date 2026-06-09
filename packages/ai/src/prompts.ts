import type { AiTopic } from "./types";

/** System instruction sent out-of-band on every turn. Keeps the model anchored
 * on Cellar's product stance: inspectable, dialect-correct SQL, read-only by
 * default, destructive statements clearly flagged (SPEC §6.7). */
export const SYSTEM_PROMPT = `You are Cellar's built-in SQL assistant, embedded in a desktop database client.

Rules:
- The user works against a real database. Prefer correct, dialect-appropriate SQL over generic advice.
- When you return SQL, put it in a single fenced \`\`\`sql code block so the app can render and gate it.
- Use the schema context provided. Never invent table or column names; if something is missing, say so.
- Cellar runs queries read-only by default. Destructive statements (DROP, TRUNCATE, DELETE/UPDATE without WHERE) must be called out explicitly with a one-line warning.
- Be concise and technical. The user understands SQL.`;

export interface TopicMeta {
  /** Bottom-bar label. */
  label: string;
  /** Hover hint / one-liner. */
  hint: string;
  /** Instruction prepended to the user's text for this preset. */
  instruction: string;
}

export const TOPICS: Record<AiTopic, TopicMeta> = {
  generate: {
    label: "generate",
    hint: "Write SQL from a description",
    instruction:
      "Generate a SQL query for the request below. Return the query in a single ```sql block, then a one-sentence explanation of what it does.",
  },
  explain: {
    label: "explain",
    hint: "Explain SQL, a table, or a result",
    instruction:
      "Explain the following clearly and concisely. If it is SQL, describe what it returns and any performance characteristics worth noting.",
  },
  optimize: {
    label: "optimize",
    hint: "Suggest a faster equivalent query",
    instruction:
      "Optimize the SQL below. Return an improved query in a ```sql block, then a short bulleted list of the changes and why they help. Preserve the original result set.",
  },
  migrate: {
    label: "migrate",
    hint: "Draft a schema migration",
    instruction:
      "Draft the SQL migration described below. Return forward DDL in a ```sql block. Flag any destructive or irreversible step explicitly, and note whether it can run inside a transaction.",
  },
  ask: {
    label: "ask",
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

/** Assemble the user-role turn for a topic: preset instruction, then schema
 * context, then the user's own text. Any part may be empty. */
export function buildUserPrompt(
  topic: AiTopic,
  userText: string,
  context?: string,
): string {
  const sections: string[] = [];
  const instruction = TOPICS[topic].instruction.trim();
  if (instruction) sections.push(instruction);
  const ctx = context?.trim();
  if (ctx) sections.push(`Schema context:\n${ctx}`);
  const text = userText.trim();
  if (text) sections.push(`Request:\n${text}`);
  return sections.join("\n\n");
}

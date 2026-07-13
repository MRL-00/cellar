import type { SettingsCatId } from "./Settings";

export type SettingsSearchEntry = {
  cat: SettingsCatId;
  category: string;
  group: string;
  section: string;
  label: string;
  terms?: string[];
};

export type SettingsSearchResult = SettingsSearchEntry & {
  score: number;
};

const ENTRIES: SettingsSearchEntry[] = [
  {
    cat: "general",
    group: "Workspace",
    category: "General",
    section: "General",
    label: "Startup",
    terms: ["Restore last session", "Empty workspace", "Show welcome"],
  },
  {
    cat: "general",
    group: "Workspace",
    category: "General",
    section: "General",
    label: "Default schema search path",
    terms: ["public", "audit", "analytics"],
  },
  {
    cat: "general",
    group: "Workspace",
    category: "General",
    section: "General",
    label: "Confirm before quitting",
  },
  {
    cat: "general",
    group: "Workspace",
    category: "General",
    section: "General",
    label: "Allow background queries",
  },
  {
    cat: "general",
    group: "Workspace",
    category: "General",
    section: "Updates",
    label: "Channel",
    terms: ["stable", "beta", "nightly"],
  },
  {
    cat: "general",
    group: "Workspace",
    category: "General",
    section: "Updates",
    label: "Auto-install on quit",
  },
  {
    cat: "appearance",
    group: "Workspace",
    category: "Appearance",
    section: "Theme",
    label: "Theme",
    terms: ["system", "dark", "light"],
  },
  {
    cat: "appearance",
    group: "Workspace",
    category: "Appearance",
    section: "Theme",
    label: "Accent",
    terms: ["color", "swatch", "palette"],
  },
  {
    cat: "appearance",
    group: "Workspace",
    category: "Appearance",
    section: "Theme",
    label: "Density",
    terms: ["compact", "comfortable"],
  },
  {
    cat: "appearance",
    group: "Workspace",
    category: "Appearance",
    section: "Type",
    label: "Interface font",
    terms: ["sans", "typeface"],
  },
  {
    cat: "appearance",
    group: "Workspace",
    category: "Appearance",
    section: "Type",
    label: "Editor / mono font",
    terms: ["monospace", "sql", "typeface"],
  },
  {
    cat: "appearance",
    group: "Workspace",
    category: "Appearance",
    section: "Type",
    label: "Font size",
    terms: ["scales the entire interface", "px"],
  },
  {
    cat: "appearance",
    group: "Workspace",
    category: "Appearance",
    section: "Window",
    label: "Show traffic lights",
  },
  {
    cat: "appearance",
    group: "Workspace",
    category: "Appearance",
    section: "Window",
    label: "Native window controls",
  },
  {
    cat: "editor",
    group: "Workspace",
    category: "Editor",
    section: "SQL editor",
    label: "Tab size",
    terms: ["2", "4", "8"],
  },
  {
    cat: "editor",
    group: "Workspace",
    category: "Editor",
    section: "SQL editor",
    label: "Indent with",
    terms: ["spaces", "tabs"],
  },
  {
    cat: "editor",
    group: "Workspace",
    category: "Editor",
    section: "SQL editor",
    label: "Auto-format on save",
    terms: ["format"],
  },
  {
    cat: "editor",
    group: "Workspace",
    category: "Editor",
    section: "SQL editor",
    label: "Keyword case",
    terms: ["UPPER", "lower", "Preserve"],
  },
  {
    cat: "editor",
    group: "Workspace",
    category: "Editor",
    section: "SQL editor",
    label: "Show line numbers",
  },
  {
    cat: "editor",
    group: "Workspace",
    category: "Editor",
    section: "SQL editor",
    label: "Soft wrap",
  },
  {
    cat: "editor",
    group: "Workspace",
    category: "Editor",
    section: "SQL editor",
    label: "Bracket matching",
  },
  {
    cat: "editor",
    group: "Workspace",
    category: "Editor",
    section: "Execution",
    label: "Statement at cursor runs",
    terms: ["current statement", "selection", "whole file", "run"],
  },
  {
    cat: "editor",
    group: "Workspace",
    category: "Editor",
    section: "Execution",
    label: "LIMIT applied to SELECT *",
    terms: ["row limit", "select"],
  },
  {
    cat: "grid",
    group: "Workspace",
    category: "Data grid",
    section: "Data grid",
    label: "Row height",
    terms: ["20px", "22px", "28px", "36px"],
  },
  {
    cat: "grid",
    group: "Workspace",
    category: "Data grid",
    section: "Data grid",
    label: "NULL display",
    terms: ["dim italic", "strong"],
  },
  {
    cat: "grid",
    group: "Workspace",
    category: "Data grid",
    section: "Data grid",
    label: "Number alignment",
    terms: ["left", "right"],
  },
  {
    cat: "grid",
    group: "Workspace",
    category: "Data grid",
    section: "Data grid",
    label: "Stripe alternating rows",
  },
  {
    cat: "grid",
    group: "Workspace",
    category: "Data grid",
    section: "Data grid",
    label: "Remember table sort",
    terms: ["last sort", "column sort", "order by", "persist sort"],
  },
  {
    cat: "grid",
    group: "Workspace",
    category: "Data grid",
    section: "Data grid",
    label: "Sticky pkey column",
    terms: ["primary key", "frozen"],
  },
  {
    cat: "grid",
    group: "Workspace",
    category: "Data grid",
    section: "Data grid",
    label: "Truncate cells over",
    terms: ["max cell preview length", "characters"],
  },
  {
    cat: "keymap",
    group: "Workspace",
    category: "Keymap",
    section: "Keymap",
    label: "Preset",
    terms: ["Cellar", "DataGrip", "VS Code", "Linear"],
  },
  {
    cat: "keymap",
    group: "Workspace",
    category: "Keymap",
    section: "Workspace",
    label: "Command palette",
    terms: ["New connection", "New SQL tab", "Close tab", "Settings"],
  },
  {
    cat: "keymap",
    group: "Workspace",
    category: "Keymap",
    section: "Editor",
    label: "Run statement",
    terms: ["Run selection", "Format", "Accept ghost text", "Show schema"],
  },
  {
    cat: "keymap",
    group: "Workspace",
    category: "Keymap",
    section: "Grid",
    label: "Edit cell",
    terms: ["Revert cell", "Commit changes", "Set NULL"],
  },
  {
    cat: "connections",
    group: "Data",
    category: "Connections",
    section: "Defaults for new connections",
    label: "Read-only by default",
  },
  {
    cat: "connections",
    group: "Data",
    category: "Connections",
    section: "Defaults for new connections",
    label: "Connection timeout",
    terms: ["seconds"],
  },
  {
    cat: "connections",
    group: "Data",
    category: "Connections",
    section: "Defaults for new connections",
    label: "Keep-alive interval",
    terms: ["seconds"],
  },
  {
    cat: "connections",
    group: "Data",
    category: "Connections",
    section: "Defaults for new connections",
    label: "Application name",
    terms: ["cellar", "client"],
  },
  {
    cat: "connections",
    group: "Data",
    category: "Connections",
    section: "Production safety",
    label: "Confirm DML on prod",
    terms: ["production", "destructive"],
  },
  {
    cat: "connections",
    group: "Data",
    category: "Connections",
    section: "Production safety",
    label: "Confirm DROP / TRUNCATE on prod",
    terms: ["production", "destructive"],
  },
  {
    cat: "connections",
    group: "Data",
    category: "Connections",
    section: "Production safety",
    label: "Block UPDATE without WHERE",
  },
  {
    cat: "connections",
    group: "Data",
    category: "Connections",
    section: "Production safety",
    label: "Block DELETE without WHERE",
  },
  {
    cat: "connections",
    group: "Data",
    category: "Connections",
    section: "Production safety",
    label: "Max rows affected before warn",
  },
  {
    cat: "history",
    group: "Data",
    category: "Query history",
    section: "Query history",
    label: "Retain history for",
    terms: ["7 days", "30 days", "90 days", "forever"],
  },
  {
    cat: "history",
    group: "Data",
    category: "Query history",
    section: "Query history",
    label: "Store query results",
  },
  {
    cat: "history",
    group: "Data",
    category: "Query history",
    section: "Query history",
    label: "Storage summary",
    terms: ["queries", "last cleared", "MB"],
  },
  {
    cat: "backups",
    group: "Data",
    category: "Backups & exports",
    section: "Backups",
    label: "Auto-snapshot before commits",
    terms: ["pg_dump", "schema-only", "affected rows"],
  },
  {
    cat: "backups",
    group: "Data",
    category: "Backups & exports",
    section: "Backups",
    label: "Snapshot location",
    terms: ["~/.cellar/snapshots"],
  },
  {
    cat: "backups",
    group: "Data",
    category: "Backups & exports",
    section: "Backups",
    label: "Retain snapshots for",
    terms: ["days"],
  },
  {
    cat: "backups",
    group: "Data",
    category: "Backups & exports",
    section: "Export defaults",
    label: "Format",
    terms: ["CSV", "JSON", "Parquet", "SQL INSERT"],
  },
  {
    cat: "backups",
    group: "Data",
    category: "Backups & exports",
    section: "Export defaults",
    label: "NULL as",
    terms: ["\\N"],
  },
  {
    cat: "backups",
    group: "Data",
    category: "Backups & exports",
    section: "Export defaults",
    label: "Include headers",
  },
  {
    cat: "ai",
    group: "Intelligence",
    category: "AI Assistant",
    section: "AI Assistant",
    label: "Bring-your-own-key",
    terms: ["schema", "queries", "results", "provider", "privacy"],
  },
  {
    cat: "ai",
    group: "Intelligence",
    category: "AI Assistant",
    section: "Provider",
    label: "Provider",
    terms: ["Anthropic", "OpenAI", "Google", "Local", "Custom", "Ollama", "LM Studio"],
  },
  {
    cat: "ai",
    group: "Intelligence",
    category: "AI Assistant",
    section: "Provider",
    label: "Model",
    terms: [
      "claude",
      "gpt",
      "gemini",
      "local-default",
      "balanced",
      "fast",
      "max",
    ],
  },
  {
    cat: "ai",
    group: "Intelligence",
    category: "AI Assistant",
    section: "Provider",
    label: "API key",
    terms: ["keychain", "stored", "secret"],
  },
  {
    cat: "ai",
    group: "Intelligence",
    category: "AI Assistant",
    section: "Provider",
    label: "Endpoint",
    terms: ["proxy", "custom router", "OpenAI-compatible"],
  },
  {
    cat: "ai",
    group: "Intelligence",
    category: "AI Assistant",
    section: "Danger zone",
    label: "Clear AI conversation history",
    terms: ["local", "delete"],
  },
  {
    cat: "ai",
    group: "Intelligence",
    category: "AI Assistant",
    section: "Danger zone",
    label: "Revoke API key",
    terms: ["remove from keychain", "provider"],
  },
  {
    cat: "privacy",
    group: "System",
    category: "Privacy & telemetry",
    section: "Telemetry",
    label: "Send anonymous usage stats",
    terms: ["counts of feature use", "no query content"],
  },
  {
    cat: "privacy",
    group: "System",
    category: "Privacy & telemetry",
    section: "Telemetry",
    label: "Send crash reports",
    terms: ["stack traces", "never DB contents"],
  },
  {
    cat: "privacy",
    group: "System",
    category: "Privacy & telemetry",
    section: "Stored locally only",
    label: "Local data",
    terms: [
      "connections.toml",
      "history.sqlite",
      "AI conversations",
      "snapshots",
      "cached schemas",
      "~/.cellar",
    ],
  },
  {
    cat: "updates",
    group: "System",
    category: "Updates",
    section: "Updates",
    label: "Version",
    terms: ["v0.1.0-alpha", "updater", "last checked", "check now"],
  },
  {
    cat: "updates",
    group: "System",
    category: "Updates",
    section: "Updates",
    label: "Channel",
    terms: ["stable", "beta", "nightly"],
  },
  {
    cat: "updates",
    group: "System",
    category: "Updates",
    section: "Updates",
    label: "Auto-install on quit",
  },
  {
    cat: "about",
    group: "System",
    category: "About",
    section: "About",
    label: "Cellar",
    terms: ["database client", "AI", "development build", "MIT licensed"],
  },
  {
    cat: "about",
    group: "System",
    category: "About",
    section: "About",
    label: "Links",
    terms: ["docs", "github", "changelog", "acknowledgements"],
  },
];

const CATEGORY_SCORE = 8;
const SECTION_SCORE = 4;
const LABEL_SCORE = 6;
const TERM_SCORE = 2;

function normalize(value: string) {
  return value.toLowerCase().replace(/\s+/g, " ").trim();
}

function tokens(query: string) {
  return normalize(query).split(" ").filter(Boolean);
}

function scoreEntry(entry: SettingsSearchEntry, queryTokens: string[]) {
  const fields = [
    entry.group,
    entry.category,
    entry.section,
    entry.label,
    ...(entry.terms ?? []),
  ].map(normalize);
  const haystack = fields.join(" ");

  if (!queryTokens.every((token) => haystack.includes(token))) return 0;

  return queryTokens.reduce((score, token) => {
    if (normalize(entry.category).includes(token)) return score + CATEGORY_SCORE;
    if (normalize(entry.label).includes(token)) return score + LABEL_SCORE;
    if (normalize(entry.section).includes(token)) return score + SECTION_SCORE;
    return score + TERM_SCORE;
  }, 0);
}

export function searchSettings(query: string): SettingsSearchResult[] {
  const queryTokens = tokens(query);
  if (!queryTokens.length) return [];

  return ENTRIES.map((entry) => ({ ...entry, score: scoreEntry(entry, queryTokens) }))
    .filter((entry) => entry.score > 0)
    .sort((a, b) => b.score - a.score || a.category.localeCompare(b.category));
}

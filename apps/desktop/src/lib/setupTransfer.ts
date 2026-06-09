// Share-your-setup import/export. Builds a portable JSON "setup bundle" out of
// the user's connections, appearance settings, and per-table grid layouts, and
// merges a bundle back in with per-item review + duplicate detection.
//
// Security: passwords are never part of a bundle. Connection secrets live in
// the OS keychain, not in `ConnectionConfig`, and `coerceConnection` rebuilds
// each connection from a known field allowlist so a hand-edited bundle can't
// smuggle a `password` field back into the app.

import type { ConnectionConfig, Engine, EnvTag, SslMode } from "@cellar/ipc";
import type { GridColumnLayout } from "@cellar/data-grid";
import type { TableLayouts } from "../state/tabs";
import { DEFAULTS as SETTINGS_DEFAULTS, sanitize as sanitizeSettings, type Settings } from "./settings";

export const SETUP_FORMAT = "cellar.setup";
export const SETUP_VERSION = 1;

export type SetupSectionKey = "settings" | "connections" | "tableLayouts";

export interface SetupBundle {
  format: typeof SETUP_FORMAT;
  version: number;
  exportedAt: string;
  app: string;
  sections: {
    settings?: Settings;
    connections?: ConnectionConfig[];
    tableLayouts?: TableLayouts;
  };
}

export type SetupSelection = Record<SetupSectionKey, boolean>;

export interface SetupSources {
  settings: Settings;
  connections: ConnectionConfig[];
  tableLayouts: TableLayouts;
}

const ENGINES: Engine[] = [
  "postgres",
  "mysql",
  "sqlite",
  "mssql",
  "azure",
  "firestore",
];
const SSL_MODES: SslMode[] = [
  "disable",
  "prefer",
  "require",
  "verify-ca",
  "verify-full",
];
const ENV_TAGS: EnvTag[] = ["local", "dev", "staging", "prod"];
const THEMES: Settings["theme"][] = ["system", "dark", "light"];
const DENSITIES: Settings["density"][] = ["compact", "comfortable"];

// ---------------------------------------------------------------------------
// Build (export)
// ---------------------------------------------------------------------------

export function buildBundle(
  selection: SetupSelection,
  sources: SetupSources,
  meta: { app: string; exportedAt: string },
): SetupBundle {
  const sections: SetupBundle["sections"] = {};
  if (selection.settings) {
    sections.settings = sanitizeSettings({
      ...SETTINGS_DEFAULTS,
      ...sources.settings,
    });
  }
  if (selection.connections) {
    sections.connections = sources.connections
      .map(coerceConnection)
      .filter((c): c is ConnectionConfig => c !== null);
  }
  if (selection.tableLayouts) {
    sections.tableLayouts = coerceTableLayouts(sources.tableLayouts);
  }
  return {
    format: SETUP_FORMAT,
    version: SETUP_VERSION,
    exportedAt: meta.exportedAt,
    app: meta.app,
    sections,
  };
}

export function serializeBundle(bundle: SetupBundle): string {
  return JSON.stringify(bundle, null, 2);
}

/** Count of items per section, for UI summaries. */
export function sectionCounts(sources: SetupSources): Record<SetupSectionKey, number> {
  return {
    settings: 1,
    connections: sources.connections.length,
    tableLayouts: Object.keys(sources.tableLayouts).length,
  };
}

// ---------------------------------------------------------------------------
// Parse (import)
// ---------------------------------------------------------------------------

export type ParseResult =
  | { ok: true; bundle: SetupBundle }
  | { ok: false; error: string };

export function parseBundle(text: string): ParseResult {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch {
    return { ok: false, error: "That doesn't look like valid JSON." };
  }
  if (!raw || typeof raw !== "object") {
    return { ok: false, error: "The file is empty or not a setup bundle." };
  }
  const obj = raw as Record<string, unknown>;
  if (obj.format !== SETUP_FORMAT) {
    return {
      ok: false,
      error: "Not a Cellar setup file (missing the cellar.setup marker).",
    };
  }
  const version = typeof obj.version === "number" ? obj.version : 0;
  if (version > SETUP_VERSION) {
    return {
      ok: false,
      error: `This file was exported by a newer Cellar (v${version}). Update Cellar to import it.`,
    };
  }

  const sectionsRaw =
    obj.sections && typeof obj.sections === "object"
      ? (obj.sections as Record<string, unknown>)
      : {};
  const sections: SetupBundle["sections"] = {};

  if (sectionsRaw.settings && typeof sectionsRaw.settings === "object") {
    sections.settings = coerceSettings(
      sectionsRaw.settings as Record<string, unknown>,
    );
  }
  if (Array.isArray(sectionsRaw.connections)) {
    sections.connections = sectionsRaw.connections
      .map(coerceConnection)
      .filter((c): c is ConnectionConfig => c !== null);
  }
  if (sectionsRaw.tableLayouts && typeof sectionsRaw.tableLayouts === "object") {
    sections.tableLayouts = coerceTableLayouts(sectionsRaw.tableLayouts);
  }

  const hasSettings = Boolean(sections.settings);
  const hasConnections = Boolean(sections.connections?.length);
  const hasLayouts =
    Boolean(sections.tableLayouts) &&
    Object.keys(sections.tableLayouts ?? {}).length > 0;
  if (!hasSettings && !hasConnections && !hasLayouts) {
    return { ok: false, error: "This file has no importable sections." };
  }

  return {
    ok: true,
    bundle: {
      format: SETUP_FORMAT,
      version,
      exportedAt: typeof obj.exportedAt === "string" ? obj.exportedAt : "",
      app: typeof obj.app === "string" ? obj.app : "",
      sections,
    },
  };
}

/**
 * Rebuild a `ConnectionConfig` from a known field allowlist. Returns null if it
 * lacks the minimum to be usable. This is the choke point that guarantees no
 * secret/extra field survives a round-trip.
 */
function coerceConnection(raw: unknown): ConnectionConfig | null {
  if (!raw || typeof raw !== "object") return null;
  const c = raw as Record<string, unknown>;
  const host = asString(c.host);
  const name = asString(c.name);
  const id = asString(c.id);
  // Need at least something to identify and reach the server.
  if (!host && !name && !id) return null;
  return {
    id,
    name: name || host,
    engine: ENGINES.includes(c.engine as Engine) ? (c.engine as Engine) : "postgres",
    host,
    port: Number.isFinite(Number(c.port)) ? Number(c.port) : 0,
    database: asString(c.database),
    user: asString(c.user),
    ssl_mode: SSL_MODES.includes(c.ssl_mode as SslMode)
      ? (c.ssl_mode as SslMode)
      : "prefer",
    env_tag: ENV_TAGS.includes(c.env_tag as EnvTag) ? (c.env_tag as EnvTag) : null,
    application_name: c.application_name != null ? asString(c.application_name) : null,
    color: c.color != null ? asString(c.color) : null,
  };
}

function coerceSettings(raw: Record<string, unknown>): Settings {
  const picked: Partial<Settings> = {};
  if (THEMES.includes(raw.theme as Settings["theme"])) {
    picked.theme = raw.theme as Settings["theme"];
  }
  if (DENSITIES.includes(raw.density as Settings["density"])) {
    picked.density = raw.density as Settings["density"];
  }
  if (typeof raw.accent === "string") picked.accent = raw.accent;
  if (typeof raw.fontSizePx === "number") picked.fontSizePx = raw.fontSizePx;
  if (typeof raw.interfaceFont === "string") picked.interfaceFont = raw.interfaceFont;
  if (typeof raw.monoFont === "string") picked.monoFont = raw.monoFont;
  return sanitizeSettings({ ...SETTINGS_DEFAULTS, ...picked });
}

/** Mirror of `loadTableLayouts` coercion in state/tabs.ts. */
function coerceTableLayouts(raw: unknown): TableLayouts {
  const out: TableLayouts = {};
  if (!raw || typeof raw !== "object") return out;
  for (const [key, value] of Object.entries(raw)) {
    if (!value || typeof value !== "object") continue;
    const item = value as Partial<GridColumnLayout>;
    out[key] = {
      order: Array.isArray(item.order)
        ? item.order.filter((k): k is string => typeof k === "string")
        : [],
      widths:
        item.widths && typeof item.widths === "object"
          ? Object.fromEntries(
              Object.entries(item.widths).filter(
                ([k, w]) => typeof k === "string" && typeof w === "number",
              ),
            )
          : {},
    };
  }
  return out;
}

function asString(value: unknown): string {
  return typeof value === "string" ? value : value == null ? "" : String(value);
}

// ---------------------------------------------------------------------------
// Import plan (per-item review)
// ---------------------------------------------------------------------------

export type ConnDecision = "skip" | "add" | "replace" | "copy";
export type LayoutDecision = "skip" | "add" | "replace";

export interface ConnImportItem {
  incoming: ConnectionConfig;
  identity: string;
  /** Existing connection with the same identity, if any. */
  duplicateOfId: string | null;
  duplicateOfName: string | null;
  decision: ConnDecision;
}

export interface LayoutImportItem {
  key: string;
  /** `database.schema.table` parsed from the storage key, for display. */
  tablePath: string;
  connectionId: string;
  layout: GridColumnLayout;
  exists: boolean;
  decision: LayoutDecision;
}

export interface SettingsImportItem {
  settings: Settings;
  apply: boolean;
}

export interface ImportPlan {
  connections: ConnImportItem[];
  layouts: LayoutImportItem[];
  settings: SettingsImportItem | null;
}

export function connectionIdentity(c: ConnectionConfig): string {
  const norm = (s: string) => s.trim().toLowerCase();
  return [c.engine, norm(c.host), c.port, norm(c.database), norm(c.user)].join("|");
}

export function tablePathFromKey(key: string): string {
  const idx = key.indexOf("::");
  return idx >= 0 ? key.slice(idx + 2) : key;
}

export function connectionIdFromKey(key: string): string {
  const idx = key.indexOf("::");
  return idx >= 0 ? key.slice(0, idx) : "";
}

export function computeImportPlan(
  bundle: SetupBundle,
  current: { connections: ConnectionConfig[]; tableLayouts: TableLayouts },
): ImportPlan {
  const byIdentity = new Map<string, ConnectionConfig>();
  for (const c of current.connections) {
    byIdentity.set(connectionIdentity(c), c);
  }

  const connections: ConnImportItem[] = (bundle.sections.connections ?? []).map(
    (incoming) => {
      const identity = connectionIdentity(incoming);
      const dup = byIdentity.get(identity) ?? null;
      return {
        incoming,
        identity,
        duplicateOfId: dup?.id ?? null,
        duplicateOfName: dup?.name ?? null,
        // Don't import the same thing twice by default.
        decision: dup ? "skip" : "add",
      };
    },
  );

  const layouts: LayoutImportItem[] = Object.entries(
    bundle.sections.tableLayouts ?? {},
  ).map(([key, layout]) => {
    const exists = Object.prototype.hasOwnProperty.call(
      current.tableLayouts,
      key,
    );
    return {
      key,
      tablePath: tablePathFromKey(key),
      connectionId: connectionIdFromKey(key),
      layout,
      exists,
      decision: exists ? "skip" : "add",
    };
  });

  const settings = bundle.sections.settings
    ? { settings: bundle.sections.settings, apply: true }
    : null;

  return { connections, layouts, settings };
}

// ---------------------------------------------------------------------------
// Id allocation
// ---------------------------------------------------------------------------

export function slugifyId(s: string): string {
  return (
    s
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 64) || "connection"
  );
}

export function uniqueConnectionId(base: string, taken: Set<string>): string {
  const slug = slugifyId(base);
  if (!taken.has(slug)) return slug;
  let n = 2;
  while (taken.has(`${slug}-${n}`)) n += 1;
  return `${slug}-${n}`;
}

// ---------------------------------------------------------------------------
// Apply
// ---------------------------------------------------------------------------

export interface ImportDeps {
  existingConnectionIds: string[];
  saveConnection: (
    config: ConnectionConfig,
    password: string | null,
  ) => Promise<unknown>;
  importSettings: (settings: Partial<Settings>) => void;
  importTableLayouts: (entries: TableLayouts) => void;
}

export interface ImportResult {
  connectionsAdded: number;
  connectionsReplaced: number;
  connectionsSkipped: number;
  layoutsAdded: number;
  layoutsReplaced: number;
  layoutsSkipped: number;
  settingsApplied: boolean;
}

export async function applyImportPlan(
  plan: ImportPlan,
  deps: ImportDeps,
): Promise<ImportResult> {
  const result: ImportResult = {
    connectionsAdded: 0,
    connectionsReplaced: 0,
    connectionsSkipped: 0,
    layoutsAdded: 0,
    layoutsReplaced: 0,
    layoutsSkipped: 0,
    settingsApplied: false,
  };

  const taken = new Set(deps.existingConnectionIds);

  // Sequential so generated ids stay deterministic and collision-free.
  for (const item of plan.connections) {
    if (item.decision === "skip") {
      result.connectionsSkipped += 1;
      continue;
    }
    if (item.decision === "replace" && item.duplicateOfId) {
      // Reuse the existing id so the keychain password (kept by passing null)
      // and any open state stay attached to the same connection.
      await deps.saveConnection(
        { ...item.incoming, id: item.duplicateOfId },
        null,
      );
      result.connectionsReplaced += 1;
      continue;
    }

    const isCopy = item.decision === "copy";
    const name = isCopy
      ? `${item.incoming.name} (imported)`
      : item.incoming.name || item.incoming.host;
    const preferredId = isCopy ? "" : item.incoming.id;
    const id =
      preferredId && !taken.has(preferredId)
        ? preferredId
        : uniqueConnectionId(name || preferredId || "connection", taken);
    taken.add(id);
    await deps.saveConnection({ ...item.incoming, id, name }, null);
    result.connectionsAdded += 1;
  }

  const layoutsToApply: TableLayouts = {};
  for (const item of plan.layouts) {
    if (item.decision === "skip") {
      result.layoutsSkipped += 1;
      continue;
    }
    layoutsToApply[item.key] = item.layout;
    if (item.decision === "replace") result.layoutsReplaced += 1;
    else result.layoutsAdded += 1;
  }
  if (Object.keys(layoutsToApply).length > 0) {
    deps.importTableLayouts(layoutsToApply);
  }

  if (plan.settings?.apply) {
    deps.importSettings(plan.settings.settings);
    result.settingsApplied = true;
  }

  return result;
}

import { describe, expect, it } from "vitest";
import type { ConnectionConfig } from "@cellar/ipc";

import {
  applyImportPlan,
  buildBundle,
  computeImportPlan,
  connectionIdentity,
  parseBundle,
  serializeBundle,
  uniqueConnectionId,
  type SetupBundle,
} from "./setupTransfer";

function conn(overrides: Partial<ConnectionConfig> = {}): ConnectionConfig {
  return {
    id: "local-pg",
    name: "Local Postgres",
    engine: "postgres",
    host: "localhost",
    port: 5432,
    database: "app",
    user: "postgres",
    ssl_mode: "prefer",
    env_tag: "local",
    application_name: "cellar",
    color: "#4f8ff7",
    ...overrides,
  };
}

describe("connectionIdentity", () => {
  it("matches on engine/host/port/db/user, ignoring id, name and case", () => {
    const a = conn({ id: "a", name: "A", host: "LocalHost", user: "Postgres" });
    const b = conn({ id: "b", name: "B different", host: "localhost", user: "postgres" });
    expect(connectionIdentity(a)).toBe(connectionIdentity(b));
  });

  it("differs when the target server differs", () => {
    expect(connectionIdentity(conn({ database: "app" }))).not.toBe(
      connectionIdentity(conn({ database: "other" })),
    );
  });
});

describe("buildBundle", () => {
  const sources = {
    settings: {
      theme: "dark" as const,
      density: "compact" as const,
      accent: "#a78bfa",
      fontSizePx: 12.5,
      interfaceFont: "Geist",
      monoFont: "JetBrains Mono",
      editor: {
        tabSize: 4 as const,
        softWrap: false,
        lineNumbers: true,
        bracketMatching: true,
      },
      grid: {
        nullDisplay: "NULL" as const,
        stripeRows: false,
        rememberTableSort: true,
      },
    },
    connections: [conn()],
    tableLayouts: { "local-pg::app.public.orders": { order: ["id"], widths: { id: 80 } } },
  };

  it("only includes selected sections", () => {
    const bundle = buildBundle(
      { settings: true, connections: false, tableLayouts: false },
      sources,
      { app: "0.1.0", exportedAt: "2026-06-09T00:00:00Z" },
    );
    expect(bundle.sections.settings).toBeDefined();
    expect(bundle.sections.connections).toBeUndefined();
    expect(bundle.sections.tableLayouts).toBeUndefined();
  });

  it("never serializes a password, even if one was attached", () => {
    const dirty = { ...conn(), password: "hunter2", secret: "x" } as ConnectionConfig;
    const bundle = buildBundle(
      { settings: false, connections: true, tableLayouts: false },
      { ...sources, connections: [dirty] },
      { app: "0.1.0", exportedAt: "2026-06-09T00:00:00Z" },
    );
    const json = serializeBundle(bundle);
    expect(json).not.toContain("hunter2");
    expect(json).not.toContain("password");
    expect(json).not.toContain("secret");
  });

  it("exports only the selected connection ids", () => {
    const other = conn({
      id: "staging-pg",
      name: "Staging",
      database: "staging",
      env_tag: "staging",
    });
    const bundle = buildBundle(
      { settings: false, connections: true, tableLayouts: false },
      { ...sources, connections: [conn(), other] },
      { app: "0.1.0", exportedAt: "2026-06-09T00:00:00Z" },
      { connectionIds: new Set(["staging-pg"]) },
    );
    expect(bundle.sections.connections).toHaveLength(1);
    expect(bundle.sections.connections?.[0]?.id).toBe("staging-pg");
  });

  it("omits table layouts for connections that were not selected", () => {
    const other = conn({ id: "staging-pg", name: "Staging", database: "staging" });
    const bundle = buildBundle(
      { settings: false, connections: true, tableLayouts: true },
      {
        ...sources,
        connections: [conn(), other],
        tableLayouts: {
          "local-pg::app.public.orders": { order: ["id"], widths: { id: 80 } },
          "staging-pg::staging.public.users": {
            order: ["email"],
            widths: { email: 120 },
          },
        },
      },
      { app: "0.1.0", exportedAt: "2026-06-09T00:00:00Z" },
      { connectionIds: new Set(["staging-pg"]) },
    );
    expect(bundle.sections.connections?.map((c) => c.id)).toEqual(["staging-pg"]);
    expect(Object.keys(bundle.sections.tableLayouts ?? {})).toEqual([
      "staging-pg::staging.public.users",
    ]);
  });

  it("keeps all table layouts when no connection filter is provided", () => {
    const bundle = buildBundle(
      { settings: false, connections: false, tableLayouts: true },
      {
        ...sources,
        tableLayouts: {
          "local-pg::app.public.orders": { order: ["id"], widths: { id: 80 } },
          "staging-pg::staging.public.users": {
            order: ["email"],
            widths: { email: 120 },
          },
        },
      },
      { app: "0.1.0", exportedAt: "2026-06-09T00:00:00Z" },
    );
    expect(bundle.sections.connections).toBeUndefined();
    expect(Object.keys(bundle.sections.tableLayouts ?? {})).toHaveLength(2);
  });

  it("filters table layouts even when the connections section is omitted", () => {
    // Mirrors Export setup with Connections checked but every row unchecked:
    // connections are skipped, but layouts must still respect the empty filter.
    const bundle = buildBundle(
      { settings: false, connections: false, tableLayouts: true },
      {
        ...sources,
        tableLayouts: {
          "local-pg::app.public.orders": { order: ["id"], widths: { id: 80 } },
          "staging-pg::staging.public.users": {
            order: ["email"],
            widths: { email: 120 },
          },
        },
      },
      { app: "0.1.0", exportedAt: "2026-06-09T00:00:00Z" },
      { connectionIds: new Set() },
    );
    expect(bundle.sections.connections).toBeUndefined();
    expect(Object.keys(bundle.sections.tableLayouts ?? {})).toHaveLength(0);
  });
});

describe("parseBundle", () => {
  function validJson(): string {
    return serializeBundle(
      buildBundle(
        { settings: false, connections: true, tableLayouts: false },
        {
          settings: {
            theme: "dark",
            density: "compact",
            accent: "#a78bfa",
            fontSizePx: 12.5,
            interfaceFont: "Geist",
            monoFont: "JetBrains Mono",
            editor: {
              tabSize: 4 as const,
              softWrap: false,
              lineNumbers: true,
              bracketMatching: true,
            },
            grid: {
              nullDisplay: "NULL" as const,
              stripeRows: false,
              rememberTableSort: true,
            },
          },
          connections: [conn()],
          tableLayouts: {},
        },
        { app: "0.1.0", exportedAt: "2026-06-09T00:00:00Z" },
      ),
    );
  }

  it("rejects non-JSON", () => {
    const r = parseBundle("not json");
    expect(r.ok).toBe(false);
  });

  it("rejects files without the cellar.setup marker", () => {
    const r = parseBundle(JSON.stringify({ foo: "bar" }));
    expect(r.ok).toBe(false);
  });

  it("rejects a newer bundle version", () => {
    const r = parseBundle(
      JSON.stringify({ format: "cellar.setup", version: 999, sections: { connections: [conn()] } }),
    );
    expect(r.ok).toBe(false);
  });

  it("accepts a valid bundle and strips unknown connection fields", () => {
    const r = parseBundle(
      JSON.stringify({
        format: "cellar.setup",
        version: 1,
        sections: { connections: [{ ...conn(), password: "leak" }] },
      }),
    );
    expect(r.ok).toBe(true);
    if (r.ok) {
      const c = r.bundle.sections.connections?.[0] as Record<string, unknown>;
      expect(c.password).toBeUndefined();
      expect(c.host).toBe("localhost");
    }
  });

  it("round-trips a built bundle", () => {
    const r = parseBundle(validJson());
    expect(r.ok).toBe(true);
  });

  it("round-trips a bundle with editor and grid settings", () => {
    const bundleWithSettings = serializeBundle(
      buildBundle(
        { settings: true, connections: false, tableLayouts: false },
        {
          settings: {
            theme: "dark",
            density: "compact",
            accent: "#a78bfa",
            fontSizePx: 12.5,
            interfaceFont: "Geist",
            monoFont: "JetBrains Mono",
            editor: { tabSize: 2, softWrap: true, lineNumbers: false, bracketMatching: false },
            grid: { nullDisplay: "∅", stripeRows: true, rememberTableSort: true },
          },
          connections: [],
          tableLayouts: {},
        },
        { app: "0.1.0", exportedAt: "2026-06-09T00:00:00Z" },
      ),
    );
    const r = parseBundle(bundleWithSettings);
    expect(r.ok).toBe(true);
    if (r.ok) {
      const s = r.bundle.sections.settings!;
      expect(s.editor.tabSize).toBe(2);
      expect(s.editor.softWrap).toBe(true);
      expect(s.editor.lineNumbers).toBe(false);
      expect(s.grid.nullDisplay).toBe("∅");
      expect(s.grid.stripeRows).toBe(true);
      expect(s.grid.rememberTableSort).toBe(true);
    }
  });

  it("rejects a bundle with no importable sections", () => {
    const r = parseBundle(JSON.stringify({ format: "cellar.setup", version: 1, sections: {} }));
    expect(r.ok).toBe(false);
  });
});

describe("computeImportPlan", () => {
  const bundle: SetupBundle = {
    format: "cellar.setup",
    version: 1,
    exportedAt: "",
    app: "",
    sections: {
      connections: [conn({ id: "incoming", name: "Imported PG" }), conn({ database: "fresh" })],
      tableLayouts: {
        "x::app.public.orders": { order: [], widths: {} },
        "x::app.public.users": { order: [], widths: {} },
      },
      settings: {
        theme: "light",
        density: "comfortable",
        accent: "#fff",
        fontSizePx: 14,
        interfaceFont: "Geist",
        monoFont: "JetBrains Mono",
        editor: { tabSize: 4, softWrap: false, lineNumbers: true, bracketMatching: true },
        grid: { nullDisplay: "NULL", stripeRows: false, rememberTableSort: true },
      },
    },
  };

  it("defaults duplicates to skip and new items to add", () => {
    const plan = computeImportPlan(bundle, {
      connections: [conn()], // same identity as first incoming
      tableLayouts: { "x::app.public.orders": { order: [], widths: {} } },
    });
    const dup = plan.connections[0]!;
    const fresh = plan.connections[1]!;
    expect(dup.duplicateOfId).toBe("local-pg");
    expect(dup.decision).toBe("skip");
    expect(fresh.duplicateOfId).toBeNull();
    expect(fresh.decision).toBe("add");

    const existing = plan.layouts.find((l) => l.key === "x::app.public.orders");
    const newLayout = plan.layouts.find((l) => l.key === "x::app.public.users");
    expect(existing?.decision).toBe("skip");
    expect(newLayout?.decision).toBe("add");
    expect(plan.settings?.apply).toBe(true);
  });
});

describe("uniqueConnectionId", () => {
  it("appends a numeric suffix on collision", () => {
    const taken = new Set(["local-pg", "local-pg-2"]);
    expect(uniqueConnectionId("Local PG", taken)).toBe("local-pg-3");
    expect(uniqueConnectionId("Brand New", new Set())).toBe("brand-new");
  });
});

describe("applyImportPlan", () => {
  it("adds, replaces, skips, and routes settings/layouts to deps", async () => {
    const plan = computeImportPlan(
      {
        format: "cellar.setup",
        version: 1,
        exportedAt: "",
        app: "",
        sections: {
          connections: [
            conn({ id: "incoming", name: "Dup" }), // duplicate -> we'll replace
            conn({ database: "fresh", id: "fresh" }), // new -> add
            conn({ database: "skipme", id: "skipme" }), // new -> we'll skip
          ],
          tableLayouts: { "x::a.b.c": { order: ["id"], widths: {} } },
          settings: {
            theme: "light",
            density: "comfortable",
            accent: "#fff",
            fontSizePx: 14,
            interfaceFont: "Geist",
            monoFont: "JetBrains Mono",
            editor: { tabSize: 4 as const, softWrap: false, lineNumbers: true, bracketMatching: true },
            grid: { nullDisplay: "NULL" as const, stripeRows: false, rememberTableSort: true },
          },
        },
      },
      { connections: [conn({ id: "existing-1" })], tableLayouts: {} },
    );

    // duplicate -> replace; first new -> add; second new -> skip
    plan.connections[0]!.decision = "replace";
    plan.connections[1]!.decision = "add";
    plan.connections[2]!.decision = "skip";

    const saved: { id: string; password: string | null }[] = [];
    let settingsApplied = false;
    let layoutsApplied: Record<string, unknown> = {};

    const result = await applyImportPlan(plan, {
      existingConnectionIds: ["existing-1"],
      saveConnection: async (config, password) => {
        saved.push({ id: config.id, password });
        return config;
      },
      importSettings: () => {
        settingsApplied = true;
      },
      importTableLayouts: (entries) => {
        layoutsApplied = entries;
      },
    });

    expect(result.connectionsReplaced).toBe(1);
    expect(result.connectionsAdded).toBe(1);
    expect(result.connectionsSkipped).toBe(1);
    expect(result.layoutsAdded).toBe(1);
    expect(result.settingsApplied).toBe(true);
    expect(settingsApplied).toBe(true);
    expect(Object.keys(layoutsApplied)).toContain("x::a.b.c");

    // Replace reuses the existing connection id; add never collides with it.
    expect(saved.find((s) => s.id === "existing-1")).toBeDefined();
    expect(saved.every((s) => s.password === null)).toBe(true);
    expect(saved).toHaveLength(2); // skipped one never saved
  });
});

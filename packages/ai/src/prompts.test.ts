import { describe, expect, it } from "vitest";
import {
  buildUserPrompt,
  formatTodayContext,
  ORDERED_TOPICS,
  SYSTEM_PROMPT,
  TOPICS,
} from "./prompts";
import { buildSchemaContext } from "./context";
import { PROVIDERS, getProvider } from "./providers";

describe("SYSTEM_PROMPT", () => {
  it("forbids inventing columns and DECLARE/@variables for auto-run", () => {
    expect(SYSTEM_PROMPT).toMatch(/Never invent table or column names/i);
    expect(SYSTEM_PROMPT).toMatch(/DECLARE\/@variables/i);
    expect(SYSTEM_PROMPT).toMatch(/Lookup hits/i);
    expect(SYSTEM_PROMPT).toMatch(/Today:/i);
  });
});

describe("formatTodayContext", () => {
  it("includes ISO date and current year", () => {
    const out = formatTodayContext(new Date(2026, 6, 17)); // local 17 Jul 2026
    expect(out).toContain("2026-07-17");
    expect(out).toContain("Current year: 2026");
    expect(out).toMatch(/Use this year for dates that omit one/);
  });
});

describe("topics", () => {
  it("exposes the four presets plus ask in order", () => {
    expect(ORDERED_TOPICS).toEqual([
      "generate",
      "explain",
      "optimize",
      "migrate",
      "ask",
    ]);
  });

  it("every topic has a hint", () => {
    // ponytail: label was dropped (was identical to key); hint remains
    for (const t of ORDERED_TOPICS) {
      expect(TOPICS[t].hint.length).toBeGreaterThan(0);
    }
  });
});

describe("buildUserPrompt", () => {
  const now = new Date(2026, 6, 17);

  it("prepends the preset instruction and includes context + request", () => {
    const out = buildUserPrompt(
      "generate",
      "top 10 customers by spend",
      "ctx here",
      undefined,
      now,
    );
    expect(out).toContain(TOPICS.generate.instruction);
    expect(out).toContain("Today: 2026-07-17");
    expect(out).toContain("Schema context:\nctx here");
    expect(out).toContain("Request:\ntop 10 customers by spend");
  });

  it("ask still includes today even without a topic instruction", () => {
    const out = buildUserPrompt("ask", "what is a CTE?", undefined, undefined, now);
    expect(out).toContain("Today: 2026-07-17");
    expect(out).toContain("Request:\nwhat is a CTE?");
    expect(out).not.toContain(TOPICS.generate.instruction);
  });

  it("omits empty sections", () => {
    const out = buildUserPrompt("explain", "", undefined, undefined, now);
    expect(out).toContain(TOPICS.explain.instruction);
    expect(out).toContain("Today: 2026-07-17");
    expect(out).not.toContain("Request:");
  });

  it("includes lookup hits when provided", () => {
    const out = buildUserPrompt(
      "ask",
      "customers for shotover",
      "schema",
      "Lookup hits:\n- epiczone.Companies: TenantId=abc",
      now,
    );
    expect(out).toContain("Schema context:\nschema");
    expect(out).toContain("Lookup hits:\n- epiczone.Companies: TenantId=abc");
    expect(out).toContain("Request:\ncustomers for shotover");
  });
});

describe("buildSchemaContext", () => {
  it("renders header and table DDL", () => {
    const ctx = buildSchemaContext({
      engine: "postgres",
      database: "shop",
      schema: "public",
      tables: [
        {
          schema: "public",
          name: "orders",
          columns: [
            { name: "id", data_type: "int4", nullable: false, is_primary_key: true },
            { name: "total", data_type: "numeric", nullable: true, is_primary_key: false },
          ],
        },
      ],
    });
    expect(ctx).toContain("Engine: postgres");
    expect(ctx).toContain("public.orders (");
    expect(ctx).toContain("id int4 [pk, not null]");
    expect(ctx).toContain("total numeric");
  });

  it("returns empty string when there is nothing to send", () => {
    expect(buildSchemaContext({})).toBe("");
  });

  it("skips tables with no columns", () => {
    const ctx = buildSchemaContext({
      tables: [{ schema: "public", name: "empty", columns: [] }],
    });
    expect(ctx).toBe("");
  });

  it("renders foreign keys so the model can join lookup tables", () => {
    const ctx = buildSchemaContext({
      tables: [
        {
          schema: "epiczone",
          name: "Payments",
          columns: [
            { name: "TenantId", data_type: "uniqueidentifier", nullable: false, is_primary_key: false },
          ],
          foreign_keys: [
            {
              columns: ["TenantId"],
              referenced_schema: "epiczone",
              referenced_table: "Companies",
              referenced_columns: ["TenantId"],
            },
          ],
        },
      ],
    });
    expect(ctx).toContain(
      "FK epiczone.Payments(TenantId) -> epiczone.Companies(TenantId)",
    );
  });
});

describe("providers", () => {
  it("enables only google", () => {
    const enabled = PROVIDERS.filter((p) => p.enabled).map((p) => p.id);
    expect(enabled).toEqual(["google"]);
  });
  it("getProvider throws on unknown", () => {
    // @ts-expect-error testing the runtime guard
    expect(() => getProvider("nope")).toThrow();
  });
});

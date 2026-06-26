import { describe, expect, it } from "vitest";
import { buildUserPrompt, ORDERED_TOPICS, TOPICS } from "./prompts";
import { buildSchemaContext } from "./context";
import { PROVIDERS, getProvider } from "./providers";

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
  it("prepends the preset instruction and includes context + request", () => {
    const out = buildUserPrompt("generate", "top 10 customers by spend", "ctx here");
    expect(out).toContain(TOPICS.generate.instruction);
    expect(out).toContain("Schema context:\nctx here");
    expect(out).toContain("Request:\ntop 10 customers by spend");
  });

  it("ask has no instruction prefix", () => {
    const out = buildUserPrompt("ask", "what is a CTE?");
    expect(out).toBe("Request:\nwhat is a CTE?");
  });

  it("omits empty sections", () => {
    const out = buildUserPrompt("explain", "");
    expect(out).toBe(TOPICS.explain.instruction);
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

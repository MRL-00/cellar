import { describe, expect, it } from "vitest";
import { qualifiedName, quoteIdent, selectAllStatement } from "./sqlIdent";

describe("quoteIdent", () => {
  it("wraps identifiers in double quotes", () => {
    expect(quoteIdent("orders")).toBe('"orders"');
  });

  it("escapes embedded double quotes", () => {
    expect(quoteIdent('weird"name')).toBe('"weird""name"');
  });
});

describe("qualifiedName", () => {
  it("quotes and dot-joins each part", () => {
    expect(qualifiedName("public", "orders")).toBe('"public"."orders"');
  });
});

describe("selectAllStatement", () => {
  it("builds a quoted starter SELECT with a default limit", () => {
    expect(selectAllStatement("public", "orders")).toBe(
      'SELECT *\nFROM "public"."orders"\nLIMIT 100;',
    );
  });
});

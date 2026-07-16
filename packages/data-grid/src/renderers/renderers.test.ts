/**
 * DOM-free unit tests for the renderer pure helpers and registry resolution.
 * The React rendering itself is exercised by the app; here we lock in the
 * parsing/detection/dispatch logic that the renderers depend on.
 */
import { describe, expect, it } from "vitest";
import type { GridColumn } from "../types";
import { createRendererRegistry, defaultRendererRegistry } from "./registry";
import { jsonRenderer, jsonKind, jsonSummary, safeParseJson } from "./json";
import { arrayRenderer, elementOf, parsePgArray } from "./array";
import {
  byteaInfo,
  byteaRenderer,
  formatByteSize,
  hexDump,
  parseHexBytes,
  sniffImageMime,
} from "./bytes";
import { geometryRenderer, geometryLabel } from "./geometry";
import {
  arrayElementType,
  isArrayType,
  isByteaType,
  isGeometryType,
  isGuidType,
  isJsonType,
} from "./typeMatch";

function col(type: string): GridColumn {
  return { key: "c", name: "c", type, width: 120 };
}

describe("type predicates", () => {
  it("detects json types", () => {
    expect(isJsonType("json")).toBe(true);
    expect(isJsonType("JSONB")).toBe(true);
    expect(isJsonType("object")).toBe(true);
    expect(isJsonType("array")).toBe(true);
    expect(isJsonType("map")).toBe(true);
    expect(isJsonType("text")).toBe(false);
  });

  it("detects guid types", () => {
    expect(isGuidType("uuid")).toBe(true);
    expect(isGuidType("GUID")).toBe(true);
    expect(isGuidType("uniqueidentifier")).toBe(true);
    expect(isGuidType("text")).toBe(false);
  });

  it("detects array types and element types", () => {
    expect(isArrayType("int4[]")).toBe(true);
    expect(isArrayType("_text")).toBe(true);
    expect(isArrayType("integer array")).toBe(true);
    expect(isArrayType("jsonb")).toBe(false);
    // Document-store `array` is JSON, not a Postgres array literal.
    expect(isArrayType("array")).toBe(false);
    expect(isArrayType("text")).toBe(false);
    expect(arrayElementType("int4[]")).toBe("int4");
    expect(arrayElementType("_text")).toBe("text");
    expect(arrayElementType("integer array")).toBe("integer");
  });

  it("detects bytea and geometry types", () => {
    expect(isByteaType("bytea")).toBe(true);
    expect(isByteaType("BLOB")).toBe(true);
    expect(isByteaType("longblob")).toBe(true);
    expect(isByteaType("image")).toBe(true);
    expect(isGeometryType("geometry")).toBe(true);
    expect(isGeometryType("geography(Point,4326)")).toBe(true);
    expect(isGeometryType("text")).toBe(false);
  });
});

describe("registry resolution", () => {
  it("routes each complex type to its renderer", () => {
    expect(defaultRendererRegistry.resolve(col("jsonb"), "{}")?.id).toBe(jsonRenderer.id);
    expect(defaultRendererRegistry.resolve(col("object"), '{"a":1}')?.id).toBe(
      jsonRenderer.id,
    );
    expect(defaultRendererRegistry.resolve(col("int4[]"), "{1,2}")?.id).toBe(arrayRenderer.id);
    expect(defaultRendererRegistry.resolve(col("bytea"), "\\x00")?.id).toBe(byteaRenderer.id);
    expect(defaultRendererRegistry.resolve(col("geometry"), "POINT(0 0)")?.id).toBe(
      geometryRenderer.id,
    );
  });

  it("routes geometry decoded as raw bytes to the bytea renderer", () => {
    // MySQL decodes GEOMETRY as bytes → `\x…`; that should get hex/image/save,
    // not the WKT geometry UI.
    expect(defaultRendererRegistry.resolve(col("geometry"), "\\x0101000000")?.id).toBe(
      byteaRenderer.id,
    );
    // Textual WKT still goes to the geometry renderer.
    expect(geometryRenderer.appliesTo(col("geometry"), "POINT(0 0)")).toBe(true);
    expect(geometryRenderer.appliesTo(col("geometry"), "\\x0101")).toBe(false);
  });

  it("returns null for plain types so the default path renders them", () => {
    expect(defaultRendererRegistry.resolve(col("text"), "hello")).toBeNull();
    expect(defaultRendererRegistry.resolve(col("int4"), 42)).toBeNull();
  });

  it("does not claim malformed array literals (no closing brace)", () => {
    expect(arrayRenderer.appliesTo(col("int4[]"), "{1,2")).toBe(false);
    expect(arrayRenderer.appliesTo(col("int4[]"), "{1,2,3}")).toBe(true);
    expect(arrayRenderer.appliesTo(col("int4[]"), "{}")).toBe(true);
  });

  it("does not claim an array literal that is not an array column", () => {
    expect(arrayRenderer.appliesTo(col("text"), "{not,an,array}")).toBe(false);
  });

  it("honours priority — a custom renderer wins ties", () => {
    const custom = {
      id: "custom",
      priority: 100,
      appliesTo: () => true,
      renderInline: () => null,
    };
    const registry = createRendererRegistry([custom, ...defaultRendererRegistry.renderers]);
    expect(registry.resolve(col("jsonb"), "{}")?.id).toBe("custom");
  });
});

describe("json helpers", () => {
  it("parses safely and summarises", () => {
    expect(safeParseJson('{"a":1}')).toEqual({ ok: true, value: { a: 1 } });
    expect(safeParseJson("{oops")).toEqual({ ok: false });
    expect(jsonKind([1, 2])).toBe("array");
    expect(jsonKind({})).toBe("object");
    expect(jsonKind(null)).toBe("null");
    expect(jsonSummary({ a: 1, b: 2 })).toBe("{ 2 keys }");
    expect(jsonSummary([1, 2, 3])).toBe("[ 3 items ]");
    expect(jsonSummary([])).toBe("[ ]");
    expect(jsonSummary("hi")).toBe('"hi"');
  });
});

describe("array helpers", () => {
  it("splits top-level elements respecting quotes and nesting", () => {
    expect(parsePgArray("{1,2,3}")).toEqual(["1", "2", "3"]);
    expect(parsePgArray("{}")).toEqual([]);
    expect(parsePgArray('{"a","b,c",NULL}')).toEqual(['"a"', '"b,c"', "NULL"]);
    expect(parsePgArray("{{1,2},{3,4}}")).toEqual(["{1,2}", "{3,4}"]);
  });

  it("classifies elements by type and quoting", () => {
    expect(elementOf("42", "int4")).toEqual({ text: "42", kind: "number" });
    expect(elementOf("NULL", "int4")).toEqual({ text: "NULL", kind: "null" });
    expect(elementOf("t", "bool")).toEqual({ text: "true", kind: "boolean" });
    expect(elementOf('"hi, there"', "text")).toEqual({ text: "hi, there", kind: "string" });
    expect(elementOf('"a\\"b"', "text")).toEqual({ text: 'a"b', kind: "string" });
  });
});

describe("bytea helpers", () => {
  it("decodes hex, tolerating truncation markers", () => {
    expect(Array.from(parseHexBytes("\\x00ff10"))).toEqual([0, 255, 16]);
    expect(Array.from(parseHexBytes("\\xdeadbeef… (4 bytes)"))).toEqual([
      0xde, 0xad, 0xbe, 0xef,
    ]);
    expect(parseHexBytes("\\x").length).toBe(0);
  });

  it("sniffs image magic bytes", () => {
    const png = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00]);
    const jpeg = new Uint8Array([0xff, 0xd8, 0xff, 0xe0]);
    const gif = new Uint8Array([0x47, 0x49, 0x46, 0x38, 0x39, 0x61]);
    expect(sniffImageMime(png)).toBe("image/png");
    expect(sniffImageMime(jpeg)).toBe("image/jpeg");
    expect(sniffImageMime(gif)).toBe("image/gif");
    expect(sniffImageMime(new Uint8Array([0x00, 0x01, 0x02]))).toBeNull();
  });

  it("recovers true size and flags truncation via byteaInfo", () => {
    const full = byteaInfo("\\x00ff10");
    expect(full.truncated).toBe(false);
    expect(full.total).toBe(3);
    const partial = byteaInfo("\\xdeadbeef… (1024 bytes)");
    expect(partial.truncated).toBe(true);
    expect(partial.total).toBe(1024);
    expect(partial.bytes.length).toBe(4);
  });

  it("formats byte sizes", () => {
    expect(formatByteSize(900)).toBe("900 B");
    expect(formatByteSize(2048)).toBe("2.0 KB");
    expect(formatByteSize(5 * 1024 * 1024)).toBe("5.0 MB");
  });

  it("builds a capped hex dump", () => {
    const bytes = new Uint8Array([0x48, 0x69, 0x21]); // "Hi!"
    const rows = hexDump(bytes);
    expect(rows).toHaveLength(1);
    expect(rows[0]?.hex).toBe("48 69 21");
    expect(rows[0]?.ascii).toBe("Hi!");
    expect(hexDump(new Uint8Array(40)).length).toBe(3);
  });
});

describe("geometry helpers", () => {
  it("labels by WKT keyword, ignoring SRID prefix", () => {
    expect(geometryLabel("POINT(0 0)")).toBe("POINT");
    expect(geometryLabel("SRID=4326;POLYGON((0 0,1 1,1 0,0 0))")).toBe("POLYGON");
    expect(geometryLabel("")).toBe("GEOMETRY");
  });
});

import { describe, expect, it } from "vitest";

import { commands, IpcError, isTauri, unwrap } from "./index";
import type {
  CellValue,
  Database,
  QueryResult,
  Result,
  CellarError,
} from "./generated";

describe("@cellar/ipc", () => {
  it("exposes the full command surface", () => {
    // If a command name changes, this test should fail so the UI gets a heads-up.
    expect(Object.keys(commands).sort()).toEqual(
      [
        "commitTableChanges",
        "connect",
        "deleteConnection",
        "disconnect",
        "introspect",
        "listQueryHistory",
        "listConnections",
        "previewTableChanges",
        "runQuery",
        "saveConnection",
        "testConnection",
      ].sort(),
    );
  });

  it("runs the web-mode mock when not in Tauri", () => {
    expect(isTauri).toBe(false);
  });

  it("returns empty connection list in mock mode", async () => {
    const result = await commands.listConnections();
    expect(result.status).toBe("ok");
    if (result.status === "ok") {
      expect(result.data).toEqual([]);
    }
  });

  it("introspect returns an empty schema tree in mock mode", async () => {
    const result: Result<Database[], CellarError> = await commands.introspect(
      "any",
      false,
    );
    expect(result.status).toBe("ok");
    if (result.status === "ok") {
      expect(Array.isArray(result.data)).toBe(true);
    }
  });

  it("preserves the QueryResult shape end-to-end", async () => {
    const result: Result<QueryResult, CellarError> = await commands.runQuery(
      "any",
      "SELECT 1",
      10,
      null,
      null,
    );
    expect(result.status).toBe("ok");
    if (result.status === "ok") {
      expect(result.data).toMatchObject({
        columns: expect.any(Array),
        rows: expect.any(Array),
        notices: expect.any(Array),
        notice_capture: {
          supported: expect.any(Boolean),
        },
        duration_ms: expect.any(Number),
        truncated: false,
      });
    }
  });

  it("returns empty query history in mock mode", async () => {
    const result = await commands.listQueryHistory("any", null, null, "select", 20);
    expect(result.status).toBe("ok");
    if (result.status === "ok") {
      expect(result.data).toEqual([]);
    }
  });

  it("unwrap throws an IpcError on the error branch", async () => {
    const err: Promise<Result<number, CellarError>> = Promise.resolve({
      status: "error",
      error: { kind: "Connection", detail: "boom" },
    });
    await expect(unwrap(err)).rejects.toBeInstanceOf(IpcError);
  });

  it("Database tree round-trips structurally", () => {
    const tree: Database = {
      name: "shop_eu",
      is_default: true,
      schemas: [
        {
          name: "public",
          tables: [
            {
              name: "orders",
              schema: "public",
              row_count: 100,
              columns: [
                {
                  name: "id",
                  data_type: "int8",
                  nullable: false,
                  default: null,
                  is_primary_key: true,
                  ordinal: 1,
                  comment: null,
                },
              ],
              primary_key: ["id"],
              foreign_keys: [],
              indexes: [],
            },
          ],
          views: [],
        },
      ],
    };
    const json = JSON.stringify(tree);
    expect(JSON.parse(json)).toEqual(tree);
  });

  it("CellValue null/text variants are exported correctly", () => {
    const nullValue: CellValue = { type: "Null" };
    const textValue: CellValue = { type: "Text", value: "hello" };
    expect(nullValue.type).toBe("Null");
    expect(textValue.type).toBe("Text");
    if (textValue.type === "Text") {
      expect(textValue.value).toBe("hello");
    }
  });
});

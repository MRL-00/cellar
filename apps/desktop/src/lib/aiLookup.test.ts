import { describe, expect, it } from "vitest";
import {
  buildLookupSql,
  extractEntityPhrases,
  formatLookupHits,
  pickLookupTargets,
} from "./aiLookup";
import type { ContextTable } from "@cellar/ai";
import type { QueryResult } from "@cellar/ipc";

describe("extractEntityPhrases", () => {
  it("pulls the company name after 'for'", () => {
    expect(
      extractEntityPhrases(
        "Give me all customers for shotover jet, based on the company tenant id for 16th june to 16th july NZ time",
      ),
    ).toContain("shotover jet");
  });

  it("keeps quoted names", () => {
    expect(extractEntityPhrases('payments for "Shotover Jet Ltd"')).toContain(
      "Shotover Jet Ltd",
    );
  });
});

describe("pickLookupTargets", () => {
  it("prefers Companies with TenantId + name columns", () => {
    const tables: ContextTable[] = [
      {
        schema: "epiczone",
        name: "Payments",
        columns: [
          { name: "Id", data_type: "uniqueidentifier", nullable: false, is_primary_key: true },
          { name: "TenantId", data_type: "uniqueidentifier", nullable: false, is_primary_key: false },
        ],
      },
      {
        schema: "epiczone",
        name: "Companies",
        columns: [
          { name: "TenantId", data_type: "uniqueidentifier", nullable: false, is_primary_key: true },
          { name: "CompanyName", data_type: "nvarchar", nullable: false, is_primary_key: false },
        ],
      },
      {
        schema: "epiczone",
        name: "Customers",
        columns: [
          { name: "Id", data_type: "uniqueidentifier", nullable: false, is_primary_key: true },
          { name: "Name", data_type: "nvarchar", nullable: false, is_primary_key: false },
          { name: "TenantId", data_type: "uniqueidentifier", nullable: false, is_primary_key: false },
        ],
      },
    ];
    const targets = pickLookupTargets(tables);
    expect(targets[0]?.table).toBe("Companies");
    expect(targets[0]?.keyColumns[0]).toBe("TenantId");
    expect(targets[0]?.nameColumns).toContain("CompanyName");
  });
});

describe("buildLookupSql", () => {
  it("builds a SQL Server TOP/LIKE lookup", () => {
    const sql = buildLookupSql(
      {
        schema: "epiczone",
        table: "Companies",
        keyColumns: ["TenantId"],
        nameColumns: ["CompanyName"],
      },
      "Shotover Jet",
      "mssql",
    );
    expect(sql).toContain("SELECT TOP 5");
    expect(sql).toContain("[epiczone].[Companies]");
    expect(sql).toContain("[CompanyName] LIKE N'%Shotover Jet%'");
  });
});

describe("formatLookupHits", () => {
  it("formats rows as equality-ready key hints", () => {
    const result = {
      columns: [
        { name: "TenantId", data_type: "uniqueidentifier", nullable: false },
        { name: "CompanyName", data_type: "nvarchar", nullable: false },
      ],
      rows: [
        [
          { type: "Text", value: "aaa-bbb" },
          { type: "Text", value: "Shotover Jet" },
        ],
      ],
      notices: [],
      notice_capture: { supported: false, reason: "" },
      rows_affected: null,
      duration_ms: 1,
      truncated: false,
      total_rows: null,
    } as unknown as QueryResult;

    const text = formatLookupHits(
      {
        schema: "epiczone",
        table: "Companies",
        keyColumns: ["TenantId"],
        nameColumns: ["CompanyName"],
      },
      "shotover jet",
      result,
    );
    expect(text).toContain("TenantId=aaa-bbb");
    expect(text).toContain("use these keys with = / IN, not LIKE");
  });
});

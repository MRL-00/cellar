import { describe, expect, it } from "vitest";
import type { Schema } from "@cellar/ipc";
import {
  expandWithRelatedTables,
  extractSqlTableRefs,
  isNoiseSchema,
  pickDefaultSchema,
  resolveQueryContextTables,
  resolveTableContextTables,
  stripSqlNoise,
} from "./aiContext";
import type { ForeignKey } from "@cellar/ipc";

function col(name: string, isPk = false) {
  return {
    name,
    data_type: "nvarchar",
    nullable: !isPk,
    default: null,
    is_primary_key: isPk,
    ordinal: 0,
    comment: null,
  };
}

function table(
  schema: string,
  name: string,
  columns: string[],
  foreign_keys: ForeignKey[] = [],
): Schema["tables"][number] {
  return {
    name,
    schema,
    row_count: null,
    columns: columns.map((c) => col(c, c === "Id" || c === "TenantId" && name === "Companies")),
    primary_key: columns.includes("Id") ? ["Id"] : [],
    foreign_keys,
    indexes: [],
  };
}

function schema(name: string, tables: Schema["tables"]): Schema {
  return { name, tables, views: [] };
}

describe("isNoiseSchema", () => {
  it("flags SQL Server fixed-role schemas", () => {
    expect(isNoiseSchema("db_accessadmin", "mssql")).toBe(true);
    expect(isNoiseSchema("guest", "azure")).toBe(true);
    expect(isNoiseSchema("epiczone", "mssql")).toBe(false);
  });

  it("flags Postgres catalog schemas", () => {
    expect(isNoiseSchema("pg_catalog", "postgres")).toBe(true);
    expect(isNoiseSchema("public", "postgres")).toBe(false);
  });
});

describe("stripSqlNoise", () => {
  it("removes comments and string literals but keeps identifiers", () => {
    const cleaned = stripSqlNoise(
      "SELECT * FROM epiczone.Payments -- note\nWHERE Name = N'Shotover'",
    );
    expect(cleaned).toContain("epiczone.Payments");
    expect(cleaned).not.toContain("note");
    expect(cleaned).not.toContain("Shotover");
  });
});

describe("extractSqlTableRefs", () => {
  it("finds schema.table refs including bracketed T-SQL forms", () => {
    const refs = extractSqlTableRefs(
      "SELECT * FROM epiczone.Payments p JOIN [epiczone].[Companies] c ON 1=1",
    );
    expect(refs).toEqual(
      expect.arrayContaining([
        { schema: "epiczone", table: "Payments" },
        { schema: "epiczone", table: "Companies" },
      ]),
    );
  });

  it("ignores aliases and string contents", () => {
    const refs = extractSqlTableRefs(
      "SELECT * FROM epiczone.Payments p WHERE p.CustomerName = 'foo.bar'",
    );
    expect(refs).toEqual([{ schema: "epiczone", table: "Payments" }]);
  });
});

describe("pickDefaultSchema", () => {
  it("skips empty SQL Server role schemas and picks the busiest app schema", () => {
    const schemas = [
      schema("db_accessadmin", []),
      schema("dbo", [table("dbo", "sysdiagrams", ["id"])]),
      schema("epiczone", [
        table("epiczone", "Payments", ["Id"]),
        table("epiczone", "Companies", ["Id"]),
        table("epiczone", "Tenants", ["Id"]),
      ]),
    ];
    expect(pickDefaultSchema(schemas, "mssql")?.name).toBe("epiczone");
  });

  it("prefers public for Postgres when populated", () => {
    const schemas = [
      schema("public", [table("public", "orders", ["id"])]),
      schema("analytics", [
        table("analytics", "a", ["id"]),
        table("analytics", "b", ["id"]),
      ]),
    ];
    // analytics has more tables, so it wins over public.
    expect(pickDefaultSchema(schemas, "postgres")?.name).toBe("analytics");
  });
});

describe("resolveQueryContextTables", () => {
  const schemas = [
    schema("db_accessadmin", []),
    schema("epiczone", [
      table("epiczone", "Payments", ["Id", "CustomerName", "TenantId"]),
      table("epiczone", "Companies", ["Id", "CompanyName", "TenantId"]),
    ]),
  ];

  it("pins context to tables referenced in the open SQL", () => {
    const resolved = resolveQueryContextTables(
      schemas,
      "SELECT * FROM epiczone.Payments p\nJOIN epiczone.Companies c ON p.TenantId = c.TenantId",
      "mssql",
    );
    expect(resolved.schemaName).toBe("epiczone");
    expect(resolved.tables.map((t) => t.name).sort()).toEqual([
      "Companies",
      "Payments",
    ]);
    const companies = resolved.tables.find((t) => t.name === "Companies");
    expect(companies?.columns.map((c) => c.name)).toContain("CompanyName");
    expect(companies?.columns.map((c) => c.name)).not.toContain("Name");
  });

  it("falls back to the busiest non-noise schema when SQL is empty", () => {
    const resolved = resolveQueryContextTables(schemas, "", "mssql");
    expect(resolved.schemaName).toBe("epiczone");
    expect(resolved.tables.length).toBeGreaterThan(0);
  });
});

describe("expandWithRelatedTables", () => {
  it("pulls Companies in via a formal FK from Payments", () => {
    const schemas = [
      schema("epiczone", [
        table(
          "epiczone",
          "Payments",
          ["Id", "CustomerName", "TenantId"],
          [
            {
              name: "FK_Payments_Companies",
              columns: ["TenantId"],
              referenced_schema: "epiczone",
              referenced_table: "Companies",
              referenced_columns: ["TenantId"],
            },
          ],
        ),
        table("epiczone", "Companies", ["TenantId", "CompanyName"]),
        table("epiczone", "Unrelated", ["Id", "Note"]),
      ]),
    ];
    const payments = schemas[0]!.tables[0]!;
    const expanded = expandWithRelatedTables(schemas, [
      {
        schema: payments.schema,
        name: payments.name,
        columns: payments.columns,
        foreign_keys: payments.foreign_keys,
      },
    ]);
    expect(expanded.map((t) => t.name).sort()).toEqual(["Companies", "Payments"]);
  });

  it("pulls Companies in via shared TenantId when no FK exists", () => {
    const schemas = [
      schema("epiczone", [
        table("epiczone", "Payments", ["Id", "CustomerName", "TenantId"]),
        table("epiczone", "Companies", ["TenantId", "CompanyName"]),
      ]),
    ];
    const payments = schemas[0]!.tables[0]!;
    const expanded = expandWithRelatedTables(schemas, [
      {
        schema: payments.schema,
        name: payments.name,
        columns: payments.columns,
        foreign_keys: [],
      },
    ]);
    expect(expanded.map((t) => t.name)).toContain("Companies");
    expect(expanded.find((t) => t.name === "Companies")?.columns.map((c) => c.name)).toContain(
      "CompanyName",
    );
  });
});

describe("resolveTableContextTables", () => {
  it("includes FK neighbors for a focused table tab", () => {
    const schemas = [
      schema("epiczone", [
        table(
          "epiczone",
          "Payments",
          ["Id", "TenantId"],
          [
            {
              name: "fk",
              columns: ["TenantId"],
              referenced_schema: "epiczone",
              referenced_table: "Companies",
              referenced_columns: ["TenantId"],
            },
          ],
        ),
        table("epiczone", "Companies", ["TenantId", "CompanyName"]),
      ]),
    ];
    const resolved = resolveTableContextTables(schemas, "epiczone", "Payments");
    expect(resolved.tables.map((t) => t.name).sort()).toEqual(["Companies", "Payments"]);
    const paymentsCtx = resolved.tables.find((t) => t.name === "Payments");
    expect(paymentsCtx?.foreign_keys?.[0]?.referenced_table).toBe("Companies");
  });
});

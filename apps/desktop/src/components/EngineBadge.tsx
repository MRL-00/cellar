export type Engine = "postgres" | "mysql" | "mssql" | "azure" | "sqlite";

export const ENGINE_META: Record<
  Engine,
  { label: string; color: string; letter: string }
> = {
  postgres: { label: "PostgreSQL", color: "var(--eng-postgres)", letter: "P" },
  mysql: { label: "MySQL", color: "var(--eng-mysql)", letter: "M" },
  mssql: { label: "SQL Server", color: "var(--eng-mssql)", letter: "S" },
  azure: { label: "Azure SQL", color: "var(--eng-azure)", letter: "A" },
  sqlite: { label: "SQLite", color: "var(--eng-sqlite)", letter: "L" },
};

export function EngineBadge({
  engine,
  size = 12,
}: {
  engine: Engine;
  size?: number;
}) {
  const m = ENGINE_META[engine];
  return (
    <span
      title={m.label}
      style={{
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        width: size + 4,
        height: size + 4,
        fontFamily: "var(--font-mono)",
        fontSize: size - 3,
        fontWeight: 600,
        color: m.color,
        background: `color-mix(in oklab, ${m.color} 14%, transparent)`,
        border: `1px solid color-mix(in oklab, ${m.color} 36%, transparent)`,
        borderRadius: 3,
        lineHeight: 1,
        flexShrink: 0,
      }}
    >
      {m.letter}
    </span>
  );
}

import {
  useCallback,
  useEffect,
  useMemo,
  useState,
} from "react";
import type { Database, Engine } from "@cellar/ipc";
import { SqlCodeEditor } from "@cellar/sql-editor";

import {
  splitStatements,
  statementAtOffset,
  type SqlStatement,
} from "../lib/sqlStatements";
import { useQueryRunner } from "../hooks/useQueryRunner";
import { useBottomPanel } from "../state/bottomPanel";
import { useConnections } from "../state/connections";
import type { QueryTab } from "../state/tabs";
import { useTabs } from "../state/tabs";
import { Icon } from "./icons";

const DIALECTS: Record<Engine, string> = {
  postgres: "PostgreSQL",
  mysql: "MySQL",
  sqlite: "SQLite",
  mssql: "SQL Server",
  azure: "Azure SQL",
  firestore: "Firestore",
};

const PLACEHOLDER =
  "Write SQL here…  ⌘⏎ runs the statement under the cursor, ⌘⇧⏎ runs all.";

const EMPTY_DATABASES: Database[] = [];

export function SqlEditor({ tab }: { tab: QueryTab }) {
  const setQuerySql = useTabs((s) => s.setQuerySql);
  const connectionState = useConnections((s) => s.byId[tab.connectionId]);
  const status = connectionState?.status;
  const databases = connectionState?.databases ?? EMPTY_DATABASES;
  const loadingSchema = connectionState?.loadingSchema ?? false;
  const engine = useConnections(
    (s) => s.connections.find((c) => c.id === tab.connectionId)?.engine,
  );
  const refreshSchema = useConnections((s) => s.refreshSchema);
  const setBottomTab = useBottomPanel((s) => s.setActive);
  const requestExplain = useBottomPanel((s) => s.requestExplain);

  const { running, errorLine, run, clearError } = useQueryRunner(tab);

  const [caret, setCaret] = useState(0);
  const [wrap, setWrap] = useState(false);

  const sql = tab.sql;
  const connected = status === "connected";
  const isPostgres = engine === "postgres" || engine === undefined;
  const supportsSql = engine !== "firestore";

  const statements = useMemo(() => splitStatements(sql), [sql]);
  const current = useMemo(
    () => statementAtOffset(sql, caret),
    [sql, caret],
  );
  const range = current ? ([current.startLine, current.endLine] as const) : null;

  const runStatementAt = useCallback((offset: number) => {
    const stmt = statementAtOffset(sql, offset);
    if (!stmt) return;
    setBottomTab("results");
    run(stmt.text, { label: statementLabel(stmt, statements), errorLine: stmt.startLine });
  }, [sql, run, setBottomTab, statements]);

  const runStatement = useCallback(() => {
    runStatementAt(caret);
  }, [caret, runStatementAt]);

  const runAll = useCallback(() => {
    if (statements.length === 0) return;
    setBottomTab("results");
    run(sql, {
      label: statements.length > 1 ? "all statements" : "statement",
      errorLine: statements[0]?.startLine ?? 1,
    });
  }, [sql, statements, run, setBottomTab]);

  const handleEditorChange = useCallback((next: string) => {
    setQuerySql(tab.id, next);
    if (errorLine != null) clearError();
  }, [clearError, errorLine, setQuerySql, tab.id]);

  useEffect(() => {
    if (!connected || loadingSchema || databases.length > 0) return;
    void refreshSchema(tab.connectionId);
  }, [
    connected,
    databases.length,
    loadingSchema,
    refreshSchema,
    tab.connectionId,
  ]);

  const canRunStatement = connected && supportsSql && !running && current != null;
  const canRunAll = connected && supportsSql && !running && statements.length > 0;
  const canExplain = connected && !running && current != null && isPostgres;
  const runTitle = !connected
    ? "Connect this tab's database to run SQL"
    : !supportsSql
      ? "Firestore query execution is not supported yet"
    : current
      ? "Run the statement under the cursor"
      : "Place the cursor in a statement to run it";

  const dialect = engine ? DIALECTS[engine] : "PostgreSQL";
  const dotEngine: Engine = engine ?? "postgres";
  const sqlEditorEngine = supportsSql ? engine : undefined;
  const preview = current ? firstLine(current.text) : "—";

  return (
    <div className={"ed-root mono" + (wrap ? " wrap" : "")}>
      <div className="ed-toolbar">
        <div className="ed-toolbar-left">
          <button
            className="ed-run primary"
            onClick={runStatement}
            disabled={!canRunStatement}
            title={runTitle}
          >
            <Icon.playSm size={11} />
            <span>{running ? "Running…" : "Run"}</span>
            <span className="kbd">⌘⏎</span>
          </button>
          <button
            className="ed-run subtle"
            onClick={runAll}
            disabled={!canRunAll}
            title={
              !connected
                ? "Connect to run SQL"
                : supportsSql
                  ? "Run the entire editor buffer"
                  : "Firestore query execution is not supported yet"
            }
          >
            <Icon.play size={11} />
            <span>Run all</span>
            <span className="kbd">⌘⇧⏎</span>
          </button>
          <div className="cellar-titlebar-divider" />
          <button
            className="icon-btn"
            disabled
            title="SQL formatting lands with the cellar-sql formatter"
          >
            <Icon.format size={12} />
          </button>
          <button
            className={"icon-btn" + (wrap ? " active" : "")}
            onClick={() => setWrap((w) => !w)}
            title={wrap ? "Disable line wrapping" : "Wrap long lines"}
          >
            <Icon.wrap size={12} />
          </button>
          <button
            className="icon-btn"
            onClick={requestExplain}
            disabled={!canExplain}
            title={
              !connected
                ? "Connect to inspect the plan"
                : !isPostgres
                  ? "Execution plans are Postgres-only for now"
                  : !current
                    ? "Place the cursor in a statement to explain it"
                    : "Explain the statement under the cursor"
            }
          >
            <Icon.tree size={12} />
          </button>
          <button
            className="icon-btn"
            disabled
            title="Bookmarks arrive with query history work"
          >
            <Icon.star size={12} />
          </button>
        </div>
        <div className="ed-toolbar-right">
          <span className="ed-stmt-note">
            <span style={{ color: "var(--fg-3)" }}>statement under cursor:</span>
            <span className="ed-stmt-sql" style={{ color: "var(--fg-1)" }}>
              {preview}
            </span>
          </span>
          <div className="cellar-titlebar-divider" />
          <span className="ed-dialect">
            <span className="dot" style={{ background: `var(--eng-${dotEngine})` }} />
            <span>{dialect}</span>
          </span>
        </div>
      </div>

      <div className="ed-scroll">
        <SqlCodeEditor
          value={sql}
          engine={sqlEditorEngine}
          databases={databases}
          database={tab.database}
          placeholder={PLACEHOLDER}
          wrap={wrap}
          currentStatementRange={range}
          errorLine={errorLine}
          onChange={handleEditorChange}
          onCursorChange={setCaret}
          onRunStatement={runStatementAt}
          onRunAll={runAll}
        />

        <div className="ed-ai-strip" aria-hidden="true">
          <span className="ed-ai-strip-prompt">
            <Icon.sparkles size={11} style={{ color: "var(--accent)" }} />
            <span>Ask AI to edit or extend this query…</span>
          </span>
          <span className="ed-ai-strip-kbd">
            <kbd className="kbd">⌘</kbd>
            <kbd className="kbd">I</kbd>
          </span>
        </div>
      </div>
    </div>
  );
}

function statementLabel(stmt: SqlStatement, all: SqlStatement[]): string {
  if (all.length <= 1) return "statement";
  const idx = all.findIndex((s) => s.start === stmt.start);
  return idx >= 0 ? `statement ${idx + 1} of ${all.length}` : "statement";
}

function firstLine(text: string): string {
  const line = text.split("\n").find((l) => l.trim().length > 0) ?? text;
  const trimmed = line.trim();
  return trimmed.length > 80 ? trimmed.slice(0, 79) + "…" : trimmed;
}

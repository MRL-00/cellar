import {
  useCallback,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { Engine } from "@cellar/ipc";

import { renderTokens, tokenizeSql, tokensToLines } from "../lib/sqlTokens";
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
};

const PLACEHOLDER =
  "Write SQL here…  ⌘⏎ runs the statement under the cursor, ⌘⇧⏎ runs all.";

export function SqlEditor({ tab }: { tab: QueryTab }) {
  const setQuerySql = useTabs((s) => s.setQuerySql);
  const status = useConnections((s) => s.byId[tab.connectionId]?.status);
  const engine = useConnections(
    (s) => s.connections.find((c) => c.id === tab.connectionId)?.engine,
  );
  const setBottomTab = useBottomPanel((s) => s.setActive);
  const requestExplain = useBottomPanel((s) => s.requestExplain);

  const { running, errorLine, run, clearError } = useQueryRunner(tab);

  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const pendingCaret = useRef<number | null>(null);
  const [caret, setCaret] = useState(0);
  const [wrap, setWrap] = useState(false);

  const sql = tab.sql;
  const connected = status === "connected";
  const isPostgres = engine === "postgres" || engine === undefined;

  const lines = useMemo(() => tokensToLines(tokenizeSql(sql)), [sql]);
  const statements = useMemo(() => splitStatements(sql), [sql]);
  const current = useMemo(
    () => statementAtOffset(sql, caret),
    [sql, caret],
  );
  const range = current ? ([current.startLine, current.endLine] as const) : null;

  // Restore the caret after a programmatic edit (Tab insertion) lands.
  useLayoutEffect(() => {
    if (pendingCaret.current != null && textareaRef.current) {
      const pos = pendingCaret.current;
      textareaRef.current.selectionStart = pos;
      textareaRef.current.selectionEnd = pos;
      pendingCaret.current = null;
      setCaret(pos);
    }
  }, [sql]);

  const syncCaret = useCallback((el: HTMLTextAreaElement) => {
    setCaret(el.selectionStart);
  }, []);

  const runStatement = useCallback(() => {
    const stmt = statementAtOffset(sql, caret);
    if (!stmt) return;
    setBottomTab("results");
    run(stmt.text, { label: statementLabel(stmt, statements), errorLine: stmt.startLine });
  }, [sql, caret, run, setBottomTab, statements]);

  const runAll = useCallback(() => {
    if (statements.length === 0) return;
    setBottomTab("results");
    run(sql, {
      label: statements.length > 1 ? "all statements" : "statement",
      errorLine: statements[0]?.startLine ?? 1,
    });
  }, [sql, statements, run, setBottomTab]);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    const mod = e.metaKey || e.ctrlKey;
    if (mod && e.key === "Enter") {
      e.preventDefault();
      if (e.shiftKey) runAll();
      else runStatement();
      return;
    }
    if (e.key === "Tab") {
      e.preventDefault();
      const el = e.currentTarget;
      const start = el.selectionStart;
      const end = el.selectionEnd;
      const next = sql.slice(0, start) + "  " + sql.slice(end);
      pendingCaret.current = start + 2;
      setQuerySql(tab.id, next);
    }
  };

  const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setQuerySql(tab.id, e.target.value);
    syncCaret(e.target);
    if (errorLine != null) clearError();
  };

  const canRunStatement = connected && !running && current != null;
  const canRunAll = connected && !running && statements.length > 0;
  const canExplain = connected && !running && current != null && isPostgres;
  const runTitle = !connected
    ? "Connect this tab's database to run SQL"
    : current
      ? "Run the statement under the cursor"
      : "Place the cursor in a statement to run it";

  const dialect = engine ? DIALECTS[engine] : "PostgreSQL";
  const dotEngine: Engine = engine ?? "postgres";
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
            title={connected ? "Run the entire editor buffer" : "Connect to run SQL"}
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
        <div className="ed-doc">
          <div className="ed-grid" aria-hidden="true">
            {lines.map((lineTokens, idx) => {
              const lineNo = idx + 1;
              const inStmt = range != null && lineNo >= range[0] && lineNo <= range[1];
              const isError = errorLine === lineNo;
              return (
                <div className="ed-row" key={idx}>
                  <div className={"ed-gutter" + (inStmt ? " in-stmt" : "")}>
                    <span className="ed-lineno">{lineNo}</span>
                  </div>
                  <div
                    className={
                      "ed-line" +
                      (inStmt ? " in-stmt" : "") +
                      (isError ? " has-error" : "")
                    }
                  >
                    {renderTokens(lineTokens)}
                  </div>
                </div>
              );
            })}
          </div>
          <textarea
            ref={textareaRef}
            className="ed-input"
            value={sql}
            spellCheck={false}
            autoCapitalize="off"
            autoCorrect="off"
            wrap={wrap ? "soft" : "off"}
            placeholder={PLACEHOLDER}
            onChange={handleChange}
            onKeyDown={handleKeyDown}
            onSelect={(e) => syncCaret(e.currentTarget)}
            onClick={(e) => syncCaret(e.currentTarget)}
          />
        </div>

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

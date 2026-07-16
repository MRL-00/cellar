import {
  useCallback,
  useEffect,
  useMemo,
  useState,
} from "react";
import { commands, unwrap } from "@cellar/ipc";
import type { Database, DetectedParameter, Engine, QueryParam } from "@cellar/ipc";
import { SqlCodeEditor } from "@cellar/sql-editor";

import {
  splitStatements,
  statementAtOffset,
  type SqlStatement,
} from "../lib/sqlStatements";
import { useQueryRunner } from "../hooks/useQueryRunner";
import {
  defaultParamValue,
  isFilled,
  toCellValue,
  type ParamValue,
} from "../lib/queryParamValues";
import { useBottomPanel } from "../state/bottomPanel";
import { useConnections } from "../state/connections";
import { useQueryParams } from "../state/queryParams";
import type { QueryTab } from "../state/tabs";
import { useTabs } from "../state/tabs";
import { useSettings } from "../lib/settings";
import { Icon } from "./icons";
import { ParameterPanel } from "./ParameterPanel";
import { SaveTemplateModal } from "./modals/SaveTemplateModal";

const DIALECTS: Record<Engine, string> = {
  postgres: "PostgreSQL",
  mysql: "MySQL",
  sqlite: "SQLite",
  mssql: "SQL Server",
  azure: "Azure SQL",
  firestore: "Firestore",
  convex: "Convex",
  cosmos: "Cosmos DB",
  supabase: "Supabase",
  neon: "Neon",
  planetscale: "PlanetScale",
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
  const { settings } = useSettings();

  const {
    running,
    cancelRequested,
    errorLine,
    run,
    cancel,
    clearError,
  } = useQueryRunner(tab);

  const openPanel = useQueryParams((s) => s.openPanel);
  const requestParamFocus = useQueryParams((s) => s.requestFocus);
  const rememberParams = useQueryParams((s) => s.remember);
  const closePanel = useQueryParams((s) => s.closePanel);
  const hasParamPanel = useQueryParams((s) => Boolean(s.panels[tab.id]));

  const [caret, setCaret] = useState(0);
  const [saveTemplateOpen, setSaveTemplateOpen] = useState(false);
  // Per-editor wrap toggle overrides the global default.
  const [wrapOverride, setWrapOverride] = useState<boolean | null>(null);
  const wrap = wrapOverride !== null ? wrapOverride : settings.editor.softWrap;

  const sql = tab.sql;
  const connected = status === "connected";
  const isPostgres =
    engine === "postgres" ||
    engine === "supabase" ||
    engine === "neon" ||
    engine === undefined;
  const supportsSql =
    engine !== "firestore" && engine !== "convex" && engine !== "cosmos";

  const statements = useMemo(() => splitStatements(sql), [sql]);
  const current = useMemo(
    () => statementAtOffset(sql, caret),
    [sql, caret],
  );
  const range = current ? ([current.startLine, current.endLine] as const) : null;

  // Collect the panel's current values, bind-validate them, and run — or, if
  // any are empty/invalid, focus the first one instead of running.
  const runPanel = useCallback(() => {
    const panel = useQueryParams.getState().panels[tab.id];
    if (!panel) return;
    const params: QueryParam[] = [];
    let complete = true;
    for (const param of panel.params) {
      const pv = panel.values[param.name] ?? { type: "text", value: "" };
      if (!isFilled(pv)) {
        complete = false;
        break;
      }
      const conversion = toCellValue(pv);
      if (!conversion.ok) {
        complete = false;
        break;
      }
      params.push({ name: param.name, value: conversion.value });
    }
    if (!complete) {
      requestParamFocus(tab.id);
      return;
    }
    rememberParams(panel.values);
    setBottomTab("results");
    run(panel.sql, { label: panel.label, errorLine: panel.errorLine }, params);
  }, [tab.id, requestParamFocus, rememberParams, run, setBottomTab]);

  // Detect placeholders in `sql`; run directly when there are none, otherwise
  // open the parameter panel pre-filled from session memory / column hints.
  const prepareAndRun = useCallback(
    async (statementSql: string, label: string, errorLine: number | null) => {
      const trimmed = statementSql.trim();
      if (!trimmed) return;

      const runRaw = () => {
        closePanel(tab.id);
        setBottomTab("results");
        run(trimmed, { label, errorLine });
      };

      // Parameter binding is only implemented for the Postgres driver today.
      // For other engines, run the statement as-is rather than opening a panel
      // that implies binding we cannot perform (the engine reports any error).
      const bindsParams =
        engine === undefined ||
        engine === "postgres" ||
        engine === "supabase" ||
        engine === "neon";
      if (!bindsParams) {
        runRaw();
        return;
      }

      let detected: DetectedParameter[];
      try {
        detected = await unwrap(
          commands.detectQueryParameters(trimmed, engine ?? "postgres"),
        );
      } catch (err) {
        // Detection only fails on un-tokenizable SQL. Run it raw so the engine
        // surfaces the real syntax error instead of us swallowing it.
        console.warn("[SqlEditor] parameter detection failed", err);
        runRaw();
        return;
      }

      if (detected.length === 0) {
        runRaw();
        return;
      }

      const state = useQueryParams.getState();
      const values: Record<string, ParamValue> = {};
      for (const param of detected) {
        values[param.name] =
          state.panels[tab.id]?.values[param.name] ??
          state.remembered[param.name] ??
          defaultParamValue(param, databases, tab.database || null);
      }
      openPanel(tab.id, { sql: trimmed, label, errorLine, params: detected, values });

      if (detected.every((p) => isFilled(values[p.name]!))) {
        runPanel();
      } else {
        requestParamFocus(tab.id);
      }
    },
    [
      engine,
      databases,
      tab.id,
      tab.database,
      closePanel,
      openPanel,
      requestParamFocus,
      run,
      runPanel,
      setBottomTab,
    ],
  );

  const runStatementAt = useCallback(
    (offset: number) => {
      const stmt = statementAtOffset(sql, offset);
      if (!stmt) return;
      void prepareAndRun(
        stmt.text,
        statementLabel(stmt, statements),
        stmt.startLine,
      );
    },
    [sql, prepareAndRun, statements],
  );

  const runStatement = useCallback(() => {
    runStatementAt(caret);
  }, [caret, runStatementAt]);

  const runAll = useCallback(() => {
    if (statements.length === 0) return;
    void prepareAndRun(
      sql,
      statements.length > 1 ? "all statements" : "statement",
      statements[0]?.startLine ?? 1,
    );
  }, [sql, statements, prepareAndRun]);

  const handleEditorChange = useCallback((next: string) => {
    setQuerySql(tab.id, next);
    if (errorLine != null) clearError();
    // The open panel captured a snapshot of the SQL and its parameters; once
    // the buffer changes that snapshot is stale, so close it. Re-running
    // re-detects against the current statement and restores remembered values.
    closePanel(tab.id);
  }, [clearError, closePanel, errorLine, setQuerySql, tab.id]);

  // ⌘. / Ctrl+. cancels the in-flight statement, matching the toolbar button.
  // Window-level so it works while focus sits inside the code editor.
  useEffect(() => {
    if (!running) return;
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === ".") {
        e.preventDefault();
        cancel();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [running, cancel]);

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
      ? "Query execution is not supported for this engine yet"
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
          {running && (
            <button
              className="ed-run subtle"
              onClick={cancel}
              disabled={cancelRequested}
              title={
                cancelRequested
                  ? "Waiting for the running statement to settle"
                  : "Ask the server to stop the running statement"
              }
            >
              <Icon.stop size={11} />
              <span>{cancelRequested ? "Cancelling…" : "Cancel"}</span>
              <span className="kbd">⌘.</span>
            </button>
          )}
          <button
            className="ed-run subtle"
            onClick={runAll}
            disabled={!canRunAll}
            title={
              !connected
                ? "Connect to run SQL"
                : supportsSql
                  ? "Run the entire editor buffer"
                  : "Query execution is not supported for this engine yet"
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
            onClick={() => setWrapOverride((prev) => {
              const globalDefault = settings.editor.softWrap;
              const desired = !(prev !== null ? prev : globalDefault);
              // If the desired value equals the global default, clear the override
              // so future global-setting changes are not silently ignored.
              return desired === globalDefault ? null : desired;
            })}
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
            onClick={() => setSaveTemplateOpen(true)}
            disabled={sql.trim().length === 0}
            title="Save this query to your local template library"
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

      {hasParamPanel && (
        <ParameterPanel
          tabId={tab.id}
          running={running}
          onRun={runPanel}
          onClose={() => closePanel(tab.id)}
          onEdit={clearError}
        />
      )}

      <div className="ed-scroll">
        <SqlCodeEditor
          value={sql}
          engine={sqlEditorEngine}
          databases={databases}
          database={tab.database}
          placeholder={PLACEHOLDER}
          wrap={wrap}
          showLineNumbers={settings.editor.lineNumbers}
          enableBracketMatching={settings.editor.bracketMatching}
          tabSize={settings.editor.tabSize}
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

      {saveTemplateOpen && (
        <SaveTemplateModal
          sql={sql}
          defaultName={tab.title.replace(/\.sql$/, "")}
          onClose={() => setSaveTemplateOpen(false)}
        />
      )}
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

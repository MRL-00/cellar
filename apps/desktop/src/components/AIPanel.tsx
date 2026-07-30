import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { ORDERED_TOPICS, TOPICS, type AiTopic } from "@cellar/ai";
import { commands, unwrap, type QueryResult } from "@cellar/ipc";
import { Icon } from "./icons";
import { useAi } from "../state/ai";
import { useTabs } from "../state/tabs";
import { noteConnectionIssue, useConnections } from "../state/connections";
import { buildActiveContext, type AiContextChip } from "../lib/aiContext";
import { runEntityLookups } from "../lib/aiLookup";
import { AIMessage, firstRunnableSql } from "./AIMessage";
import { QUERY_ROW_LIMIT } from "../hooks/useQueryRunner";
import { cellValueToGrid, queryResultToGrid } from "../lib/gridMapping";
import {
  buildRunErrorMessage,
  buildRunResultMessages,
  buildRunStartedMessage,
  type QueryRunContext,
} from "../lib/queryMessages";
import { useQueryMessages } from "../state/queryMessages";
import { queryResultSource, useTabResults } from "../state/tabResults";
import { useBottomPanel } from "../state/bottomPanel";
import { revealBottomPanel } from "../state/layout";
import { useStatus } from "../state/status";

const chipMeta: Record<AiContextChip["kind"], { icon: ReactNode; color: string }> = {
  schema: { icon: <Icon.schema size={9} />, color: "var(--fg-1)" },
  table: { icon: <Icon.table size={9} />, color: "var(--syn-tbl)" },
  query: { icon: <Icon.terminal size={9} />, color: "var(--syn-fn)" },
};

function ContextChip({ chip }: { chip: AiContextChip }) {
  const m = chipMeta[chip.kind];
  return (
    <span className="inline-flex h-5 items-center gap-1 whitespace-nowrap rounded-[4px] border border-border-default bg-bg-1 px-1.5 pl-[5px] text-[11.5px] text-fg-1">
      <span
        className="inline-flex items-center gap-[3px] text-[10.5px] lowercase opacity-85"
        style={{ color: m.color }}
      >
        {m.icon}
        <span>{chip.kind}</span>
      </span>
      <span className="text-fg-3">:</span>
      <span className="font-mono text-[11.5px] text-fg-0">{chip.value}</span>
    </span>
  );
}

const DISABLED_ICON =
  "icon-btn cursor-not-allowed opacity-45 hover:bg-transparent hover:text-fg-2";

const TOPIC_BUTTONS: AiTopic[] = ORDERED_TOPICS.filter((t) => t !== "ask");

type SqlScope = {
  connectionId: string;
  database: string;
  tabId: string;
  title: string;
};

function formatAiQueryAnswer(result: QueryResult): string {
  if (result.rows_affected != null) {
    return `Result: ${result.rows_affected.toLocaleString()} row${result.rows_affected === 1 ? "" : "s"} affected.`;
  }
  if (result.rows.length === 0) return "Result: no rows returned.";

  const headers = result.columns.map((c) => c.name);
  const rows = result.rows.slice(0, 5).map((row) =>
    row.map((cell) => String(cellValueToGrid(cell) ?? "NULL")),
  );

  if (headers.length === 1 && rows.length === 1) {
    return `Result: ${headers[0]} = ${rows[0]?.[0] ?? "NULL"}.`;
  }
  if (rows.length === 1) {
    return [
      "Result:",
      ...headers.map((h, i) => `${h}: ${rows[0]?.[i] ?? "NULL"}`),
    ].join("\n");
  }

  const body = rows.map((row) =>
    headers.map((h, i) => `${h}: ${row[i] ?? "NULL"}`).join(", "),
  );
  const suffix = result.rows.length > rows.length ? `\nShowing ${rows.length} of ${result.rows.length} rows.` : "";
  return ["Result:", ...body].join("\n") + suffix;
}

export function AIPanel({
  onClose,
  onOpenSettings,
}: {
  onClose: () => void;
  onOpenSettings?: () => void;
}) {
  const [draft, setDraft] = useState("");
  const [topic, setTopic] = useState<AiTopic>("ask");
  const [preparing, setPreparing] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const init = useAi((s) => s.init);
  const modelId = useAi((s) => s.modelId);
  const keyConfigured = useAi((s) => s.keyConfigured);
  const messages = useAi((s) => s.messages);
  const sending = useAi((s) => s.sending);
  const send = useAi((s) => s.send);
  const newThread = useAi((s) => s.newThread);
  const setQuerySql = useTabs((s) => s.setQuerySql);
  const newQueryTab = useTabs((s) => s.newQueryTab);
  const setActiveTab = useTabs((s) => s.setActive);
  const setBottomTab = useBottomPanel((s) => s.setActive);
  const [runningSql, setRunningSql] = useState(false);
  const autoRanMessageId = useRef<string | null>(null);
  const mounted = useRef(true);
  const runSeq = useRef(0);

  // Recompute context whenever the active tab, its SQL, or schema changes.
  const activeId = useTabs((s) => s.activeId);
  const activeSql = useTabs((s) => {
    const tab = s.tabs.find((t) => t.id === s.activeId);
    return tab?.kind === "query" ? tab.sql : "";
  });
  const activeTableKey = useTabs((s) => {
    const tab = s.tabs.find((t) => t.id === s.activeId);
    return tab?.kind === "table" ? `${tab.schema}.${tab.table}` : "";
  });
  const byId = useConnections((s) => s.byId);
  const context = useMemo(
    () => buildActiveContext(),
    // byId carries introspected schema; activeId/SQL/table pin scope.
    [activeId, activeSql, activeTableKey, byId],
  );

  useEffect(() => {
    void init();
  }, [init]);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      runSeq.current++;
    };
  }, []);

  const ready = keyConfigured && !!modelId;
  const canSend = ready && draft.trim().length > 0 && !sending && !preparing;

  const submit = () => {
    if (!canSend) return;
    const text = draft;
    setDraft("");
    // Resolve company/tenant names against live lookup tables first so the
    // model gets real TenantId values instead of guessing with LIKE.
    void (async () => {
      setPreparing(true);
      let lookupHits = "";
      try {
        lookupHits = await runEntityLookups(text);
      } catch {
        lookupHits = "";
      } finally {
        setPreparing(false);
      }
      await send(topic, text, context.text, lookupHits);
    })();
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key !== "Enter" || e.nativeEvent.isComposing) return;
    // Enter sends; Shift+Enter inserts a newline. ⌘/Ctrl+Enter also send.
    if (e.shiftKey) return;
    e.preventDefault();
    submit();
  };

  const pickTopic = (t: AiTopic) => {
    setTopic((cur) => (cur === t ? "ask" : t));
    textareaRef.current?.focus();
  };

  const insertSql = useCallback((sql: string): SqlScope | null => {
    const state = useTabs.getState();
    const active = state.tabs.find((t) => t.id === state.activeId);
    if (!active) return null;
    const id = active.kind === "query"
      ? active.id
      : newQueryTab(active.connectionId, active.database);
    setQuerySql(id, sql.trim() + "\n");
    setActiveTab(id);
    const tab = useTabs.getState().tabs.find((t) => t.id === id && t.kind === "query");
    if (!tab || tab.kind !== "query") return null;
    return {
      connectionId: tab.connectionId,
      database: tab.database,
      tabId: tab.id,
      title: tab.title,
    };
  }, [newQueryTab, setActiveTab, setQuerySql]);

  const activeScope = useCallback((): SqlScope | null => {
    const state = useTabs.getState();
    const active = state.tabs.find((t) => t.id === state.activeId);
    if (!active) return null;
    return {
      connectionId: active.connectionId,
      database: active.database,
      tabId: active.id,
      title: active.kind === "table" ? `${active.schema}.${active.table}` : active.title,
    };
  }, []);

  const appendAiResult = useCallback((content: string, error = false) => {
    useAi.setState((s) => ({
      messages: [
        ...s.messages,
        {
          id: `ai-result-${crypto.randomUUID()}`,
          role: "model",
          topic: "explain",
          content,
          error,
        },
      ],
    }));
  }, []);

  const scopeStillCurrent = useCallback((scope: SqlScope, sql: string) => {
    const tab = useTabs.getState().tabs.find((t) => t.id === scope.tabId);
    return tab?.kind === "query" && tab.sql.trim() === sql.trim();
  }, []);

  const runSql = useCallback((sql: string, opts?: { insert?: boolean; answerInPanel?: boolean }) => {
    if (runningSql) return;
    const scope = opts?.insert ? insertSql(sql) : activeScope();
    if (!scope) return;
    const trimmed = sql.trim();
    const database = scope.database || null;
    const source = queryResultSource(
      scope.connectionId,
      database,
      scope.tabId,
      scope.title,
      trimmed,
      QUERY_ROW_LIMIT,
    );
    const context: QueryRunContext = {
      tabId: scope.tabId,
      connectionId: scope.connectionId,
      database: scope.database || undefined,
      label: "AI SQL",
      sql: trimmed,
      maxRows: QUERY_ROW_LIMIT,
    };

    setRunningSql(true);
    const token = ++runSeq.current;
    if (!opts?.answerInPanel) {
      revealBottomPanel();
      setBottomTab("results");
      useTabResults.getState().setLoading(scope.tabId, source);
      useQueryMessages
        .getState()
        .replaceForTab(scope.tabId, [buildRunStartedMessage(context)]);
    }

    void (async () => {
      try {
        const result = await unwrap(
          commands.runReadOnlyQuery(
            scope.connectionId,
            trimmed,
            QUERY_ROW_LIMIT,
            database,
          ),
        );
        if (!mounted.current || token !== runSeq.current) return;
        if (opts?.insert && !scopeStillCurrent(scope, trimmed)) return;
        if (opts?.answerInPanel) {
          appendAiResult(formatAiQueryAnswer(result));
        } else {
          const { columns, rows } = queryResultToGrid(result);
          useTabResults.getState().setReady(scope.tabId, {
            source,
            columns,
            rows,
            rowCount: rows.length,
            truncated: result.truncated,
            durationMs: result.duration_ms,
            rowsAffected: result.rows_affected,
          });
          useQueryMessages
            .getState()
            .addMessages(buildRunResultMessages(context, result));
          useTabs.getState().markQueryRun(scope.tabId);
        }
        useStatus.getState().setLastQuery({
          connectionId: scope.connectionId,
          tabId: scope.tabId,
          rowCount: result.rows.length,
          truncated: result.truncated,
          durationMs: result.duration_ms,
        });
      } catch (err) {
        if (!mounted.current || token !== runSeq.current) return;
        noteConnectionIssue(scope.connectionId, err);
        const message = err instanceof Error ? err.message : String(err);
        if (opts?.answerInPanel) {
          appendAiResult(`Could not run the generated query: ${message}`, true);
        } else {
          useTabResults.getState().setError(scope.tabId, source, message);
          useQueryMessages
            .getState()
            .addMessage(buildRunErrorMessage(context, err));
        }
      } finally {
        if (mounted.current && token === runSeq.current) {
          setRunningSql(false);
        }
      }
    })();
  }, [activeScope, appendAiResult, insertSql, runningSql, scopeStillCurrent, setBottomTab]);

  useEffect(() => {
    const last = messages[messages.length - 1];
    if (!last || last.role !== "model" || last.error) return;
    if (last.id === autoRanMessageId.current) return;
    if (last.topic !== "explain" && last.topic !== "ask") return;
    const sql = firstRunnableSql(last.content);
    if (!sql) return;
    autoRanMessageId.current = last.id;
    runSql(sql, { answerInPanel: true });
  }, [messages, runSql]);

  return (
    <div className="flex h-full flex-col bg-bg-1 text-[13.5px]">
      <div className="flex h-8 shrink-0 items-center justify-between border-b border-border-default pl-2.5 pr-2">
        <div className="flex min-w-0 flex-1 items-center gap-1.5 text-[12.5px]">
          <span className="inline-flex text-accent">
            <Icon.sparkles size={12} />
          </span>
          <span className="whitespace-nowrap font-semibold text-fg-0">
            AI Assistant
          </span>
          <span className="min-w-0 overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[11.5px] text-fg-2">
            {modelId ? `· ${modelId}` : "· not configured"}
          </span>
        </div>
        <div className="flex gap-px">
          <button
            className={DISABLED_ICON}
            disabled
            title="AI history is not wired yet"
          >
            <Icon.history size={12} />
          </button>
          <button
            className={
              messages.length ? "icon-btn" : DISABLED_ICON
            }
            disabled={!messages.length}
            onClick={newThread}
            title="New thread"
          >
            <Icon.plus size={12} />
          </button>
          <button
            className="icon-btn"
            onClick={onOpenSettings}
            title="AI settings"
          >
            <Icon.settings size={12} />
          </button>
          <button className="icon-btn" onClick={onClose} title="Close">
            <Icon.close size={12} />
          </button>
        </div>
      </div>

      <div className="flex shrink-0 flex-wrap items-start gap-1.5 border-b border-border-default bg-bg-2 px-2 pt-1.5 pb-[7px]">
        <div className="inline-flex h-[18px] shrink-0 items-center gap-1 pt-0.5 text-[11px] font-semibold uppercase tracking-[0.05em] text-fg-3">
          <Icon.context size={10} />
          <span>context</span>
        </div>
        <div className="flex flex-1 flex-wrap gap-1">
          {context.chips.length ? (
            context.chips.map((chip, i) => (
              <ContextChip key={`${chip.kind}-${i}`} chip={chip} />
            ))
          ) : (
            <span className="inline-flex h-5 items-center text-[11.5px] text-fg-3">
              no active connection
            </span>
          )}
        </div>
      </div>

      <div className="flex flex-1 flex-col gap-3 overflow-y-auto px-2.5 pt-3 pb-4">
        {messages.length === 0 ? (
          <div className="flex flex-1 flex-col items-center justify-center gap-1.5 p-6 text-center text-[12.5px] text-fg-3">
            <span className="mb-1 text-accent">
              <Icon.sparkles size={22} />
            </span>
            <div className="text-sm font-medium text-fg-1">Ask Cellar AI</div>
            <div className="max-w-[280px] text-[11.5px] leading-[1.5] text-fg-3">
              Generate SQL with full schema context, explain a result, or have it
              review a slow query. Bring your own API key; Cellar never proxies.
            </div>
            {!ready && (
              <button
                onClick={onOpenSettings}
                className="mt-2 inline-flex h-[24px] items-center gap-1.5 rounded-[5px] border border-accent-line bg-accent-soft px-2.5 text-[12px] font-medium text-accent hover:brightness-110"
              >
                <Icon.settings size={11} />
                <span>
                  {keyConfigured ? "Select a model" : "Configure a provider"}
                </span>
              </button>
            )}
          </div>
        ) : (
          messages.map((m) => (
            <AIMessage
              key={m.id}
              entry={m}
              onInsertSql={insertSql}
              onRunSql={(sql) => runSql(sql, { insert: true })}
              runningSql={runningSql}
            />
          ))
        )}
        {(preparing || sending) && (
          <div className="flex items-center gap-2 px-1 text-[12px] text-fg-3">
            <span className="inline-flex text-accent animate-spinner" aria-hidden>
              <Icon.spinner size={13} sw={1.8} />
            </span>
            <span>{preparing ? "looking up…" : "thinking…"}</span>
          </div>
        )}
      </div>

      <div className="shrink-0 border-t border-border-default bg-bg-1">
        <div className="flex items-center gap-0.5 border-b border-border-divider px-1.5 py-[5px]">
          {TOPIC_BUTTONS.map((t) => {
            const active = topic === t;
            return (
              <button
                key={t}
                onClick={() => pickTopic(t)}
                title={TOPICS[t].hint}
                className={
                  "inline-flex h-[22px] items-center gap-1 rounded-[4px] px-2 text-[12px] transition-colors " +
                  (active
                    ? "bg-accent-soft text-accent"
                    : "text-fg-2 hover:bg-bg-2 hover:text-fg-1")
                }
              >
                {t === "generate" && <Icon.sparkles size={10} />}
                <span>{t}</span>
              </button>
            );
          })}
          <div className="flex-1" />
          <span className="inline-flex h-[22px] items-center gap-1 rounded-[4px] px-2 text-[11.5px] text-fg-3">
            <span style={{ color: topic === "ask" ? "var(--accent)" : "var(--fg-2)" }}>
              ask
            </span>
            <span style={{ color: "var(--fg-3)" }}> · read-only</span>
          </span>
        </div>
        <div className="px-1.5 py-1.5">
          <textarea
            ref={textareaRef}
            placeholder={
              ready
                ? "Ask, generate, or paste an error…  ⏎ to send, ⇧⏎ for newline"
                : "Configure a provider in AI settings to start…"
            }
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={onKeyDown}
            rows={2}
            disabled={!ready}
            className="w-full resize-none rounded-[5px] border border-border-default bg-bg-inset px-2 py-[7px] text-sm leading-[1.45] text-fg-0 outline-none placeholder:text-fg-3 focus:border-accent-line font-sans min-h-[50px] disabled:opacity-60"
          />
          <div className="mt-1.5 flex items-center justify-between">
            <div className="flex items-center gap-1">
              <button
                className={DISABLED_ICON}
                disabled
                title="Query attachments are not wired yet"
              >
                <Icon.paperclip size={11} />
              </button>
              <span className="ml-1 inline-flex items-center gap-[3px] text-[11.5px]">
                {ready ? (
                  <span style={{ color: "var(--fg-3)" }}>
                    {topic} · {modelId}
                  </span>
                ) : (
                  <button
                    onClick={onOpenSettings}
                    className="text-fg-3 underline decoration-dotted underline-offset-2 hover:text-fg-1"
                  >
                    provider not configured
                  </button>
                )}
              </span>
            </div>
            <button
              onClick={submit}
              disabled={!canSend}
              className="inline-flex h-[22px] items-center gap-1.5 rounded-[4px] bg-accent px-2 text-[12px] font-medium text-accent-fg hover:brightness-[1.07] disabled:opacity-40 disabled:hover:brightness-100"
            >
              <span>Send</span>
              <Icon.send size={11} />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

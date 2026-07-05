import { useState } from "react";
import { Icon } from "./icons";
import type { AiChatEntry } from "../state/ai";
import { tokenizeSql, tokensToLines, renderTokens } from "../lib/sqlTokens";

const SQL_LANGS = /^(sql|postgres|postgresql|pgsql|mysql|sqlite|tsql|mssql)$/i;

/** A run of message content: either prose or a fenced code block. */
type Segment =
  | { kind: "text"; text: string }
  | { kind: "code"; lang: string; code: string };

const FENCE = /```([\w-]*)\n?([\s\S]*?)```/g;
const READ_ONLY_SQL =
  /^(?:\s|\/\*[\s\S]*?\*\/|--[^\n]*\n)*(select|with|show|describe|desc|explain)\b/i;
const WRITE_SQL =
  /\b(insert|update|delete|drop|truncate|alter|create|merge|grant|revoke|vacuum|analyze)\b/i;

/** Split markdown-ish content into prose and fenced code blocks. We only need
 * enough fidelity to render SQL in a mono box with a copy button. */
export function parseSegments(content: string): Segment[] {
  const out: Segment[] = [];
  let last = 0;
  let m: RegExpExecArray | null;
  FENCE.lastIndex = 0;
  while ((m = FENCE.exec(content)) !== null) {
    if (m.index > last) {
      const text = content.slice(last, m.index).trim();
      if (text) out.push({ kind: "text", text });
    }
    out.push({ kind: "code", lang: m[1] || "", code: (m[2] ?? "").trim() });
    last = m.index + m[0].length;
  }
  if (last < content.length) {
    const text = content.slice(last).trim();
    if (text) out.push({ kind: "text", text });
  }
  if (out.length === 0) out.push({ kind: "text", text: content });
  return out;
}

export function canRunFromAi(lang: string, code: string): boolean {
  const sql = code.trim();
  return SQL_LANGS.test(lang.trim()) && READ_ONLY_SQL.test(sql) && !WRITE_SQL.test(sql);
}

export function firstRunnableSql(content: string): string | null {
  for (const seg of parseSegments(content)) {
    if (seg.kind === "code" && canRunFromAi(seg.lang, seg.code)) return seg.code;
  }
  return null;
}

function TextBlock({ text }: { text: string }) {
  const lines = text.split("\n").filter((line) => line.trim().length > 0);
  return (
    <div className="space-y-1 text-sm leading-[1.55] text-fg-1">
      {lines.map((line, i) => {
        const trimmed = line.trim();
        const heading = trimmed.match(/^#{1,4}\s+(.+)$/);
        const bullet = trimmed.match(/^[-*]\s+(.+)$/);
        const body = (heading?.[1] ?? bullet?.[1] ?? trimmed)
          .replace(/\*\*([^*]+)\*\*/g, "$1")
          .replace(/`([^`]+)`/g, "$1");
        return (
          <div
            key={i}
            className={
              heading
                ? "pt-1 font-semibold text-fg-0"
                : bullet
                  ? "pl-3 before:mr-2 before:content-['-']"
                  : ""
            }
          >
            {body}
          </div>
        );
      })}
    </div>
  );
}

function CodeBlock({
  lang,
  code,
  onInsert,
  onRun,
  running,
}: {
  lang: string;
  code: string;
  onInsert?: (code: string) => void;
  onRun?: (code: string) => void;
  running?: boolean;
}) {
  const [copied, setCopied] = useState(false);
  const runnable = canRunFromAi(lang, code);
  const copy = () => {
    void navigator.clipboard?.writeText(code).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1200);
    });
  };
  return (
    <div className="overflow-hidden rounded-[5px] border border-border-default bg-bg-inset">
      <div className="flex items-center justify-between gap-2 border-b border-border-divider px-2 py-1">
        <span className="font-mono text-[10.5px] uppercase tracking-[0.05em] text-fg-3">
          {lang || "code"}
        </span>
        <div className="flex items-center gap-1">
          <button
            onClick={copy}
            className="inline-flex items-center gap-1 rounded-[3px] px-1 text-[11px] text-fg-2 hover:bg-bg-3 hover:text-fg-0"
            title="Copy"
          >
            <Icon.copy size={10} />
            <span>{copied ? "copied" : "copy"}</span>
          </button>
          {SQL_LANGS.test(lang.trim()) && (
            <>
              <button
                onClick={() => onInsert?.(code)}
                className="inline-flex items-center gap-1 rounded-[3px] px-1 text-[11px] text-fg-2 hover:bg-bg-3 hover:text-fg-0"
                title="Insert into a query tab"
              >
                <Icon.edit size={10} />
                <span>insert</span>
              </button>
              <button
                onClick={() => onRun?.(code)}
                disabled={!runnable || running}
                className="inline-flex items-center gap-1 rounded-[3px] px-1 text-[11px] text-accent hover:bg-accent-soft disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-transparent"
                title={
                  runnable
                    ? "Run read-only SQL"
                    : "Only read-only AI SQL can run directly"
                }
              >
                <Icon.playSm size={10} />
                <span>{running ? "running" : "run"}</span>
              </button>
            </>
          )}
        </div>
      </div>
      <pre className="overflow-x-auto px-2.5 py-2 font-mono text-sm leading-[1.5] text-fg-0">
        {SQL_LANGS.test(lang.trim()) ? (
          tokensToLines(tokenizeSql(code)).map((toks, i) => (
            <div key={i} className="whitespace-pre">
              {toks.length ? renderTokens(toks) : " "}
            </div>
          ))
        ) : (
          <code>{code}</code>
        )}
      </pre>
    </div>
  );
}

export function AIMessage({
  entry,
  onInsertSql,
  onRunSql,
  runningSql,
}: {
  entry: AiChatEntry;
  onInsertSql?: (sql: string) => void;
  onRunSql?: (sql: string) => void;
  runningSql?: boolean;
}) {
  if (entry.role === "user") {
    return (
      <div className="flex flex-col items-end gap-1">
        <div className="max-w-[88%] rounded-[7px] rounded-tr-[2px] border border-accent-line bg-accent-soft px-2.5 py-1.5">
          {entry.topic && entry.topic !== "ask" && (
            <span className="mb-0.5 block font-mono text-[10.5px] uppercase tracking-[0.05em] text-accent opacity-80">
              {entry.topic}
            </span>
          )}
          <div className="whitespace-pre-wrap text-sm leading-[1.5] text-fg-0">
            {entry.content || <span className="text-fg-3 italic">(empty)</span>}
          </div>
        </div>
      </div>
    );
  }

  if (entry.error) {
    return (
      <div className="flex items-start gap-2 rounded-[6px] border border-delete-line bg-delete-bg px-2.5 py-2 text-sm text-delete">
        <span className="mt-px shrink-0">
          <Icon.warn size={12} />
        </span>
        <div className="whitespace-pre-wrap leading-[1.5]">{entry.content}</div>
      </div>
    );
  }

  const segments = parseSegments(entry.content);
  return (
    <div className="flex flex-col gap-2">
      {segments.map((seg, i) =>
        seg.kind === "code" ? (
          <CodeBlock
            key={i}
            lang={seg.lang}
            code={seg.code}
            onInsert={onInsertSql}
            onRun={onRunSql}
            running={runningSql}
          />
        ) : (
          <TextBlock key={i} text={seg.text} />
        ),
      )}
      {entry.usage && (
        <div className="text-[10.5px] text-fg-3">
          {entry.usage.totalTokens} tokens · {entry.usage.promptTokens} in /{" "}
          {entry.usage.completionTokens} out
        </div>
      )}
    </div>
  );
}

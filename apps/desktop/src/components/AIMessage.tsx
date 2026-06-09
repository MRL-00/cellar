import { useState } from "react";
import { TOPICS } from "@cellar/ai";
import { Icon } from "./icons";
import type { AiChatEntry } from "../state/ai";

/** A run of message content: either prose or a fenced code block. */
type Segment =
  | { kind: "text"; text: string }
  | { kind: "code"; lang: string; code: string };

const FENCE = /```([\w-]*)\n?([\s\S]*?)```/g;

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

function CodeBlock({ lang, code }: { lang: string; code: string }) {
  const [copied, setCopied] = useState(false);
  const copy = () => {
    void navigator.clipboard?.writeText(code).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1200);
    });
  };
  return (
    <div className="overflow-hidden rounded-[5px] border border-border-default bg-bg-inset">
      <div className="flex items-center justify-between border-b border-border-divider px-2 py-1">
        <span className="font-mono text-[9.5px] uppercase tracking-[0.05em] text-fg-3">
          {lang || "code"}
        </span>
        <button
          onClick={copy}
          className="inline-flex items-center gap-1 rounded-[3px] px-1 text-[10px] text-fg-2 hover:bg-bg-3 hover:text-fg-0"
          title="Copy"
        >
          <Icon.copy size={10} />
          <span>{copied ? "copied" : "copy"}</span>
        </button>
      </div>
      <pre className="overflow-x-auto px-2.5 py-2 font-mono text-[11.5px] leading-[1.5] text-fg-0">
        <code>{code}</code>
      </pre>
    </div>
  );
}

export function AIMessage({ entry }: { entry: AiChatEntry }) {
  if (entry.role === "user") {
    return (
      <div className="flex flex-col items-end gap-1">
        <div className="max-w-[88%] rounded-[7px] rounded-tr-[2px] border border-accent-line bg-accent-soft px-2.5 py-1.5">
          {entry.topic && entry.topic !== "ask" && (
            <span className="mb-0.5 block font-mono text-[9.5px] uppercase tracking-[0.05em] text-accent opacity-80">
              {TOPICS[entry.topic].label}
            </span>
          )}
          <div className="whitespace-pre-wrap text-[12px] leading-[1.5] text-fg-0">
            {entry.content || <span className="text-fg-3 italic">(empty)</span>}
          </div>
        </div>
      </div>
    );
  }

  if (entry.error) {
    return (
      <div className="flex items-start gap-2 rounded-[6px] border border-delete-line bg-delete-bg px-2.5 py-2 text-[11.5px] text-delete">
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
          <CodeBlock key={i} lang={seg.lang} code={seg.code} />
        ) : (
          <div
            key={i}
            className="whitespace-pre-wrap text-[12px] leading-[1.55] text-fg-1"
          >
            {seg.text}
          </div>
        ),
      )}
      {entry.usage && (
        <div className="text-[9.5px] text-fg-3">
          {entry.usage.totalTokens} tokens · {entry.usage.promptTokens} in /{" "}
          {entry.usage.completionTokens} out
        </div>
      )}
    </div>
  );
}

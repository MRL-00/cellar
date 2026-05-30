import { useState } from "react";

import {
  formatMessageTime,
  QUERY_MESSAGE_SEVERITIES,
  severityCounts,
  type QueryMessage,
  type QueryMessageSeverity,
} from "../lib/queryMessages";

export function MessagesView({
  messages,
  hasActiveTab = true,
}: {
  messages: QueryMessage[];
  hasActiveTab?: boolean;
}) {
  const [filter, setFilter] = useState<QueryMessageSeverity | "all">("all");
  const counts = severityCounts(messages);
  const visible =
    filter === "all" ? messages : messages.filter((m) => m.severity === filter);

  if (!hasActiveTab) {
    return (
      <EmptyMessagePanel
        title="No active tab"
        detail="Open a table or query tab to see execution feedback."
      />
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-col bg-bg-inset">
      <div className="flex h-8 shrink-0 items-center justify-between border-b border-border-divider px-2">
        <div className="flex items-center gap-1">
          <SeverityFilter
            label="all"
            active={filter === "all"}
            count={messages.length}
            onClick={() => setFilter("all")}
          />
          {QUERY_MESSAGE_SEVERITIES.map((severity) => (
            <SeverityFilter
              key={severity}
              label={severity}
              active={filter === severity}
              count={counts[severity]}
              onClick={() => setFilter(severity)}
            />
          ))}
        </div>
        <div className="font-mono text-[10px] text-fg-3">
          {messages.length === 0 ? "no messages" : `${messages.length} total`}
        </div>
      </div>
      {messages.length === 0 ? (
        <EmptyMessagePanel
          title="No execution messages"
          detail="Run or refresh the active tab to populate status, warnings, and errors."
        />
      ) : visible.length === 0 ? (
        <EmptyMessagePanel
          title="No matching messages"
          detail="Adjust the severity filter to inspect the current execution feedback."
        />
      ) : (
        <div className="min-h-0 flex-1 overflow-auto">
          <div className="grid min-w-[720px] grid-cols-[92px_74px_86px_minmax(220px,1fr)_160px] border-b border-border-divider px-2 py-1 font-mono text-[10px] uppercase text-fg-3">
            <span>time</span>
            <span>level</span>
            <span>source</span>
            <span>message</span>
            <span>metrics</span>
          </div>
          {visible.map((message) => (
            <MessageRow key={message.id} message={message} />
          ))}
        </div>
      )}
    </div>
  );
}

function SeverityFilter({
  label,
  count,
  active,
  onClick,
}: {
  label: string;
  count: number;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={
        "inline-flex h-[22px] items-center gap-1 rounded-[4px] px-2 font-mono text-[10.5px] " +
        (active
          ? "bg-accent-soft text-accent"
          : "text-fg-2 hover:bg-bg-2 hover:text-fg-0")
      }
    >
      <span>{label}</span>
      <span className={active ? "text-accent" : "text-fg-3"}>{count}</span>
    </button>
  );
}

function MessageRow({ message }: { message: QueryMessage }) {
  return (
    <div className="grid min-w-[720px] grid-cols-[92px_74px_86px_minmax(220px,1fr)_160px] items-start border-b border-border-subtle px-2 py-1.5 font-mono text-[10.5px] leading-[1.45] hover:bg-bg-1">
      <span className="text-fg-3">{formatMessageTime(message.timestamp)}</span>
      <span className={severityClass(message.severity)}>{message.severity}</span>
      <span className="text-fg-2">{message.source}</span>
      <span className="min-w-0 whitespace-pre-wrap break-words pr-3 text-fg-1">
        {message.text}
      </span>
      <span className="text-fg-3">{metricsText(message)}</span>
    </div>
  );
}

function EmptyMessagePanel({
  title,
  detail,
}: {
  title: string;
  detail: string;
}) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-1.5 bg-bg-inset p-6 text-center text-[11.5px] text-fg-3">
      <div className="text-[12px] font-medium text-fg-1">{title}</div>
      <div className="max-w-[460px] text-[10.5px] leading-[1.5] text-fg-3">
        {detail}
      </div>
    </div>
  );
}

function metricsText(message: QueryMessage): string {
  const parts: string[] = [];
  if (message.durationMs != null) parts.push(`${message.durationMs} ms`);
  if (message.rowCount != null) {
    parts.push(`${message.rowCount.toLocaleString()} row${message.rowCount === 1 ? "" : "s"}`);
  }
  if (message.statementIndex != null) parts.push(`stmt ${message.statementIndex + 1}`);
  return parts.length ? parts.join(" | ") : "-";
}

function severityClass(severity: QueryMessageSeverity): string {
  switch (severity) {
    case "success":
      return "text-insert";
    case "warning":
      return "text-warn";
    case "error":
      return "text-delete";
    case "info":
      return "text-info";
  }
}

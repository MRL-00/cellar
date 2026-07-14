import {
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type PointerEvent as ReactPointerEvent,
} from "react";

import {
  formatMessageTime,
  QUERY_MESSAGE_SEVERITIES,
  severityCounts,
  type QueryMessage,
  type QueryMessageSeverity,
} from "../lib/queryMessages";

type MessageColumnKey = "time" | "level" | "source" | "message" | "metrics";

type MessageColumnWidths = Record<MessageColumnKey, number>;

type ColumnResizeState = {
  columnKey: MessageColumnKey;
  startX: number;
  startWidth: number;
};

const MESSAGE_COLUMNS: ReadonlyArray<{
  key: MessageColumnKey;
  label: string;
}> = [
  { key: "time", label: "time" },
  { key: "level", label: "level" },
  { key: "source", label: "source" },
  { key: "message", label: "message" },
  { key: "metrics", label: "metrics" },
];

/** Defaults sized for JetBrains Mono at 14px so values don't collide. */
const DEFAULT_COLUMN_WIDTHS: MessageColumnWidths = {
  time: 130,
  level: 88,
  source: 108,
  message: 480,
  metrics: 180,
};

const MIN_COLUMN_WIDTHS: MessageColumnWidths = {
  time: 88,
  level: 64,
  source: 72,
  message: 180,
  metrics: 96,
};

export function MessagesView({
  messages,
  hasActiveTab = true,
}: {
  messages: QueryMessage[];
  hasActiveTab?: boolean;
}) {
  const [filter, setFilter] = useState<QueryMessageSeverity | "all">("all");
  const [widths, setWidths] = useState<MessageColumnWidths>(DEFAULT_COLUMN_WIDTHS);
  const [resizing, setResizing] = useState<ColumnResizeState | null>(null);
  const resizingRef = useRef<ColumnResizeState | null>(null);
  const widthsRef = useRef(widths);
  const counts = severityCounts(messages);
  const visible =
    filter === "all" ? messages : messages.filter((m) => m.severity === filter);

  useEffect(() => {
    widthsRef.current = widths;
  }, [widths]);

  useEffect(() => {
    resizingRef.current = resizing;
  }, [resizing]);

  useEffect(() => {
    if (!resizing) return;
    const previousCursor = document.body.style.cursor;
    const previousUserSelect = document.body.style.userSelect;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";

    const onPointerMove = (event: PointerEvent) => {
      const active = resizingRef.current;
      if (!active) return;
      const nextWidth = Math.max(
        MIN_COLUMN_WIDTHS[active.columnKey],
        Math.round(active.startWidth + event.clientX - active.startX),
      );
      setWidths({
        ...widthsRef.current,
        [active.columnKey]: nextWidth,
      });
    };

    const onPointerUp = () => {
      resizingRef.current = null;
      setResizing(null);
    };

    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp, { once: true });
    window.addEventListener("pointercancel", onPointerUp, { once: true });
    return () => {
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onPointerUp);
      window.removeEventListener("pointercancel", onPointerUp);
      document.body.style.cursor = previousCursor;
      document.body.style.userSelect = previousUserSelect;
    };
  }, [resizing]);

  function beginColumnResize(
    event: ReactPointerEvent<HTMLSpanElement>,
    columnKey: MessageColumnKey,
  ) {
    event.preventDefault();
    event.stopPropagation();
    const next: ColumnResizeState = {
      columnKey,
      startX: event.clientX,
      startWidth: widthsRef.current[columnKey],
    };
    resizingRef.current = next;
    setResizing(next);
  }

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
        <div className="font-mono text-[12px] text-fg-3">
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
        <div className="min-h-0 min-w-0 flex-1 overflow-auto">
          <div className="inline-flex min-w-full flex-col font-mono text-sm">
            <div className="sticky top-0 z-[1] flex border-b border-border-divider bg-bg-inset text-[12px] uppercase text-fg-3">
              {MESSAGE_COLUMNS.map((column) => (
                <HeaderCell
                  key={column.key}
                  columnKey={column.key}
                  label={column.label}
                  width={widths[column.key]}
                  resizing={resizing?.columnKey === column.key}
                  onResizeStart={(event) => beginColumnResize(event, column.key)}
                />
              ))}
            </div>
            {visible.map((message) => (
              <MessageRow key={message.id} message={message} widths={widths} />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function HeaderCell({
  columnKey,
  label,
  width,
  resizing,
  onResizeStart,
}: {
  columnKey: MessageColumnKey;
  label: string;
  width: number;
  resizing: boolean;
  onResizeStart: (event: ReactPointerEvent<HTMLSpanElement>) => void;
}) {
  return (
    <div
      className="relative shrink-0 border-r border-border-subtle"
      style={columnStyle(width)}
    >
      <span className="block truncate px-2.5 py-1">{label}</span>
      <span
        className={
          "absolute top-0 right-[-2px] bottom-0 z-[5] w-1.5 cursor-col-resize touch-none hover:bg-accent-line " +
          (resizing ? "bg-accent-line" : "")
        }
        role="separator"
        aria-orientation="vertical"
        aria-label={`Resize ${label} column`}
        aria-valuemin={MIN_COLUMN_WIDTHS[columnKey]}
        aria-valuenow={width}
        tabIndex={-1}
        onPointerDown={onResizeStart}
      />
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
        "inline-flex h-[22px] items-center gap-1 rounded-[4px] px-2 font-mono text-[12px] " +
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

function MessageRow({
  message,
  widths,
}: {
  message: QueryMessage;
  widths: MessageColumnWidths;
}) {
  return (
    <div className="flex items-start border-b border-border-subtle hover:bg-bg-1">
      <span
        className="shrink-0 truncate overflow-hidden border-r border-border-subtle px-2.5 py-1.5 leading-[1.45] text-fg-3"
        style={columnStyle(widths.time)}
      >
        {formatMessageTime(message.timestamp)}
      </span>
      <span
        className={
          "shrink-0 truncate overflow-hidden border-r border-border-subtle px-2.5 py-1.5 leading-[1.45] " +
          severityClass(message.severity)
        }
        style={columnStyle(widths.level)}
      >
        {message.severity}
      </span>
      <span
        className="shrink-0 truncate overflow-hidden border-r border-border-subtle px-2.5 py-1.5 leading-[1.45] text-fg-2"
        style={columnStyle(widths.source)}
      >
        {message.source}
      </span>
      <span
        className="shrink-0 overflow-hidden border-r border-border-subtle whitespace-pre-wrap break-words px-2.5 py-1.5 leading-[1.45] text-fg-1"
        style={columnStyle(widths.message)}
      >
        {message.text}
      </span>
      <span
        className="shrink-0 truncate overflow-hidden px-2.5 py-1.5 leading-[1.45] text-fg-3"
        style={columnStyle(widths.metrics)}
      >
        {metricsText(message)}
      </span>
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
    <div className="flex h-full flex-col items-center justify-center gap-1.5 bg-bg-inset p-6 text-center text-[12.5px] text-fg-3">
      <div className="text-[13px] font-medium text-fg-1">{title}</div>
      <div className="max-w-[460px] text-[11.5px] leading-[1.5] text-fg-3">
        {detail}
      </div>
    </div>
  );
}

function columnStyle(width: number): CSSProperties {
  return {
    width,
    flexBasis: width,
    maxWidth: width,
  };
}

function metricsText(message: QueryMessage): string {
  const parts: string[] = [];
  if (message.durationMs != null) parts.push(`${message.durationMs} ms`);
  if (message.rowCount != null) {
    parts.push(
      `${message.rowCount.toLocaleString("en-US")} row${message.rowCount === 1 ? "" : "s"}`,
    );
  }
  if (message.statementIndex != null) {
    parts.push(`stmt ${message.statementIndex + 1}`);
  }
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

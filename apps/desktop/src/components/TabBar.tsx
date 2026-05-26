import { Icon } from "./icons";

export type Tab = {
  id: string;
  title: string;
  kind: "query" | "grid";
  dirty?: boolean;
};

const SAMPLE_TABS: Tab[] = [
  { id: "tab-query-1", title: "revenue_by_country.sql", kind: "query", dirty: true },
  { id: "tab-orders", title: "public.orders", kind: "grid" },
  { id: "tab-customers", title: "public.customers", kind: "grid" },
  { id: "tab-query-2", title: "untitled-3.sql", kind: "query" },
];

export function TabBar({
  activeId = "tab-query-1",
  onActivate,
}: {
  activeId?: string;
  onActivate?: (id: string) => void;
}) {
  return (
    <div className="tabs-root">
      <div className="tabs-scroll">
        {SAMPLE_TABS.map((t) => {
          const isActive = t.id === activeId;
          const isQuery = t.kind === "query";
          return (
            <div
              key={t.id}
              className={"tab" + (isActive ? " active" : "")}
              onClick={() => onActivate?.(t.id)}
            >
              <span
                className="tab-accent"
                style={{ background: "var(--eng-postgres)" }}
              />
              <span
                className="tab-icon"
                style={{ color: isQuery ? "var(--syn-fn)" : "var(--fg-1)" }}
              >
                {isQuery ? (
                  <Icon.terminal size={11} />
                ) : (
                  <Icon.table size={11} />
                )}
              </span>
              <span className="tab-title">{t.title}</span>
              {t.dirty && <span className="tab-dot" title="Unsaved" />}
              <button
                className="tab-close"
                onClick={(e) => e.stopPropagation()}
                title="Close"
              >
                <Icon.close size={10} />
              </button>
            </div>
          );
        })}
        <button className="tab-new" title="New query tab">
          <Icon.plus size={11} />
        </button>
      </div>
      <div className="tabs-actions">
        <button className="icon-btn" title="Split horizontal">
          <Icon.splitH size={12} />
        </button>
        <button className="icon-btn" title="Split vertical">
          <Icon.splitV size={12} />
        </button>
        <button className="icon-btn" title="Re-open closed">
          <Icon.history size={12} />
        </button>
      </div>
    </div>
  );
}

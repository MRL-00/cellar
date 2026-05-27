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
    <div className="flex h-[30px] items-stretch shrink-0 border-b border-border-default bg-bg-1">
      <div className="flex flex-1 min-w-0 overflow-x-auto">
        {SAMPLE_TABS.map((t) => {
          const isActive = t.id === activeId;
          const isQuery = t.kind === "query";
          return (
            <div
              key={t.id}
              onClick={() => onActivate?.(t.id)}
              className={
                "group relative inline-flex items-center gap-1.5 h-full pl-2.5 pr-2 max-w-[220px] shrink-0 border-r border-border-default text-[11.5px] cursor-pointer transition-[background,color] duration-100 " +
                (isActive
                  ? "bg-bg-0 text-fg-0 border-b border-bg-0 -mb-px"
                  : "bg-bg-1 text-fg-2 hover:bg-bg-2 hover:text-fg-1")
              }
            >
              <span
                className={
                  "absolute left-0 top-0 h-full w-0.5 transition-opacity duration-150 " +
                  (isActive ? "opacity-100" : "opacity-0")
                }
                style={{ background: "var(--eng-postgres)" }}
              />
              <span
                className="inline-flex"
                style={{ color: isQuery ? "var(--syn-fn)" : "var(--fg-1)" }}
              >
                {isQuery ? (
                  <Icon.terminal size={11} />
                ) : (
                  <Icon.table size={11} />
                )}
              </span>
              <span className="overflow-hidden text-ellipsis whitespace-nowrap">
                {t.title}
              </span>
              {t.dirty && (
                <span
                  className="h-1.5 w-1.5 shrink-0 rounded-full bg-accent"
                  title="Unsaved"
                />
              )}
              <button
                onClick={(e) => e.stopPropagation()}
                title="Close"
                className={
                  "ml-0.5 inline-flex h-4 w-4 items-center justify-center rounded-[3px] text-fg-3 transition-opacity duration-100 hover:bg-bg-3 hover:text-fg-0 " +
                  (isActive ? "opacity-100" : "opacity-0 group-hover:opacity-100")
                }
              >
                <Icon.close size={10} />
              </button>
            </div>
          );
        })}
        <button
          title="New query tab"
          className="inline-flex w-7 items-center justify-center border-r border-border-default text-fg-3 transition-[background,color] duration-100 hover:bg-bg-2 hover:text-fg-0"
        >
          <Icon.plus size={11} />
        </button>
      </div>
      <div className="flex items-center gap-px border-l border-border-default px-1.5">
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

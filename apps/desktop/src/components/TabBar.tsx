import { Icon } from "./icons";
import { useTabs } from "../state/tabs";

export function TabBar() {
  const tabs = useTabs((s) => s.tabs);
  const activeId = useTabs((s) => s.activeId);
  const setActive = useTabs((s) => s.setActive);
  const closeTab = useTabs((s) => s.closeTab);

  return (
    <div className="flex h-[30px] items-stretch shrink-0 border-b border-border-default bg-bg-1">
      <div className="flex flex-1 min-w-0 overflow-x-auto">
        {tabs.length === 0 && (
          <div className="inline-flex items-center px-3 text-[11px] text-fg-3">
            no tabs — double-click a table in the sidebar
          </div>
        )}
        {tabs.map((t) => {
          const isActive = t.id === activeId;
          return (
            <div
              key={t.id}
              onClick={() => setActive(t.id)}
              className={
                "group relative inline-flex items-center gap-1.5 h-full pl-2.5 pr-2 max-w-[260px] shrink-0 border-r border-border-default text-[11.5px] cursor-pointer transition-[background,color] duration-100 " +
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
              <span className="inline-flex" style={{ color: "var(--fg-1)" }}>
                {t.kind === "query" ? (
                  <Icon.terminal size={11} />
                ) : (
                  <Icon.table size={11} />
                )}
              </span>
              <span className="overflow-hidden text-ellipsis whitespace-nowrap font-mono">
                {t.kind === "query" ? t.title : `${t.schema}.${t.table}`}
              </span>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  closeTab(t.id);
                }}
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

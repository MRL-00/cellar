import { getCurrentWindow } from "@tauri-apps/api/window";
import { Icon } from "./icons";
import { useTabs } from "../state/tabs";
import { useConnections } from "../state/connections";
import { ENGINE_META } from "./EngineBadge";
import type { PanelId, Panels } from "../state/layout";

// WKWebView ignores Electron's `-webkit-app-region`, so window dragging and
// double-click-to-maximize are driven manually. The catch: calling
// `startDragging()` on the first press consumes the native click sequence, so
// WebKit's own double-click counter (`e.detail`) never reaches 2 on the second
// press. We therefore detect the double-click ourselves by timing the gap
// between presses (with `e.detail` kept as a fallback), and call
// `toggleMaximize()` explicitly so it doesn't depend on the macOS
// "double-click a window's title bar to…" system setting.
let lastTitleBarPress = 0;

function onTitleBarMouseDown(e: React.MouseEvent<HTMLDivElement>) {
  if (e.button !== 0) return;
  if ((e.target as HTMLElement).closest("button, input, a")) return;
  if (!("__TAURI_INTERNALS__" in window)) return;
  const win = getCurrentWindow();
  const doubleClick = e.detail === 2 || e.timeStamp - lastTitleBarPress < 400;
  lastTitleBarPress = doubleClick ? 0 : e.timeStamp;
  if (doubleClick) void win.toggleMaximize();
  else void win.startDragging();
}

export function TitleBar({
  panels,
  onTogglePanel,
  empty,
  onToggleEmpty,
  onOpenPalette,
}: {
  panels: Panels;
  onTogglePanel: (k: PanelId) => void;
  empty?: boolean;
  onToggleEmpty?: () => void;
  onOpenPalette?: () => void;
}) {
  const activeId = useTabs((s) => s.activeId);
  const tabs = useTabs((s) => s.tabs);
  const connections = useConnections((s) => s.connections);
  const activeTab = tabs.find((t) => t.id === activeId) ?? null;
  const activeConn = activeTab
    ? connections.find((c) => c.id === activeTab.connectionId) ?? null
    : null;

  return (
    <div
      onMouseDown={onTitleBarMouseDown}
      className={
        "relative flex shrink-0 h-[34px] items-center gap-2.5 px-2.5 " +
        (empty
          ? "bg-bg-0 border-b border-transparent"
          : "bg-bg-1 border-b border-border-default")
      }
    >
      <div className="flex shrink-0 items-center gap-1.5">
        {/* Reserves room for macOS native traffic lights (positioned via
            tauri.conf.json `trafficLightPosition`). */}
        <div
          aria-hidden="true"
          className="pointer-events-none h-full w-[68px] shrink-0"
        />
        <div className="flex items-center gap-1.5 px-1">
          <span
            className="relative h-[14px] w-[14px] rounded-[3px]"
            style={{
              background:
                "linear-gradient(135deg, #c4b5fd 0%, var(--accent) 55%, #6d4ed1 100%)",
              boxShadow: "0 0 0 1px rgba(0, 0, 0, 0.2) inset",
            }}
          >
            <span
              className="absolute inset-[2px] rounded-[1px] bg-bg-1"
              style={{
                clipPath:
                  "polygon(0 0, 100% 0, 100% 35%, 35% 35%, 35% 65%, 100% 65%, 100% 100%, 0 100%)",
              }}
            />
          </span>
          <span className="text-[12px] font-semibold tracking-[0.02em] text-fg-0">
            Cellar
          </span>
        </div>
        {!empty && activeTab && (
          <>
            <div className="mx-0.5 h-4 w-px bg-border-default" />
            <div className="flex items-center gap-0.5 max-[1080px]:[&_svg]:hidden">
              <span className="inline-flex items-center gap-[5px] whitespace-nowrap rounded-[4px] px-1.5 py-[3px] text-[11.5px] text-fg-1">
                <Icon.database size={12} />
                <span>{activeConn?.name ?? activeTab.connectionId}</span>
              </span>
              <Icon.chevronRight size={11} style={{ opacity: 0.4 }} />
              <span className="inline-flex items-center gap-[5px] whitespace-nowrap rounded-[4px] px-1.5 py-[3px] text-[11.5px] text-fg-1 max-[1080px]:hidden">
                <span style={{ color: ENGINE_META[activeConn?.engine ?? "postgres"].color }}>●</span>
                <span>{activeTab.database}</span>
              </span>
              <Icon.chevronRight size={11} style={{ opacity: 0.4 }} />
              <span className="inline-flex items-center gap-[5px] whitespace-nowrap rounded-[4px] px-1.5 py-[3px] text-[11.5px] text-fg-1 max-[1080px]:hidden">
                {activeTab.kind === "query" ? (
                  <Icon.terminal size={11} />
                ) : (
                  <Icon.schema size={11} />
                )}
                <span>{activeTab.kind === "query" ? activeTab.title : activeTab.schema}</span>
              </span>
            </div>
          </>
        )}
      </div>

      <button
        type="button"
        onClick={onOpenPalette}
        className="absolute left-1/2 top-1/2 flex h-[24px] min-w-0 w-[320px] max-w-[320px] -translate-x-1/2 -translate-y-1/2 items-center gap-2 rounded-[5px] border border-border-default bg-bg-inset px-2 text-[11.5px] text-fg-3 transition-[border-color] duration-150 hover:border-border-strong"
      >
        <Icon.search size={12} />
        <span className="flex-1 text-left">
          Search tables, columns, queries…
        </span>
        <span className="inline-flex gap-0.5">
          <kbd className="kbd">⌘</kbd>
          <kbd className="kbd">K</kbd>
        </span>
      </button>

      <div className="ml-auto flex shrink-0 items-center gap-1.5">
        {!empty && (
          <>
            <button
              type="button"
              className={"icon-btn" + (panels.left ? " active" : "")}
              onClick={() => onTogglePanel("left")}
              title="Toggle connections panel"
            >
              <Icon.panelLeft size={13} />
            </button>
            <button
              type="button"
              className={"icon-btn" + (panels.bottom ? " active" : "")}
              onClick={() => onTogglePanel("bottom")}
              title="Toggle output panel"
            >
              <Icon.panelBottom size={13} />
            </button>
            <button
              type="button"
              className={"icon-btn" + (panels.right ? " active" : "")}
              onClick={() => onTogglePanel("right")}
              title="Toggle AI panel"
            >
              <Icon.panelRight size={13} />
            </button>
            <div className="mx-0.5 h-4 w-px bg-border-default" />
          </>
        )}
        {onToggleEmpty && (
          <button
            type="button"
            className={"icon-btn" + (empty ? " active" : "")}
            onClick={onToggleEmpty}
            title={empty ? "Show workspace" : "Show empty state"}
          >
            <Icon.layout size={13} />
          </button>
        )}
      </div>
    </div>
  );
}

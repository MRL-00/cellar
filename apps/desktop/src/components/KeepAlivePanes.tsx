import { memo, useRef, type ReactNode } from "react";

import type { WorkspaceTab } from "../state/tabs";

/**
 * Decide which tabs stay mounted after a swap.
 *
 * A tab is mounted the first time it becomes active and stays mounted (hidden)
 * until it is closed. Never-visited tabs are not pre-mounted so opening many
 * tables doesn't fire a query for each of them.
 */
export function nextMountedTabIds(
  previouslyMounted: readonly string[],
  activeId: string | null,
  openIds: readonly string[],
): string[] {
  const open = new Set(openIds);
  const next: string[] = [];
  const seen = new Set<string>();
  for (const id of previouslyMounted) {
    if (!open.has(id) || seen.has(id)) continue;
    seen.add(id);
    next.push(id);
  }
  if (activeId && open.has(activeId) && !seen.has(activeId)) {
    next.push(activeId);
  }
  return next;
}

/**
 * Keep the heavy pane (grid, editor) from re-rendering when some other tab
 * becomes active. Visibility is handled by the wrapper; this only receives
 * the tab and a stable render callback.
 */
const FrozenPane = memo(function FrozenPane({
  tab,
  render,
}: {
  tab: WorkspaceTab;
  render: (tab: WorkspaceTab) => ReactNode;
}) {
  return <>{render(tab)}</>;
});

/**
 * Render visited tabs and hide the inactive ones instead of unmounting them.
 * Table filters, loaded rows, and editor caret survive tab swaps; close still
 * tears the pane down.
 *
 * `children` must be a stable callback (useCallback). An inline function
 * would re-render every frozen pane on each tab click.
 */
export function KeepAlivePanes({
  tabs,
  activeId,
  children,
}: {
  tabs: WorkspaceTab[];
  activeId: string | null;
  children: (tab: WorkspaceTab) => ReactNode;
}) {
  const openIds = tabs.map((tab) => tab.id);
  const mountedRef = useRef<string[]>([]);
  const mountedIds = nextMountedTabIds(
    mountedRef.current,
    activeId,
    openIds,
  );
  mountedRef.current = mountedIds;
  const byId = new Map(tabs.map((tab) => [tab.id, tab]));

  return (
    <div className="relative flex min-h-0 flex-1 flex-col overflow-hidden">
      {mountedIds.map((id) => {
        const tab = byId.get(id);
        if (!tab) return null;
        const active = id === activeId;
        return (
          <div
            key={id}
            className={
              "absolute inset-0 flex flex-col overflow-hidden " +
              (active ? "z-10" : "invisible pointer-events-none z-0")
            }
            aria-hidden={!active}
          >
            <FrozenPane tab={tab} render={children} />
          </div>
        );
      })}
    </div>
  );
}

import { beforeEach, describe, expect, it } from "vitest";

import type { PendingChanges } from "@cellar/data-grid";
import { useNotices } from "./notices";
import { useQueryMessages } from "./queryMessages";
import { tableResultSource, useTabResults } from "./tabResults";
import { useTabs, type QueryTab } from "./tabs";

const source = tableResultSource(
  "conn-1",
  "app",
  "public",
  "orders",
  "SELECT * FROM public.orders LIMIT 100",
  100,
);

const changes: PendingChanges = {
  "row-1": { kind: "update", edits: { name: { from: "old", to: "new" } } },
};

describe("tab workspace state", () => {
  beforeEach(() => {
    useTabs.setState({
      tabs: [],
      activeId: null,
      closedTabs: [],
      split: null,
      tabPane: {},
      paneActive: [null, null],
      focusedPane: 0,
      tableChanges: {},
      tableLayouts: {},
      tableSorts: {},
      tableFilters: {},
      refreshKeys: {},
    });
    if (typeof localStorage !== "undefined") localStorage.clear();
    useTabResults.setState({ byTabId: {} });
    useQueryMessages.setState({ messages: [] });
    useNotices.setState({ byScope: {} });
  });

  it("moves the active tab into the secondary pane when splitting", () => {
    const store = useTabs.getState();
    const first = store.newQueryTab("conn-1", "app");
    const second = useTabs.getState().newQueryTab("conn-1", "app");

    useTabs.getState().splitActiveTab("vertical");

    const s = useTabs.getState();
    expect(s.split).toBe("vertical");
    expect(s.tabPane[second]).toBe(1);
    expect(s.paneActive).toEqual([first, second]);
    expect(s.focusedPane).toBe(1);
    expect(s.activeId).toBe(second);
  });

  it("toggles a split off when the active split button is clicked again", () => {
    const store = useTabs.getState();
    store.newQueryTab("conn-1", "app");
    useTabs.getState().newQueryTab("conn-1", "app");

    useTabs.getState().splitActiveTab("horizontal");
    expect(useTabs.getState().split).toBe("horizontal");

    useTabs.getState().splitActiveTab("horizontal");
    expect(useTabs.getState().split).toBeNull();
    expect(useTabs.getState().tabPane).toEqual({});
  });

  it("focuses the primary pane when selecting one of its tabs", () => {
    const store = useTabs.getState();
    const first = store.newQueryTab("conn-1", "app");
    useTabs.getState().newQueryTab("conn-1", "app");
    const third = useTabs.getState().newQueryTab("conn-1", "app");

    // `third` is active, so it moves to the secondary pane; the rest stay.
    useTabs.getState().splitActiveTab("vertical");
    useTabs.getState().setActive(first);

    const s = useTabs.getState();
    expect(s.split).toBe("vertical");
    expect(s.focusedPane).toBe(0);
    expect(s.activeId).toBe(first);
    expect(s.paneActive).toEqual([first, third]);
  });

  it("collapses the split when a pane is left empty", () => {
    const store = useTabs.getState();
    const first = store.newQueryTab("conn-1", "app");
    const second = useTabs.getState().newQueryTab("conn-1", "app");

    useTabs.getState().splitActiveTab("vertical");
    // `second` is the only tab in the secondary pane — closing it collapses.
    useTabs.getState().closeTab(second);

    const s = useTabs.getState();
    expect(s.split).toBeNull();
    expect(s.activeId).toBe(first);
    expect(s.tabPane).toEqual({});
  });

  it("creates a split by dropping a tab on the right edge", () => {
    const store = useTabs.getState();
    const first = store.newQueryTab("conn-1", "app");
    const second = useTabs.getState().newQueryTab("conn-1", "app");
    const third = useTabs.getState().newQueryTab("conn-1", "app");

    // Drop `first` on the right → vertical split, `first` alone on the right,
    // everything else on the left.
    useTabs.getState().dropTabToSplit(first, "right");

    const s = useTabs.getState();
    expect(s.split).toBe("vertical");
    expect(s.tabPane[first]).toBe(1);
    expect(s.tabPane[second] ?? 0).toBe(0);
    expect(s.tabPane[third] ?? 0).toBe(0);
    expect(s.focusedPane).toBe(1);
    expect(s.activeId).toBe(first);
    expect(s.draggingTabId).toBeNull();
  });

  it("splits below the rest when dropping on the bottom edge", () => {
    const store = useTabs.getState();
    const first = store.newQueryTab("conn-1", "app");
    const second = useTabs.getState().newQueryTab("conn-1", "app");

    useTabs.getState().dropTabToSplit(second, "bottom");

    const s = useTabs.getState();
    expect(s.split).toBe("horizontal");
    expect(s.tabPane[second]).toBe(1);
    expect(s.tabPane[first] ?? 0).toBe(0);
  });

  it("moves a tab across panes", () => {
    const store = useTabs.getState();
    const first = store.newQueryTab("conn-1", "app");
    const second = useTabs.getState().newQueryTab("conn-1", "app");
    const third = useTabs.getState().newQueryTab("conn-1", "app");

    // third → secondary pane; first & second stay in primary.
    useTabs.getState().splitActiveTab("vertical");
    useTabs.getState().moveTabToPane(first, 1);

    const s = useTabs.getState();
    expect(s.tabPane[first]).toBe(1);
    expect(s.tabPane[third]).toBe(1);
    expect(s.tabPane[second] ?? 0).toBe(0);
    expect(s.focusedPane).toBe(1);
    expect(s.activeId).toBe(first);
  });

  it("reopens the most recently closed tab with its query buffer intact", () => {
    const id = useTabs.getState().newQueryTab("conn-1", "app");
    useTabs.getState().setQuerySql(id, "select 1;");

    useTabs.getState().closeTab(id);
    expect(useTabs.getState().tabs).toHaveLength(0);
    expect(useTabs.getState().closedTabs).toHaveLength(1);

    useTabs.getState().reopenClosedTab();

    const reopened = useTabs.getState().tabs[0];
    expect(reopened).toMatchObject({ id, kind: "query", sql: "select 1;" });
    expect(useTabs.getState().activeId).toBe(id);
    expect(useTabs.getState().closedTabs).toHaveLength(0);
  });

  it("closes other tabs and clears their scoped state", () => {
    const store = useTabs.getState();
    store.openTable("conn-1", "app", "public", "orders");
    store.openTable("conn-1", "app", "public", "customers");
    store.openTable("conn-1", "app", "events", "devices");

    const keepId = "conn-1::app.public.customers";
    const closedId = "conn-1::app.public.orders";
    useTabs.getState().setTableChanges(closedId, changes);
    useTabs.getState().refreshTable(closedId);
    useTabResults.getState().setLoading(closedId, source);
    useTabResults.getState().setLoading(keepId, source);

    useTabs.getState().closeOtherTabs(keepId);

    expect(useTabs.getState().tabs.map((t) => t.id)).toEqual([keepId]);
    expect(useTabs.getState().activeId).toBe(keepId);
    expect(useTabs.getState().closedTabs).toHaveLength(2);
    expect(useTabs.getState().split).toBeNull();
    expect(useTabs.getState().tableChanges[closedId]).toBeUndefined();
    expect(useTabs.getState().refreshKeys[closedId]).toBeUndefined();
    expect(useTabResults.getState().byTabId[closedId]).toBeUndefined();
    expect(useTabResults.getState().byTabId[keepId]?.status).toBe("loading");
  });

  it("closes every tab for a removed connection and keeps others intact", () => {
    const store = useTabs.getState();
    store.openTable("conn-1", "app", "public", "orders");
    const goneQuery = useTabs.getState().newQueryTab("conn-1", "app");
    const keepId = useTabs.getState().newQueryTab("conn-2", "analytics");

    const goneTable = "conn-1::app.public.orders";
    useTabs.getState().setTableChanges(goneTable, changes);
    useTabs.getState().refreshTable(goneTable);
    useTabResults.getState().setLoading(goneTable, source);

    useTabs.getState().closeConnectionTabs("conn-1");

    expect(useTabs.getState().tabs.map((t) => t.id)).toEqual([keepId]);
    expect(useTabs.getState().activeId).toBe(keepId);
    expect(
      useTabs.getState().tabs.some((t) => t.id === goneQuery),
    ).toBe(false);
    expect(useTabs.getState().tableChanges[goneTable]).toBeUndefined();
    expect(useTabs.getState().refreshKeys[goneTable]).toBeUndefined();
    expect(useTabResults.getState().byTabId[goneTable]).toBeUndefined();
  });

  it("does not resurrect a removed connection's tab via reopen", () => {
    const id = useTabs.getState().newQueryTab("conn-1", "app");
    useTabs.getState().closeTab(id);
    expect(useTabs.getState().closedTabs).toHaveLength(1);

    useTabs.getState().closeConnectionTabs("conn-1");
    expect(useTabs.getState().closedTabs).toHaveLength(0);

    useTabs.getState().reopenClosedTab();
    expect(useTabs.getState().tabs).toHaveLength(0);
  });

  it("moves focus left when closing tabs to the right of a tab", () => {
    const store = useTabs.getState();
    store.openTable("conn-1", "app", "public", "orders");
    store.openTable("conn-1", "app", "public", "customers");
    store.openTable("conn-1", "app", "events", "devices");

    useTabs.getState().closeTabsToRight("conn-1::app.public.orders");

    expect(useTabs.getState().tabs.map((t) => t.id)).toEqual([
      "conn-1::app.public.orders",
    ]);
    expect(useTabs.getState().activeId).toBe("conn-1::app.public.orders");
    expect(useTabs.getState().closedTabs).toHaveLength(2);
  });

  it("reorders tabs by id", () => {
    const store = useTabs.getState();
    store.openTable("conn-1", "app", "public", "orders");
    store.openTable("conn-1", "app", "public", "customers");
    store.openTable("conn-1", "app", "events", "devices");

    useTabs
      .getState()
      .reorderTab("conn-1::app.events.devices", "conn-1::app.public.customers");

    expect(useTabs.getState().tabs.map((t) => t.id)).toEqual([
      "conn-1::app.public.orders",
      "conn-1::app.events.devices",
      "conn-1::app.public.customers",
    ]);
  });

  it("clears the dirty flag when the buffer reverts to the last-run SQL", () => {
    const id = useTabs.getState().newQueryTab("conn-1", "app");
    const queryTab = () => useTabs.getState().tabs[0] as QueryTab;

    // Fresh tab: type a character, then delete it — back to the baseline.
    useTabs.getState().setQuerySql(id, "x");
    expect(queryTab().dirty).toBe(true);
    useTabs.getState().setQuerySql(id, "");
    expect(queryTab().dirty).toBe(false);

    // After a run, the baseline is the SQL at run time.
    useTabs.getState().setQuerySql(id, "select 1;");
    useTabs.getState().markQueryRun(id);
    expect(queryTab().dirty).toBe(false);

    useTabs.getState().setQuerySql(id, "select 2;");
    expect(queryTab().dirty).toBe(true);
    useTabs.getState().setQuerySql(id, "select 1;");
    expect(queryTab().dirty).toBe(false);
  });

  it("re-points a query tab at a different database without touching the buffer", () => {
    const id = useTabs.getState().newQueryTab("conn-1", "app");
    const queryTab = () => useTabs.getState().tabs[0] as QueryTab;
    useTabs.getState().setQuerySql(id, "select 1;");

    useTabs.getState().setQueryDatabase(id, "analytics");
    expect(queryTab().database).toBe("analytics");
    expect(queryTab().sql).toBe("select 1;");
  });

  it("clears stale results when switching a query tab's database", () => {
    const id = useTabs.getState().newQueryTab("conn-1", "app");
    useTabResults.getState().setReady(id, {
      source,
      columns: [],
      rows: [],
      rowCount: 0,
      truncated: false,
      durationMs: 1,
    });
    expect(useTabResults.getState().byTabId[id]).toBeDefined();

    useTabs.getState().setQueryDatabase(id, "analytics");
    expect(useTabResults.getState().byTabId[id]).toBeUndefined();
  });

  it("leaves results intact when the database is unchanged", () => {
    const id = useTabs.getState().newQueryTab("conn-1", "app");
    useTabResults.getState().setReady(id, {
      source,
      columns: [],
      rows: [],
      rowCount: 0,
      truncated: false,
      durationMs: 1,
    });

    useTabs.getState().setQueryDatabase(id, "app");
    expect(useTabResults.getState().byTabId[id]).toBeDefined();
  });

  it("drops query messages and notices for a tab when it closes", () => {
    const id = useTabs.getState().newQueryTab("conn-1", "app");
    const keepId = useTabs.getState().newQueryTab("conn-1", "app");
    const message = {
      connectionId: "conn-1",
      severity: "info" as const,
      source: "client" as const,
      text: "ran",
    };
    useQueryMessages.getState().addMessage({ ...message, tabId: id });
    useQueryMessages.getState().addMessage({ ...message, tabId: keepId });
    useNotices.getState().appendNotice(
      { tabId: id, connectionId: "conn-1", database: "app" },
      {
        severity: "notice",
        code: "00000",
        message: "hi",
        detail: null,
        hint: null,
        timestamp: "2026-06-11T00:00:00Z",
        connection_id: "conn-1",
        database: "app",
        query_id: "q1",
      },
    );

    useTabs.getState().closeTab(id);

    const tabIds = useQueryMessages.getState().messages.map((m) => m.tabId);
    expect(tabIds).toEqual([keepId]);
    expect(useNotices.getState().byScope[`tab:${id}`]).toBeUndefined();
  });

  it("keeps table layouts after a table tab is closed", () => {
    const store = useTabs.getState();
    store.openTable("conn-1", "app", "public", "orders");

    useTabs.getState().setTableLayout("conn-1::app.public.orders", {
      order: ["status", "id"],
      widths: { status: 140 },
    });
    useTabs.getState().closeTab("conn-1::app.public.orders");

    expect(useTabs.getState().tableLayouts["conn-1::app.public.orders"]).toEqual({
      order: ["status", "id"],
      widths: { status: 140 },
    });
    if (typeof localStorage !== "undefined") {
      expect(localStorage.getItem("cellar.tableLayouts.v1")).toContain("status");
    }
  });

  it("persists the last-used table sort across tab close", () => {
    const id = "conn-1::app.public.orders";
    useTabs.getState().openTable("conn-1", "app", "public", "orders");
    useTabs.getState().setTableSort(id, { columnKey: "created_at", direction: "desc" });
    useTabs.getState().closeTab(id);

    expect(useTabs.getState().tableSorts[id]).toEqual({
      columnKey: "created_at",
      direction: "desc",
    });
    if (typeof localStorage !== "undefined") {
      expect(localStorage.getItem("cellar.tableSorts.v1")).toContain("created_at");
    }

    useTabs.getState().setTableSort(id, null);
    expect(useTabs.getState().tableSorts[id]).toBeUndefined();
  });

  it("keeps active table filters across tab swaps and clears them on close", () => {
    const orders = "conn-1::app.public.orders";
    const users = "conn-1::app.public.users";
    const store = useTabs.getState();
    store.openTable("conn-1", "app", "public", "orders");
    store.openTable("conn-1", "app", "public", "users");

    useTabs.getState().setTableFilters(orders, {
      filters: [
        {
          id: "f1",
          columnKey: "status",
          operator: "equals",
          value: "open",
          logic: "and",
        },
      ],
      quickFilter: "rush",
      quickColumn: "name",
    });

    // Swap away and back — filters must still be in the store for remount.
    useTabs.getState().setActive(users);
    useTabs.getState().setActive(orders);
    expect(useTabs.getState().tableFilters[orders]).toEqual({
      filters: [
        {
          id: "f1",
          columnKey: "status",
          operator: "equals",
          value: "open",
          logic: "and",
        },
      ],
      quickFilter: "rush",
      quickColumn: "name",
    });

    // Empty toolbar drops the entry; close drops any remaining session state.
    useTabs.getState().setTableFilters(orders, {
      filters: [],
      quickFilter: "",
      quickColumn: null,
    });
    expect(useTabs.getState().tableFilters[orders]).toBeUndefined();

    useTabs.getState().setTableFilters(orders, {
      filters: [],
      quickFilter: "kept-until-close",
      quickColumn: null,
    });
    useTabs.getState().closeTab(orders);
    expect(useTabs.getState().tableFilters[orders]).toBeUndefined();
  });
});

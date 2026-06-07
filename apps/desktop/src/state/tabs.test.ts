import { beforeEach, describe, expect, it } from "vitest";

import type { PendingChanges } from "@cellar/data-grid";
import { tableResultSource, useTabResults } from "./tabResults";
import { useTabs } from "./tabs";

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
      tableChanges: {},
      tableLayouts: {},
      refreshKeys: {},
    });
    if (typeof localStorage !== "undefined") localStorage.clear();
    useTabResults.setState({ byTabId: {} });
  });

  it("splits the active tab against a neighboring tab", () => {
    const store = useTabs.getState();
    const first = store.newQueryTab("conn-1", "app");
    const second = useTabs.getState().newQueryTab("conn-1", "app");

    useTabs.getState().splitActiveTab("vertical");

    expect(useTabs.getState().split).toEqual({
      orientation: "vertical",
      primaryId: second,
      secondaryId: first,
    });
  });

  it("toggles a split off when the active split button is clicked again", () => {
    const store = useTabs.getState();
    store.newQueryTab("conn-1", "app");
    useTabs.getState().newQueryTab("conn-1", "app");

    useTabs.getState().splitActiveTab("horizontal");
    expect(useTabs.getState().split?.orientation).toBe("horizontal");

    useTabs.getState().splitActiveTab("horizontal");
    expect(useTabs.getState().split).toBeNull();
  });

  it("keeps a split useful when a different tab is selected", () => {
    const store = useTabs.getState();
    const first = store.newQueryTab("conn-1", "app");
    const second = useTabs.getState().newQueryTab("conn-1", "app");
    useTabs.getState().newQueryTab("conn-1", "app");

    useTabs.getState().splitActiveTab("vertical");
    useTabs.getState().setActive(first);

    expect(useTabs.getState().split).toMatchObject({
      primaryId: first,
      secondaryId: second,
    });
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
});

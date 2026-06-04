import { beforeEach, describe, expect, it } from "vitest";

import { useTabs } from "./tabs";

describe("tab workspace state", () => {
  beforeEach(() => {
    useTabs.setState({
      tabs: [],
      activeId: null,
      closedTabs: [],
      split: null,
      tableChanges: {},
      refreshKeys: {},
    });
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
});

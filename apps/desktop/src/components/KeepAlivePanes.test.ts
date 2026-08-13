import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { QueryTab, WorkspaceTab } from "../state/tabs";
import { KeepAlivePanes, nextMountedTabIds } from "./KeepAlivePanes";

function queryTab(id: string, title: string): QueryTab {
  return {
    id,
    kind: "query",
    connectionId: "conn-1",
    database: "app",
    title,
    sql: "",
    savedSql: "",
    dirty: false,
  };
}

describe("nextMountedTabIds", () => {
  it("mounts the active tab on first visit", () => {
    expect(nextMountedTabIds([], "orders", ["orders", "users"])).toEqual([
      "orders",
    ]);
  });

  it("keeps a visited tab mounted after swapping away", () => {
    expect(
      nextMountedTabIds(["orders"], "users", ["orders", "users"]),
    ).toEqual(["orders", "users"]);
  });

  it("does not pre-mount tabs that have never been active", () => {
    expect(
      nextMountedTabIds(["orders"], "orders", ["orders", "users", "items"]),
    ).toEqual(["orders"]);
  });

  it("drops a tab when it is closed", () => {
    expect(
      nextMountedTabIds(["orders", "users"], "users", ["users"]),
    ).toEqual(["users"]);
  });

  it("mounts a reopened tab fresh after close", () => {
    expect(nextMountedTabIds(["users"], "orders", ["users", "orders"])).toEqual(
      ["users", "orders"],
    );
  });

  it("ignores an active id that is no longer open", () => {
    expect(nextMountedTabIds(["orders"], "gone", ["orders"])).toEqual([
      "orders",
    ]);
  });

  it("returns an empty list when nothing is open", () => {
    expect(nextMountedTabIds(["orders"], null, [])).toEqual([]);
  });
});

describe("KeepAlivePanes", () => {
  it("renders the active tab and leaves never-visited tabs unmounted", () => {
    const orders = queryTab("orders", "orders.sql");
    const users = queryTab("users", "users.sql");
    const html = renderToStaticMarkup(
      createElement(KeepAlivePanes, {
        tabs: [orders, users],
        activeId: "orders",
        children: (tab: WorkspaceTab) =>
          createElement(
            "span",
            null,
            tab.kind === "query" ? tab.title : tab.id,
          ),
      }),
    );
    expect(html).toContain("orders.sql");
    expect(html).not.toContain("users.sql");
    expect(html).not.toContain("invisible");
  });
});

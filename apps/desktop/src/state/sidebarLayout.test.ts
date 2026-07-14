import { describe, expect, it } from "vitest";

import {
  moveConnectionItem,
  moveFolderItem,
  reconcileItems,
  removeFolderItem,
  useSidebarLayout,
  type SidebarItem,
} from "./sidebarLayout";

const conn = (id: string): SidebarItem => ({ kind: "connection", id });
const folder = (
  id: string,
  children: string[],
  extra?: Partial<{ name: string; collapsed: boolean; color: string | null }>,
): SidebarItem => ({
  kind: "folder",
  id,
  name: extra?.name ?? id,
  collapsed: extra?.collapsed ?? false,
  children,
  ...(extra?.color ? { color: extra.color } : {}),
});

describe("reconcileItems", () => {
  it("appends unknown connections at the root", () => {
    const out = reconcileItems([conn("a")], ["a", "b", "c"]);
    expect(out).toEqual([conn("a"), conn("b"), conn("c")]);
  });

  it("prunes deleted connections from the root and from folders", () => {
    const out = reconcileItems(
      [conn("gone"), folder("f", ["kept", "removed"]), conn("kept-root")],
      ["kept", "kept-root"],
    );
    expect(out).toEqual([folder("f", ["kept"]), conn("kept-root")]);
  });

  it("drops duplicate references, keeping the first occurrence", () => {
    const out = reconcileItems(
      [folder("f", ["a"]), conn("a"), conn("b")],
      ["a", "b"],
    );
    expect(out).toEqual([folder("f", ["a"]), conn("b")]);
  });

  it("returns the same reference when nothing changed", () => {
    const items = [folder("f", ["a"]), conn("b")];
    expect(reconcileItems(items, ["a", "b"])).toBe(items);
  });

  it("recovers from malformed persisted data", () => {
    const junk = [
      null,
      42,
      { kind: "folder" },
      { kind: "connection", id: "a" },
      { kind: "folder", id: "f", children: ["b", 7] },
    ] as unknown as SidebarItem[];
    const out = reconcileItems(junk, ["a", "b"]);
    expect(out).toEqual([conn("a"), folder("f", ["b"], { name: "Folder" })]);
  });

  it("keeps valid folder colors and drops invalid ones", () => {
    const junk = [
      {
        kind: "folder",
        id: "f",
        name: "f",
        children: ["a"],
        color: "#4f8ff7",
      },
      { kind: "folder", id: "g", name: "g", children: ["b"], color: "red" },
      { kind: "folder", id: "h", name: "h", children: [], color: "#fff" },
    ] as unknown as SidebarItem[];
    const out = reconcileItems(junk, ["a", "b"]);
    expect(out).toEqual([
      folder("f", ["a"], { color: "#4f8ff7" }),
      folder("g", ["b"]),
      folder("h", []),
    ]);
  });
});

describe("moveConnectionItem", () => {
  const base = [conn("a"), conn("b"), folder("f", ["x", "y"]), conn("c")];

  it("reorders within the root, moving down", () => {
    // Visually dropping "a" after "b" → display index 2.
    const out = moveConnectionItem(base, "a", null, 2);
    expect(out).toEqual([conn("b"), conn("a"), folder("f", ["x", "y"]), conn("c")]);
  });

  it("reorders within the root, moving up", () => {
    const out = moveConnectionItem(base, "c", null, 0);
    expect(out).toEqual([conn("c"), conn("a"), conn("b"), folder("f", ["x", "y"])]);
  });

  it("moves a root connection into a folder at a position", () => {
    const out = moveConnectionItem(base, "a", "f", 1);
    expect(out).toEqual([conn("b"), folder("f", ["x", "a", "y"]), conn("c")]);
  });

  it("moves a folder connection back to the root", () => {
    const out = moveConnectionItem(base, "y", null, 0);
    expect(out).toEqual([
      conn("y"),
      conn("a"),
      conn("b"),
      folder("f", ["x"]),
      conn("c"),
    ]);
  });

  it("reorders within a folder with the same-container adjustment", () => {
    const items = [folder("f", ["x", "y", "z"])];
    const out = moveConnectionItem(items, "x", "f", 2);
    expect(out).toEqual([folder("f", ["y", "x", "z"])]);
  });

  it("ignores moves to a folder that does not exist", () => {
    expect(moveConnectionItem(base, "a", "nope", 0)).toBe(base);
  });

  it("ignores unknown connections", () => {
    expect(moveConnectionItem(base, "ghost", null, 0)).toBe(base);
  });

  it("clamps out-of-range indices", () => {
    const out = moveConnectionItem(base, "a", "f", 99);
    expect(out).toEqual([conn("b"), folder("f", ["x", "y", "a"]), conn("c")]);
  });
});

describe("moveFolderItem", () => {
  const base = [conn("a"), folder("f", []), conn("b"), folder("g", [])];

  it("moves a folder down past later items", () => {
    // Dropping "f" after "b" → display index 3.
    const out = moveFolderItem(base, "f", 3);
    expect(out).toEqual([conn("a"), conn("b"), folder("f", []), folder("g", [])]);
  });

  it("moves a folder to the top", () => {
    const out = moveFolderItem(base, "g", 0);
    expect(out).toEqual([folder("g", []), conn("a"), folder("f", []), conn("b")]);
  });

  it("is a no-op for same position or unknown folder", () => {
    expect(moveFolderItem(base, "f", 1)).toBe(base);
    expect(moveFolderItem(base, "nope", 0)).toBe(base);
  });
});

describe("removeFolderItem", () => {
  it("releases children into the folder's slot", () => {
    const out = removeFolderItem(
      [conn("a"), folder("f", ["x", "y"]), conn("b")],
      "f",
    );
    expect(out).toEqual([conn("a"), conn("x"), conn("y"), conn("b")]);
  });

  it("ignores unknown folders", () => {
    const items = [conn("a")];
    expect(removeFolderItem(items, "nope")).toBe(items);
  });
});

describe("useSidebarLayout store", () => {
  it("creates, renames, toggles, and removes folders", () => {
    useSidebarLayout.setState({ items: [conn("a")] });
    const store = useSidebarLayout.getState();

    const id = store.createFolder("Staging");
    expect(useSidebarLayout.getState().items[0]).toMatchObject({
      kind: "folder",
      id,
      name: "Staging",
      collapsed: false,
      children: [],
    });

    store.moveToFolder("a", id);
    expect(useSidebarLayout.getState().items).toEqual([
      folder(id, ["a"], { name: "Staging" }),
    ]);

    store.renameFolder(id, "  Prod  ");
    expect(useSidebarLayout.getState().items[0]).toMatchObject({
      name: "Prod",
    });
    store.renameFolder(id, "   ");
    expect(useSidebarLayout.getState().items[0]).toMatchObject({
      name: "Prod",
    });

    store.toggleFolder(id);
    expect(useSidebarLayout.getState().items[0]).toMatchObject({
      collapsed: true,
    });

    store.removeFolder(id);
    expect(useSidebarLayout.getState().items).toEqual([conn("a")]);
  });

  it("moves a connection back to the root end via moveToFolder(null)", () => {
    useSidebarLayout.setState({
      items: [folder("f", ["a"]), conn("b")],
    });
    useSidebarLayout.getState().moveToFolder("a", null);
    expect(useSidebarLayout.getState().items).toEqual([
      folder("f", []),
      conn("b"),
      conn("a"),
    ]);
  });

  it("sets and clears folder color accents", () => {
    useSidebarLayout.setState({
      items: [folder("f", ["a"])],
    });
    const store = useSidebarLayout.getState();

    store.setFolderColor("f", "#4ade80");
    expect(useSidebarLayout.getState().items[0]).toMatchObject({
      color: "#4ade80",
    });

    store.setFolderColor("f", "not-a-color");
    expect(useSidebarLayout.getState().items[0]).toMatchObject({
      color: "#4ade80",
    });

    store.setFolderColor("f", null);
    expect(useSidebarLayout.getState().items[0]).toEqual(folder("f", ["a"]));
  });
});

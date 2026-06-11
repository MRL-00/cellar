import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { deferredLocalStorage } from "./deferredStorage";

/**
 * Sidebar presentation order for connections: a flat root sequence of
 * connections and single-level folders. This is UI preference state only —
 * connection configs themselves stay in `~/.cellar/connections.json` and the
 * layout is reconciled against whatever the backend reports.
 */
export type SidebarFolderItem = {
  kind: "folder";
  id: string;
  name: string;
  collapsed: boolean;
  /** Connection ids in display order. */
  children: string[];
};

export type SidebarItem = { kind: "connection"; id: string } | SidebarFolderItem;

/** Folder id, or null for the root list. */
export type SidebarContainer = string | null;

interface SidebarLayoutStore {
  items: SidebarItem[];

  /** Sync the layout with the saved connection list: prune ids that no
   * longer exist (and duplicates), append newcomers at the root. */
  reconcile: (connectionIds: string[]) => void;
  createFolder: (name: string) => string;
  renameFolder: (folderId: string, name: string) => void;
  /** Remove the folder, releasing its connections to the root at its slot. */
  removeFolder: (folderId: string) => void;
  toggleFolder: (folderId: string) => void;
  moveConnection: (
    connectionId: string,
    container: SidebarContainer,
    index: number,
  ) => void;
  moveFolder: (folderId: string, index: number) => void;
  /** Append a connection to a folder (or back to the root for null). */
  moveToFolder: (connectionId: string, folderId: SidebarContainer) => void;
}

function clamp(n: number, min: number, max: number): number {
  return Math.min(Math.max(n, min), max);
}

function isFolder(item: SidebarItem): item is SidebarFolderItem {
  return item.kind === "folder";
}

function sanitizeItems(raw: unknown): SidebarItem[] {
  if (!Array.isArray(raw)) return [];
  const out: SidebarItem[] = [];
  for (const item of raw) {
    if (!item || typeof item !== "object") continue;
    const it = item as {
      kind?: unknown;
      id?: unknown;
      name?: unknown;
      collapsed?: unknown;
      children?: unknown;
    };
    if (typeof it.id !== "string" || it.id.length === 0) continue;
    if (it.kind === "connection") {
      out.push({ kind: "connection", id: it.id });
    } else if (it.kind === "folder") {
      out.push({
        kind: "folder",
        id: it.id,
        name: typeof it.name === "string" && it.name ? it.name : "Folder",
        collapsed: Boolean(it.collapsed),
        children: Array.isArray(it.children)
          ? it.children.filter((c): c is string => typeof c === "string")
          : [],
      });
    }
  }
  return out;
}

export function reconcileItems(
  rawItems: SidebarItem[],
  connectionIds: string[],
): SidebarItem[] {
  const items = sanitizeItems(rawItems);
  const valid = new Set(connectionIds);
  const seen = new Set<string>();
  const out: SidebarItem[] = [];
  let changed = items.length !== rawItems.length;

  for (const item of items) {
    if (item.kind === "connection") {
      if (!valid.has(item.id) || seen.has(item.id)) {
        changed = true;
        continue;
      }
      seen.add(item.id);
      out.push(item);
    } else {
      const children: string[] = [];
      for (const id of item.children) {
        if (!valid.has(id) || seen.has(id)) continue;
        seen.add(id);
        children.push(id);
      }
      if (children.length !== item.children.length) {
        changed = true;
        out.push({ ...item, children });
      } else {
        out.push(item);
      }
    }
  }

  for (const id of connectionIds) {
    if (seen.has(id)) continue;
    out.push({ kind: "connection", id });
    changed = true;
  }

  return changed ? out : rawItems;
}

function locateConnection(
  items: SidebarItem[],
  connectionId: string,
): { container: SidebarContainer; index: number } | null {
  for (let i = 0; i < items.length; i++) {
    const item = items[i];
    if (!item) continue;
    if (item.kind === "connection" && item.id === connectionId) {
      return { container: null, index: i };
    }
    if (item.kind === "folder") {
      const ci = item.children.indexOf(connectionId);
      if (ci >= 0) return { container: item.id, index: ci };
    }
  }
  return null;
}

export function moveConnectionItem(
  items: SidebarItem[],
  connectionId: string,
  container: SidebarContainer,
  index: number,
): SidebarItem[] {
  const from = locateConnection(items, connectionId);
  if (!from) return items;
  if (container !== null && !items.some((it) => isFolder(it) && it.id === container)) {
    return items;
  }

  // The caller's index refers to the list as currently displayed; removing
  // the dragged row first shifts later slots in the same container down one.
  let target = index;
  if (from.container === container && from.index < index) target -= 1;

  const without = items
    .filter((it) => !(it.kind === "connection" && it.id === connectionId))
    .map((it) =>
      isFolder(it) && it.children.includes(connectionId)
        ? { ...it, children: it.children.filter((id) => id !== connectionId) }
        : it,
    );

  if (container === null) {
    const at = clamp(target, 0, without.length);
    return [
      ...without.slice(0, at),
      { kind: "connection", id: connectionId },
      ...without.slice(at),
    ];
  }
  return without.map((it) => {
    if (!isFolder(it) || it.id !== container) return it;
    const children = [...it.children];
    children.splice(clamp(target, 0, children.length), 0, connectionId);
    return { ...it, children };
  });
}

export function moveFolderItem(
  items: SidebarItem[],
  folderId: string,
  index: number,
): SidebarItem[] {
  const fromIndex = items.findIndex((it) => isFolder(it) && it.id === folderId);
  const folder = items[fromIndex];
  if (fromIndex < 0 || !folder) return items;
  const target = clamp(index > fromIndex ? index - 1 : index, 0, items.length - 1);
  if (target === fromIndex) return items;
  const next = items.filter((_, i) => i !== fromIndex);
  next.splice(target, 0, folder);
  return next;
}

export function removeFolderItem(
  items: SidebarItem[],
  folderId: string,
): SidebarItem[] {
  const folder = items.find((it) => isFolder(it) && it.id === folderId);
  if (!folder || !isFolder(folder)) return items;
  return items.flatMap<SidebarItem>((it) =>
    it === folder
      ? folder.children.map((id) => ({ kind: "connection", id }))
      : [it],
  );
}

function newFolderId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return `folder:${crypto.randomUUID()}`;
  }
  return `folder:${Math.random().toString(36).slice(2)}${Date.now().toString(36)}`;
}

export const useSidebarLayout = create<SidebarLayoutStore>()(
  persist(
    (set, get) => ({
      items: [],

      reconcile(connectionIds) {
        const next = reconcileItems(get().items, connectionIds);
        if (next !== get().items) set({ items: next });
      },

      createFolder(name) {
        const id = newFolderId();
        set((s) => ({
          items: [
            { kind: "folder", id, name, collapsed: false, children: [] },
            ...s.items,
          ],
        }));
        return id;
      },

      renameFolder(folderId, name) {
        const trimmed = name.trim();
        if (!trimmed) return;
        set((s) => ({
          items: s.items.map((it) =>
            isFolder(it) && it.id === folderId ? { ...it, name: trimmed } : it,
          ),
        }));
      },

      removeFolder(folderId) {
        set((s) => ({ items: removeFolderItem(s.items, folderId) }));
      },

      toggleFolder(folderId) {
        set((s) => ({
          items: s.items.map((it) =>
            isFolder(it) && it.id === folderId
              ? { ...it, collapsed: !it.collapsed }
              : it,
          ),
        }));
      },

      moveConnection(connectionId, container, index) {
        set((s) => ({
          items: moveConnectionItem(s.items, connectionId, container, index),
        }));
      },

      moveFolder(folderId, index) {
        set((s) => ({ items: moveFolderItem(s.items, folderId, index) }));
      },

      moveToFolder(connectionId, folderId) {
        set((s) => {
          const size =
            folderId === null
              ? s.items.length
              : (s.items.find((it) => isFolder(it) && it.id === folderId) as
                  | SidebarFolderItem
                  | undefined)?.children.length ?? 0;
          return {
            items: moveConnectionItem(s.items, connectionId, folderId, size),
          };
        });
      },
    }),
    {
      name: "cellar.sidebarLayout.v1",
      storage: createJSONStorage(deferredLocalStorage),
      partialize: (s) => ({ items: s.items }),
    },
  ),
);

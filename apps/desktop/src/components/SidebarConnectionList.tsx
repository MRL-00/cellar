import { Fragment, useRef, useState, type ReactNode } from "react";
import type { ConnectionConfig } from "@cellar/ipc";

import { Icon } from "./icons";
import {
  ICON_SLOT,
  META,
  ROW_BASE,
  TWISTY,
  type ConnectionDragHandles,
} from "./SidebarTree";
import type {
  SidebarContainer,
  SidebarFolderItem,
  SidebarItem,
} from "../state/sidebarLayout";

type DragItem = { type: "connection" | "folder"; id: string };

type DropSpot = {
  container: SidebarContainer;
  index: number;
  anchorKey: string;
  edge: "before" | "after" | "into";
};

const END_KEY = "__end__";

export interface SidebarConnectionListProps {
  items: SidebarItem[];
  configs: Map<string, ConnectionConfig>;
  filter: string;
  renamingFolderId: string | null;
  onCommitRename: (folderId: string, name: string) => void;
  onCancelRename: () => void;
  onToggleFolder: (folderId: string) => void;
  onFolderContextMenu: (e: React.MouseEvent, folder: SidebarFolderItem) => void;
  onMoveConnection: (
    connectionId: string,
    container: SidebarContainer,
    index: number,
  ) => void;
  onMoveFolder: (folderId: string, index: number) => void;
  renderConnection: (
    config: ConnectionConfig,
    drag: ConnectionDragHandles | undefined,
  ) => ReactNode;
}

/**
 * Orders the sidebar's connections according to the persisted layout: a root
 * sequence of connection rows and single-level folders, all reorderable via
 * native drag and drop. Dragging is disabled while a filter is active since
 * drop positions would be ambiguous against the full list.
 */
export function SidebarConnectionList({
  items,
  configs,
  filter,
  renamingFolderId,
  onCommitRename,
  onCancelRename,
  onToggleFolder,
  onFolderContextMenu,
  onMoveConnection,
  onMoveFolder,
  renderConnection,
}: SidebarConnectionListProps) {
  const [drag, setDrag] = useState<DragItem | null>(null);
  const [drop, setDrop] = useState<DropSpot | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);

  const q = filter.trim().toLowerCase();
  const filtering = q.length > 0;
  const canDrag = !filtering;
  const matches = (config: ConnectionConfig) =>
    config.name.toLowerCase().includes(q);

  const updateDrop = (next: DropSpot) =>
    setDrop((cur) =>
      cur &&
      cur.container === next.container &&
      cur.index === next.index &&
      cur.anchorKey === next.anchorKey &&
      cur.edge === next.edge
        ? cur
        : next,
    );

  const clearDragState = () => {
    setDrag(null);
    setDrop(null);
  };

  const commitDrop = (e: React.DragEvent) => {
    e.preventDefault();
    const dragged = drag;
    const spot = drop;
    clearDragState();
    if (!dragged || !spot) return;
    if (dragged.type === "connection") {
      onMoveConnection(dragged.id, spot.container, spot.index);
    } else if (spot.container === null) {
      onMoveFolder(dragged.id, spot.index);
    }
  };

  const connectionDrag = (
    id: string,
    container: SidebarContainer,
    index: number,
    key: string,
  ): ConnectionDragHandles | undefined => {
    if (!canDrag) return undefined;
    return {
      draggable: true,
      onDragStart: (e) => {
        e.dataTransfer.effectAllowed = "move";
        // WebKit needs data set for the drag to start at all.
        e.dataTransfer.setData("text/plain", id);
        setDrag({ type: "connection", id });
      },
      onDragOver: (e) => {
        if (!drag) return;
        if (drag.type === "folder" && container !== null) return;
        e.preventDefault();
        e.stopPropagation();
        if (drag.id === id) {
          setDrop(null);
          return;
        }
        e.dataTransfer.dropEffect = "move";
        const rect = e.currentTarget.getBoundingClientRect();
        const before = e.clientY < rect.top + rect.height / 2;
        updateDrop({
          container,
          index: before ? index : index + 1,
          anchorKey: key,
          edge: before ? "before" : "after",
        });
      },
      onDragEnd: clearDragState,
      dropIndicator:
        drop && drop.anchorKey === key && drop.edge !== "into"
          ? drop.edge
          : null,
    };
  };

  const onFolderDragOver = (
    e: React.DragEvent,
    folder: SidebarFolderItem,
    rootIndex: number,
    key: string,
  ) => {
    if (!drag || !canDrag) return;
    e.preventDefault();
    e.stopPropagation();
    if (drag.type === "folder" && drag.id === folder.id) {
      setDrop(null);
      return;
    }
    e.dataTransfer.dropEffect = "move";
    const rect = e.currentTarget.getBoundingClientRect();
    const y = e.clientY - rect.top;
    if (drag.type === "folder") {
      const before = y < rect.height / 2;
      updateDrop({
        container: null,
        index: before ? rootIndex : rootIndex + 1,
        anchorKey: key,
        edge: before ? "before" : "after",
      });
      return;
    }
    if (y < rect.height * 0.25) {
      updateDrop({
        container: null,
        index: rootIndex,
        anchorKey: key,
        edge: "before",
      });
    } else if (folder.collapsed && y > rect.height * 0.75) {
      updateDrop({
        container: null,
        index: rootIndex + 1,
        anchorKey: key,
        edge: "after",
      });
    } else {
      // Expanded folders insert at the top so the drop lands in view;
      // collapsed folders append.
      updateDrop({
        container: folder.id,
        index: folder.collapsed ? folder.children.length : 0,
        anchorKey: key,
        edge: "into",
      });
    }
  };

  // WebKit only allows a drop when dragenter is cancelled too, not just
  // dragover; one bubbling-phase handler here covers every row.
  const onListDragEnter = (e: React.DragEvent) => {
    if (!drag || !canDrag) return;
    e.preventDefault();
  };

  const onListDragOver = (e: React.DragEvent) => {
    if (!drag || !canDrag) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    updateDrop({
      container: null,
      index: items.length,
      anchorKey: END_KEY,
      edge: "after",
    });
  };

  const onListDragLeave = (e: React.DragEvent) => {
    const related = e.relatedTarget as Node | null;
    if (!related || !rootRef.current?.contains(related)) setDrop(null);
  };

  const rows: ReactNode[] = [];
  items.forEach((item, rootIndex) => {
    if (item.kind === "connection") {
      const config = configs.get(item.id);
      if (!config || (filtering && !matches(config))) return;
      const key = `conn:${item.id}`;
      rows.push(
        <Fragment key={key}>
          {renderConnection(config, connectionDrag(item.id, null, rootIndex, key))}
        </Fragment>,
      );
      return;
    }

    const childConfigs = item.children
      .map((id) => configs.get(id))
      .filter((c): c is ConnectionConfig => c != null);
    const visibleChildren = filtering ? childConfigs.filter(matches) : childConfigs;
    if (filtering && visibleChildren.length === 0) return;
    const expanded = filtering || !item.collapsed;
    const key = `folder:${item.id}`;
    const isDropInto =
      drop?.edge === "into" &&
      drop.anchorKey !== END_KEY &&
      drop.container === item.id;

    rows.push(
      <FolderRow
        key={key}
        folder={item}
        expanded={expanded}
        count={visibleChildren.length}
        renaming={renamingFolderId === item.id}
        draggable={canDrag && renamingFolderId !== item.id}
        dropInto={isDropInto}
        dropIndicator={
          drop && drop.anchorKey === key && drop.edge !== "into"
            ? drop.edge
            : null
        }
        onToggle={() => onToggleFolder(item.id)}
        onContextMenu={(e) => onFolderContextMenu(e, item)}
        onCommitRename={(name) => onCommitRename(item.id, name)}
        onCancelRename={onCancelRename}
        onDragStart={(e) => {
          e.dataTransfer.effectAllowed = "move";
          e.dataTransfer.setData("text/plain", item.id);
          setDrag({ type: "folder", id: item.id });
        }}
        onDragOver={(e) => onFolderDragOver(e, item, rootIndex, key)}
        onDragEnd={clearDragState}
      />,
    );

    if (expanded) {
      rows.push(
        <div
          key={`children:${item.id}`}
          className="ml-[13px] border-l border-border-default"
        >
          {visibleChildren.length === 0 ? (
            <div
              className={
                "mx-2 my-0.5 rounded-[4px] border border-dashed px-2 py-1 text-[10.5px] " +
                (isDropInto
                  ? "border-accent-line bg-accent-soft text-accent"
                  : "border-border-default text-fg-3")
              }
              onDragOver={(e) => {
                if (!drag || drag.type !== "connection") return;
                e.preventDefault();
                e.stopPropagation();
                updateDrop({
                  container: item.id,
                  index: 0,
                  anchorKey: key,
                  edge: "into",
                });
              }}
            >
              {drag ? "drop here" : "empty folder"}
            </div>
          ) : (
            visibleChildren.map((config) => {
              const childKey = `conn:${config.id}`;
              return (
                <Fragment key={childKey}>
                  {renderConnection(
                    config,
                    connectionDrag(
                      config.id,
                      item.id,
                      item.children.indexOf(config.id),
                      childKey,
                    ),
                  )}
                </Fragment>
              );
            })
          )}
        </div>,
      );
    }
  });

  return (
    <div
      ref={rootRef}
      className="flex-1"
      onDragEnter={onListDragEnter}
      onDragOver={onListDragOver}
      onDragLeave={onListDragLeave}
      onDrop={commitDrop}
    >
      {rows}
      {drop?.anchorKey === END_KEY && (
        <div
          className="mx-2 h-[2px] rounded"
          style={{ background: "var(--accent)" }}
        />
      )}
    </div>
  );
}

function FolderRow({
  folder,
  expanded,
  count,
  renaming,
  draggable,
  dropInto,
  dropIndicator,
  onToggle,
  onContextMenu,
  onCommitRename,
  onCancelRename,
  onDragStart,
  onDragOver,
  onDragEnd,
}: {
  folder: SidebarFolderItem;
  expanded: boolean;
  count: number;
  renaming: boolean;
  draggable: boolean;
  dropInto: boolean;
  dropIndicator: "before" | "after" | null;
  onToggle: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
  onCommitRename: (name: string) => void;
  onCancelRename: () => void;
  onDragStart: React.DragEventHandler;
  onDragOver: React.DragEventHandler;
  onDragEnd: () => void;
}) {
  return (
    <div
      className={
        ROW_BASE +
        " h-[26px] pl-1 font-medium text-fg-0 cursor-pointer" +
        (dropInto ? " bg-accent-soft" : "")
      }
      onClick={() => !renaming && onToggle()}
      onContextMenu={onContextMenu}
      draggable={draggable}
      onDragStart={onDragStart}
      onDragOver={onDragOver}
      onDragEnd={onDragEnd}
    >
      {dropIndicator && (
        <span
          className={
            "pointer-events-none absolute inset-x-0 z-10 h-[2px] rounded " +
            (dropIndicator === "before" ? "top-0" : "bottom-0")
          }
          style={{ background: "var(--accent)" }}
        />
      )}
      <button
        type="button"
        className={TWISTY}
        onClick={(e) => {
          e.stopPropagation();
          onToggle();
        }}
        aria-label={expanded ? "Collapse folder" : "Expand folder"}
      >
        {expanded ? (
          <Icon.chevronDown size={10} />
        ) : (
          <Icon.chevronRight size={10} />
        )}
      </button>
      <span className={ICON_SLOT} style={{ color: "var(--fg-2)" }}>
        {expanded ? <Icon.folderOpen size={12} /> : <Icon.folder size={12} />}
      </span>
      {renaming ? (
        <FolderRenameInput
          initial={folder.name}
          onCommit={onCommitRename}
          onCancel={onCancelRename}
        />
      ) : (
        <span className="flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-[12px] font-medium">
          {folder.name}
        </span>
      )}
      <span className={META + " font-mono"}>{count}</span>
      <button
        type="button"
        className="icon-btn ml-1 opacity-0 transition-opacity duration-100 group-hover:opacity-100"
        title="Folder actions"
        onClick={(e) => {
          e.stopPropagation();
          onContextMenu(e);
        }}
      >
        <Icon.more size={11} />
      </button>
    </div>
  );
}

/** Mounted only while a rename is active so each session starts fresh. */
function FolderRenameInput({
  initial,
  onCommit,
  onCancel,
}: {
  initial: string;
  onCommit: (name: string) => void;
  onCancel: () => void;
}) {
  const done = useRef(false);
  const commit = (value: string) => {
    if (done.current) return;
    done.current = true;
    onCommit(value);
  };

  return (
    <input
      autoFocus
      defaultValue={initial}
      onFocus={(e) => e.currentTarget.select()}
      onClick={(e) => e.stopPropagation()}
      onKeyDown={(e) => {
        if (e.key === "Enter") commit(e.currentTarget.value);
        if (e.key === "Escape") {
          done.current = true;
          onCancel();
        }
      }}
      onBlur={(e) => commit(e.currentTarget.value)}
      className="min-w-0 flex-1 rounded-[3px] border border-accent-line bg-bg-inset px-1 py-px text-[12px] font-medium text-fg-0 outline-none"
    />
  );
}

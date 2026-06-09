import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { isTauri, type ConnectionConfig } from "@cellar/ipc";
import { TitleBar } from "./components/TitleBar";
import { StatusBar } from "./components/StatusBar";
import { Sidebar } from "./components/Sidebar";
import { TabBar } from "./components/TabBar";
import { Workspace } from "./components/Workspace";
import { BottomPanel } from "./components/BottomPanel";
import { AIPanel } from "./components/AIPanel";
import { ConnectionDialog } from "./components/modals/ConnectionDialog";
import { CommitModal } from "./components/modals/CommitModal";
import { CommandPalette } from "./components/modals/CommandPalette";
import { EmptyState } from "./components/modals/EmptyState";
import {
  SettingsModal,
  type SettingsCatId,
} from "./components/modals/Settings";
import { ExportSetupModal } from "./components/modals/ExportSetupModal";
import { ImportSetupModal } from "./components/modals/ImportSetupModal";
import { useLayout } from "./state/layout";

type ModalId =
  | "commit"
  | "palette"
  | "settings"
  | "exportSetup"
  | "importSetup"
  | null;
type ConnDialog = { mode: "new" | "edit"; initial?: ConnectionConfig } | null;

const LEFT_MIN = 200;
const LEFT_MAX = 600;
const RIGHT_MIN = 280;
const RIGHT_MAX = 720;
const BOTTOM_MIN = 140;

function startResize(
  axis: "x" | "y",
  startValue: number,
  setValue: (v: number) => void,
  sign: 1 | -1,
  min: number,
  max: number,
) {
  return (e: React.MouseEvent) => {
    e.preventDefault();
    const start = axis === "x" ? e.clientX : e.clientY;
    const onMove = (ev: MouseEvent) => {
      const cur = axis === "x" ? ev.clientX : ev.clientY;
      const next = Math.max(min, Math.min(max, startValue + (cur - start) * sign));
      setValue(next);
    };
    const onUp = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
    document.body.style.cursor = axis === "x" ? "col-resize" : "row-resize";
    document.body.style.userSelect = "none";
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  };
}

function ResizeHandle({
  axis,
  onMouseDown,
}: {
  axis: "x" | "y";
  onMouseDown: (e: React.MouseEvent) => void;
}) {
  const vertical = axis === "x";
  return (
    <div
      role="separator"
      aria-orientation={vertical ? "vertical" : "horizontal"}
      onMouseDown={onMouseDown}
      className={
        "group relative z-10 shrink-0 " +
        (vertical
          ? "w-[7px] -mx-[3px] cursor-col-resize"
          : "h-[7px] -my-[3px] cursor-row-resize")
      }
    >
      <div
        className={
          "absolute bg-border-default transition-colors duration-100 group-hover:bg-accent-line group-active:bg-accent " +
          (vertical
            ? "inset-y-0 left-1/2 w-px -translate-x-1/2"
            : "inset-x-0 top-1/2 h-px -translate-y-1/2")
        }
      />
    </div>
  );
}

export function App() {
  const panels = useLayout((s) => s.panels);
  const togglePanel = useLayout((s) => s.togglePanel);
  const leftWidth = useLayout((s) => s.leftWidth);
  const setLeftWidth = useLayout((s) => s.setLeftWidth);
  const rightWidth = useLayout((s) => s.rightWidth);
  const setRightWidth = useLayout((s) => s.setRightWidth);
  const bottomHeight = useLayout((s) => s.bottomHeight);
  const setBottomHeight = useLayout((s) => s.setBottomHeight);
  const [modal, setModal] = useState<ModalId>(null);
  const [connDialog, setConnDialog] = useState<ConnDialog>(null);
  const [empty, setEmpty] = useState(false);
  const [settingsInitialCat, setSettingsInitialCat] =
    useState<SettingsCatId>("appearance");

  const openModal = useCallback((m: ModalId) => setModal(m), []);
  const closeModal = useCallback(() => setModal(null), []);
  const openSettings = useCallback((initialCat: SettingsCatId = "appearance") => {
    setSettingsInitialCat(initialCat);
    setModal("settings");
  }, []);
  const openNewConnection = useCallback(() => setConnDialog({ mode: "new" }), []);
  const editConnection = useCallback(
    (initial: ConnectionConfig) => setConnDialog({ mode: "edit", initial }),
    [],
  );
  const duplicateConnection = useCallback(
    (source: ConnectionConfig) =>
      setConnDialog({
        mode: "new",
        initial: { ...source, id: "", name: `${source.name} copy` },
      }),
    [],
  );

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setModal((m) => (m === "palette" ? null : "palette"));
        return;
      }
      if (mod && e.key.toLowerCase() === "n" && !e.shiftKey) {
        e.preventDefault();
        setConnDialog((d) => (d ? null : { mode: "new" }));
        return;
      }
      if (mod && e.key.toLowerCase() === "s") {
        e.preventDefault();
        setModal("commit");
        return;
      }
      if (mod && e.key === ",") {
        e.preventDefault();
        setModal((m) => (m === "settings" ? null : "settings"));
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // The native macOS app menu's "Settings…" item (see src-tauri/src/menu.rs)
  // emits this event; open the same modal as the in-app ⌘, shortcut.
  useEffect(() => {
    if (!isTauri) return;
    const unlisten = listen("menu://settings", () => openSettings());
    return () => void unlisten.then((off) => off());
  }, [openSettings]);

  return (
    <div className="flex h-full w-full flex-col bg-bg-0">
      <TitleBar
        panels={panels}
        onTogglePanel={togglePanel}
        empty={empty}
        onToggleEmpty={() => setEmpty((v) => !v)}
        onOpenPalette={() => openModal("palette")}
      />

      <div className="flex flex-1 min-h-0">
        {!empty && panels.left && (
          <>
            <div
              className="flex min-w-0 flex-col bg-bg-1"
              style={{ width: leftWidth }}
            >
              <Sidebar
                onNewConnection={openNewConnection}
                onEditConnection={editConnection}
                onDuplicateConnection={duplicateConnection}
                onOpenSettings={() => openSettings()}
              />
            </div>
            <ResizeHandle
              axis="x"
              onMouseDown={startResize(
                "x",
                leftWidth,
                setLeftWidth,
                1,
                LEFT_MIN,
                LEFT_MAX,
              )}
            />
          </>
        )}

        <div className="flex flex-1 min-w-0 flex-col bg-bg-0">
          {empty ? (
            <EmptyState onNew={openNewConnection} />
          ) : (
            <>
              <TabBar />
              <Workspace onCommit={() => openModal("commit")} />
              {panels.bottom && (
                <>
                  <ResizeHandle
                    axis="y"
                    onMouseDown={startResize(
                      "y",
                      bottomHeight,
                      setBottomHeight,
                      -1,
                      BOTTOM_MIN,
                      Math.max(BOTTOM_MIN, Math.round(window.innerHeight * 0.7)),
                    )}
                  />
                  <div
                    className="flex flex-col bg-bg-1"
                    style={{ height: bottomHeight }}
                  >
                    <BottomPanel onClose={() => togglePanel("bottom")} />
                  </div>
                </>
              )}
            </>
          )}
        </div>

        {!empty && panels.right && (
          <>
            <ResizeHandle
              axis="x"
              onMouseDown={startResize(
                "x",
                rightWidth,
                setRightWidth,
                -1,
                RIGHT_MIN,
                RIGHT_MAX,
              )}
            />
            <div
              className="flex min-w-0 flex-col bg-bg-1"
              style={{ width: rightWidth }}
            >
              <AIPanel
                onClose={() => togglePanel("right")}
                onOpenSettings={() => openSettings("ai")}
              />
            </div>
          </>
        )}
      </div>

      <StatusBar />

      {connDialog && (
        <ConnectionDialog
          onClose={() => setConnDialog(null)}
          mode={connDialog.mode}
          initial={connDialog.initial}
        />
      )}
      {modal === "commit" && <CommitModal onClose={closeModal} />}
      {modal === "palette" && (
        <CommandPalette
          panels={panels}
          onClose={closeModal}
          onNewConnection={openNewConnection}
          onOpenCommit={() => openModal("commit")}
          onOpenSettings={() => openSettings()}
          onTogglePanel={togglePanel}
          onExportSetup={() => openModal("exportSetup")}
          onImportSetup={() => openModal("importSetup")}
        />
      )}
      {modal === "settings" && (
        <SettingsModal
          onClose={closeModal}
          initialCat={settingsInitialCat}
          onExportSetup={() => openModal("exportSetup")}
          onImportSetup={() => openModal("importSetup")}
        />
      )}
      {modal === "exportSetup" && <ExportSetupModal onClose={closeModal} />}
      {modal === "importSetup" && <ImportSetupModal onClose={closeModal} />}
    </div>
  );
}

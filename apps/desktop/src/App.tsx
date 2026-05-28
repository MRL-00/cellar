import { useCallback, useEffect, useState } from "react";
import type { ConnectionConfig } from "@cellar/ipc";
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
import { SettingsModal } from "./components/modals/Settings";

type Panels = { left: boolean; right: boolean; bottom: boolean };
type ModalId = "commit" | "palette" | "settings" | null;
type ConnDialog = { mode: "new" | "edit"; initial?: ConnectionConfig } | null;

export function App() {
  const [panels, setPanels] = useState<Panels>({
    left: true,
    right: true,
    bottom: true,
  });
  const [modal, setModal] = useState<ModalId>(null);
  const [connDialog, setConnDialog] = useState<ConnDialog>(null);
  const [empty, setEmpty] = useState(false);

  const togglePanel = useCallback(
    (k: keyof Panels) => setPanels((p) => ({ ...p, [k]: !p[k] })),
    [],
  );

  const openModal = useCallback((m: ModalId) => setModal(m), []);
  const closeModal = useCallback(() => setModal(null), []);
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

  return (
    <div className="flex h-full w-full flex-col bg-bg-0">
      <TitleBar
        panels={panels}
        onTogglePanel={togglePanel}
        empty={empty}
        onToggleEmpty={() => setEmpty((v) => !v)}
        onOpenPalette={() => openModal("palette")}
        onOpenSettings={() => openModal("settings")}
      />

      <div className="flex flex-1 min-h-0">
        {!empty && panels.left && (
          <div
            className="flex min-w-0 flex-col border-r border-border-default bg-bg-1"
            style={{ width: 256 }}
          >
            <Sidebar
              onNewConnection={openNewConnection}
              onEditConnection={editConnection}
              onDuplicateConnection={duplicateConnection}
            />
          </div>
        )}

        <div className="flex flex-1 min-w-0 flex-col bg-bg-0">
          {empty ? (
            <EmptyState onNew={openNewConnection} />
          ) : (
            <>
              <TabBar />
              <Workspace onCommit={() => openModal("commit")} />
              {panels.bottom && (
                <div className="flex h-[280px] flex-col border-t border-border-default bg-bg-1">
                  <BottomPanel onClose={() => togglePanel("bottom")} />
                </div>
              )}
            </>
          )}
        </div>

        {!empty && panels.right && (
          <div
            className="flex min-w-0 flex-col border-l border-border-default bg-bg-1"
            style={{ width: 380 }}
          >
            <AIPanel onClose={() => togglePanel("right")} />
          </div>
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
      {modal === "palette" && <CommandPalette onClose={closeModal} />}
      {modal === "settings" && <SettingsModal onClose={closeModal} />}
    </div>
  );
}

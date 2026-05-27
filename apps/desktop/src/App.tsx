import { useCallback, useEffect, useState } from "react";
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

type Panels = { left: boolean; right: boolean; bottom: boolean };
type ModalId = "connection" | "commit" | "palette" | null;

export function App() {
  const [panels, setPanels] = useState<Panels>({
    left: true,
    right: true,
    bottom: true,
  });
  const [modal, setModal] = useState<ModalId>(null);
  const [empty, setEmpty] = useState(false);

  const togglePanel = useCallback(
    (k: keyof Panels) => setPanels((p) => ({ ...p, [k]: !p[k] })),
    [],
  );

  const openModal = useCallback((m: ModalId) => setModal(m), []);
  const closeModal = useCallback(() => setModal(null), []);

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
        setModal("connection");
        return;
      }
      if (mod && e.key.toLowerCase() === "s") {
        e.preventDefault();
        setModal("commit");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <div className="flex h-screen w-screen flex-col bg-bg-0">
      <TitleBar
        panels={panels}
        onTogglePanel={togglePanel}
        empty={empty}
        onToggleEmpty={() => setEmpty((v) => !v)}
        onOpenPalette={() => openModal("palette")}
      />

      <div className="flex flex-1 min-h-0">
        {!empty && panels.left && (
          <div
            className="flex min-w-0 flex-col border-r border-border-default bg-bg-1"
            style={{ width: 256 }}
          >
            <Sidebar onNewConnection={() => openModal("connection")} />
          </div>
        )}

        <div className="flex flex-1 min-w-0 flex-col bg-bg-0">
          {empty ? (
            <EmptyState onNew={() => openModal("connection")} />
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

      {modal === "connection" && <ConnectionDialog onClose={closeModal} />}
      {modal === "commit" && <CommitModal onClose={closeModal} />}
      {modal === "palette" && <CommandPalette onClose={closeModal} />}
    </div>
  );
}

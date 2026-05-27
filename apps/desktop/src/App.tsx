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
    <div className={"cellar-app" + (empty ? " cellar-app-empty" : "")}>
      <TitleBar
        panels={panels}
        onTogglePanel={togglePanel}
        empty={empty}
        onToggleEmpty={() => setEmpty((v) => !v)}
        onOpenPalette={() => openModal("palette")}
      />

      <div className="cellar-main">
        {!empty && panels.left && (
          <div className="cellar-pane cellar-pane-left" style={{ width: 256 }}>
            <Sidebar onNewConnection={() => openModal("connection")} />
          </div>
        )}

        <div className="cellar-center">
          {empty ? (
            <EmptyState onNew={() => openModal("connection")} />
          ) : (
            <>
              <TabBar />
              <Workspace onCommit={() => openModal("commit")} />
              {panels.bottom && (
                <div className="cellar-bottom">
                  <BottomPanel onClose={() => togglePanel("bottom")} />
                </div>
              )}
            </>
          )}
        </div>

        {!empty && panels.right && (
          <div className="cellar-pane cellar-pane-right" style={{ width: 380 }}>
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

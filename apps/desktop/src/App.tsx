import { useState } from "react";
import { TitleBar } from "./components/TitleBar";
import { StatusBar } from "./components/StatusBar";
import { Sidebar } from "./components/Sidebar";
import { TabBar } from "./components/TabBar";
import { Workspace } from "./components/Workspace";
import { BottomPanel } from "./components/BottomPanel";
import { AIPanel } from "./components/AIPanel";

type Panels = { left: boolean; right: boolean; bottom: boolean };

export function App() {
  const [panels, setPanels] = useState<Panels>({
    left: true,
    right: true,
    bottom: true,
  });

  const togglePanel = (k: keyof Panels) =>
    setPanels((p) => ({ ...p, [k]: !p[k] }));

  return (
    <div className="cellar-app">
      <TitleBar panels={panels} onTogglePanel={togglePanel} />

      <div className="cellar-main">
        {panels.left && (
          <div className="cellar-pane cellar-pane-left" style={{ width: 256 }}>
            <Sidebar />
          </div>
        )}

        <div className="cellar-center">
          <TabBar />
          <Workspace />
          {panels.bottom && (
            <div className="cellar-bottom">
              <BottomPanel onClose={() => togglePanel("bottom")} />
            </div>
          )}
        </div>

        {panels.right && (
          <div className="cellar-pane cellar-pane-right" style={{ width: 380 }}>
            <AIPanel onClose={() => togglePanel("right")} />
          </div>
        )}
      </div>

      <StatusBar />
    </div>
  );
}

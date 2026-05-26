import { Icon } from "./icons";

type Panels = { left: boolean; right: boolean; bottom: boolean };

export function TitleBar({
  panels,
  onTogglePanel,
}: {
  panels: Panels;
  onTogglePanel: (k: keyof Panels) => void;
}) {
  return (
    <div className="cellar-titlebar">
      <div className="cellar-titlebar-left">
        <div className="cellar-traffic">
          <span style={{ background: "#ed6a5e" }} />
          <span style={{ background: "#f5bf4f" }} />
          <span style={{ background: "#61c554" }} />
        </div>
        <div className="cellar-brand">
          <span className="cellar-brand-mark" />
          <span className="cellar-brand-name">Cellar</span>
        </div>
        <div className="cellar-titlebar-divider" />
        <div className="cellar-breadcrumbs">
          <button className="cellar-bc">
            <Icon.database size={12} />
            <span>shop-eu (prod)</span>
          </button>
          <Icon.chevronRight size={11} style={{ opacity: 0.4 }} />
          <button className="cellar-bc">
            <span style={{ color: "var(--eng-postgres)" }}>●</span>
            <span>shop_eu</span>
          </button>
          <Icon.chevronRight size={11} style={{ opacity: 0.4 }} />
          <button className="cellar-bc">
            <Icon.schema size={11} />
            <span>public</span>
          </button>
        </div>
      </div>

      <button className="cellar-cmdk">
        <Icon.search size={11} />
        <span className="cellar-cmdk-text">Search tables, columns, queries…</span>
        <span className="cellar-cmdk-kbd">
          <kbd className="kbd">⌘</kbd>
          <kbd className="kbd">K</kbd>
        </span>
      </button>

      <div className="cellar-titlebar-right">
        <button
          className={"icon-btn" + (panels.left ? " active" : "")}
          onClick={() => onTogglePanel("left")}
          title="Toggle connections panel"
        >
          <Icon.panelLeft size={13} />
        </button>
        <button
          className={"icon-btn" + (panels.bottom ? " active" : "")}
          onClick={() => onTogglePanel("bottom")}
          title="Toggle output panel"
        >
          <Icon.panelBottom size={13} />
        </button>
        <button
          className={"icon-btn" + (panels.right ? " active" : "")}
          onClick={() => onTogglePanel("right")}
          title="Toggle AI panel"
        >
          <Icon.panelRight size={13} />
        </button>
        <div className="cellar-titlebar-divider" />
        <button className="icon-btn" title="Settings">
          <Icon.settings size={13} />
        </button>
      </div>
    </div>
  );
}

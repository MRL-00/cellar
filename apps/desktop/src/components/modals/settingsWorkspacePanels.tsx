import { Icon } from "../icons";
import {
  ACCENT_SWATCHES,
  FONT_SIZE_MAX,
  FONT_SIZE_MIN,
  useSettings,
  type Density,
  type NullDisplay,
  type Theme,
} from "../../lib/settings";
import {
  CD_INPUT,
  Row,
  Section,
  Segment,
  StaticSegment,
  Toggle,
} from "./settingsPrimitives";

export function SettingsAppearance() {
  const { settings, set } = useSettings();

  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section title="Theme">
        <Row label="Theme">
          <Segment<Theme>
            value={settings.theme}
            onChange={(v) => set("theme", v)}
            options={[
              { value: "system", label: "system" },
              { value: "dark", label: "dark" },
              { value: "light", label: "light" },
            ]}
          />
        </Row>
        <Row label="Accent">
          <div className="flex flex-wrap gap-1">
            {ACCENT_SWATCHES.map((c) => {
              const active = settings.accent.toLowerCase() === c.toLowerCase();
              return (
                <button
                  type="button"
                  key={c}
                  onClick={() => set("accent", c)}
                  className={
                    "h-[18px] w-[18px] rounded-[4px] border border-white/10 transition-transform hover:scale-110 " +
                    (active
                      ? "shadow-[0_0_0_2px_var(--bg-1),0_0_0_3px_var(--fg-0)]"
                      : "")
                  }
                  style={{ background: c }}
                  title={c}
                />
              );
            })}
          </div>
        </Row>
        <Row label="Density">
          <Segment<Density>
            value={settings.density}
            onChange={(v) => set("density", v)}
            options={[
              { value: "compact", label: "compact" },
              { value: "comfortable", label: "comfortable" },
            ]}
          />
        </Row>
      </Section>

      <Section title="Type">
        <Row label="Interface font">
          <input
            className={CD_INPUT + " font-sans"}
            value={settings.interfaceFont}
            onChange={(e) => set("interfaceFont", e.target.value)}
            style={{ flex: 1 }}
          />
        </Row>
        <Row label="Editor / mono font">
          <input
            className={CD_INPUT + " font-mono"}
            value={settings.monoFont}
            onChange={(e) => set("monoFont", e.target.value)}
            style={{ flex: 1 }}
          />
        </Row>
        <Row
          label="Font size"
          hint="Scales the entire interface. Default is 12.5 px."
        >
          <input
            className={CD_INPUT + " font-mono"}
            style={{ width: 70, flex: "none" }}
            type="number"
            min={FONT_SIZE_MIN}
            max={FONT_SIZE_MAX}
            step={0.5}
            value={settings.fontSizePx}
            onChange={(e) => {
              const v = parseFloat(e.target.value);
              if (Number.isFinite(v)) {
                set("fontSizePx", Math.min(Math.max(v, FONT_SIZE_MIN), FONT_SIZE_MAX));
              }
            }}
          />
          <span className="text-[11px] text-fg-2">px</span>
          <div className="relative ml-2 w-[220px] pt-[14px]">
            <span className="absolute top-0 left-0 -translate-x-1/2 whitespace-nowrap text-[9.5px] text-fg-3">
              {FONT_SIZE_MIN}
            </span>
            <span className="absolute top-0 left-1/2 -translate-x-1/2 whitespace-nowrap text-[9.5px] text-fg-3">
              12.5
            </span>
            <span className="absolute top-0 right-0 translate-x-1/2 whitespace-nowrap text-[9.5px] text-fg-3">
              {FONT_SIZE_MAX}
            </span>
            <input
              type="range"
              min={FONT_SIZE_MIN}
              max={FONT_SIZE_MAX}
              step={0.5}
              value={settings.fontSizePx}
              onChange={(e) => set("fontSizePx", parseFloat(e.target.value))}
              className="m-0 h-[18px] w-full"
              style={{ accentColor: "var(--accent)" }}
            />
          </div>
        </Row>
      </Section>

      <Section title="Window">
        <Row label="Show traffic lights">
          <Toggle on={true} ariaLabel="Show traffic lights" />
        </Row>
        <Row label="Native window controls">
          <Toggle on={false} ariaLabel="Native window controls" />
        </Row>
      </Section>
    </div>
  );
}

export function SettingsGeneral() {
  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <p className="px-5 pt-4 text-[11.5px] text-fg-3">
        General settings will appear here as features are built.
      </p>
    </div>
  );
}

export function SettingsEditor() {
  const { settings, setEditor } = useSettings();
  const { editor } = settings;

  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section title="SQL editor">
        <Row label="Tab size">
          <Segment<"2" | "4" | "8">
            value={String(editor.tabSize) as "2" | "4" | "8"}
            onChange={(v) => setEditor({ tabSize: Number(v) as 2 | 4 | 8 })}
            options={[
              { value: "2", label: "2" },
              { value: "4", label: "4" },
              { value: "8", label: "8" },
            ]}
          />
        </Row>
        <Row label="Show line numbers">
          <Toggle
            on={editor.lineNumbers}
            onChange={(v) => setEditor({ lineNumbers: v })}
            ariaLabel="Show line numbers"
          />
        </Row>
        <Row label="Soft wrap" hint="Also toggleable per-editor with the wrap button in the toolbar.">
          <Toggle
            on={editor.softWrap}
            onChange={(v) => setEditor({ softWrap: v })}
            ariaLabel="Soft wrap"
          />
        </Row>
        <Row label="Bracket matching">
          <Toggle
            on={editor.bracketMatching}
            onChange={(v) => setEditor({ bracketMatching: v })}
            ariaLabel="Bracket matching"
          />
        </Row>
      </Section>
    </div>
  );
}

const NULL_DISPLAY_OPTIONS: { value: NullDisplay; label: string }[] = [
  { value: "NULL", label: "NULL" },
  { value: "∅", label: "∅" },
  { value: "(empty)", label: "(empty)" },
];

export function SettingsGrid() {
  const { settings, setGrid } = useSettings();
  const { grid } = settings;

  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section title="Data grid">
        <Row label="NULL display" hint="Text shown in cells where the database value is NULL.">
          <Segment<NullDisplay>
            value={grid.nullDisplay}
            onChange={(v) => setGrid({ nullDisplay: v })}
            options={NULL_DISPLAY_OPTIONS}
          />
        </Row>
        <Row label="Stripe alternating rows">
          <Toggle
            on={grid.stripeRows}
            onChange={(v) => setGrid({ stripeRows: v })}
            ariaLabel="Stripe alternating rows"
          />
        </Row>
      </Section>
    </div>
  );
}

export function SettingsKeymap() {
  const shortcuts = [
    {
      grp: "Workspace",
      items: [
        { k: "Command palette", kbd: "⌘K" },
        { k: "New connection", kbd: "⌘N" },
        { k: "New SQL tab", kbd: "⌘T" },
        { k: "Close tab", kbd: "⌘W" },
        { k: "Settings", kbd: "⌘," },
        { k: "Toggle sidebar", kbd: "⌘B" },
        { k: "Toggle AI panel", kbd: "⌘J" },
        { k: "Toggle results panel", kbd: "⌘↓" },
      ],
    },
    {
      grp: "Editor",
      items: [
        { k: "Run statement", kbd: "⌘⏎" },
        { k: "Run selection", kbd: "⌥⏎" },
        { k: "Format", kbd: "⌥⇧F" },
        { k: "Accept ghost text", kbd: "Tab" },
        { k: "Show schema for symbol", kbd: "F12" },
      ],
    },
    {
      grp: "Grid",
      items: [
        { k: "Edit cell", kbd: "⏎" },
        { k: "Revert cell", kbd: "Esc" },
        { k: "Commit changes", kbd: "⌘S" },
        { k: "Revert all pending", kbd: "⌘⇧Z" },
        { k: "Set NULL", kbd: "⌘⌫" },
      ],
    },
  ];

  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section title="Keymap" sub="Pick a preset or rebind any individual shortcut.">
        <Row label="Preset">
          <StaticSegment
            values={["Cellar", "DataGrip", "VS Code", "Linear"]}
            activeIdx={0}
          />
        </Row>
      </Section>
      {shortcuts.map((g) => (
        <Section key={g.grp} title={g.grp}>
          <div className="flex flex-col">
            {g.items.map((s) => (
              <div
                key={s.k}
                className="grid grid-cols-[1fr_auto_auto] items-center gap-3 border-b border-dashed border-border-divider py-1.5 last:border-b-0"
              >
                <span className="text-[11.5px] text-fg-1">{s.k}</span>
                <span className="inline-flex gap-0.5">
                  {[...s.kbd].map((k, i) => (
                    <kbd key={i} className="kbd">
                      {k}
                    </kbd>
                  ))}
                </span>
                <button
                  type="button"
                  disabled
                  className="icon-btn cursor-not-allowed opacity-60"
                  title="Key rebinding is not wired yet"
                >
                  <Icon.edit size={10} />
                </button>
              </div>
            ))}
          </div>
        </Section>
      ))}
    </div>
  );
}

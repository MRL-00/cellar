import { useEffect, useRef, useState, type ReactNode } from "react";
import { Icon, type IconName } from "../icons";
import { Modal } from "./Modal";
import {
  ACCENT_SWATCHES,
  FONT_SIZE_MAX,
  FONT_SIZE_MIN,
  useSettings,
  type Density,
  type Theme,
} from "../../lib/settings";

type CatId =
  | "general"
  | "appearance"
  | "editor"
  | "grid"
  | "keymap"
  | "connections"
  | "history"
  | "backups"
  | "ai"
  | "privacy"
  | "updates"
  | "about";

type CatItem = { id: CatId; label: string; icon: IconName; badge?: string };
type CatGroup = { group: string; items: CatItem[] };

const SETTINGS_CATS: CatGroup[] = [
  {
    group: "Workspace",
    items: [
      { id: "general", label: "General", icon: "settings" },
      { id: "appearance", label: "Appearance", icon: "layout" },
      { id: "editor", label: "Editor", icon: "edit" },
      { id: "grid", label: "Data grid", icon: "table" },
      { id: "keymap", label: "Keymap", icon: "terminal" },
    ],
  },
  {
    group: "Data",
    items: [
      { id: "connections", label: "Connections", icon: "database" },
      { id: "history", label: "Query history", icon: "history" },
      { id: "backups", label: "Backups & exports", icon: "cloud" },
    ],
  },
  {
    group: "Intelligence",
    items: [
      { id: "ai", label: "AI Assistant", icon: "sparkles", badge: "BYO key" },
    ],
  },
  {
    group: "System",
    items: [
      { id: "privacy", label: "Privacy & telemetry", icon: "lock" },
      { id: "updates", label: "Updates", icon: "power" },
      { id: "about", label: "About", icon: "info" },
    ],
  },
];

const ED_RUN_BASE =
  "inline-flex h-[26px] items-center gap-[5px] whitespace-nowrap rounded-[4px] border border-transparent px-2.5 text-[11.5px] font-medium text-fg-1 transition-[background,color,border-color,filter] duration-[120ms]";
const ED_RUN_SUBTLE =
  ED_RUN_BASE +
  " bg-transparent border-border-default hover:bg-bg-3 hover:border-border-strong hover:text-fg-0";
const ED_RUN_PRIMARY =
  ED_RUN_BASE + " bg-accent text-accent-fg hover:brightness-[1.07]";
const ED_RUN_DANGER =
  ED_RUN_BASE +
  " bg-transparent border-[color-mix(in_oklab,var(--delete)_30%,var(--border-default))] text-delete hover:bg-delete-bg hover:border-delete";

const CD_INPUT =
  "h-[26px] min-w-0 flex-1 rounded-[4px] border border-border-default bg-bg-inset px-2 text-[11.5px] text-fg-0 outline-none focus:border-accent-line focus:bg-bg-2";

export function SettingsModal({
  onClose,
  initialCat = "appearance",
}: {
  onClose: () => void;
  initialCat?: CatId;
}) {
  const [cat, setCat] = useState<CatId>(initialCat);
  const [q, setQ] = useState("");
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        searchRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const currentLabel = SETTINGS_CATS.flatMap((g) => g.items).find((i) => i.id === cat)
    ?.label;

  return (
    <Modal onClose={onClose} width={960} height={660} className="!max-h-[90vh]">
      {/* Head */}
      <div className="flex h-[42px] shrink-0 items-center gap-3 border-b border-border-default pl-3 pr-2">
        <span className="inline-flex text-accent">
          <Icon.settings size={14} />
        </span>
        <span className="whitespace-nowrap text-[12.5px] font-semibold text-fg-0">
          Settings
        </span>
        <span className="h-[14px] w-px bg-border-divider" />
        <span className="font-mono text-[11px] text-fg-2">{currentLabel}</span>

        <div className="ml-auto inline-flex h-[24px] max-w-[260px] flex-1 items-center gap-1.5 rounded-[4px] border border-border-default bg-bg-inset px-[7px]">
          <Icon.search size={11} stroke="var(--fg-3)" />
          <input
            ref={searchRef}
            placeholder="Search settings…"
            value={q}
            onChange={(e) => setQ(e.target.value)}
            className="min-w-0 flex-1 bg-transparent text-[11.5px] text-fg-0 outline-none placeholder:text-fg-3"
          />
          <span className="inline-flex gap-0.5">
            <kbd className="kbd">⌘F</kbd>
          </span>
        </div>

        <button className="icon-btn" onClick={onClose} title="Close">
          <Icon.close size={13} />
        </button>
      </div>

      {/* Body */}
      <div className="grid min-h-0 flex-1 overflow-hidden grid-cols-[200px_1fr]">
        <SettingsNav cat={cat} setCat={setCat} q={q} />
        <div className="flex min-h-0 min-w-0 flex-col bg-bg-1">
          {cat === "appearance" && <SettingsAppearance />}
          {cat === "general" && <SettingsGeneral />}
          {cat === "editor" && <SettingsEditor />}
          {cat === "grid" && <SettingsGrid />}
          {cat === "keymap" && <SettingsKeymap />}
          {cat === "connections" && <SettingsConnections />}
          {cat === "history" && <SettingsHistory />}
          {cat === "backups" && <SettingsBackups />}
          {cat === "ai" && <SettingsAI />}
          {cat === "privacy" && <SettingsPrivacy />}
          {cat === "updates" && <SettingsUpdates />}
          {cat === "about" && <SettingsAbout />}
        </div>
      </div>

      {/* Foot */}
      <div className="flex h-11 shrink-0 items-center justify-between gap-3 border-t border-border-default bg-bg-2 px-3">
        <span className="inline-flex items-center gap-1.5 text-[11px]">
          <span
            className="h-1.5 w-1.5 rounded-full"
            style={{ background: "var(--insert)" }}
          />
          <span className="text-fg-2">Synced to</span>
          <span className="font-mono text-fg-1">~/.cellar/settings.toml</span>
          <span className="text-fg-3">·</span>
          <button className="text-[11px] text-accent underline underline-offset-2">
            edit raw
          </button>
        </span>
        <div className="flex items-center gap-2">
          <button className={ED_RUN_SUBTLE} onClick={onClose}>
            Cancel
          </button>
          <button className={ED_RUN_SUBTLE}>
            <Icon.undo size={11} />
            <span>Reset section</span>
          </button>
          <button className={ED_RUN_PRIMARY} onClick={onClose}>
            <Icon.check size={11} />
            <span>Apply</span>
          </button>
        </div>
      </div>
    </Modal>
  );
}

function SettingsNav({
  cat,
  setCat,
  q,
}: {
  cat: CatId;
  setCat: (c: CatId) => void;
  q: string;
}) {
  const filter = q.toLowerCase().trim();
  return (
    <nav className="flex min-h-0 flex-col overflow-y-auto border-r border-border-default bg-bg-0 py-2">
      {SETTINGS_CATS.map((g) => {
        const items = g.items.filter(
          (i) => !filter || i.label.toLowerCase().includes(filter),
        );
        if (!items.length) return null;
        return (
          <div key={g.group} className="py-1">
            <div className="px-[14px] py-1 text-[9.5px] font-semibold uppercase tracking-[0.08em] text-fg-3">
              {g.group}
            </div>
            {items.map((i) => {
              const IconCmp = Icon[i.icon];
              const active = cat === i.id;
              return (
                <button
                  key={i.id}
                  onClick={() => setCat(i.id)}
                  className={
                    "flex h-[26px] w-full items-center gap-2 border-l-2 px-[10px] pl-[14px] text-left text-[11.5px] " +
                    (active
                      ? "bg-accent-soft border-l-accent text-accent font-medium"
                      : "border-l-transparent text-fg-1 hover:bg-bg-2 hover:text-fg-0")
                  }
                >
                  <span
                    className={
                      "inline-flex h-[14px] w-[14px] shrink-0 items-center justify-center " +
                      (active ? "text-accent" : "text-fg-2")
                    }
                  >
                    <IconCmp size={12} />
                  </span>
                  <span className="flex-1 whitespace-nowrap">{i.label}</span>
                  {i.badge && (
                    <span
                      className={
                        "rounded-[3px] border px-1 py-[1px] font-mono text-[9px] uppercase tracking-[0.04em] " +
                        (active
                          ? "border-accent-line bg-bg-1 text-accent"
                          : "border-accent-line bg-accent-soft text-accent")
                      }
                    >
                      {i.badge}
                    </span>
                  )}
                </button>
              );
            })}
          </div>
        );
      })}
      <div className="mt-auto flex items-center gap-1.5 border-t border-border-divider px-[14px] py-2.5 text-[10px]">
        <span className="font-mono text-fg-3">v0.4.2</span>
        <span className="text-fg-3">·</span>
        <button className="text-[10px] text-accent underline underline-offset-2">
          docs
        </button>
      </div>
    </nav>
  );
}

/* ─── Section primitives ─── */

function Section({
  title,
  sub,
  children,
}: {
  title: string;
  sub?: string;
  children: ReactNode;
}) {
  return (
    <section className="px-6 pb-1 pt-[18px] [&+section]:mt-1.5 [&+section]:border-t [&+section]:border-border-divider">
      <header className="mb-3">
        <h2 className="m-0 text-[13px] font-semibold tracking-[-0.005em] text-fg-0">
          {title}
        </h2>
        {sub && (
          <p className="m-0 mt-px max-w-[60ch] text-[11.5px] text-fg-2 text-pretty">
            {sub}
          </p>
        )}
      </header>
      <div className="flex flex-col gap-2">{children}</div>
    </section>
  );
}

function Row({
  label,
  hint,
  children,
  stack,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
  stack?: boolean;
}) {
  return (
    <div
      className={
        "grid min-h-[24px] grid-cols-[180px_1fr] gap-[14px] py-1 " +
        (stack ? "items-start" : "items-center")
      }
    >
      <div className="flex flex-col gap-[2px] text-[11.5px] font-medium text-fg-1">
        <span>{label}</span>
        {hint && (
          <span className="text-[10.5px] font-normal text-fg-3 text-pretty">
            {hint}
          </span>
        )}
      </div>
      <div
        className={
          "min-w-0 text-[11.5px] " +
          (stack
            ? "block"
            : "flex flex-wrap items-center gap-2")
        }
      >
        {children}
      </div>
    </div>
  );
}

function Toggle({
  on,
  onChange,
  locked,
}: {
  on: boolean;
  onChange?: (v: boolean) => void;
  locked?: boolean;
}) {
  return (
    <button
      onClick={() => !locked && onChange?.(!on)}
      title={locked ? "Forced on for prod connections" : undefined}
      className={
        "relative h-4 w-7 shrink-0 rounded-[10px] transition-colors duration-150 " +
        (on ? (locked ? "bg-warn" : "bg-accent") : "bg-bg-3") +
        (locked ? " cursor-not-allowed opacity-85" : "")
      }
    >
      <span
        className={
          "absolute top-[2px] h-3 w-3 rounded-full bg-white transition-[left] duration-150 " +
          (on ? "left-[14px]" : "left-[2px]")
        }
      />
    </button>
  );
}

function Segment<T extends string>({
  options,
  value,
  onChange,
}: {
  options: { value: T; label: string }[];
  value: T;
  onChange?: (v: T) => void;
}) {
  return (
    <div className="inline-flex gap-px rounded-[4px] border border-border-default bg-bg-inset p-[2px]">
      {options.map((o) => (
        <button
          key={o.value}
          onClick={() => onChange?.(o.value)}
          className={
            "h-5 rounded-[3px] px-2.5 text-[11px] " +
            (value === o.value
              ? "bg-bg-3 font-medium text-fg-0"
              : "text-fg-2 hover:text-fg-0")
          }
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

function StaticSegment({ values, activeIdx }: { values: string[]; activeIdx: number }) {
  return (
    <div className="inline-flex gap-px rounded-[4px] border border-border-default bg-bg-inset p-[2px]">
      {values.map((v, i) => (
        <button
          key={v}
          className={
            "h-5 rounded-[3px] px-2.5 text-[11px] " +
            (i === activeIdx
              ? "bg-bg-3 font-medium text-fg-0"
              : "text-fg-2 hover:text-fg-0")
          }
        >
          {v}
        </button>
      ))}
    </div>
  );
}

function StubBanner({ children }: { children: ReactNode }) {
  return (
    <div className="mx-6 my-2 flex items-center gap-1.5 rounded-[4px] border border-dashed border-border-default bg-bg-inset px-3 py-2 text-[11px] text-fg-2">
      <Icon.info size={12} stroke="var(--fg-3)" />
      <span>{children}</span>
    </div>
  );
}

/* ─── Appearance (real settings) ─── */

function SettingsAppearance() {
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
              if (Number.isFinite(v)) set("fontSizePx", v);
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
          <Toggle on={true} />
        </Row>
        <Row label="Native window controls">
          <Toggle on={false} />
        </Row>
      </Section>
    </div>
  );
}

/* ─── Stub panels ─── */

function SettingsGeneral() {
  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section title="General">
        <Row label="Startup">
          <StaticSegment
            values={["Restore last session", "Empty workspace", "Show welcome"]}
            activeIdx={0}
          />
        </Row>
        <Row label="Default schema search path">
          <input
            className={CD_INPUT + " font-mono"}
            defaultValue="public, audit, analytics"
            style={{ flex: 1 }}
          />
        </Row>
        <Row label="Confirm before quitting">
          <Toggle on={true} />
        </Row>
        <Row label="Allow background queries">
          <Toggle on={true} />
        </Row>
      </Section>
      <Section title="Updates">
        <Row label="Channel">
          <StaticSegment values={["stable", "beta", "nightly"]} activeIdx={0} />
        </Row>
        <Row label="Auto-install on quit">
          <Toggle on={true} />
        </Row>
      </Section>
    </div>
  );
}

function SettingsEditor() {
  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section title="SQL editor">
        <Row label="Tab size">
          <StaticSegment values={["2", "4", "8"]} activeIdx={1} />
        </Row>
        <Row label="Indent with">
          <StaticSegment values={["spaces", "tabs"]} activeIdx={0} />
        </Row>
        <Row label="Auto-format on save">
          <Toggle on={true} />
        </Row>
        <Row label="Keyword case">
          <StaticSegment values={["UPPER", "lower", "Preserve"]} activeIdx={0} />
        </Row>
        <Row label="Show line numbers">
          <Toggle on={true} />
        </Row>
        <Row label="Soft wrap">
          <Toggle on={false} />
        </Row>
        <Row label="Bracket matching">
          <Toggle on={true} />
        </Row>
      </Section>
      <Section title="Execution">
        <Row label="Statement at cursor runs">
          <StaticSegment
            values={["current statement", "selection", "whole file"]}
            activeIdx={0}
          />
        </Row>
        <Row label="LIMIT applied to SELECT *">
          <input
            className={CD_INPUT + " font-mono"}
            defaultValue="500"
            style={{ width: 70, flex: "none" }}
          />
        </Row>
      </Section>
    </div>
  );
}

function SettingsGrid() {
  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section title="Data grid">
        <Row label="Row height">
          <StaticSegment values={["20px", "22px", "28px", "36px"]} activeIdx={1} />
        </Row>
        <Row label="NULL display">
          <input
            className={CD_INPUT + " font-mono"}
            defaultValue="NULL"
            style={{ width: 120, flex: "none" }}
          />
          <StaticSegment values={["dim italic", "strong"]} activeIdx={0} />
        </Row>
        <Row label="Number alignment">
          <StaticSegment values={["left", "right"]} activeIdx={1} />
        </Row>
        <Row label="Stripe alternating rows">
          <Toggle on={false} />
        </Row>
        <Row label="Sticky pkey column">
          <Toggle on={true} />
        </Row>
        <Row label="Truncate cells over">
          <input
            className={CD_INPUT + " font-mono"}
            defaultValue="200"
            style={{ width: 70, flex: "none" }}
          />
          <span className="text-[11px] text-fg-2">chars</span>
        </Row>
      </Section>
    </div>
  );
}

function SettingsKeymap() {
  const SHORTCUTS: { grp: string; items: { k: string; kbd: string }[] }[] = [
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
      {SHORTCUTS.map((g) => (
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
                <button className="icon-btn" title="rebind">
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

function SettingsConnections() {
  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section
        title="Defaults for new connections"
        sub="Applied when you create a connection. Per-connection overrides win."
      >
        <Row label="Read-only by default">
          <Toggle on={true} />
        </Row>
        <Row label="Connection timeout">
          <input
            className={CD_INPUT + " font-mono"}
            defaultValue="10"
            style={{ width: 70, flex: "none" }}
          />
          <span className="text-[11px] text-fg-2">seconds</span>
        </Row>
        <Row label="Keep-alive interval">
          <input
            className={CD_INPUT + " font-mono"}
            defaultValue="30"
            style={{ width: 70, flex: "none" }}
          />
          <span className="text-[11px] text-fg-2">seconds</span>
        </Row>
        <Row label="Application name">
          <input
            className={CD_INPUT + " font-mono"}
            defaultValue="cellar (alice@laptop)"
            style={{ flex: 1 }}
          />
        </Row>
      </Section>
      <Section
        title="Production safety"
        sub="Cellar will require you to type the connection name before running these against any 'prod' connection."
      >
        <Row label="Confirm DML on prod">
          <Toggle on={true} locked />
        </Row>
        <Row label="Confirm DROP / TRUNCATE on prod">
          <Toggle on={true} locked />
        </Row>
        <Row label="Block UPDATE without WHERE">
          <Toggle on={true} />
        </Row>
        <Row label="Block DELETE without WHERE">
          <Toggle on={true} />
        </Row>
        <Row label="Max rows affected before warn">
          <input
            className={CD_INPUT + " font-mono"}
            defaultValue="100"
            style={{ width: 70, flex: "none" }}
          />
          <span className="text-[11px] text-fg-2">rows</span>
        </Row>
      </Section>
    </div>
  );
}

function SettingsHistory() {
  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section title="Query history">
        <Row label="Retain history for">
          <StaticSegment
            values={["7 days", "30 days", "90 days", "forever"]}
            activeIdx={2}
          />
        </Row>
        <Row label="Store query results">
          <Toggle on={false} />
        </Row>
        <Row
          label="Sync history across machines"
          hint="end-to-end encrypted via your Anthropic key"
        >
          <Toggle on={false} />
        </Row>
      </Section>
      <StubBanner>23,418 queries · 14.2 MB · last cleared 12 days ago</StubBanner>
    </div>
  );
}

function SettingsBackups() {
  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section title="Backups">
        <Row
          label="Auto-snapshot before commits"
          hint="pg_dump --schema-only + affected rows"
        >
          <Toggle on={true} />
        </Row>
        <Row label="Snapshot location">
          <input
            className={CD_INPUT + " font-mono"}
            defaultValue="~/.cellar/snapshots"
            style={{ flex: 1 }}
          />
          <button className="inline-flex h-[26px] items-center gap-1 rounded-[4px] border border-border-default bg-bg-2 px-2 text-[11px] text-fg-1 hover:bg-bg-3">
            <Icon.fileText size={11} />
            <span>Browse</span>
          </button>
        </Row>
        <Row label="Retain snapshots for">
          <input
            className={CD_INPUT + " font-mono"}
            defaultValue="30"
            style={{ width: 70, flex: "none" }}
          />
          <span className="text-[11px] text-fg-2">days</span>
        </Row>
      </Section>
      <Section title="Export defaults">
        <Row label="Format">
          <StaticSegment
            values={["CSV", "JSON", "Parquet", "SQL INSERT"]}
            activeIdx={0}
          />
        </Row>
        <Row label="NULL as">
          <input
            className={CD_INPUT + " font-mono"}
            defaultValue="\\N"
            style={{ width: 120, flex: "none" }}
          />
        </Row>
        <Row label="Include headers">
          <Toggle on={true} />
        </Row>
      </Section>
    </div>
  );
}

function SettingsAI() {
  const [provider, setProvider] = useState("anthropic");
  const [model, setModel] = useState("claude-sonnet-4.5");
  const [reveal, setReveal] = useState(false);

  const PROVIDERS = [
    { id: "anthropic", label: "Anthropic", sub: "claude-* family" },
    { id: "openai", label: "OpenAI", sub: "gpt-* family" },
    { id: "google", label: "Google", sub: "gemini-* family" },
    { id: "local", label: "Local", sub: "Ollama, LM Studio" },
    { id: "custom", label: "Custom", sub: "OpenAI-compatible URL" },
  ];
  const MODELS = [
    {
      id: "claude-sonnet-4.5",
      ctx: "200k",
      tag: "balanced" as const,
      def: true,
    },
    { id: "claude-opus-4", ctx: "200k", tag: "max" as const },
    { id: "claude-haiku-4.5", ctx: "200k", tag: "fast" as const },
  ];
  const tagClass: Record<"balanced" | "max" | "fast", string> = {
    balanced: "text-accent bg-accent-soft",
    max: "text-update bg-update-bg",
    fast: "text-insert bg-insert-bg",
  };

  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section
        title="AI Assistant"
        sub="Cellar's AI runs entirely on your key. Your queries, schemas and results never touch our servers."
      >
        <div className="flex gap-2.5 rounded-[6px] border border-accent-line bg-accent-soft px-3.5 py-3">
          <span className="inline-flex h-[22px] w-[22px] shrink-0 items-center justify-center rounded-[5px] border border-accent-line bg-bg-1">
            <Icon.sparkles size={12} stroke="var(--accent)" />
          </span>
          <div className="text-pretty">
            <div className="mb-0.5 text-[12px] font-semibold text-fg-0">
              Bring-your-own-key, by design
            </div>
            <div className="text-[11.5px] leading-[1.45] text-fg-1">
              All AI requests go directly from your machine to the provider. We
              see nothing. Cellar is open-source — verify the network path at{" "}
              <span className="font-mono text-fg-0">
                github.com/cellar/ai-router
              </span>
              .
            </div>
          </div>
        </div>
      </Section>

      <Section title="Provider">
        <Row label="Provider">
          <div className="grid w-full grid-cols-5 gap-1.5">
            {PROVIDERS.map((p) => {
              const active = provider === p.id;
              return (
                <button
                  key={p.id}
                  onClick={() => setProvider(p.id)}
                  className={
                    "relative flex flex-col items-start gap-0.5 rounded-[5px] border px-[9px] py-2 text-left " +
                    (active
                      ? "border-accent bg-accent-soft shadow-[inset_0_0_0_1px_var(--accent)]"
                      : "border-border-default bg-bg-2 hover:border-border-strong")
                  }
                >
                  <span className="text-[11.5px] font-medium text-fg-0">
                    {p.label}
                  </span>
                  <span
                    className={
                      "font-mono text-[9.5px] " +
                      (active ? "text-accent opacity-85" : "text-fg-3")
                    }
                  >
                    {p.sub}
                  </span>
                  {active && (
                    <span className="absolute right-[5px] top-[5px] inline-flex h-3 w-3 items-center justify-center rounded-full bg-accent text-accent-fg">
                      <Icon.check size={9} />
                    </span>
                  )}
                </button>
              );
            })}
          </div>
        </Row>

        <Row label="Model" hint="used for chat, generation, ghost text">
          <div className="flex w-full flex-col gap-[3px]">
            {MODELS.map((m) => {
              const active = model === m.id;
              return (
                <label
                  key={m.id}
                  className={
                    "grid cursor-pointer grid-cols-[14px_1fr_auto_auto_auto] items-center gap-2.5 rounded-[4px] border px-2.5 py-1.5 " +
                    (active
                      ? "border-accent-line bg-accent-soft"
                      : "border-border-default bg-bg-2 hover:border-border-strong")
                  }
                >
                  <span
                    className={
                      "relative inline-block h-3 w-3 rounded-full border " +
                      (active ? "border-accent" : "border-border-strong")
                    }
                  >
                    {active && (
                      <span
                        className="absolute inset-[2px] rounded-full"
                        style={{ background: "var(--accent)" }}
                      />
                    )}
                  </span>
                  <input
                    type="radio"
                    className="hidden"
                    checked={active}
                    onChange={() => setModel(m.id)}
                  />
                  <span className="font-mono text-[11.5px] text-fg-0">
                    {m.id}
                  </span>
                  <span className="font-mono text-[10px] text-fg-3">
                    {m.ctx} ctx
                  </span>
                  <span
                    className={
                      "rounded-[3px] px-1.5 py-px font-mono text-[9.5px] uppercase tracking-[0.04em] " +
                      tagClass[m.tag]
                    }
                  >
                    {m.tag}
                  </span>
                  {m.def && (
                    <span className="text-[9.5px] italic text-fg-2">
                      recommended
                    </span>
                  )}
                </label>
              );
            })}
          </div>
        </Row>

        <Row
          label="API key"
          hint="stored in OS keychain, never written to disk"
        >
          <div className="inline-flex min-w-0 flex-1 items-center gap-1">
            <span className="inline-flex h-[26px] items-center rounded-l-[4px] border border-r-0 border-border-default bg-bg-inset px-1.5 font-mono text-[11px] text-fg-2">
              sk-ant-
            </span>
            <input
              className="-ml-px h-[26px] min-w-0 flex-1 border border-border-default bg-bg-inset px-2 font-mono text-[11.5px] text-fg-0 outline-none focus:border-accent-line focus:bg-bg-2"
              type={reveal ? "text" : "password"}
              defaultValue={
                reveal
                  ? "api03-rXg7vQ2hN8jK4pL6mZ9bW3yT1uF5aE0dC2sB"
                  : "•••••••••••••••••••••••••••••••••••"
              }
              spellCheck={false}
            />
            <button
              onClick={() => setReveal(!reveal)}
              className="inline-flex h-[26px] items-center gap-1 rounded-[4px] border border-border-default bg-bg-2 px-2 text-[10.5px] text-fg-1 hover:bg-bg-3"
            >
              {reveal ? "hide" : "reveal"}
            </button>
            <button className="inline-flex h-[26px] items-center gap-1 rounded-[4px] border border-border-default bg-bg-2 px-2 text-[11px] text-fg-1 hover:bg-bg-3">
              <Icon.edit size={11} />
              <span>change</span>
            </button>
          </div>
          <span className="inline-flex items-center gap-1.5 text-[10.5px]">
            <Icon.check size={10} stroke="var(--insert)" />
            <span className="text-insert">Verified</span>
            <span className="text-fg-3">·</span>
            <span className="font-mono text-fg-2">tier 3</span>
            <span className="text-fg-3">·</span>
            <span className="font-mono text-fg-2">last used 4m ago</span>
          </span>
        </Row>

        <Row label="Endpoint" hint="override for proxies, custom routers">
          <input
            className={CD_INPUT + " font-mono"}
            defaultValue="https://api.anthropic.com/v1"
            style={{ flex: 1 }}
          />
        </Row>
      </Section>

      <Section title="Danger zone">
        <Row
          label="Clear AI conversation history"
          hint="20 conversations, 3.2 MB locally"
        >
          <button className={ED_RUN_DANGER}>
            <Icon.trash size={11} />
            <span>Clear all</span>
          </button>
        </Row>
        <Row
          label="Revoke API key"
          hint="remove from keychain — does not affect provider"
        >
          <button className={ED_RUN_DANGER}>
            <Icon.close size={11} />
            <span>Revoke</span>
          </button>
        </Row>
      </Section>
    </div>
  );
}

function SettingsPrivacy() {
  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section title="Telemetry">
        <Row
          label="Send anonymous usage stats"
          hint="counts of feature use, no query content"
        >
          <Toggle on={false} />
        </Row>
        <Row label="Send crash reports" hint="stack traces only, never DB contents">
          <Toggle on={true} />
        </Row>
      </Section>
      <Section
        title="Stored locally only"
        sub="Cellar never uploads any of these. Open ~/.cellar to inspect."
      >
        <div className="w-full overflow-hidden rounded-[5px] border border-border-default">
          {[
            { k: "Connections", v: "12 connections", path: "connections.toml" },
            {
              k: "Query history",
              v: "23,418 queries · 14.2 MB",
              path: "history.sqlite",
            },
            {
              k: "AI conversations",
              v: "20 conversations · 3.2 MB",
              path: "ai/",
            },
            { k: "Snapshots", v: "84 snapshots · 412 MB", path: "snapshots/" },
            { k: "Cached schemas", v: "12 dbs · 8.4 MB", path: "cache/" },
          ].map((x, i, arr) => (
            <div
              key={x.k}
              className={
                "grid grid-cols-[160px_1fr_auto_22px] items-center gap-2.5 bg-bg-2 px-2.5 py-1.5 text-[11px] hover:bg-bg-3 " +
                (i !== arr.length - 1
                  ? "border-b border-border-divider"
                  : "")
              }
            >
              <span className="font-medium text-fg-0">{x.k}</span>
              <span className="text-fg-2">{x.v}</span>
              <span className="font-mono text-[10.5px] text-fg-3">
                ~/.cellar/{x.path}
              </span>
              <button className="icon-btn" title="open">
                <Icon.chevronRight size={10} />
              </button>
            </div>
          ))}
        </div>
      </Section>
    </div>
  );
}

function SettingsUpdates() {
  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section title="Updates">
        <div className="mb-2 flex items-center justify-between rounded-[5px] border border-border-default bg-bg-inset px-3 py-2.5">
          <div className="flex items-center gap-2.5">
            <span className="font-mono text-[13px] font-semibold text-fg-0">
              v0.4.2
            </span>
            <span className="inline-flex items-center gap-1 text-[11px]">
              <Icon.check size={11} stroke="var(--insert)" />
              <span className="text-insert">You're up to date</span>
            </span>
            <span className="text-[11px] text-fg-3">last checked 2 minutes ago</span>
          </div>
          <button className="inline-flex h-[26px] items-center gap-1 rounded-[4px] border border-border-default bg-bg-2 px-2 text-[11px] text-fg-1 hover:bg-bg-3">
            <Icon.power size={11} />
            <span>Check now</span>
          </button>
        </div>
        <Row label="Channel">
          <StaticSegment values={["stable", "beta", "nightly"]} activeIdx={0} />
        </Row>
        <Row label="Auto-install on quit">
          <Toggle on={true} />
        </Row>
      </Section>
    </div>
  );
}

function SettingsAbout() {
  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <Section title="About">
        <div className="flex items-start gap-4">
          <span
            className="relative h-12 w-12 shrink-0 rounded-[10px]"
            style={{
              background:
                "linear-gradient(135deg, var(--accent), color-mix(in oklab, var(--accent) 50%, var(--syn-kw)))",
              boxShadow: "0 0 24px var(--accent-soft)",
            }}
          >
            <span
              className="absolute inset-2 rounded-[4px] bg-bg-1"
              style={{
                clipPath:
                  "polygon(0 0, 100% 0, 100% 35%, 35% 35%, 35% 65%, 100% 65%, 100% 100%, 0 100%)",
              }}
            />
          </span>
          <div>
            <div className="text-[18px] font-semibold tracking-[-0.01em] text-fg-0">
              Cellar
            </div>
            <div className="mb-2 text-[12px] text-fg-2">
              A fast, native database client with AI built in.
            </div>
            <div className="mb-2.5 flex gap-1.5 font-mono text-[10.5px] text-fg-2">
              <span>v0.4.2 · darwin-arm64</span>
              <span className="text-fg-3">·</span>
              <span>MIT licensed</span>
              <span className="text-fg-3">·</span>
              <span>commit 8e4f2a1</span>
            </div>
            <div className="flex gap-1.5 text-[11px]">
              <button className="text-accent underline underline-offset-2">
                docs
              </button>
              <span className="text-fg-3">·</span>
              <button className="text-accent underline underline-offset-2">
                github
              </button>
              <span className="text-fg-3">·</span>
              <button className="text-accent underline underline-offset-2">
                changelog
              </button>
              <span className="text-fg-3">·</span>
              <button className="text-accent underline underline-offset-2">
                acknowledgements
              </button>
            </div>
          </div>
        </div>
      </Section>
    </div>
  );
}

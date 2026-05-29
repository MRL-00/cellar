import { useEffect, useRef, useState } from "react";
import { Icon, type IconName } from "../icons";
import { Modal } from "./Modal";
import { SettingsAI } from "./settingsAIPanel";
import {
  SettingsBackups,
  SettingsConnections,
  SettingsHistory,
} from "./settingsDataPanels";
import {
  SettingsAbout,
  SettingsPrivacy,
  SettingsUpdates,
} from "./settingsSystemPanels";
import {
  SettingsAppearance,
  SettingsEditor,
  SettingsGeneral,
  SettingsGrid,
  SettingsKeymap,
} from "./settingsWorkspacePanels";
import {
  ED_RUN_PRIMARY,
  ED_RUN_SUBTLE,
} from "./settingsPrimitives";

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

        <button type="button" className="icon-btn" onClick={onClose} title="Close">
          <Icon.close size={13} />
        </button>
      </div>

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

      <div className="flex h-11 shrink-0 items-center justify-between gap-3 border-t border-border-default bg-bg-2 px-3">
        <span className="inline-flex items-center gap-1.5 text-[11px]">
          <span
            className="h-1.5 w-1.5 rounded-full"
            style={{ background: "var(--insert)" }}
          />
          <span className="text-fg-2">Saved locally</span>
          <span className="font-mono text-fg-1">browser storage</span>
          <span className="text-fg-3">·</span>
          <button
            type="button"
            disabled
            className="cursor-not-allowed text-[11px] text-fg-3 underline underline-offset-2"
            title="Raw settings editing is not wired yet"
          >
            edit raw
          </button>
        </span>
        <div className="flex items-center gap-2">
          <button
            type="button"
            disabled
            title="Section reset is not wired yet"
            className={ED_RUN_SUBTLE + " cursor-not-allowed opacity-60"}
          >
            <Icon.undo size={11} />
            <span>Reset section</span>
          </button>
          <button type="button" className={ED_RUN_PRIMARY} onClick={onClose}>
            <Icon.check size={11} />
            <span>Done</span>
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
                  type="button"
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
        <span className="font-mono text-fg-3">v0.1.0-alpha</span>
        <span className="text-fg-3">·</span>
        <button type="button" className="text-[10px] text-accent underline underline-offset-2">
          docs
        </button>
      </div>
    </nav>
  );
}

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
import {
  searchSettings,
  type SettingsSearchResult,
} from "./settingsSearch";

export type SettingsCatId =
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

type CatItem = { id: SettingsCatId; label: string; icon: IconName; badge?: string };
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
  initialCat?: SettingsCatId;
}) {
  const [cat, setCat] = useState<SettingsCatId>(initialCat);
  const [q, setQ] = useState("");
  const searchRef = useRef<HTMLInputElement>(null);
  const searchResults = searchSettings(q);
  const isSearching = q.trim().length > 0;

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
        <SettingsNav
          cat={cat}
          setCat={setCat}
          q={q}
          searchResults={searchResults}
        />
        <div className="flex min-h-0 min-w-0 flex-col bg-bg-1">
          {isSearching ? (
            <SettingsSearchResults
              q={q}
              results={searchResults}
              onOpen={(nextCat) => {
                setCat(nextCat);
                setQ("");
              }}
            />
          ) : (
            <>
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
            </>
          )}
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
  searchResults,
}: {
  cat: SettingsCatId;
  setCat: (c: SettingsCatId) => void;
  q: string;
  searchResults: SettingsSearchResult[];
}) {
  const filter = q.toLowerCase().trim();
  const matchingCats = new Set(searchResults.map((result) => result.cat));
  const counts = searchResults.reduce<Partial<Record<SettingsCatId, number>>>(
    (acc, result) => {
      acc[result.cat] = (acc[result.cat] ?? 0) + 1;
      return acc;
    },
    {},
  );

  return (
    <nav className="flex min-h-0 flex-col overflow-y-auto border-r border-border-default bg-bg-0 py-2">
      {SETTINGS_CATS.map((g) => {
        const groupMatches = filter && g.group.toLowerCase().includes(filter);
        const items = g.items.filter((i) => {
          if (!filter) return true;
          return (
            groupMatches ||
            i.label.toLowerCase().includes(filter) ||
            matchingCats.has(i.id)
          );
        });
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
                  {filter && counts[i.id] ? (
                    <span
                      className={
                        "font-mono text-[10px] " +
                        (active ? "text-accent" : "text-fg-3")
                      }
                    >
                      {counts[i.id]}
                    </span>
                  ) : i.badge ? (
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
                  ) : null}
                </button>
              );
            })}
          </div>
        );
      })}
      <div className="mt-auto flex items-center gap-1.5 border-t border-border-divider px-[14px] py-2.5 text-[10px]">
        <span className="font-mono text-fg-3">v0.1.0-alpha</span>
        <span className="text-fg-3">·</span>
        <button
          type="button"
          disabled
          className="cursor-not-allowed text-[10px] text-fg-3 underline underline-offset-2 opacity-70"
          title="Documentation links are not wired in the desktop shell yet"
        >
          docs
        </button>
      </div>
    </nav>
  );
}

function SettingsSearchResults({
  q,
  results,
  onOpen,
}: {
  q: string;
  results: SettingsSearchResult[];
  onOpen: (cat: SettingsCatId) => void;
}) {
  return (
    <div className="flex-1 overflow-y-auto pb-6 pt-1">
      <section className="px-6 pb-1 pt-[18px]">
        <header className="mb-3 flex items-end justify-between gap-3">
          <div>
            <h2 className="m-0 text-[13px] font-semibold tracking-[-0.005em] text-fg-0">
              Search results
            </h2>
            <p className="m-0 mt-px max-w-[60ch] text-[11.5px] text-fg-2 text-pretty">
              {results.length
                ? `${results.length} match${results.length === 1 ? "" : "es"} for "${q.trim()}"`
                : `No settings match "${q.trim()}"`}
            </p>
          </div>
          <span className="font-mono text-[10.5px] text-fg-3">
            labels, sections, values
          </span>
        </header>

        {results.length ? (
          <div className="overflow-hidden rounded-[5px] border border-border-default">
            {results.map((result, idx) => (
              <button
                type="button"
                key={`${result.cat}:${result.section}:${result.label}`}
                onClick={() => onOpen(result.cat)}
                className={
                  "grid w-full grid-cols-[150px_1fr_auto] items-center gap-3 bg-bg-2 px-3 py-2 text-left hover:bg-bg-3 " +
                  (idx !== results.length - 1
                    ? "border-b border-border-divider"
                    : "")
                }
              >
                <span className="min-w-0">
                  <span className="block truncate text-[11.5px] font-medium text-fg-0">
                    {result.label}
                  </span>
                  <span className="block truncate text-[10.5px] text-fg-3">
                    {result.section}
                  </span>
                </span>
                <span className="min-w-0 truncate font-mono text-[11px] text-fg-2">
                  {result.category}
                </span>
                <span className="inline-flex items-center gap-1 text-[10.5px] text-fg-3">
                  <span>open</span>
                  <Icon.chevronRight size={10} />
                </span>
              </button>
            ))}
          </div>
        ) : (
          <div className="rounded-[5px] border border-dashed border-border-default bg-bg-inset px-3 py-6 text-center">
            <div className="text-[12px] font-medium text-fg-1">
              No matching settings
            </div>
            <div className="mt-1 text-[11px] text-fg-3">
              Try a label, category, provider, shortcut, or stored value.
            </div>
          </div>
        )}
      </section>
    </div>
  );
}

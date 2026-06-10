import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

export type Theme = "system" | "dark" | "light";
export type Density = "compact" | "comfortable";
export type NullDisplay = "NULL" | "∅" | "(empty)";

export const ACCENT_SWATCHES = [
  "#4ade80",
  "#60a5fa",
  "#a78bfa",
  "#fbbf24",
  "#e3b341",
  "#e07a5f",
  "#b5b3e8",
  "#b8d670",
  "#d878a8",
  "#4dd4d4",
  "#c44a4a",
  "#c9a86a",
  "#a8475c",
  "#ffd60a",
] as const;

export type EditorSettings = {
  tabSize: 2 | 4 | 8;
  softWrap: boolean;
  lineNumbers: boolean;
  bracketMatching: boolean;
};

export type GridSettings = {
  nullDisplay: NullDisplay;
  stripeRows: boolean;
};

export type Settings = {
  theme: Theme;
  density: Density;
  accent: string;
  fontSizePx: number;
  interfaceFont: string;
  monoFont: string;
  editor: EditorSettings;
  grid: GridSettings;
};

export const DEFAULTS: Settings = {
  theme: "dark",
  density: "compact",
  accent: "#a78bfa",
  fontSizePx: 13.5,
  interfaceFont: "Geist",
  monoFont: "JetBrains Mono",
  editor: {
    tabSize: 4,
    softWrap: false,
    lineNumbers: true,
    bracketMatching: true,
  },
  grid: {
    nullDisplay: "NULL",
    stripeRows: false,
  },
};

const STORAGE_KEY = "cellar.settings.v1";
const FONT_SIZE_BASELINE = 13.5;
export const FONT_SIZE_MIN = 10;
export const FONT_SIZE_MAX = 22;

function load(): Settings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULTS;
    const parsed = JSON.parse(raw) as Partial<Settings>;
    // Deep-merge nested sub-objects so new fields added to DEFAULTS are
    // forward-compatible with older persisted values that lack them.
    return sanitize({
      ...DEFAULTS,
      ...parsed,
      editor: { ...DEFAULTS.editor, ...(parsed.editor ?? {}) },
      grid: { ...DEFAULTS.grid, ...(parsed.grid ?? {}) },
    });
  } catch {
    return DEFAULTS;
  }
}

const TAB_SIZES: EditorSettings["tabSize"][] = [2, 4, 8];
const NULL_DISPLAYS: NullDisplay[] = ["NULL", "∅", "(empty)"];

export function sanitize(s: Settings): Settings {
  const editor: EditorSettings = {
    tabSize: TAB_SIZES.includes(s.editor?.tabSize) ? s.editor.tabSize : DEFAULTS.editor.tabSize,
    softWrap: typeof s.editor?.softWrap === "boolean" ? s.editor.softWrap : DEFAULTS.editor.softWrap,
    lineNumbers: typeof s.editor?.lineNumbers === "boolean" ? s.editor.lineNumbers : DEFAULTS.editor.lineNumbers,
    bracketMatching: typeof s.editor?.bracketMatching === "boolean" ? s.editor.bracketMatching : DEFAULTS.editor.bracketMatching,
  };
  const grid: GridSettings = {
    nullDisplay: NULL_DISPLAYS.includes(s.grid?.nullDisplay) ? s.grid.nullDisplay : DEFAULTS.grid.nullDisplay,
    stripeRows: typeof s.grid?.stripeRows === "boolean" ? s.grid.stripeRows : DEFAULTS.grid.stripeRows,
  };
  return {
    ...s,
    fontSizePx: clamp(s.fontSizePx, FONT_SIZE_MIN, FONT_SIZE_MAX),
    editor,
    grid,
  };
}

function applySideEffects(s: Settings) {
  const html = document.documentElement;
  const body = document.body;

  const resolvedTheme =
    s.theme === "system"
      ? window.matchMedia("(prefers-color-scheme: light)").matches
        ? "light"
        : "dark"
      : s.theme;
  html.setAttribute("data-theme", resolvedTheme);
  html.setAttribute("data-density", s.density);

  html.style.setProperty("--accent", s.accent);
  html.style.setProperty(
    "--accent-soft",
    hexToRgba(s.accent, resolvedTheme === "light" ? 0.1 : 0.14),
  );
  html.style.setProperty(
    "--accent-line",
    hexToRgba(s.accent, resolvedTheme === "light" ? 0.28 : 0.32),
  );

  const scale = clamp(s.fontSizePx, FONT_SIZE_MIN, FONT_SIZE_MAX) / FONT_SIZE_BASELINE;
  // Compensate body's layout box for zoom so the App, which fills body, still
  // renders at exactly viewport size after scaling. Without this, w-full/h-full
  // resolve to 100% of an over-sized body and the right edge clips off-screen.
  html.style.setProperty("--ui-scale", String(scale));
  body.style.width = `${100 / scale}vw`;
  body.style.height = `${100 / scale}vh`;
  (body.style as CSSStyleDeclaration & { zoom?: string }).zoom = String(scale);
}

function clamp(n: number, min: number, max: number) {
  return Math.min(Math.max(n, min), max);
}

function hexToRgba(hex: string, alpha: number) {
  const m = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  if (!m || !m[1] || !m[2] || !m[3]) return hex;
  const r = parseInt(m[1], 16);
  const g = parseInt(m[2], 16);
  const b = parseInt(m[3], 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

type Ctx = {
  settings: Settings;
  set: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
  setEditor: (patch: Partial<EditorSettings>) => void;
  setGrid: (patch: Partial<GridSettings>) => void;
  /** Merge a partial settings object (e.g. an imported setup) over current. */
  importSettings: (partial: Partial<Settings>) => void;
  reset: () => void;
};

const SettingsContext = createContext<Ctx | null>(null);

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<Settings>(() => {
    const initial = load();
    applySideEffects(initial);
    return initial;
  });

  const set = useCallback(<K extends keyof Settings>(key: K, value: Settings[K]) => {
    setSettings((prev) => {
      const next = sanitize({ ...prev, [key]: value });
      localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
      applySideEffects(next);
      return next;
    });
  }, []);

  const setEditor = useCallback((patch: Partial<EditorSettings>) => {
    setSettings((prev) => {
      const next = sanitize({
        ...prev,
        editor: { ...prev.editor, ...patch },
      });
      localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
      applySideEffects(next);
      return next;
    });
  }, []);

  const setGrid = useCallback((patch: Partial<GridSettings>) => {
    setSettings((prev) => {
      const next = sanitize({
        ...prev,
        grid: { ...prev.grid, ...patch },
      });
      localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
      applySideEffects(next);
      return next;
    });
  }, []);

  const importSettings = useCallback((partial: Partial<Settings>) => {
    setSettings((prev) => {
      const next = sanitize({
        ...prev,
        ...partial,
        editor: { ...prev.editor, ...(partial.editor ?? {}) },
        grid: { ...prev.grid, ...(partial.grid ?? {}) },
      });
      localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
      applySideEffects(next);
      return next;
    });
  }, []);

  const reset = useCallback(() => {
    localStorage.removeItem(STORAGE_KEY);
    applySideEffects(DEFAULTS);
    setSettings(DEFAULTS);
  }, []);

  useEffect(() => {
    if (settings.theme !== "system") return;
    const mq = window.matchMedia("(prefers-color-scheme: light)");
    const onChange = () => applySideEffects(settings);
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, [settings]);

  const value = useMemo<Ctx>(
    () => ({ settings, set, setEditor, setGrid, importSettings, reset }),
    [settings, set, setEditor, setGrid, importSettings, reset],
  );

  return <SettingsContext.Provider value={value}>{children}</SettingsContext.Provider>;
}

export function useSettings(): Ctx {
  const ctx = useContext(SettingsContext);
  if (!ctx) throw new Error("useSettings must be used inside <SettingsProvider>");
  return ctx;
}

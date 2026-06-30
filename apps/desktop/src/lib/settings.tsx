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
  "#8a8a8a",
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
  interfaceFont: "SF Pro Text",
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
// The interface is authored compact and zoomed up for comfort: the font-size
// setting divided by this baseline gives the global UI scale, so the default
// 13.5px setting renders at ~1.125× rather than being a no-op.
const FONT_SIZE_BASELINE = 12;
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

export function applySettingsSideEffects(s: Settings) {
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

  // User-selected fonts override the leading family; the rest of the stack in
  // tokens.css remains as graceful fallback for anything not installed.
  // The user-selected family leads; the rest is a graceful fallback only used
  // when that font isn't installed.
  const interfaceStack = `"${s.interfaceFont}", "Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif`;
  const monoStack = `"${s.monoFont}", "Geist Mono", ui-monospace, "SF Mono", Menlo, Consolas, monospace`;
  html.style.setProperty("--font-sans", interfaceStack);
  html.style.setProperty("--font-interface", interfaceStack);
  html.style.setProperty("--font-mono", monoStack);
  html.style.setProperty("--font-data", monoStack);

  // Use an accent that's guaranteed visible against the theme background, so a
  // dark/neutral pick doesn't render accent-colored UI (tabs, selection, icons)
  // invisible. The raw value stays in settings (and drives the swatch picker).
  const accent = visibleAccent(s.accent, resolvedTheme !== "light");
  html.style.setProperty("--accent", accent);
  html.style.setProperty(
    "--accent-soft",
    hexToRgba(accent, resolvedTheme === "light" ? 0.1 : 0.14),
  );
  html.style.setProperty(
    "--accent-line",
    hexToRgba(accent, resolvedTheme === "light" ? 0.28 : 0.32),
  );
  html.style.setProperty("--accent-fg", readableOn(accent));
  // Editor ghost/inline-suggestion text tracks the accent too.
  html.style.setProperty("--syn-ghost", hexToRgba(accent, 0.55));

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

function parseHex(hex: string): [number, number, number] | null {
  const m = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  if (!m || !m[1] || !m[2] || !m[3]) return null;
  return [parseInt(m[1], 16), parseInt(m[2], 16), parseInt(m[3], 16)];
}

function hexToRgba(hex: string, alpha: number) {
  const rgb = parseHex(hex);
  if (!rgb) return hex;
  return `rgba(${rgb[0]}, ${rgb[1]}, ${rgb[2]}, ${alpha})`;
}

function toHex([r, g, b]: [number, number, number]): string {
  const byte = (n: number) => clamp(Math.round(n), 0, 255).toString(16).padStart(2, "0");
  return `#${byte(r)}${byte(g)}${byte(b)}`;
}

// WCAG relative luminance (0 = black, 1 = white) for an sRGB 0-255 triple.
function relLuminance([r, g, b]: [number, number, number]): number {
  const lin = (c: number) => {
    const v = c / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

function contrastRatio(l1: number, l2: number): number {
  return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
}

// Text/icon color to sit on top of the accent (e.g. the primary button fill).
// Picks dark or white by whichever has the higher WCAG contrast ratio against
// the accent, so dark accents (black) and mid neutrals (#8a8a8a) both stay
// legible — a fixed brightness cutoff mis-picks white on mid grays.
function readableOn(hex: string): string {
  const rgb = parseHex(hex);
  if (!rgb) return "#ffffff";
  const lum = relLuminance(rgb);
  return contrastRatio(lum, 0) >= contrastRatio(lum, 1) ? "#0a0b0e" : "#ffffff";
}

// The accent is used directly as a foreground/border/indicator color in dozens
// of places, so an accent too close to the theme background (e.g. black on the
// dark theme) renders those elements invisible. Nudge such accents toward the
// opposite end until they clear a minimum contrast against bg-0, keeping the
// user's hue while guaranteeing it shows up. Vivid accents are left untouched.
function visibleAccent(hex: string, dark: boolean): string {
  let rgb = parseHex(hex);
  if (!rgb) return hex;
  const bgLum = dark ? 0.006 : 0.92; // ≈ luminance of --bg-0 per theme
  const target = dark ? 255 : 0; // blend toward white on dark, black on light
  for (let i = 0; i < 12 && contrastRatio(relLuminance(rgb), bgLum) < 2.2; i++) {
    rgb = [
      rgb[0] + (target - rgb[0]) * 0.12,
      rgb[1] + (target - rgb[1]) * 0.12,
      rgb[2] + (target - rgb[2]) * 0.12,
    ];
  }
  return toHex(rgb);
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
    applySettingsSideEffects(initial);
    return initial;
  });

  const set = useCallback(<K extends keyof Settings>(key: K, value: Settings[K]) => {
    setSettings((prev) => {
      const next = sanitize({ ...prev, [key]: value });
      localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
      applySettingsSideEffects(next);
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
      applySettingsSideEffects(next);
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
      applySettingsSideEffects(next);
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
      applySettingsSideEffects(next);
      return next;
    });
  }, []);

  const reset = useCallback(() => {
    localStorage.removeItem(STORAGE_KEY);
    applySettingsSideEffects(DEFAULTS);
    setSettings(DEFAULTS);
  }, []);

  useEffect(() => {
    if (settings.theme !== "system") return;
    const mq = window.matchMedia("(prefers-color-scheme: light)");
    const onChange = () => applySettingsSideEffects(settings);
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

import "./styles.css";

/* ───────────── theme toggle ───────────── */

type Theme = "light" | "dark";

const THEME_KEY = "cellar-theme";
const root = document.documentElement;

const saved = localStorage.getItem(THEME_KEY);
if (saved === "light" || saved === "dark") {
  root.setAttribute("data-theme", saved);
}

function currentTheme(): Theme {
  return root.getAttribute("data-theme") === "light" ? "light" : "dark";
}

const themeListeners = new Set<(theme: Theme) => void>();

function setTheme(next: Theme) {
  root.setAttribute("data-theme", next);
  localStorage.setItem(THEME_KEY, next);
  for (const fn of themeListeners) fn(next);
}

const themeBtn = document.getElementById("themeBtn");
themeBtn?.addEventListener("click", () => {
  setTheme(currentTheme() === "dark" ? "light" : "dark");
});

/* ───────────── theme-aware screenshot swap ─────────────
   When the site is in light mode, swap each product screenshot
   to its light-theme variant. Light variants are looked up on
   load and only enabled if the asset actually exists, so the
   site stays correct until real light-theme screenshots ship.
   Naming convention: foo.png ↔ foo-light.png. */

type SwapEntry = { dark: string; light: string | null };
const swappable = new Map<HTMLImageElement, SwapEntry>();

function deriveLightPath(darkSrc: string): string {
  return darkSrc.replace(/\.(png|jpg|jpeg|webp|avif)(\?.*)?$/i, "-light.$1$2");
}

/* Test that a URL actually resolves to a loadable image.
   We can't just HEAD-probe: dev servers (and many static hosts)
   return 200 + HTML for missing assets via SPA fallback, which
   would fool the swap into pointing at a non-image response. */
function probe(url: string): Promise<boolean> {
  return new Promise((resolve) => {
    const probeImg = new Image();
    probeImg.onload = () => resolve(probeImg.naturalWidth > 0);
    probeImg.onerror = () => resolve(false);
    probeImg.src = url;
  });
}

function applyImageTheme(theme: Theme) {
  for (const [img, paths] of swappable) {
    const target = theme === "light" && paths.light ? paths.light : paths.dark;
    const current = img.getAttribute("src");
    if (current !== target) img.setAttribute("src", target);
  }
}

async function initImageSwap() {
  const candidates = Array.from(
    document.querySelectorAll<HTMLImageElement>("img[data-theme-swap]"),
  );

  await Promise.all(
    candidates.map(async (img) => {
      const dark = img.getAttribute("src");
      if (!dark) return;
      const lightPath = deriveLightPath(dark);
      const exists = await probe(lightPath);
      swappable.set(img, { dark, light: exists ? lightPath : null });
    }),
  );

  applyImageTheme(currentTheme());
  themeListeners.add(applyImageTheme);
}

initImageSwap();

/* ───────────── engine chips ───────────── */

type Engine = { name: string; color: string };

const engines: ReadonlyArray<Engine> = [
  { name: "PostgreSQL", color: "var(--eng-postgres)" },
  { name: "MySQL", color: "var(--eng-mysql)" },
  { name: "SQL Server", color: "var(--eng-mssql)" },
  { name: "Azure SQL", color: "var(--eng-azure)" },
  { name: "SQLite", color: "var(--eng-sqlite)" },
];

function renderEngineChips(el: HTMLElement | null) {
  if (!el) return;
  el.replaceChildren(
    ...engines.map((engine) => {
      const chip = document.createElement("span");
      chip.className = "eng-chip";
      const dot = document.createElement("span");
      dot.className = "eng-dot";
      dot.style.background = engine.color;
      chip.appendChild(dot);
      chip.append(engine.name);
      return chip;
    }),
  );
}

renderEngineChips(document.getElementById("engRow"));
renderEngineChips(document.getElementById("engRow2"));

/* ───────────── release download ───────────── */

const GITHUB_RELEASE_API = "https://api.github.com/repos/MRL-00/cellar/releases/latest";
const GITHUB_LATEST_RELEASE = "https://github.com/MRL-00/cellar/releases/latest";

type ReleaseAsset = {
  name: string;
  size: number;
  browser_download_url: string;
};

type GitHubRelease = {
  tag_name: string;
  assets: ReleaseAsset[];
};

function formatBytes(bytes: number): string {
  const mb = bytes / 1024 / 1024;
  return `${mb.toFixed(mb >= 10 ? 0 : 1)} MB`;
}

function setMacDownload(url: string, label?: string) {
  const links = document.querySelectorAll<HTMLAnchorElement>('[data-download="macos-aarch64"]');
  for (const link of links) {
    link.href = url;
    link.rel = "noopener";
    if (label) link.title = label;
  }
}

async function initDownloadLink() {
  setMacDownload(GITHUB_LATEST_RELEASE, "Open the latest Cellar release");

  try {
    const res = await fetch(GITHUB_RELEASE_API, {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!res.ok) return;

    const release = (await res.json()) as GitHubRelease;
    const appleSiliconDmg = release.assets.find((asset) =>
      /^Cellar_[^/]+_aarch64\.dmg$/i.test(asset.name),
    );
    if (!appleSiliconDmg) return;

    setMacDownload(
      appleSiliconDmg.browser_download_url,
      `Download Cellar ${release.tag_name} for Apple Silicon Macs`,
    );

    const sizeEls = document.querySelectorAll<HTMLElement>("[data-release-size]");
    for (const el of sizeEls) el.textContent = formatBytes(appleSiliconDmg.size);
  } catch {
    /* Keep the release-page fallback. */
  }
}

initDownloadLink();

/* ───────────── faq ───────────── */

type Qa = readonly [question: string, answer: string];

const qas: ReadonlyArray<Qa> = [
  [
    "Is Cellar really free?",
    "Yes. Cellar is free and open source under the MIT license. There is no paid tier, no account, and no usage limit. If you use the AI features, you pay your AI provider directly for tokens, and Cellar never marks that up.",
  ],
  [
    "Do I need an AI key to use it?",
    "No. Everything except the AI Assistant works with zero configuration. The AI features are optional and activate only when you add your own Anthropic or OpenAI key in Settings.",
  ],
  [
    "Where do my credentials and AI key live?",
    "In your operating system’s keychain, never in a plaintext config file and never synced to a server. Cellar is local-first: the only outbound connections are to your databases and, if enabled, your AI provider.",
  ],
  [
    "Which databases are supported?",
    "PostgreSQL, MySQL, SQL Server, Azure SQL, and SQLite, each with engine-specific connection fields, SSH tunneling, and SSL/TLS. More engines are on the roadmap.",
  ],
  [
    "What about Windows and Linux?",
    "The macOS build (Apple Silicon) is the first target. Windows and Linux builds are in active development. Star the repo or join Discord to hear when they land.",
  ],
  [
    "Can I trust the AI not to touch production?",
    "Yes. Read-only mode and a hard spend cap are on by default, generated SQL is shown as a reviewable diff before it runs, and you can redact column values before they’re sent as context.",
  ],
];

const PLUS_ICON =
  '<svg class="plus" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M12 5v14M5 12h14"/></svg>';

function renderFaq(el: HTMLElement | null) {
  if (!el) return;
  el.replaceChildren(
    ...qas.map(([question, answer]) => {
      const details = document.createElement("details");
      details.className = "qa";
      const summary = document.createElement("summary");
      summary.append(question);
      summary.insertAdjacentHTML("beforeend", PLUS_ICON);
      const p = document.createElement("p");
      p.className = "ans";
      p.textContent = answer;
      details.append(summary, p);
      return details;
    }),
  );
}

renderFaq(document.getElementById("faqList"));

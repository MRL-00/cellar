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

type Engine = { name: string; color: string; status: "supported" | "soon" };

const engines: ReadonlyArray<Engine> = [
  { name: "PostgreSQL", color: "var(--eng-postgres)", status: "supported" },
  { name: "SQL Server", color: "var(--eng-mssql)", status: "supported" },
  { name: "Azure SQL", color: "var(--eng-azure)", status: "supported" },
  { name: "Firestore", color: "var(--eng-firestore)", status: "supported" },
  { name: "MySQL", color: "var(--eng-mysql)", status: "soon" },
  { name: "SQLite", color: "var(--eng-sqlite)", status: "soon" },
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
      if (engine.status === "soon") {
        const status = document.createElement("span");
        status.className = "eng-status soon";
        status.textContent = "coming soon";
        chip.appendChild(status);
      }
      return chip;
    }),
  );
}

renderEngineChips(document.getElementById("engRow"));
renderEngineChips(document.getElementById("engRow2"));

/* ───────────── download menu ───────────── */

const downloadPickers = Array.from(document.querySelectorAll<HTMLElement>(".download-picker"));

function setDownloadMenu(picker: HTMLElement, open: boolean) {
  const trigger = picker.querySelector<HTMLButtonElement>("[data-download-trigger]");
  const menu = picker.querySelector<HTMLElement>(".download-menu");
  if (!trigger || !menu) return;

  trigger.setAttribute("aria-expanded", String(open));
  menu.hidden = !open;
}

function closeDownloadMenus(except?: HTMLElement) {
  for (const picker of downloadPickers) {
    if (picker !== except) setDownloadMenu(picker, false);
  }
}

for (const picker of downloadPickers) {
  const trigger = picker.querySelector<HTMLButtonElement>("[data-download-trigger]");
  const menu = picker.querySelector<HTMLElement>(".download-menu");
  if (!trigger || !menu) continue;

  trigger.addEventListener("click", () => {
    const willOpen = trigger.getAttribute("aria-expanded") !== "true";
    closeDownloadMenus(picker);
    setDownloadMenu(picker, willOpen);
  });

  trigger.addEventListener("keydown", (event) => {
    if (event.key !== "ArrowDown") return;
    event.preventDefault();
    closeDownloadMenus(picker);
    setDownloadMenu(picker, true);
    menu.querySelector<HTMLAnchorElement>("a")?.focus();
  });

  menu.addEventListener("click", (event) => {
    if (!(event.target instanceof Element)) return;
    if (event.target.closest("a")) setDownloadMenu(picker, false);
  });
}

document.addEventListener("click", (event) => {
  const target = event.target;
  if (!(target instanceof Node)) return;
  if (downloadPickers.some((picker) => picker.contains(target))) return;
  closeDownloadMenus();
});

document.addEventListener("keydown", (event) => {
  if (event.key === "Escape") closeDownloadMenus();
});

/* ───────────── release download ───────────── */

const GITHUB_RELEASES_URL = "https://github.com/MRL-00/cellar/releases";
const GITHUB_RELEASE_API = "https://api.github.com/repos/MRL-00/cellar/releases";

/* Installer asset filename → counts as a "download". Excludes checksum and
   signature sidecar files that ship alongside the real installers. */
function isInstaller(name: string): boolean {
  return /\.(dmg|pkg|exe|msi|appimage|deb|rpm)$/i.test(name);
}

const downloads = {
  "macos-arm64": {
    assetNames: ["Cellar-mac-arm64.dmg"],
    assetPattern: /^Cellar_.+_aarch64\.dmg$/,
    fallbackUrl: GITHUB_RELEASES_URL,
    label: "Apple Silicon Macs",
  },
  "macos-x64": {
    assetNames: ["Cellar-mac-x64.dmg"],
    assetPattern: /^Cellar_.+_x64\.dmg$/,
    fallbackUrl: GITHUB_RELEASES_URL,
    label: "Intel Macs",
  },
} as const;

type DownloadKey = keyof typeof downloads;

type ReleaseAsset = {
  name: string;
  size: number;
  browser_download_url: string;
  download_count: number;
};

type GitHubRelease = {
  tag_name: string;
  draft?: boolean;
  assets: ReleaseAsset[];
};

function formatBytes(bytes: number): string {
  const mb = bytes / 1024 / 1024;
  return `${mb.toFixed(mb >= 10 ? 0 : 1)} MB`;
}

function setDownloadLink(key: DownloadKey, url: string, label?: string) {
  const links = document.querySelectorAll<HTMLAnchorElement>(`[data-download="${key}"]`);
  for (const link of links) {
    link.href = url;
    link.rel = "noopener";
    if (label) link.title = label;
  }
}

function setDownloadButtonLabel(label: string) {
  const labels = document.querySelectorAll<HTMLElement>("[data-download-label]");
  for (const el of labels) el.textContent = label;
}

function findDownloadAsset(releases: GitHubRelease[], key: DownloadKey) {
  const download = downloads[key];
  for (const release of releases) {
    if (release.draft) continue;
    const asset = release.assets.find((candidate) => {
      return download.assetNames.some((name) => name === candidate.name) || download.assetPattern.test(candidate.name);
    });
    if (asset) return { release, asset };
  }
  return null;
}

/* Reveal the live download counter. Hidden by default so the site shows
   nothing (rather than a bare "0") until real downloads have accrued. */
function setDownloadCount(total: number) {
  const text = `${total.toLocaleString("en-US")} download${total === 1 ? "" : "s"}`;
  for (const el of document.querySelectorAll<HTMLElement>("[data-download-count]")) {
    el.textContent = text;
    el.hidden = false;
  }
  for (const sep of document.querySelectorAll<HTMLElement>("[data-download-count-sep]")) {
    sep.hidden = false;
  }
}

async function initDownloadLink() {
  for (const [key, download] of Object.entries(downloads) as [DownloadKey, (typeof downloads)[DownloadKey]][]) {
    setDownloadLink(key, download.fallbackUrl, `View Cellar releases for ${download.label}`);
  }

  try {
    const res = await fetch(GITHUB_RELEASE_API, {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!res.ok) return;

    const releases = (await res.json()) as GitHubRelease[];
    if (!Array.isArray(releases)) return;

    /* Total downloads across every published release, so the counter
       reflects the full history rather than just the current build. */
    const totalDownloads = releases.reduce(
      (sum, release) =>
        release.draft
          ? sum
          : sum +
            release.assets
              .filter((asset) => isInstaller(asset.name))
              .reduce((assetSum, asset) => assetSum + (asset.download_count ?? 0), 0),
      0,
    );
    if (totalDownloads > 0) setDownloadCount(totalDownloads);

    const availableSizes: number[] = [];

    for (const [key, download] of Object.entries(downloads) as [DownloadKey, (typeof downloads)[DownloadKey]][]) {
      const match = findDownloadAsset(releases, key);
      if (!match) continue;

      setDownloadLink(
        key,
        match.asset.browser_download_url,
        `Download Cellar ${match.release.tag_name} for ${download.label}`,
      );
      availableSizes.push(match.asset.size);
    }

    if (availableSizes.length === 0) return;

    setDownloadButtonLabel("Download for Mac");

    const sizeEls = document.querySelectorAll<HTMLElement>("[data-release-size]");
    const largestSize = Math.max(...availableSizes);
    for (const el of sizeEls) el.textContent = formatBytes(largestSize);
  } catch {
    /* Keep the GitHub Releases fallback links. */
  }
}

initDownloadLink();

/* ───────────── faq ───────────── */

type Qa = readonly [question: string, answer: string];

const qas: ReadonlyArray<Qa> = [
  [
    "What is Cellar?",
    "Cellar is a desktop database client for developers, DBAs, and analysts. It helps you browse schemas, run SQL, inspect execution plans, and review data changes before committing.",
  ],
  [
    "Is Cellar really free?",
    "Yes. Cellar is free and open source under the MIT license. There is no paid tier, no account, and no usage limit. AI provider support is coming soon, and will use your own provider key directly.",
  ],
  [
    "Do I need an AI key to use it?",
    "No. The core database workflow does not need AI. The AI features are coming soon and are being designed around bring-your-own provider keys.",
  ],
  [
    "Where do my credentials and AI key live?",
    "Database credentials live in your operating system’s keychain, never in a plaintext config file and never synced to a server. Cellar is local-first: the only required outbound connections are to your databases.",
  ],
  [
    "Which databases are supported?",
    "PostgreSQL, SQL Server, Azure SQL, and Firestore are supported today. MySQL and SQLite are coming soon.",
  ],
  [
    "What about Windows and Linux?",
    "The macOS build (Apple Silicon) is the first target. Windows and Linux builds are in active development. Star the repo or join Discord to hear when they land.",
  ],
  [
    "Can I trust the AI not to touch production?",
    "That is the design goal for the coming-soon AI work: generated SQL should be visible, gated, and reviewable before execution, with production connections clearly marked and easy to keep read-only.",
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

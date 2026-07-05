import "./styles.css";

/* ───────────── engine chips ───────────── */

type Engine = { name: string; color: string; status: "supported" | "soon" };

const engines: ReadonlyArray<Engine> = [
  { name: "PostgreSQL", color: "var(--eng-postgres)", status: "supported" },
  { name: "SQL Server", color: "var(--eng-mssql)", status: "supported" },
  { name: "Azure SQL", color: "var(--eng-azure)", status: "supported" },
  { name: "Firestore", color: "var(--eng-firestore)", status: "supported" },
  { name: "MySQL", color: "var(--eng-mysql)", status: "supported" },
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
    "PostgreSQL, SQL Server, Azure SQL, Firestore, and MySQL are supported today. SQLite is coming soon.",
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

/* ───────────── scroll reveal ─────────────
   Fade + lift elements as they enter the viewport. Elements only
   start hidden when `.js` is on <html> (set in the head), so a
   no-JS page shows everything. Siblings stagger; reduced-motion
   skips the animation entirely. */

const revealables = Array.from(document.querySelectorAll<HTMLElement>(".reveal"));
const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/* Enable the hide-then-reveal CSS only now that the reveal logic is running.
   If anything above threw, `.reveal-ready` is never set and content stays
   visible rather than stuck hidden. */
document.documentElement.classList.add("reveal-ready");

if (reduceMotion || !("IntersectionObserver" in window)) {
  for (const el of revealables) el.classList.add("in");
} else {
  // Stagger each element against its reveal siblings under the same parent.
  const indexInParent = new Map<HTMLElement, number>();
  for (const el of revealables) {
    const siblings = el.parentElement
      ? Array.from(el.parentElement.children).filter((c) => c.classList.contains("reveal"))
      : [el];
    indexInParent.set(el, siblings.indexOf(el));
  }

  const observer = new IntersectionObserver(
    (entries, obs) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        const el = entry.target as HTMLElement;
        el.style.transitionDelay = `${Math.min(indexInParent.get(el) ?? 0, 5) * 70}ms`;
        el.classList.add("in");
        obs.unobserve(el);
      }
    },
    { rootMargin: "0px 0px -10% 0px", threshold: 0.1 },
  );

  for (const el of revealables) observer.observe(el);
}

/* ───────────── header ─────────────
   Transparent over the hero, solid once scrolled; scroll-spy marks the
   active section in the nav; an accessible disclosure drives the mobile
   menu (Escape, click-outside, link-tap, and resize all close it). */

const nav = document.getElementById("siteNav");

if (nav) {
  const setScrolled = () => nav.classList.toggle("scrolled", window.scrollY > 8);
  setScrolled();
  window.addEventListener("scroll", setScrolled, { passive: true });

  // Scroll-spy: light up the nav link for whichever section crosses mid-viewport.
  const linkBySection = new Map<string, HTMLAnchorElement[]>();
  const spyLinks = nav.querySelectorAll<HTMLAnchorElement>(
    '.nav-links a[href^="#"], .nav-mobile a[href^="#"]:not(.btn)',
  );
  for (const link of spyLinks) {
    const id = link.getAttribute("href")?.slice(1);
    if (!id) continue;
    const links = linkBySection.get(id) ?? [];
    links.push(link);
    linkBySection.set(id, links);
  }
  const sections = [...linkBySection.keys()]
    .map((id) => document.getElementById(id))
    .filter((el): el is HTMLElement => el !== null);

  if ("IntersectionObserver" in window && sections.length) {
    const spy = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          for (const links of linkBySection.values())
            for (const l of links) l.classList.remove("active");
          for (const l of linkBySection.get(entry.target.id) ?? []) l.classList.add("active");
        }
      },
      { rootMargin: "-50% 0px -50% 0px", threshold: 0 },
    );
    for (const section of sections) spy.observe(section);
  }

  // Mobile menu disclosure.
  const toggle = document.getElementById("navToggle");
  const menu = document.getElementById("navMobile");
  if (toggle && menu) {
    const setMenu = (open: boolean) => {
      toggle.setAttribute("aria-expanded", String(open));
      toggle.setAttribute("aria-label", open ? "Close menu" : "Open menu");
      menu.hidden = !open;
    };
    toggle.addEventListener("click", () => {
      setMenu(toggle.getAttribute("aria-expanded") !== "true");
    });
    menu.addEventListener("click", (event) => {
      if (event.target instanceof Element && event.target.closest("a")) setMenu(false);
    });
    document.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && toggle.getAttribute("aria-expanded") === "true") {
        setMenu(false);
        toggle.focus();
      }
    });
    document.addEventListener("click", (event) => {
      if (event.target instanceof Node && !nav.contains(event.target)) setMenu(false);
    });
    window.addEventListener("resize", () => {
      if (window.innerWidth > 880) setMenu(false);
    });
  }
}

/* ───────────── hero scene playback ─────────────
   The hero backdrop video has no autoplay attribute, so it stays on
   its poster frame until we opt in: only when motion is allowed, and
   only while the hero is on screen (pause when scrolled away to save
   battery/CPU). Reduced-motion users keep the still poster. */

const heroVideo = document.getElementById("heroVideo") as HTMLVideoElement | null;

if (heroVideo && !reduceMotion) {
  const playSafe = () => heroVideo.play().catch(() => {});
  if ("IntersectionObserver" in window) {
    const videoObserver = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) playSafe();
          else heroVideo.pause();
        }
      },
      { threshold: 0.05 },
    );
    videoObserver.observe(heroVideo);
  } else {
    playSafe();
  }
}

import { useEffect, useMemo, useState } from "react";
import {
  ArrowDown,
  ArrowUpRight,
  ChevronDown,
  Code2,
  Command,
  Database,
  Download,
  LockKeyhole,
  Menu,
  MousePointer2,
  ShieldCheck,
  Sparkles,
  X,
  Zap,
} from "lucide-react";
import { buttonVariants } from "./components/ui/button";
import { cn } from "./lib/utils";

const GITHUB = "https://github.com/MRL-00/cellar";
const RELEASES = `${GITHUB}/releases`;
const RELEASE_API = "https://api.github.com/repos/MRL-00/cellar/releases";

type ReleaseAsset = { name: string; size: number; browser_download_url: string };
type Release = { draft?: boolean; tag_name: string; assets: ReleaseAsset[] };
type Installer = { href: string; meta: string };
type DownloadInfo = { label: string; meta: string; silicon: Installer; intel: Installer };

const defaultDownload: DownloadInfo = {
  label: "Choose Mac version",
  meta: "macOS 13+",
  silicon: { href: RELEASES, meta: "M1 or newer" },
  intel: { href: RELEASES, meta: "Intel processor" },
};
let downloadRequest: Promise<DownloadInfo> | undefined;

const engines = ["PostgreSQL", "SQL Server", "Azure SQL", "Firestore", "MySQL", "SQLite soon"];

const faqs = [
  [
    "What can I use today?",
    "Cellar is early-access software with live schema browsing, query execution, table data, and a reviewable local edit workflow. PostgreSQL, SQL Server, Azure SQL, Firestore, and MySQL are supported today. SQLite is next.",
  ],
  [
    "Does Cellar upload my data?",
    "No. Cellar is a desktop app with no account or cloud sync. Database credentials stay in your operating system keychain, and telemetry is off by default.",
  ],
  [
    "Is it actually free?",
    "Yes. Cellar is MIT licensed, with no paid tier and no feature gate. Read the source, change it, or ship a pull request.",
  ],
  [
    "What about AI features?",
    "They are in active development. The design is bring-your-own-provider, with visible context and SQL that you inspect before it runs. The core database workflow does not require AI.",
  ],
] as const;

function formatSize(bytes: number) {
  const mb = bytes / 1024 / 1024;
  return `${mb.toFixed(mb >= 10 ? 0 : 1)} MB`;
}

function lookupDownload() {
  downloadRequest ??= fetch(RELEASE_API, { headers: { Accept: "application/vnd.github+json" } })
    .then((response) => (response.ok ? response.json() : Promise.reject()))
    .then((releases: Release[]) => {
      for (const release of releases) {
        if (release.draft) continue;
        const silicon = release.assets.find(
          ({ name }) => name === "Cellar-mac-arm64.dmg" || /^Cellar_.+_aarch64\.dmg$/.test(name),
        );
        const intel = release.assets.find(
          ({ name }) => name === "Cellar-mac-x64.dmg" || /^Cellar_.+_x64\.dmg$/.test(name),
        );
        if (!silicon && !intel) continue;
        return {
          label: "Download for Mac",
          meta: `${release.tag_name} · macOS 13+`,
          silicon: {
            href: silicon?.browser_download_url ?? RELEASES,
            meta: silicon ? `M1 or newer · ${formatSize(silicon.size)}` : "M1 or newer · View releases",
          },
          intel: {
            href: intel?.browser_download_url ?? RELEASES,
            meta: intel ? `Intel processor · ${formatSize(intel.size)}` : "Intel processor · View releases",
          },
        };
      }
      return defaultDownload;
    })
    .catch(() => defaultDownload);
  return downloadRequest;
}

function useDownload() {
  const [download, setDownload] = useState(defaultDownload);
  useEffect(() => {
    let active = true;
    lookupDownload().then((result) => {
      if (active) setDownload(result);
    });
    return () => {
      active = false;
    };
  }, []);

  return download;
}

function usePageMotion() {
  useEffect(() => {
    const root = document.documentElement;
    const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const revealItems = document.querySelectorAll<HTMLElement>("[data-reveal]");
    const reveal = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          entry.target.setAttribute("data-visible", "true");
          reveal.unobserve(entry.target);
        }
      },
      { threshold: 0.12, rootMargin: "0px 0px -4%" },
    );
    revealItems.forEach((item) => reveal.observe(item));

    if (reducedMotion) return () => reveal.disconnect();

    const parallaxItems = [...document.querySelectorAll<HTMLElement>("[data-parallax]")];
    let frame = 0;
    const paint = () => {
      frame = 0;
      for (const item of parallaxItems) {
        const speed = Number(item.dataset.parallax ?? 0.08);
        const rect = item.getBoundingClientRect();
        const distance = (window.innerHeight * 0.5 - (rect.top + rect.height * 0.5)) * speed;
        item.style.setProperty("--parallax", `${distance.toFixed(2)}px`);
      }
      root.style.setProperty("--scroll-y", `${window.scrollY}px`);
    };
    const queuePaint = () => {
      if (!frame) frame = requestAnimationFrame(paint);
    };
    const moveSpotlight = (event: PointerEvent) => {
      root.style.setProperty("--pointer-x", `${event.clientX}px`);
      root.style.setProperty("--pointer-y", `${event.clientY}px`);
    };
    paint();
    window.addEventListener("scroll", queuePaint, { passive: true });
    window.addEventListener("resize", queuePaint);
    window.addEventListener("pointermove", moveSpotlight, { passive: true });
    return () => {
      reveal.disconnect();
      if (frame) cancelAnimationFrame(frame);
      window.removeEventListener("scroll", queuePaint);
      window.removeEventListener("resize", queuePaint);
      window.removeEventListener("pointermove", moveSpotlight);
    };
  }, []);
}

function DownloadPicker({
  className,
  compact = false,
  align = "left",
  side = "bottom",
}: {
  className?: string;
  compact?: boolean;
  align?: "left" | "center" | "right";
  side?: "bottom" | "top";
}) {
  const download = useDownload();
  const alignment = {
    left: "left-0",
    center: "left-1/2 -translate-x-1/2",
    right: "right-0",
  }[align];
  return (
    <details className={cn("group relative z-30 inline-block", className)}>
      <summary
        className={cn(
          buttonVariants({ size: compact ? "default" : "lg" }),
          "w-full cursor-pointer list-none marker:hidden [&::-webkit-details-marker]:hidden",
        )}
      >
        <Download className="size-4" aria-hidden="true" />
        {compact ? "Download" : download.label}
        <ChevronDown className="size-4 transition-transform duration-300 ease-out-quint group-open:rotate-180" aria-hidden="true" />
      </summary>
      <div
        className={cn(
          "absolute z-50 w-[min(19rem,calc(100vw-2.5rem))] rounded-2xl border border-line bg-oxide p-2 text-left text-paper shadow-[0_24px_70px_oklch(0.03_0.003_255/0.65)]",
          side === "bottom" ? "top-full mt-2" : "bottom-full mb-2",
          alignment,
        )}
        aria-label="Choose Mac installer"
      >
        {([
          ["M", "Apple Silicon", download.silicon],
          ["I", "Intel Mac", download.intel],
        ] as const).map(([mark, label, installer]) => (
          <a
            key={label}
            href={installer.href}
            className="flex items-center gap-3 rounded-xl px-3 py-3 transition-colors hover:bg-paper/7 focus-visible:bg-paper/7 focus-visible:outline-none"
            rel="noopener"
          >
            <span className="grid size-9 shrink-0 place-items-center rounded-full border border-line bg-paper/5 font-mono text-xs text-paper-muted">
              {mark}
            </span>
            <span className="grid gap-0.5">
              <strong className="text-sm font-medium text-paper">{label}</strong>
              <span className="font-mono text-[10px] uppercase tracking-[0.06em] text-paper-dim">{installer.meta}</span>
            </span>
          </a>
        ))}
      </div>
    </details>
  );
}

function Nav() {
  const [open, setOpen] = useState(false);
  const links = useMemo(
    () => [
      ["Product", "#product"],
      ["Privacy", "#privacy"],
      ["Open source", "#open-source"],
    ],
    [],
  );

  return (
    <header className="fixed inset-x-0 top-0 z-50 border-b border-transparent bg-ink/70 backdrop-blur-xl supports-[backdrop-filter]:bg-ink/62">
      <nav className="mx-auto flex h-17 max-w-[1480px] items-center px-5 md:px-9" aria-label="Primary navigation">
        <a href="#top" className="group flex items-center gap-3" aria-label="Cellar home">
          <img
            className="size-11 transition-transform duration-500 ease-out-quint group-hover:rotate-3"
            src="/assets/cellar-mark-mono-white.svg"
            width="44"
            height="44"
            alt=""
          />
          <span className="text-base font-semibold tracking-[-0.03em]">CELLAR</span>
        </a>
        <div className="ml-auto hidden items-center gap-8 md:flex">
          {links.map(([label, href]) => (
            <a key={href} href={href} className="text-sm text-paper-muted transition-colors hover:text-paper">
              {label}
            </a>
          ))}
          <a href={GITHUB} className="text-sm text-paper-muted transition-colors hover:text-paper" rel="noopener">
            GitHub
          </a>
          <DownloadPicker compact align="right" />
        </div>
        <button
          className="ml-auto grid size-10 place-items-center rounded-full border border-line text-paper md:hidden"
          type="button"
          aria-label={open ? "Close navigation" : "Open navigation"}
          aria-expanded={open}
          onClick={() => setOpen((value) => !value)}
        >
          {open ? <X className="size-4" /> : <Menu className="size-4" />}
        </button>
      </nav>
      {open && (
        <div className="border-t border-line bg-ink px-5 py-4 md:hidden">
          <div className="grid gap-1">
            {links.map(([label, href]) => (
              <a key={href} href={href} className="py-3 text-lg" onClick={() => setOpen(false)}>
                {label}
              </a>
            ))}
            <a href={GITHUB} className="py-3 text-lg" rel="noopener">
              GitHub
            </a>
            <DownloadPicker className="mt-3 w-full" compact align="right" />
          </div>
        </div>
      )}
    </header>
  );
}

function Hero() {
  const download = useDownload();
  return (
    <section id="top" className="hero-shell relative min-h-[980px] overflow-hidden border-b border-line pt-17 lg:min-h-[100svh]">
      <div className="pointer-glow" aria-hidden="true" />
      <div className="absolute inset-0 overflow-hidden" aria-hidden="true">
        <div className="hero-orbit hero-orbit-one" data-parallax="0.035" />
        <div className="hero-orbit hero-orbit-two" data-parallax="-0.025" />
        <div className="hero-index">01</div>
      </div>
      <div className="relative mx-auto grid min-h-[calc(100svh-4.25rem)] max-w-[1480px] items-center gap-12 px-5 py-20 md:px-9 lg:grid-cols-[0.82fr_1.18fr] lg:py-24">
        <div className="relative z-10 max-w-[680px] lg:-top-16" data-reveal>
          <div className="mb-8 flex items-center gap-3 font-mono text-[11px] uppercase tracking-[0.16em] text-paper-dim">
            <span className="pulse-dot" />
            Open source · Early access
          </div>
          <h1 className="text-[clamp(4rem,8.2vw,9.7rem)] font-medium leading-[0.78] tracking-[-0.075em] text-paper">
            Data,
            <br />
            without
            <br />
            the drag.
          </h1>
          <p className="mt-9 max-w-[34rem] text-[clamp(1rem,1.5vw,1.28rem)] leading-relaxed text-paper-muted">
            A fast desktop database client for people who live in SQL. Browse schemas, edit data, and review every change before it commits.
          </p>
          <div className="mt-8 flex flex-wrap items-center gap-3">
            <DownloadPicker />
            <a href="#product" className={buttonVariants({ variant: "outline", size: "lg" })}>
              See it move <ArrowDown className="size-4" aria-hidden="true" />
            </a>
          </div>
          <p className="mt-4 font-mono text-[11px] uppercase tracking-[0.08em] text-paper-dim">{download.meta}</p>
        </div>

        <div className="relative min-h-[520px] lg:min-h-[720px]" data-reveal>
          <div className="absolute left-[4%] top-[3%] z-20 hidden items-center gap-2 rounded-full border border-coral/35 bg-ink px-3 py-2 font-mono text-[10px] uppercase tracking-[0.12em] text-coral shadow-2xl md:flex">
            <Zap className="size-3" /> Live workspace
          </div>
          <div className="product-stage" data-parallax="0.075">
            <img
              src="/assets/cellar-main.png"
              alt="Cellar workspace with database schemas, table data, and a query assistant"
              width="3040"
              height="1786"
            />
          </div>
          <div className="query-strip query-strip-one" data-parallax="-0.12" aria-hidden="true">
            <span>SELECT</span> customer_id, status <span>FROM</span> orders
          </div>
          <div className="query-strip query-strip-two" data-parallax="0.18" aria-hidden="true">
            48 rows <span>·</span> 12 ms <span>·</span> no pending changes
          </div>
        </div>
      </div>
      <div className="absolute bottom-5 left-5 hidden items-center gap-3 font-mono text-[10px] uppercase tracking-[0.16em] text-paper-dim md:flex">
        <MousePointer2 className="size-3" /> Scroll to inspect
      </div>
    </section>
  );
}

function EngineRail() {
  const repeated = [...engines, ...engines];
  return (
    <div className="overflow-hidden border-b border-line bg-coral py-4 text-ink" aria-label="Supported database engines">
      <div className="engine-rail flex w-max items-center">
        {repeated.map((engine, index) => (
          <div key={`${engine}-${index}`} className="flex items-center gap-5 px-7 font-mono text-[11px] font-medium uppercase tracking-[0.13em]">
            <Database className="size-4" strokeWidth={1.8} />
            {engine}
            <span className="text-ink/30">◆</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function ProductStory() {
  return (
    <section id="product" className="relative overflow-hidden bg-paper text-ink">
      <div className="absolute right-0 top-0 font-mono text-[30vw] leading-none tracking-[-0.12em] text-ink/[0.025]" aria-hidden="true">02</div>
      <div className="mx-auto max-w-[1480px] px-5 py-28 md:px-9 md:py-40">
        <div className="grid items-end gap-12 lg:grid-cols-[0.7fr_1.3fr]" data-reveal>
          <div>
            <p className="section-kicker text-ink/55">One workspace, no ceremony</p>
            <h2 className="mt-7 max-w-[9ch] text-[clamp(3.6rem,7vw,8.3rem)] font-medium leading-[0.84] tracking-[-0.07em]">
              Keep the whole database in view.
            </h2>
          </div>
          <p className="max-w-[38rem] pb-2 text-lg leading-relaxed text-ink/60 lg:ml-auto">
            Move from schema to query to result without rebuilding your mental map. Cellar keeps navigation, SQL, data, and pending changes in one honest frame.
          </p>
        </div>

        <div className="story-window mt-20 md:mt-28" data-reveal data-parallax="0.04">
          <img
            src="/assets/cellar-main-light.png"
            alt="Cellar light workspace showing schemas and table data"
            width="1920"
            height="1080"
            loading="lazy"
          />
          <div className="story-callout story-callout-left">Schema stays one click away</div>
          <div className="story-callout story-callout-right">Pending edits stay visible</div>
        </div>

        <div className="mt-24 grid border-t border-ink/15 md:grid-cols-3 md:divide-x md:divide-ink/15" data-reveal>
          {[
            ["01", "Browse", "Live schema trees make the shape of a database legible before you write a line."],
            ["02", "Query", "A focused SQL workspace keeps execution close to the data it produces."],
            ["03", "Review", "Edits remain pending until you inspect the diff and choose to commit."],
          ].map(([index, title, copy]) => (
            <article key={title} className="border-b border-ink/15 py-8 md:border-b-0 md:px-8 md:first:pl-0 md:last:pr-0">
              <div className="flex items-center justify-between font-mono text-[10px] uppercase tracking-[0.15em] text-ink/40">
                {index}<span className="size-1.5 rounded-full bg-coral" />
              </div>
              <h3 className="mt-14 text-3xl font-medium tracking-[-0.05em]">{title}</h3>
              <p className="mt-4 max-w-[31ch] leading-relaxed text-ink/55">{copy}</p>
            </article>
          ))}
        </div>
      </div>
    </section>
  );
}

function CommandSection() {
  return (
    <section className="relative overflow-hidden border-y border-line bg-oxide py-28 md:py-40">
      <div className="mx-auto grid max-w-[1480px] items-center gap-16 px-5 md:px-9 lg:grid-cols-[1.08fr_0.92fr]">
        <div className="relative" data-reveal>
          <div className="command-shot" data-parallax="0.06">
            <img src="/assets/cmd.png" alt="Cellar command palette open above the database workspace" width="3024" height="1770" loading="lazy" />
          </div>
          <div className="key-cloud key-cloud-one" data-parallax="-0.18" aria-hidden="true"><Command /> K</div>
          <div className="key-cloud key-cloud-two" data-parallax="0.22" aria-hidden="true">↵</div>
        </div>
        <div className="lg:pl-[8%]" data-reveal>
          <p className="section-kicker">Keyboard first</p>
          <h2 className="mt-7 max-w-[10ch] text-[clamp(3.3rem,6vw,7rem)] font-medium leading-[0.86] tracking-[-0.07em]">
            Move at thought speed.
          </h2>
          <p className="mt-8 max-w-[32rem] text-lg leading-relaxed text-paper-muted">
            Open tables, switch connections, and reach workspace actions from one command palette. The mouse is welcome, never required.
          </p>
          <div className="mt-12 divide-y divide-line border-y border-line">
            {[
              ["Command palette", "⌘ K"],
              ["Run statement", "⌘ ↵"],
              ["Commit changes", "⌘ S"],
            ].map(([label, keys]) => (
              <div key={label} className="flex items-center justify-between py-4 text-sm text-paper-muted">
                <span>{label}</span><kbd>{keys}</kbd>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}

function PrivacySection() {
  return (
    <section id="privacy" className="relative overflow-hidden bg-ink py-28 md:py-44">
      <div className="privacy-rings" aria-hidden="true" data-parallax="0.035"><LockKeyhole /></div>
      <div className="relative mx-auto max-w-[1480px] px-5 md:px-9">
        <div className="grid gap-16 lg:grid-cols-[1.2fr_0.8fr]" data-reveal>
          <div>
            <p className="section-kicker text-coral">Local means local</p>
            <h2 className="mt-7 max-w-[11ch] text-[clamp(3.7rem,8vw,9.2rem)] font-medium leading-[0.82] tracking-[-0.075em]">
              Your data does not visit us.
            </h2>
          </div>
          <div className="self-end lg:pb-5">
            <p className="max-w-[34rem] text-lg leading-relaxed text-paper-muted">
              No account. No cloud sync. No hosted proxy between you and your database. Credentials live in the OS keychain, and telemetry starts off.
            </p>
            <div className="mt-10 grid gap-4">
              {([
                [ShieldCheck, "No telemetry by default"],
                [LockKeyhole, "Credentials stay in your keychain"],
                [Sparkles, "BYO AI provider, coming soon"],
              ] as const).map(([Icon, label]) => (
                <div key={label} className="flex items-center gap-4 border-b border-line pb-4 text-sm">
                  <span className="grid size-9 place-items-center rounded-full bg-paper/6 text-coral"><Icon className="size-4" /></span>
                  {label}
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

function OpenSourceSection() {
  return (
    <section id="open-source" className="relative overflow-hidden bg-coral text-ink">
      <div className="mx-auto grid min-h-[720px] max-w-[1480px] items-center gap-14 px-5 py-28 md:px-9 lg:grid-cols-[0.85fr_1.15fr]">
        <div data-reveal>
          <Code2 className="size-9" strokeWidth={1.5} />
          <h2 className="mt-8 max-w-[9ch] text-[clamp(3.8rem,7vw,8.5rem)] font-medium leading-[0.82] tracking-[-0.075em]">
            Open by default.
          </h2>
          <p className="mt-8 max-w-[33rem] text-lg leading-relaxed text-ink/65">
            MIT licensed. No paid tier. No mystery service. Every product decision and every line of code is open to inspect.
          </p>
          <a href={GITHUB} className={cn(buttonVariants({ size: "lg" }), "mt-9 bg-ink text-paper hover:bg-ink/90")} rel="noopener">
            Explore the source <ArrowUpRight className="size-4" />
          </a>
        </div>
        <div className="source-panel" data-reveal data-parallax="0.05">
          <div className="source-panel-head">
            <span className="flex items-center gap-2"><span className="size-2 rounded-full bg-coral" /> cellar-core</span>
            <span>MIT</span>
          </div>
          <pre aria-label="Illustrative Cellar driver interface"><code><span className="code-muted">// Every driver speaks one typed contract.</span>{"\n"}<span className="code-key">pub trait</span> Driver {` {`}{"\n"}  <span className="code-key">async fn</span> connect(&amp;self) -&gt; Result&lt;Connection&gt;;{"\n"}  <span className="code-key">async fn</span> introspect(&amp;self) -&gt; Result&lt;Schema&gt;;{"\n"}  <span className="code-key">async fn</span> execute(&amp;self, query: Query);{"\n"}{`}`}</code></pre>
          <div className="source-panel-foot"><span>Rust + TypeScript</span><span>Contributions welcome ↗</span></div>
        </div>
      </div>
    </section>
  );
}

function FaqSection() {
  return (
    <section className="bg-paper py-28 text-ink md:py-40">
      <div className="mx-auto grid max-w-[1480px] gap-16 px-5 md:px-9 lg:grid-cols-[0.72fr_1.28fr]">
        <div data-reveal>
          <p className="section-kicker text-ink/45">The useful details</p>
          <h2 className="mt-7 text-[clamp(3.4rem,6vw,7rem)] font-medium leading-[0.85] tracking-[-0.07em]">Before you install.</h2>
        </div>
        <div className="border-t border-ink/20" data-reveal>
          {faqs.map(([question, answer]) => (
            <details key={question} className="group border-b border-ink/20">
              <summary className="flex cursor-pointer list-none items-center justify-between gap-6 py-6 text-xl font-medium tracking-[-0.025em] marker:hidden">
                {question}<ChevronDown className="size-5 shrink-0 transition-transform duration-300 ease-out-quint group-open:rotate-180" />
              </summary>
              <p className="max-w-[60ch] pb-7 leading-relaxed text-ink/60">{answer}</p>
            </details>
          ))}
        </div>
      </div>
    </section>
  );
}

function FinalCta() {
  return (
    <section className="relative overflow-hidden border-t border-line bg-ink py-28 text-center md:py-44">
      <div className="final-mark" aria-hidden="true"><img src="/assets/cellar-mark-mono-white.svg" alt="" /></div>
      <div className="relative mx-auto flex max-w-[960px] flex-col items-center px-5" data-reveal>
        <p className="section-kicker text-coral">Free · Open source · macOS</p>
        <h2 className="mt-8 text-[clamp(4rem,9vw,10rem)] font-medium leading-[0.78] tracking-[-0.08em]">Meet your data.</h2>
        <p className="mt-8 max-w-[38rem] text-lg leading-relaxed text-paper-muted">Download the early-access build, connect a database, and get back to the work.</p>
        <DownloadPicker className="mt-10" align="center" side="top" />
      </div>
    </section>
  );
}

function Footer() {
  return (
    <footer className="border-t border-line bg-ink px-5 py-9 text-sm text-paper-dim md:px-9">
      <div className="mx-auto flex max-w-[1480px] flex-col gap-6 md:flex-row md:items-center">
        <a href="#top" className="flex items-center gap-3 text-paper">
          <img className="size-8" src="/assets/cellar-mark-mono-white.svg" width="32" height="32" alt="" /> Cellar
        </a>
        <p className="md:ml-auto">© 2026 Cellar · MIT License</p>
        <a href={`${GITHUB}/issues`} className="transition-colors hover:text-paper" rel="noopener">Report an issue</a>
        <a href={GITHUB} className="transition-colors hover:text-paper" rel="noopener">GitHub ↗</a>
      </div>
    </footer>
  );
}

export function App() {
  usePageMotion();
  return (
    <>
      <a className="skip-link" href="#product">Skip to content</a>
      <Nav />
      <main>
        <Hero />
        <EngineRail />
        <ProductStory />
        <CommandSection />
        <PrivacySection />
        <OpenSourceSection />
        <FaqSection />
        <FinalCta />
      </main>
      <Footer />
    </>
  );
}

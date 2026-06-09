#!/usr/bin/env node
/**
 * Report Cellar download counts straight from the GitHub Releases API.
 *
 * GitHub counts every download of a release asset for free, so this needs no
 * tracking on the site itself. Run it anytime:
 *
 *   pnpm --filter @cellar/site downloads:stats        # human-readable table
 *   pnpm --filter @cellar/site downloads:stats --json # machine-readable JSON
 *
 * Set GITHUB_TOKEN to raise the API rate limit (60→5000/hr) and include drafts.
 */

const REPO = process.env.CELLAR_REPO ?? "MRL-00/cellar";
const API = `https://api.github.com/repos/${REPO}/releases?per_page=100`;
const asJson = process.argv.includes("--json");

/** Installer asset filename → counts as a real download (skips checksums/sigs). */
function isInstaller(name) {
  return /\.(dmg|pkg|exe|msi|appimage|deb|rpm)$/i.test(name);
}

function formatBytes(bytes) {
  const mb = bytes / 1024 / 1024;
  return `${mb.toFixed(mb >= 10 ? 0 : 1)} MB`;
}

function downloadsLabel(count) {
  return `${count.toLocaleString("en-US")} download${count === 1 ? "" : "s"}`;
}

async function fetchReleases() {
  const headers = { Accept: "application/vnd.github+json" };
  if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;

  const res = await fetch(API, { headers });
  if (!res.ok) {
    const body = await res.text().catch(() => "");
    throw new Error(`GitHub API ${res.status} ${res.statusText}\n${body}`);
  }
  return res.json();
}

function summarize(releases) {
  const perRelease = releases.map((release) => {
    const assets = release.assets
      .filter((asset) => isInstaller(asset.name))
      .map((asset) => ({ name: asset.name, downloads: asset.download_count ?? 0, size: asset.size }));
    const downloads = assets.reduce((sum, asset) => sum + asset.downloads, 0);
    return {
      tag: release.tag_name,
      name: release.name,
      published_at: release.published_at,
      prerelease: release.prerelease,
      downloads,
      assets,
    };
  });
  const total = perRelease.reduce((sum, release) => sum + release.downloads, 0);
  return { repo: REPO, total, releases: perRelease };
}

function printReport({ repo, total, releases }) {
  if (releases.length === 0) {
    console.log(`No releases published yet for ${repo}.`);
    console.log("Download counts will start accruing once a release with installer assets is published.");
    return;
  }

  console.log(`\nCellar downloads — ${repo}\n`);
  for (const release of releases) {
    const tag = release.prerelease ? `${release.tag} (prerelease)` : release.tag;
    const when = release.published_at ? release.published_at.slice(0, 10) : "unpublished";
    console.log(`${tag}  ·  ${when}  ·  ${downloadsLabel(release.downloads)}`);
    for (const asset of release.assets) {
      const downloads = String(asset.downloads).padStart(7);
      console.log(`   ${downloads}  ${asset.name}  (${formatBytes(asset.size)})`);
    }
    if (release.assets.length === 0) console.log("   (no installer assets)");
    console.log("");
  }
  console.log(`TOTAL: ${downloadsLabel(total)} across ${releases.length} release(s)\n`);
}

try {
  const summary = summarize(await fetchReleases());
  if (asJson) {
    console.log(JSON.stringify(summary, null, 2));
  } else {
    printReport(summary);
  }
} catch (error) {
  console.error(`Failed to fetch download stats: ${error.message}`);
  process.exit(1);
}

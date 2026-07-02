// The updater manifest ships the full CHANGELOG so users jumping multiple
// versions see every release in between. Trim it client-side to the sections
// newer than the installed version.

function compareVersions(a: string, b: string): number {
  const pa = a.split(".").map(Number);
  const pb = b.split(".").map(Number);
  for (let i = 0; i < 3; i++) {
    const d = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (d) return d;
  }
  return 0;
}

// Keep only `## x.y.z` sections newer than the installed version. Notes with
// no version headings (single-release bodies, the old placeholder) pass
// through untouched, as do all notes when the installed version is unknown.
export function notesSince(notes: string, installed: string): string {
  if (!installed) return notes;
  const lines = notes.replace(/\r\n/g, "\n").split("\n");
  const out: string[] = [];
  let keep = false;
  let sawVersionHeading = false;
  for (const line of lines) {
    const m = /^##\s+(\d+\.\d+\.\d+)\s*$/.exec(line);
    if (m) {
      sawVersionHeading = true;
      keep = compareVersions(m[1]!, installed) > 0;
    }
    if (keep) out.push(line);
  }
  if (!sawVersionHeading) return notes;
  return out.join("\n").trim();
}

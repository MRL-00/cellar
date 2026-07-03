import { describe, expect, it } from "vitest";
import { notesSince } from "./releaseNotes";

const changelog = [
  "# Changelog",
  "",
  "## 0.3.2",
  "",
  "- fonts",
  "",
  "## 0.3.1",
  "",
  "- theme",
  "",
  "## 0.2.0",
  "",
  "- grid",
].join("\n");

describe("notesSince", () => {
  it("keeps only sections newer than the installed version", () => {
    const out = notesSince(changelog, "0.2.0");
    expect(out).toContain("## 0.3.2");
    expect(out).toContain("## 0.3.1");
    expect(out).not.toContain("## 0.2.0");
    expect(out).not.toContain("# Changelog");
  });

  it("keeps only the latest section when one version behind", () => {
    const out = notesSince(changelog, "0.3.1");
    expect(out).toContain("## 0.3.2");
    expect(out).not.toContain("## 0.3.1");
  });

  it("compares numerically, not lexically", () => {
    expect(notesSince("## 0.10.0\n- ten", "0.9.0")).toContain("## 0.10.0");
  });

  it("matches headings with a date suffix", () => {
    const out = notesSince("## 0.3.2 - 2026-07-01\n- fonts\n## 0.3.1 - 2026-06-30\n- theme", "0.3.1");
    expect(out).toContain("- fonts");
    expect(out).not.toContain("- theme");
  });

  it("treats malformed segments as 0, over-showing rather than dropping", () => {
    // "0.3.2-beta.1" parses as 0.3.0, so 0.3.x sections still show.
    expect(notesSince("## 0.4.0\n- next", "0.3.2-beta.1")).toContain("- next");
    expect(notesSince(changelog, "0.3.2-beta.1")).toContain("## 0.3.1");
  });

  it("passes through notes without version headings", () => {
    const single = "### Features\n- something";
    expect(notesSince(single, "0.2.0")).toBe(single);
  });

  it("passes everything through when installed version is unknown", () => {
    expect(notesSince(changelog, "")).toBe(changelog);
  });
});

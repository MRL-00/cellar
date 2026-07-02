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

  it("passes through notes without version headings", () => {
    const single = "### Features\n- something";
    expect(notesSince(single, "0.2.0")).toBe(single);
  });

  it("passes everything through when installed version is unknown", () => {
    expect(notesSince(changelog, "")).toBe(changelog);
  });
});

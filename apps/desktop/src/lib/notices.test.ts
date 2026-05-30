import { describe, expect, it } from "vitest";

import {
  countNoticeSeverities,
  formatNoticeTime,
  toneForSeverity,
} from "./notices";
import type { DatabaseNotice } from "@cellar/ipc";

function notice(severity: DatabaseNotice["severity"]): DatabaseNotice {
  return {
    severity,
    code: null,
    message: severity,
    detail: null,
    hint: null,
    timestamp: "2026-05-30T01:02:03Z",
    connection_id: "local",
    database: "postgres",
    query_id: "q1",
  };
}

describe("notice helpers", () => {
  it("counts severities without dropping zero-value buckets", () => {
    const counts = countNoticeSeverities([
      notice("notice"),
      notice("warning"),
      notice("warning"),
    ]);

    expect(counts.notice).toBe(1);
    expect(counts.warning).toBe(2);
    expect(counts.error).toBe(0);
  });

  it("maps severities to log tones", () => {
    expect(toneForSeverity("error")).toBe("danger");
    expect(toneForSeverity("warning")).toBe("warning");
    expect(toneForSeverity("notice")).toBe("info");
    expect(toneForSeverity("debug")).toBe("muted");
  });

  it("keeps invalid timestamps readable", () => {
    expect(formatNoticeTime("not-a-date")).toBe("not-a-date");
  });
});

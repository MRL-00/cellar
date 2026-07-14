import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { MessagesView } from "./BottomMessagesPanel";
import type { QueryMessage } from "../lib/queryMessages";

describe("MessagesView", () => {
  it("renders an honest empty state without an active tab", () => {
    const html = renderToStaticMarkup(
      <MessagesView messages={[]} hasActiveTab={false} />,
    );

    expect(html).toContain("No active tab");
    expect(html).toContain("Open a table or query tab");
  });

  it("renders execution feedback with severity filters and metrics", () => {
    const html = renderToStaticMarkup(
      <MessagesView
        messages={[
          message("msg-1", "success", "Loaded public.orders: 2 rows in 12 ms."),
          message("msg-2", "warning", "Result hit row limit 500."),
        ]}
      />,
    );

    expect(html).toContain("success");
    expect(html).toContain("warning");
    expect(html).toContain("Loaded public.orders");
    expect(html).toContain("12 ms");
    expect(html).toContain("2 rows");
    expect(html).toContain("Resize time column");
    expect(html).toContain("Resize level column");
    expect(html).toContain("Resize source column");
    expect(html).toContain("Resize message column");
    expect(html).toContain("Resize metrics column");
  });
});

function message(
  id: string,
  severity: QueryMessage["severity"],
  text: string,
): QueryMessage {
  return {
    id,
    tabId: "tab-1",
    connectionId: "conn-1",
    severity,
    source: "execution",
    text,
    timestamp: "2026-05-30T00:00:00.000Z",
    durationMs: 12,
    rowCount: 2,
  };
}

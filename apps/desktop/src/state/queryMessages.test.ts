import { beforeEach, describe, expect, it } from "vitest";
import { useQueryMessages } from "./queryMessages";

describe("useQueryMessages", () => {
  beforeEach(() => {
    useQueryMessages.getState().clear();
  });

  it("normalizes and appends pending messages", () => {
    useQueryMessages.getState().addMessage({
      id: "msg-1",
      tabId: "tab-1",
      connectionId: "conn-1",
      severity: "info",
      source: "client",
      text: "Loading",
      timestamp: "2026-05-30T00:00:00.000Z",
    });

    expect(useQueryMessages.getState().messages).toEqual([
      {
        id: "msg-1",
        tabId: "tab-1",
        connectionId: "conn-1",
        severity: "info",
        source: "client",
        text: "Loading",
        timestamp: "2026-05-30T00:00:00.000Z",
      },
    ]);
  });

  it("replaces only messages for the target tab", () => {
    const store = useQueryMessages.getState();
    store.addMessages([
      fixedMessage("old-1", "tab-1"),
      fixedMessage("keep-1", "tab-2"),
    ]);

    store.replaceForTab("tab-1", [fixedMessage("new-1", "tab-1")]);

    expect(useQueryMessages.getState().messages.map((m) => m.id)).toEqual([
      "keep-1",
      "new-1",
    ]);
  });
});

function fixedMessage(id: string, tabId: string) {
  return {
    id,
    tabId,
    connectionId: "conn-1",
    severity: "info" as const,
    source: "client" as const,
    text: id,
    timestamp: "2026-05-30T00:00:00.000Z",
  };
}

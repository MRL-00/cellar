import { beforeEach, describe, expect, it, vi } from "vitest";

// Mock only the network functions of @cellar/ai; keep the real prompt/provider
// helpers. The keychain commands run against the @cellar/ipc web-mode mock
// (isTauri is false under vitest), which holds keys in memory.
vi.mock("@cellar/ai", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@cellar/ai")>();
  return {
    ...actual,
    listGeminiModels: vi.fn(async () => [
      { id: "gemini-2.5-pro", label: "Gemini 2.5 Pro", ctx: "1m" },
    ]),
    generateContent: vi.fn(async () => ({
      text: "```sql\nSELECT 1;\n```",
      usage: { promptTokens: 1, completionTokens: 2, totalTokens: 3 },
    })),
  };
});

import { commands, unwrap } from "@cellar/ipc";
import * as ai from "@cellar/ai";
import { useAi } from "./ai";

function reset() {
  useAi.setState({
    providerId: "google",
    modelId: null,
    openAiAuthMode: "apiKey",
    models: [],
    modelsLoading: false,
    modelsError: null,
    keyConfigured: false,
    configured: false,
    oauthStatus: null,
    login: null,
    authLoading: false,
    openAiThreadId: null,
    messages: [],
    sending: false,
    initialized: false,
  });
}

beforeEach(async () => {
  await commands.aiDeleteKey("google");
  await commands.aiDeleteKey("openai");
  await commands.aiOpenaiLogout();
  reset();
  vi.clearAllMocks();
});

describe("useAi store", () => {
  it("saveKey stores the key and discovers models", async () => {
    await useAi.getState().saveKey("AIzaTEST");
    const s = useAi.getState();
    expect(s.keyConfigured).toBe(true);
    expect(s.models.map((m) => m.id)).toEqual(["gemini-2.5-pro"]);
    expect(s.modelId).toBe("gemini-2.5-pro");
    expect(ai.listGeminiModels).toHaveBeenCalledOnce();
  });

  it("send appends the user turn then the model reply", async () => {
    await useAi.getState().saveKey("AIzaTEST");
    await useAi.getState().send("generate", "top customers", "ctx");

    const msgs = useAi.getState().messages;
    expect(msgs).toHaveLength(2);
    expect(msgs[0]).toMatchObject({ role: "user", topic: "generate", content: "top customers" });
    // the API turn wraps the preset instruction + context around the raw text
    expect(msgs[0]!.apiContent).toContain("Schema context:\nctx");
    expect(msgs[1]!.role).toBe("model");
    expect(msgs[1]!.topic).toBe("generate");
    expect(msgs[1]!.error).toBeFalsy();
    expect(msgs[1]!.content).toContain("SELECT 1;");
    expect(useAi.getState().sending).toBe(false);
  });

  it("send without a key records an error turn", async () => {
    useAi.setState({ modelId: "gemini-2.5-pro" });
    await useAi.getState().send("ask", "hello");
    const msgs = useAi.getState().messages;
    expect(msgs).toHaveLength(2);
    expect(msgs[1]).toMatchObject({ role: "model", error: true });
    expect(ai.generateContent).not.toHaveBeenCalled();
  });

  it("retrying after a failed turn sends clean alternating history", async () => {
    // First turn fails (no key) -> error model entry recorded.
    useAi.setState({ modelId: "gemini-2.5-pro" });
    await useAi.getState().send("ask", "first try");
    expect(useAi.getState().messages.at(-1)).toMatchObject({ error: true });

    // Now configure a key and send again; history must not contain the
    // orphaned user turn from the failed attempt.
    await useAi.getState().saveKey("AIzaTEST");
    await useAi.getState().send("ask", "second try");

    const sent = vi.mocked(ai.generateContent).mock.calls.at(-1)![0];
    const roles = sent.messages.map((m) => m.role);
    // strictly alternating, ending on the new user turn
    expect(roles).toEqual(["user"]);
    expect(sent.messages[0]!.content).toContain("second try");
  });

  it("clearKey revokes the key and resets models", async () => {
    await useAi.getState().saveKey("AIzaTEST");
    await useAi.getState().clearKey();
    const s = useAi.getState();
    expect(s.keyConfigured).toBe(false);
    expect(s.models).toEqual([]);
    expect(await commands.aiHasKey("google")).toMatchObject({ data: false });
  });

  it("newThread clears the conversation", () => {
    useAi.setState({
      messages: [{ id: "x", role: "user", content: "hi" }],
      openAiThreadId: "thread-1",
    });
    useAi.getState().newThread();
    expect(useAi.getState().messages).toEqual([]);
    expect(useAi.getState().openAiThreadId).toBeNull();
  });

  it("keeps an OpenAI API key out of the renderer generation path", async () => {
    useAi.setState({ providerId: "openai", openAiAuthMode: "apiKey" });
    const loadKey = vi.spyOn(commands, "aiLoadKey");

    await useAi.getState().saveKey("sk-test");
    await useAi.getState().send("ask", "write a query");

    expect(loadKey).not.toHaveBeenCalled();
    expect(useAi.getState().modelId).toBe("gpt-5.6-sol");
    expect(useAi.getState().messages.at(-1)).toMatchObject({
      role: "model",
    });
    expect(useAi.getState().messages.at(-1)?.error).toBeFalsy();
  });

  it("rejects direct renderer reads of the OpenAI key", async () => {
    await commands.aiStoreKey("openai", "sk-test");
    await expect(unwrap(commands.aiLoadKey("openai"))).rejects.toThrow(
      "backend-only",
    );
  });

  it("connects ChatGPT OAuth and retains the app-server thread id", async () => {
    useAi.setState({ providerId: "openai", openAiAuthMode: "chatgpt" });
    await useAi.getState().startOpenAiLogin("browser");
    const status = await useAi.getState().refreshOAuthStatus();
    expect(status.signed_in).toBe(true);

    await useAi.getState().refreshModels();
    await useAi.getState().send("ask", "hello");
    expect(useAi.getState().openAiThreadId).toBe("mock-openai-thread");
  });
});

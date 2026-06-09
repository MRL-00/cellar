import { describe, expect, it, vi } from "vitest";
import {
  GeminiError,
  generateContent,
  humanizeTokenLimit,
  listGeminiModels,
  modelVersion,
} from "./gemini";
import type { FetchLike } from "./types";

function jsonResponse(status: number, body: unknown): Awaited<ReturnType<FetchLike>> {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
    text: async () => JSON.stringify(body),
  };
}

describe("humanizeTokenLimit", () => {
  it("formats millions and thousands", () => {
    expect(humanizeTokenLimit(1_048_576)).toBe("1m");
    expect(humanizeTokenLimit(2_000_000)).toBe("2m");
    expect(humanizeTokenLimit(200_000)).toBe("200k");
    expect(humanizeTokenLimit(900)).toBe("900");
  });
  it("returns undefined for missing/zero", () => {
    expect(humanizeTokenLimit(0)).toBeUndefined();
    expect(humanizeTokenLimit(undefined)).toBeUndefined();
  });
});

describe("modelVersion", () => {
  it("extracts major.minor", () => {
    expect(modelVersion("gemini-2.5-pro")).toBeCloseTo(2.05);
    expect(modelVersion("gemini-1.5-flash")).toBeCloseTo(1.05);
    expect(modelVersion("gemini-pro")).toBe(-1);
  });
});

describe("listGeminiModels", () => {
  it("filters to chat-capable gemini models and sorts newest first", async () => {
    const fetchImpl = vi.fn(async () =>
      jsonResponse(200, {
        models: [
          {
            name: "models/gemini-1.5-flash",
            displayName: "Gemini 1.5 Flash",
            inputTokenLimit: 1_000_000,
            supportedGenerationMethods: ["generateContent"],
          },
          {
            name: "models/gemini-2.5-pro",
            displayName: "Gemini 2.5 Pro",
            inputTokenLimit: 2_000_000,
            supportedGenerationMethods: ["generateContent", "countTokens"],
          },
          {
            name: "models/embedding-001",
            displayName: "Embedding",
            supportedGenerationMethods: ["embedContent"],
          },
          {
            name: "models/gemini-1.0-pro-vision",
            displayName: "Vision",
            // no generateContent -> filtered out
            supportedGenerationMethods: ["countTokens"],
          },
        ],
      }),
    );

    const models = await listGeminiModels("AIzaTEST", { fetchImpl });
    expect(models.map((m) => m.id)).toEqual([
      "gemini-2.5-pro",
      "gemini-1.5-flash",
    ]);
    expect(models[0]).toMatchObject({ label: "Gemini 2.5 Pro", ctx: "2m" });
    // key must be passed in the query string, never a header/body
    const url = fetchImpl.mock.calls[0]![0] as string;
    expect(url).toContain("key=AIzaTEST");
  });

  it("falls back to id when displayName is missing", async () => {
    const fetchImpl = vi.fn(async () =>
      jsonResponse(200, {
        models: [
          {
            name: "models/gemini-2.0-flash",
            supportedGenerationMethods: ["generateContent"],
          },
        ],
      }),
    );
    const models = await listGeminiModels("k", { fetchImpl });
    expect(models[0]).toMatchObject({ id: "gemini-2.0-flash", label: "gemini-2.0-flash" });
  });

  it("throws a GeminiError with the provider message on failure", async () => {
    const fetchImpl = vi.fn(async () =>
      jsonResponse(400, { error: { message: "API key not valid" } }),
    );
    await expect(listGeminiModels("bad", { fetchImpl })).rejects.toMatchObject({
      name: "GeminiError",
      status: 400,
      message: "API key not valid",
    });
  });

  it("rejects an empty key without calling fetch", async () => {
    const fetchImpl = vi.fn();
    await expect(listGeminiModels("", { fetchImpl })).rejects.toBeInstanceOf(
      GeminiError,
    );
    expect(fetchImpl).not.toHaveBeenCalled();
  });
});

describe("generateContent", () => {
  it("posts contents and system instruction, returns joined text + usage", async () => {
    const fetchImpl = vi.fn(async () =>
      jsonResponse(200, {
        candidates: [{ content: { parts: [{ text: "SELECT " }, { text: "1;" }] } }],
        usageMetadata: {
          promptTokenCount: 10,
          candidatesTokenCount: 5,
          totalTokenCount: 15,
        },
      }),
    );

    const result = await generateContent(
      {
        apiKey: "k",
        model: "gemini-2.5-pro",
        systemInstruction: "be terse",
        messages: [{ role: "user", content: "give me one" }],
      },
      { fetchImpl },
    );

    expect(result.text).toBe("SELECT 1;");
    expect(result.usage).toEqual({
      promptTokens: 10,
      completionTokens: 5,
      totalTokens: 15,
    });

    const [url, init] = fetchImpl.mock.calls[0]!;
    expect(url).toContain("gemini-2.5-pro:generateContent");
    const body = JSON.parse((init as { body: string }).body);
    expect(body.systemInstruction.parts[0].text).toBe("be terse");
    expect(body.contents[0]).toEqual({
      role: "user",
      parts: [{ text: "give me one" }],
    });
  });

  it("throws when the prompt is blocked", async () => {
    const fetchImpl = vi.fn(async () =>
      jsonResponse(200, { promptFeedback: { blockReason: "SAFETY" } }),
    );
    await expect(
      generateContent(
        { apiKey: "k", model: "m", messages: [{ role: "user", content: "x" }] },
        { fetchImpl },
      ),
    ).rejects.toThrow(/blocked/i);
  });

  it("throws on an empty candidate", async () => {
    const fetchImpl = vi.fn(async () =>
      jsonResponse(200, { candidates: [{ finishReason: "MAX_TOKENS", content: { parts: [] } }] }),
    );
    await expect(
      generateContent(
        { apiKey: "k", model: "m", messages: [{ role: "user", content: "x" }] },
        { fetchImpl },
      ),
    ).rejects.toThrow(/MAX_TOKENS/);
  });

  it("surfaces HTTP errors", async () => {
    const fetchImpl = vi.fn(async () =>
      jsonResponse(429, { error: { message: "Quota exceeded" } }),
    );
    await expect(
      generateContent(
        { apiKey: "k", model: "m", messages: [{ role: "user", content: "x" }] },
        { fetchImpl },
      ),
    ).rejects.toMatchObject({ status: 429, message: "Quota exceeded" });
  });
});

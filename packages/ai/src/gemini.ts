// Gemini (Google Generative Language API) client.
//
// Per SPEC §3 and §6.7 the request goes directly from the user's machine to the
// provider with their own key — Cellar never proxies. This module is pure data
// plumbing: it takes an API key + payload and talks HTTP. The key is supplied by
// the caller (loaded from the OS keychain) and is never persisted here.

import type {
  AiModel,
  FetchLike,
  GenerateRequest,
  GenerateResult,
} from "./types";
import { getProvider } from "./providers";

const BASE = getProvider("google").endpoint; // ponytail: single source of truth for the endpoint URL

/** Error carrying the provider's HTTP status and message so the UI can show a
 * useful reason (bad key, quota, model not found) instead of a generic failure. */
export class GeminiError extends Error {
  override readonly name = "GeminiError";
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
  }
}

interface ListModelsResponse {
  models?: Array<{
    name?: string;
    displayName?: string;
    description?: string;
    inputTokenLimit?: number;
    supportedGenerationMethods?: string[];
  }>;
}

interface GenerateContentResponse {
  candidates?: Array<{
    content?: { parts?: Array<{ text?: string }> };
    finishReason?: string;
  }>;
  promptFeedback?: { blockReason?: string };
  usageMetadata?: {
    promptTokenCount?: number;
    candidatesTokenCount?: number;
    totalTokenCount?: number;
  };
  error?: { message?: string };
}

/** Collapse a raw token limit into a compact display string (`1m`, `200k`). */
export function humanizeTokenLimit(n: number | undefined): string | undefined {
  if (!n || n <= 0) return undefined;
  if (n >= 1_000_000) return `${Math.round(n / 1_000_000)}m`;
  if (n >= 1_000) return `${Math.round(n / 1_000)}k`;
  return String(n);
}

/** Pull a comparable version number out of a model id (`gemini-2.5-pro` -> 2.5).
 * Used only to surface newer models first; unknown shapes sort last. */
export function modelVersion(id: string): number {
  const m = /(\d+)(?:\.(\d+))?/.exec(id);
  if (!m) return -1;
  const major = Number(m[1]);
  const minor = m[2] ? Number(m[2]) : 0;
  return major + minor / 100;
}

function sortModels(models: AiModel[]): AiModel[] {
  return [...models].sort((a, b) => {
    const dv = modelVersion(b.id) - modelVersion(a.id);
    if (dv !== 0) return dv;
    return a.id.localeCompare(b.id);
  });
}

async function errorFrom(
  res: { status: number; json: () => Promise<unknown>; text: () => Promise<string> },
  fallback: string,
): Promise<GeminiError> {
  try {
    const body = (await res.json()) as { error?: { message?: string } };
    const msg = body?.error?.message;
    if (msg) return new GeminiError(res.status, msg);
  } catch {
    // fall through to status-only message
  }
  return new GeminiError(res.status, `${fallback} (HTTP ${res.status})`);
}

/** Discover the models this API key can call. Filters to chat-capable Gemini
 * models (those exposing `generateContent`) and surfaces newest first. */
export async function listGeminiModels(
  apiKey: string,
  opts: { fetchImpl?: FetchLike; signal?: AbortSignal } = {},
): Promise<AiModel[]> {
  if (!apiKey) throw new GeminiError(401, "An API key is required to list models.");
  const f = opts.fetchImpl ?? (fetch as unknown as FetchLike); // ponytail: inlined resolveFetch
  const res = await f(
    `${BASE}/models?pageSize=1000&key=${encodeURIComponent(apiKey)}`,
    { method: "GET", signal: opts.signal },
  );
  if (!res.ok) throw await errorFrom(res, "Failed to list models");

  const body = (await res.json()) as ListModelsResponse;
  const models: AiModel[] = [];
  for (const m of body.models ?? []) {
    const name = m.name ?? "";
    const id = name.startsWith("models/") ? name.slice("models/".length) : name;
    if (!id) continue;
    const methods = m.supportedGenerationMethods ?? [];
    if (!methods.includes("generateContent")) continue;
    if (!id.startsWith("gemini")) continue;
    models.push({
      id,
      label: m.displayName?.trim() || id,
      ctx: humanizeTokenLimit(m.inputTokenLimit),
    });
  }
  return sortModels(models);
}

/** Run one non-streaming generation. Returns the joined candidate text plus
 * token usage. Throws {@link GeminiError} on HTTP failure or a blocked prompt. */
export async function generateContent(
  req: GenerateRequest,
  opts: { fetchImpl?: FetchLike } = {},
): Promise<GenerateResult> {
  if (!req.apiKey) throw new GeminiError(401, "An API key is required.");
  if (!req.model) throw new GeminiError(400, "No model selected.");
  const f = opts.fetchImpl ?? (fetch as unknown as FetchLike);

  const body: Record<string, unknown> = {
    contents: req.messages.map((m) => ({
      role: m.role,
      parts: [{ text: m.content }],
    })),
  };
  if (req.systemInstruction) {
    body.systemInstruction = { parts: [{ text: req.systemInstruction }] };
  }

  const res = await f(
    `${BASE}/models/${encodeURIComponent(req.model)}:generateContent?key=${encodeURIComponent(req.apiKey)}`,
    {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
      signal: req.signal,
    },
  );
  if (!res.ok) throw await errorFrom(res, "Generation failed");

  const data = (await res.json()) as GenerateContentResponse;
  if (data.promptFeedback?.blockReason) {
    throw new GeminiError(
      400,
      `Request blocked by the provider (${data.promptFeedback.blockReason}).`,
    );
  }
  const parts = data.candidates?.[0]?.content?.parts ?? [];
  const text = parts.map((p) => p.text ?? "").join("");
  if (!text) {
    const reason = data.candidates?.[0]?.finishReason;
    throw new GeminiError(
      502,
      reason
        ? `The model returned no text (finish reason: ${reason}).`
        : "The model returned an empty response.",
    );
  }

  const usage = data.usageMetadata
    ? {
        promptTokens: data.usageMetadata.promptTokenCount ?? 0,
        completionTokens: data.usageMetadata.candidatesTokenCount ?? 0,
        totalTokens: data.usageMetadata.totalTokenCount ?? 0,
      }
    : undefined;

  return { text, usage };
}

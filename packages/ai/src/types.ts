// Shared AI types. This package is provider-agnostic at the type level: the
// concrete provider clients live either alongside it (Gemini) or behind typed
// desktop IPC when credentials must stay out of the renderer (OpenAI).

/** Providers Cellar knows about. */
export type AiProviderId = "google" | "anthropic" | "openai" | "local" | "custom";

/** OpenAI supports usage-based Platform API keys and ChatGPT subscription
 * access. The latter is implemented by the local Codex app-server OAuth flow. */
export type OpenAiAuthMode = "apiKey" | "chatgpt";

/** The four task presets the AI panel exposes, plus the free-form `ask`. These
 * map 1:1 to the bottom-bar buttons in the right-hand AI panel. */
export type AiTopic = "generate" | "explain" | "optimize" | "migrate" | "ask";

/** A model offered by a provider. For Gemini these are discovered live from the
 * API key rather than hardcoded. */
export interface AiModel {
  /** API id used in request paths, e.g. `gemini-2.5-pro` (no `models/` prefix). */
  id: string;
  /** Human label, e.g. `Gemini 2.5 Pro`. Falls back to `id` when absent. */
  label: string;
  /** Context-window descriptor for display, e.g. `1m`, `200k`. */
  ctx?: string;
}

/** One turn in a conversation. Gemini uses `model` (not `assistant`) for the
 * provider's role; we keep that vocabulary end-to-end to avoid translation. */
export interface ChatMessage {
  role: "user" | "model";
  content: string;
}

/** Token accounting returned by the provider, when available. */
export interface TokenUsage {
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
}

export interface GenerateRequest {
  apiKey: string;
  model: string;
  messages: ChatMessage[];
  /** Optional system instruction prepended out-of-band from the turn history. */
  systemInstruction?: string;
  signal?: AbortSignal;
}

export interface GenerateResult {
  text: string;
  usage?: TokenUsage;
}

/** The subset of `fetch` this package relies on. Injectable so tests can run
 * without a real network and callers can swap in a Tauri http client later. */
export type FetchLike = (
  input: string,
  init?: {
    method?: string;
    headers?: Record<string, string>;
    body?: string;
    signal?: AbortSignal;
  },
) => Promise<{
  ok: boolean;
  status: number;
  json: () => Promise<unknown>;
  text: () => Promise<string>;
}>;

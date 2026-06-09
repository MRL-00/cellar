import type { AiProviderId } from "./types";

/** Static metadata for the provider picker. `enabled` gates whether a provider
 * can be selected — only Google/Gemini is wired today; everything else renders
 * with a "coming soon" tag and is non-interactive. */
export interface ProviderMeta {
  id: AiProviderId;
  label: string;
  /** Short mono subtitle, e.g. `gemini-* family`. */
  sub: string;
  enabled: boolean;
  /** Leading hint shown next to the API-key input. */
  keyPrefix: string;
  /** Base endpoint, shown read-only in settings. */
  endpoint: string;
}

export const PROVIDERS: ProviderMeta[] = [
  {
    id: "google",
    label: "Google",
    sub: "gemini-* family",
    enabled: true,
    keyPrefix: "AIza",
    endpoint: "https://generativelanguage.googleapis.com/v1beta",
  },
  {
    id: "anthropic",
    label: "Anthropic",
    sub: "claude-* family",
    enabled: false,
    keyPrefix: "sk-ant-",
    endpoint: "https://api.anthropic.com/v1",
  },
  {
    id: "openai",
    label: "OpenAI",
    sub: "gpt-* family",
    enabled: false,
    keyPrefix: "sk-",
    endpoint: "https://api.openai.com/v1",
  },
  {
    id: "local",
    label: "Local",
    sub: "Ollama, LM Studio",
    enabled: false,
    keyPrefix: "none",
    endpoint: "http://localhost:11434/v1",
  },
  {
    id: "custom",
    label: "Custom",
    sub: "OpenAI-compatible URL",
    enabled: false,
    keyPrefix: "key",
    endpoint: "https://example.invalid/v1",
  },
];

export function getProvider(id: AiProviderId): ProviderMeta {
  const p = PROVIDERS.find((x) => x.id === id);
  if (!p) throw new Error(`unknown AI provider: ${id}`);
  return p;
}

/** The provider Cellar defaults to and the only one wired end-to-end. */
export const DEFAULT_PROVIDER: AiProviderId = "google";

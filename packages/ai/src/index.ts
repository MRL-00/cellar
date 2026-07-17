// AI providers, prompts, and context building.
//
// Today this wires Gemini (Google Generative Language API) end-to-end; other
// providers are declared in `providers.ts` but disabled until implemented.

export type {
  AiProviderId,
  AiTopic,
  AiModel,
  ChatMessage,
  TokenUsage,
  GenerateRequest,
  GenerateResult,
  FetchLike,
} from "./types";

export {
  PROVIDERS,
  DEFAULT_PROVIDER,
  getProvider,
  type ProviderMeta,
} from "./providers";

export {
  GeminiError,
  listGeminiModels,
  generateContent,
  humanizeTokenLimit,
  modelVersion,
} from "./gemini";

export {
  SYSTEM_PROMPT,
  TOPICS,
  ORDERED_TOPICS,
  buildUserPrompt,
  formatTodayContext,
  type TopicMeta,
} from "./prompts";

export {
  buildSchemaContext,
  type SchemaContextInput,
  type ContextTable,
  type ContextColumn,
  type ContextForeignKey,
} from "./context";

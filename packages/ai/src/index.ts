// AI providers, prompts, and context building.
//
// Gemini runs directly from the renderer. OpenAI and DeepSeek run behind typed
// desktop IPC so their API credentials never enter the webview.

export type {
  AiProviderId,
  OpenAiAuthMode,
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

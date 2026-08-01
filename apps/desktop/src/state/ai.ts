import { create } from "zustand";
import {
  buildUserPrompt,
  DEFAULT_PROVIDER,
  GeminiError,
  generateContent,
  listGeminiModels,
  SYSTEM_PROMPT,
  type AiModel,
  type AiProviderId,
  type AiTopic,
  type ChatMessage,
  type OpenAiAuthMode,
  type TokenUsage,
} from "@cellar/ai";
import {
  commands,
  unwrap,
  type OpenAiLoginMethod,
  type OpenAiLoginStart,
  type OpenAiOAuthStatus,
} from "@cellar/ipc";

/** One rendered row in the AI thread. `content` is what we show; `apiContent`
 * (when set) is what was actually sent to the provider. */
export interface AiChatEntry {
  id: string;
  role: "user" | "model";
  topic?: AiTopic;
  content: string;
  apiContent?: string;
  error?: boolean;
  usage?: TokenUsage;
}

interface Persisted {
  providerId: AiProviderId;
  modelId: string | null;
  openAiAuthMode: OpenAiAuthMode;
}

const STORAGE_KEY = "cellar.ai.v2";
const DEFAULT_OPENAI_AUTH: OpenAiAuthMode = "apiKey";

function defaults(): Persisted {
  return {
    providerId: DEFAULT_PROVIDER,
    modelId: null,
    openAiAuthMode: DEFAULT_OPENAI_AUTH,
  };
}

function loadPersisted(): Persisted {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return defaults();
    const parsed = JSON.parse(raw) as Partial<Persisted>;
    return {
      providerId: parsed.providerId ?? DEFAULT_PROVIDER,
      modelId: parsed.modelId ?? null,
      openAiAuthMode: parsed.openAiAuthMode ?? DEFAULT_OPENAI_AUTH,
    };
  } catch {
    return defaults();
  }
}

function persist(value: Persisted) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
  } catch {
    // Non-fatal: selection just will not survive a restart.
  }
}

let entrySeq = 0;
let modelDiscoverySeq = 0;

function nextId(): string {
  entrySeq += 1;
  return `ai-${entrySeq}`;
}

function describeError(error: unknown): string {
  if (error instanceof GeminiError) return error.message;
  if (error instanceof Error) return error.message;
  return "Unexpected error talking to the provider.";
}

function cleanHistory(messages: AiChatEntry[]): ChatMessage[] {
  const history: ChatMessage[] = [];
  for (const message of messages) {
    if (message.role === "model" && message.error) {
      if (history.at(-1)?.role === "user") history.pop();
      continue;
    }
    history.push({
      role: message.role,
      content:
        message.role === "user"
          ? (message.apiContent ?? message.content)
          : message.content,
    });
  }
  return history;
}

interface AiStore {
  providerId: AiProviderId;
  modelId: string | null;
  openAiAuthMode: OpenAiAuthMode;
  models: AiModel[];
  modelsLoading: boolean;
  modelsError: string | null;
  keyConfigured: boolean;
  configured: boolean;
  oauthStatus: OpenAiOAuthStatus | null;
  login: OpenAiLoginStart | null;
  authLoading: boolean;
  openAiThreadId: string | null;
  messages: AiChatEntry[];
  sending: boolean;
  initialized: boolean;

  init: () => Promise<void>;
  setProvider: (id: AiProviderId) => void;
  setOpenAiAuthMode: (mode: OpenAiAuthMode) => void;
  setModel: (id: string) => void;
  saveKey: (key: string) => Promise<void>;
  clearKey: () => Promise<void>;
  refreshModels: () => Promise<void>;
  refreshOAuthStatus: () => Promise<OpenAiOAuthStatus>;
  startOpenAiLogin: (method: OpenAiLoginMethod) => Promise<OpenAiLoginStart>;
  cancelOpenAiLogin: () => Promise<void>;
  logoutOpenAi: () => Promise<void>;
  send: (
    topic: AiTopic,
    text: string,
    context?: string,
    lookupHits?: string,
  ) => Promise<void>;
  newThread: () => void;
}

export const useAi = create<AiStore>((set, get) => ({
  ...loadPersisted(),
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

  async init() {
    if (get().initialized) return;
    set({ initialized: true, modelsError: null });
    try {
      const { providerId, openAiAuthMode } = get();
      if (providerId === "openai" && openAiAuthMode === "chatgpt") {
        const status = await get().refreshOAuthStatus();
        if (status.signed_in) await get().refreshModels();
        return;
      }
      const hasKey = await unwrap(commands.aiHasKey(providerId));
      set({ keyConfigured: hasKey, configured: hasKey });
      if (hasKey) await get().refreshModels();
    } catch (error) {
      set({ modelsError: describeError(error), configured: false });
    }
  },

  setProvider(id) {
    modelDiscoverySeq += 1;
    set({
      providerId: id,
      modelId: null,
      initialized: false,
      keyConfigured: false,
      configured: false,
      oauthStatus: null,
      login: null,
      openAiThreadId: null,
      models: [],
      modelsLoading: false,
      modelsError: null,
    });
    persist({
      providerId: id,
      modelId: null,
      openAiAuthMode: get().openAiAuthMode,
    });
    void get().init();
  },

  setOpenAiAuthMode(mode) {
    modelDiscoverySeq += 1;
    set({
      openAiAuthMode: mode,
      modelId: null,
      initialized: false,
      keyConfigured: false,
      configured: false,
      oauthStatus: null,
      login: null,
      openAiThreadId: null,
      models: [],
      modelsLoading: false,
      modelsError: null,
    });
    persist({ providerId: get().providerId, modelId: null, openAiAuthMode: mode });
    void get().init();
  },

  setModel(id) {
    set({ modelId: id, openAiThreadId: null });
    persist({
      providerId: get().providerId,
      modelId: id,
      openAiAuthMode: get().openAiAuthMode,
    });
  },

  async saveKey(key) {
    const provider = get().providerId;
    await unwrap(commands.aiStoreKey(provider, key));
    set({ keyConfigured: true, configured: true, modelsError: null });
    await get().refreshModels();
  },

  async clearKey() {
    const provider = get().providerId;
    modelDiscoverySeq += 1;
    await unwrap(commands.aiDeleteKey(provider));
    set({
      keyConfigured: false,
      configured: false,
      models: [],
      modelsLoading: false,
      modelId: null,
      modelsError: null,
      openAiThreadId: null,
    });
  },

  async refreshOAuthStatus() {
    set({ authLoading: true, modelsError: null });
    try {
      const status = await unwrap(commands.aiOpenaiOauthStatus());
      set({
        oauthStatus: status,
        configured: status.signed_in,
        authLoading: false,
        login: status.signed_in ? null : get().login,
      });
      return status;
    } catch (error) {
      set({ authLoading: false, configured: false, modelsError: describeError(error) });
      throw error;
    }
  },

  async startOpenAiLogin(method) {
    set({ authLoading: true, modelsError: null });
    try {
      const login = await unwrap(commands.aiOpenaiStartLogin(method));
      set({ login, authLoading: false });
      return login;
    } catch (error) {
      set({ authLoading: false, modelsError: describeError(error) });
      throw error;
    }
  },

  async cancelOpenAiLogin() {
    const login = get().login;
    if (!login) return;
    await unwrap(commands.aiOpenaiCancelLogin(login.login_id));
    set({ login: null, authLoading: false });
  },

  async logoutOpenAi() {
    modelDiscoverySeq += 1;
    await unwrap(commands.aiOpenaiLogout());
    set({
      oauthStatus: null,
      login: null,
      configured: false,
      models: [],
      modelsLoading: false,
      modelId: null,
      openAiThreadId: null,
    });
  },

  async refreshModels() {
    const discoveryId = ++modelDiscoverySeq;
    const { providerId, openAiAuthMode } = get();
    const isCurrentDiscovery = () => {
      const state = get();
      return (
        discoveryId === modelDiscoverySeq &&
        state.providerId === providerId &&
        state.openAiAuthMode === openAiAuthMode
      );
    };
    set({ modelsLoading: true, modelsError: null });
    try {
      let models: AiModel[];
      let preferred: string | undefined;
      if (providerId === "google") {
        const key = await unwrap(commands.aiLoadKey(providerId));
        if (!key) {
          if (!isCurrentDiscovery()) return;
          set({ modelsLoading: false, models: [], keyConfigured: false, configured: false });
          return;
        }
        models = await listGeminiModels(key);
      } else if (providerId === "openai") {
        const discovered = await unwrap(commands.aiOpenaiListModels(openAiAuthMode));
        preferred = discovered.find((model) => model.is_default)?.id;
        models = discovered.map((model) => ({
          id: model.id,
          label: model.label,
        }));
      } else {
        throw new Error("This AI provider is not implemented yet.");
      }
      if (!isCurrentDiscovery()) return;
      const current = get().modelId;
      const modelId =
        current && models.some((model) => model.id === current)
          ? current
          : (preferred ?? models[0]?.id ?? null);
      set({ models, modelsLoading: false, configured: true, modelId });
      persist({ providerId, modelId, openAiAuthMode });
    } catch (error) {
      if (!isCurrentDiscovery()) return;
      set({ modelsLoading: false, modelsError: describeError(error) });
    }
  },

  async send(topic, text, context, lookupHits) {
    if (get().sending) return;
    const { providerId, modelId: model } = get();
    const userEntry: AiChatEntry = {
      id: nextId(),
      role: "user",
      topic,
      content: text.trim(),
      apiContent: buildUserPrompt(topic, text, context, lookupHits),
    };
    set((state) => ({ messages: [...state.messages, userEntry], sending: true }));

    try {
      if (!get().configured) {
        throw new Error("No provider credentials configured. Open AI settings to connect one.");
      }
      if (!model) throw new Error("No model selected. Pick one in AI settings.");
      const history = cleanHistory(get().messages);
      let content: string;
      let usage: TokenUsage | undefined;

      if (providerId === "google") {
        const key = await unwrap(commands.aiLoadKey(providerId));
        if (!key) throw new GeminiError(401, "No API key configured. Add one in AI settings.");
        const result = await generateContent({
          apiKey: key,
          model,
          systemInstruction: SYSTEM_PROMPT,
          messages: history,
        });
        content = result.text;
        usage = result.usage;
      } else if (providerId === "openai") {
        const { openAiAuthMode, openAiThreadId } = get();
        const result = await unwrap(
          commands.aiOpenaiGenerate(openAiAuthMode, {
            model,
            messages: history,
            system_instruction: SYSTEM_PROMPT,
            thread_id: openAiAuthMode === "chatgpt" ? openAiThreadId : null,
          }),
        );
        content = result.text;
        usage = result.usage
          ? {
              promptTokens: result.usage.prompt_tokens,
              completionTokens: result.usage.completion_tokens,
              totalTokens: result.usage.total_tokens,
            }
          : undefined;
        set({ openAiThreadId: result.thread_id });
      } else {
        throw new Error("This AI provider is not implemented yet.");
      }

      set((state) => ({
        sending: false,
        messages: [
          ...state.messages,
          { id: nextId(), role: "model", topic, content, usage },
        ],
      }));
    } catch (error) {
      set((state) => ({
        sending: false,
        messages: [
          ...state.messages,
          {
            id: nextId(),
            role: "model",
            topic,
            content: describeError(error),
            error: true,
          },
        ],
      }));
    }
  },

  newThread() {
    set({ messages: [], openAiThreadId: null });
  },
}));

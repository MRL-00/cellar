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
  type TokenUsage,
} from "@cellar/ai";
import { commands, unwrap } from "@cellar/ipc";

/** One rendered row in the AI thread. `content` is what we show; `apiContent`
 * (when set) is what was actually sent to the provider — the user sees their
 * own text, the model sees the preset instruction + schema context wrapped in. */
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
}

const STORAGE_KEY = "cellar.ai.v1";

function loadPersisted(): Persisted {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { providerId: DEFAULT_PROVIDER, modelId: null };
    const parsed = JSON.parse(raw) as Partial<Persisted>;
    return {
      providerId: parsed.providerId ?? DEFAULT_PROVIDER,
      modelId: parsed.modelId ?? null,
    };
  } catch {
    return { providerId: DEFAULT_PROVIDER, modelId: null };
  }
}

function persist(p: Persisted) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(p));
  } catch {
    // non-fatal: selection just won't survive a restart
  }
}

let entrySeq = 0;
function nextId(): string {
  entrySeq += 1;
  return `ai-${entrySeq}`;
}

function describeError(e: unknown): string {
  if (e instanceof GeminiError) return e.message;
  if (e instanceof Error) return e.message;
  return "Unexpected error talking to the provider.";
}

interface AiStore {
  providerId: AiProviderId;
  modelId: string | null;
  models: AiModel[];
  modelsLoading: boolean;
  modelsError: string | null;
  keyConfigured: boolean;
  messages: AiChatEntry[];
  sending: boolean;
  initialized: boolean;

  /** Load persisted selection + key status; fetch models if a key exists. */
  init: () => Promise<void>;
  setProvider: (id: AiProviderId) => void;
  setModel: (id: string) => void;
  /** Persist a key to the keychain, then (re)discover models. */
  saveKey: (key: string) => Promise<void>;
  /** Remove the key from the keychain and reset discovered models. */
  clearKey: () => Promise<void>;
  refreshModels: () => Promise<void>;
  send: (topic: AiTopic, text: string, context?: string) => Promise<void>;
  newThread: () => void;
}

export const useAi = create<AiStore>((set, get) => ({
  ...loadPersisted(),
  models: [],
  modelsLoading: false,
  modelsError: null,
  keyConfigured: false,
  messages: [],
  sending: false,
  initialized: false,

  async init() {
    if (get().initialized) return;
    set({ initialized: true });
    try {
      const has = await unwrap(commands.aiHasKey(get().providerId));
      set({ keyConfigured: has });
      if (has) await get().refreshModels();
    } catch (e) {
      set({ modelsError: describeError(e) });
    }
  },

  setProvider(id) {
    // Switching providers resets discovery state and re-evaluates the new
    // provider's key/models from scratch.
    set({
      providerId: id,
      initialized: false,
      keyConfigured: false,
      models: [],
      modelsError: null,
    });
    persist({ providerId: id, modelId: get().modelId });
    void get().init();
  },

  setModel(id) {
    set({ modelId: id });
    persist({ providerId: get().providerId, modelId: id });
  },

  async saveKey(key) {
    const provider = get().providerId;
    await unwrap(commands.aiStoreKey(provider, key));
    set({ keyConfigured: true, modelsError: null });
    await get().refreshModels();
  },

  async clearKey() {
    const provider = get().providerId;
    await unwrap(commands.aiDeleteKey(provider));
    set({ keyConfigured: false, models: [], modelsError: null });
  },

  async refreshModels() {
    const provider = get().providerId;
    set({ modelsLoading: true, modelsError: null });
    try {
      const key = await unwrap(commands.aiLoadKey(provider));
      if (!key) {
        set({ modelsLoading: false, models: [], keyConfigured: false });
        return;
      }
      const models = await listGeminiModels(key);
      const current = get().modelId;
      const stillValid = current && models.some((m) => m.id === current);
      const modelId = stillValid ? current : (models[0]?.id ?? null);
      set({ models, modelsLoading: false, keyConfigured: true, modelId });
      persist({ providerId: provider, modelId });
    } catch (e) {
      set({ modelsLoading: false, modelsError: describeError(e) });
    }
  },

  async send(topic, text, context) {
    if (get().sending) return;
    const provider = get().providerId;
    const model = get().modelId;

    const userEntry: AiChatEntry = {
      id: nextId(),
      role: "user",
      topic,
      content: text.trim(),
      apiContent: buildUserPrompt(topic, text, context),
    };
    set((s) => ({ messages: [...s.messages, userEntry], sending: true }));

    try {
      const key = await unwrap(commands.aiLoadKey(provider));
      if (!key) throw new GeminiError(401, "No API key configured. Add one in AI settings.");
      if (!model) throw new GeminiError(400, "No model selected. Pick one in AI settings.");

      // Gemini requires strictly alternating user/model turns. Drop failed
      // model turns *and* the user turn that triggered them so we never send
      // two consecutive user messages. The just-added user turn is last and
      // has no response yet, so it ends the (clean) history correctly.
      const history: ChatMessage[] = [];
      for (const m of get().messages) {
        if (m.role === "model" && m.error) {
          if (history.length && history[history.length - 1]!.role === "user") {
            history.pop();
          }
          continue;
        }
        history.push({
          role: m.role,
          content: m.role === "user" ? (m.apiContent ?? m.content) : m.content,
        });
      }

      const result = await generateContent({
        apiKey: key,
        model,
        systemInstruction: SYSTEM_PROMPT,
        messages: history,
      });

      set((s) => ({
        sending: false,
        messages: [
          ...s.messages,
          { id: nextId(), role: "model", content: result.text, usage: result.usage },
        ],
      }));
    } catch (e) {
      set((s) => ({
        sending: false,
        messages: [
          ...s.messages,
          { id: nextId(), role: "model", content: describeError(e), error: true },
        ],
      }));
    }
  },

  newThread() {
    set({ messages: [] });
  },
}));

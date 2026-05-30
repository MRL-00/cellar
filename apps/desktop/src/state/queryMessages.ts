import { create } from "zustand";
import type { PendingQueryMessage, QueryMessage } from "../lib/queryMessages";

const MAX_MESSAGES = 300;
let sequence = 0;

interface QueryMessagesStore {
  messages: QueryMessage[];
  addMessage: (message: PendingQueryMessage) => void;
  addMessages: (messages: PendingQueryMessage[]) => void;
  replaceForTab: (tabId: string, messages: PendingQueryMessage[]) => void;
  clearForTab: (tabId: string) => void;
  clear: () => void;
}

export const useQueryMessages = create<QueryMessagesStore>((set) => ({
  messages: [],

  addMessage(message) {
    set((s) => ({
      messages: trimMessages([...s.messages, normalizeMessage(message)]),
    }));
  },

  addMessages(messages) {
    if (messages.length === 0) return;
    set((s) => ({
      messages: trimMessages([
        ...s.messages,
        ...messages.map((m) => normalizeMessage(m)),
      ]),
    }));
  },

  replaceForTab(tabId, messages) {
    set((s) => ({
      messages: trimMessages([
        ...s.messages.filter((m) => m.tabId !== tabId),
        ...messages.map((m) => normalizeMessage(m)),
      ]),
    }));
  },

  clearForTab(tabId) {
    set((s) => ({ messages: s.messages.filter((m) => m.tabId !== tabId) }));
  },

  clear() {
    set({ messages: [] });
  },
}));

function normalizeMessage(message: PendingQueryMessage): QueryMessage {
  const timestamp = message.timestamp ?? new Date().toISOString();
  return {
    ...message,
    id: message.id ?? `query-message-${Date.now()}-${++sequence}`,
    timestamp,
  };
}

function trimMessages(messages: QueryMessage[]): QueryMessage[] {
  if (messages.length <= MAX_MESSAGES) return messages;
  return messages.slice(messages.length - MAX_MESSAGES);
}

import type { StateStorage } from "zustand/middleware";

const WRITE_DELAY_MS = 50;

/**
 * Synchronous reads keep persisted stores available during app startup, while
 * deferred writes avoid blocking UI paint in the click handler that changed
 * the store.
 */
export function deferredLocalStorage(): StateStorage {
  // No window means a test (or SSR-like) environment: keep persisted stores
  // functional with in-memory storage instead of warning on every write.
  if (typeof window === "undefined") {
    const mem = new Map<string, string>();
    return {
      getItem: (name) => mem.get(name) ?? null,
      setItem: (name, value) => void mem.set(name, value),
      removeItem: (name) => void mem.delete(name),
    };
  }

  const pending = new Map<string, string>();
  let timer: number | null = null;

  const flush = () => {
    if (timer != null) {
      window.clearTimeout(timer);
      timer = null;
    }
    for (const [name, value] of pending) {
      window.localStorage.setItem(name, value);
    }
    pending.clear();
  };

  const schedule = () => {
    if (timer != null) return;
    timer = window.setTimeout(flush, WRITE_DELAY_MS);
  };

  window.addEventListener("beforeunload", flush);

  return {
    getItem: (name) => window.localStorage.getItem(name),
    setItem: (name, value) => {
      pending.set(name, value);
      schedule();
    },
    removeItem: (name) => {
      pending.delete(name);
      window.localStorage.removeItem(name);
    },
  };
}

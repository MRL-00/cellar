import { create } from "zustand";

// Promise-based confirmation. Tauri's WKWebView doesn't reliably show
// `window.confirm`, so destructive actions route through this in-app dialog
// instead. `ask()` resolves true on confirm, false on cancel/dismiss.
export interface ConfirmOptions {
  title: string;
  message: string;
  confirmLabel?: string;
  danger?: boolean;
}

interface ConfirmRequest extends ConfirmOptions {
  resolve: (ok: boolean) => void;
}

interface ConfirmStore {
  request: ConfirmRequest | null;
  ask: (opts: ConfirmOptions) => Promise<boolean>;
  resolve: (ok: boolean) => void;
}

export const useConfirm = create<ConfirmStore>((set, get) => ({
  request: null,
  ask: (opts) =>
    new Promise<boolean>((resolve) => {
      // Resolve any in-flight request as cancelled before replacing it, so a
      // second ask() never leaves the first promise unsettled.
      get().request?.resolve(false);
      set({ request: { ...opts, resolve } });
    }),
  resolve: (ok) => {
    const req = get().request;
    if (req) req.resolve(ok);
    set({ request: null });
  },
}));

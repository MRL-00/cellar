// Public IPC surface. The actual command wrappers come from `generated.ts`
// (written by tauri-specta — never edited by hand). In `pnpm dev:web` mode
// the Tauri runtime is absent; we swap in `mockCommands` so the UI still
// renders with empty data instead of throwing on every IPC call.

import { commands as generatedCommands } from "./generated";
import { mockCommands } from "./mock";

export * from "./generated";

/**
 * `true` when the page is running inside a Tauri webview. The check looks for
 * the `__TAURI_INTERNALS__` global the runtime injects; `import.meta.env`
 * picks up Vite's TAURI_ENV_* prefix so a forced override is possible from
 * `.env.local` during testing.
 */
export const isTauri: boolean = (() => {
  if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
    return true;
  }
  const env = (import.meta as { env?: Record<string, string | undefined> }).env;
  if (env && env["TAURI_ENV_PLATFORM"]) {
    return true;
  }
  return false;
})();

/**
 * Thin typed wrapper around the generated `commands`. Use this instead of
 * importing from `./generated` directly so that web-only mode swaps in a
 * mock automatically.
 */
export const commands: typeof generatedCommands = isTauri
  ? generatedCommands
  : (mockCommands as unknown as typeof generatedCommands);

/**
 * Unwrap a `Result<T, CellarError>` into a plain promise that rejects with a
 * formatted error. Saves every call site from rewriting the `if (status ===
 * "error") throw …` boilerplate.
 */
export async function unwrap<T>(
  p: Promise<{ status: "ok"; data: T } | { status: "error"; error: { kind: string; detail: string } }>,
): Promise<T> {
  const r = await p;
  if (r.status === "ok") return r.data;
  throw new IpcError(r.error.kind, r.error.detail);
}

export class IpcError extends Error {
  override readonly name = "IpcError";
  constructor(
    readonly kind: string,
    readonly detail: string,
  ) {
    super(`${kind}: ${detail}`);
  }
}

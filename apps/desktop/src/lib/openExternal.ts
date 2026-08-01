import { isTauri } from "@cellar/ipc";
import { openUrl } from "@tauri-apps/plugin-opener";

/**
 * Open a URL in the user's default browser. Uses the Tauri opener plugin in
 * the desktop shell; falls back to `window.open` in `pnpm dev:web` mode.
 */
export async function openExternal(url: string): Promise<void> {
  if (isTauri) {
    await openUrl(url);
  } else {
    window.open(url, "_blank", "noopener");
  }
}

import { isTauri } from "@cellar/ipc";
import { openUrl } from "@tauri-apps/plugin-opener";

/**
 * Open a URL in the user's default browser. Uses the Tauri opener plugin in
 * the desktop shell; falls back to `window.open` in `pnpm dev:web` mode.
 */
export function openExternal(url: string): void {
  if (isTauri) {
    void openUrl(url);
  } else {
    window.open(url, "_blank", "noopener");
  }
}

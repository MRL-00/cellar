import { create } from "zustand";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { getVersion } from "@tauri-apps/api/app";

export type UpdaterStatus =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "available"; version: string; update: Update }
  | { kind: "up-to-date" }
  | { kind: "downloading"; fraction: number }
  | { kind: "installing" }
  | { kind: "error"; message: string };

const LAST_CHECKED_KEY = "cellar.updater.lastChecked";

function readLastChecked(): string | null {
  try {
    return localStorage.getItem(LAST_CHECKED_KEY);
  } catch {
    return null;
  }
}

function writeLastChecked(value: string) {
  try {
    localStorage.setItem(LAST_CHECKED_KEY, value);
  } catch {
    /* ignore quota / private mode */
  }
}

interface UpdaterState {
  appVersion: string;
  status: UpdaterStatus;
  lastChecked: string | null;
  checkForUpdate: () => Promise<void>;
  downloadAndInstall: () => Promise<void>;
}

// Shared store rather than per-component state: the startup check (App.tsx),
// the update toast, and the Settings > Updates panel all read the same status,
// so clicking "Update" in the toast lands on a panel that already knows an
// update is available and holds the Update object ready to install.
export const useUpdater = create<UpdaterState>((set, get) => ({
  appVersion: "",
  status: { kind: "idle" },
  lastChecked: readLastChecked(),

  checkForUpdate: async () => {
    set({ status: { kind: "checking" } });
    try {
      const update = await check();
      const now = new Date().toISOString();
      writeLastChecked(now);
      set({ lastChecked: now });
      if (update?.available) {
        set({ status: { kind: "available", version: update.version, update } });
      } else {
        set({ status: { kind: "up-to-date" } });
      }
    } catch (err) {
      set({
        status: {
          kind: "error",
          message: err instanceof Error ? err.message : String(err),
        },
      });
    }
  },

  downloadAndInstall: async () => {
    const { status } = get();
    if (status.kind !== "available") return;
    const { update } = status;
    try {
      set({ status: { kind: "downloading", fraction: 0 } });
      let total = 0;
      let downloaded = 0;
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            total = event.data.contentLength ?? 0;
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            if (total > 0) {
              set({ status: { kind: "downloading", fraction: downloaded / total } });
            }
            break;
          case "Finished":
            set({ status: { kind: "installing" } });
            break;
        }
      });
      await relaunch();
    } catch (err) {
      set({
        status: {
          kind: "error",
          message: err instanceof Error ? err.message : String(err),
        },
      });
    }
  },
}));

// Fetch the running version once; harmless no-op outside Tauri (web dev).
getVersion()
  .then((v) => useUpdater.setState({ appVersion: v }))
  .catch(() => {});

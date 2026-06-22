import { useCallback, useEffect, useState } from "react";
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

export function useUpdater() {
  const [appVersion, setAppVersion] = useState<string>("");
  const [status, setStatus] = useState<UpdaterStatus>({ kind: "idle" });
  const [lastChecked, setLastChecked] = useState<string | null>(() => readLastChecked());

  useEffect(() => {
    getVersion()
      .then(setAppVersion)
      .catch(() => setAppVersion(""));
  }, []);

  const checkForUpdate = useCallback(async () => {
    setStatus({ kind: "checking" });
    try {
      const update = await check();
      const now = new Date().toISOString();
      writeLastChecked(now);
      setLastChecked(now);
      if (update?.available) {
        setStatus({ kind: "available", version: update.version, update });
      } else {
        setStatus({ kind: "up-to-date" });
      }
    } catch (err) {
      setStatus({
        kind: "error",
        message: err instanceof Error ? err.message : String(err),
      });
    }
  }, []);

  const downloadAndInstall = useCallback(async () => {
    if (status.kind !== "available") return;
    const { update } = status;
    try {
      setStatus({ kind: "downloading", fraction: 0 });
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
              setStatus({ kind: "downloading", fraction: downloaded / total });
            }
            break;
          case "Finished":
            setStatus({ kind: "installing" });
            break;
        }
      });
      await relaunch();
    } catch (err) {
      setStatus({
        kind: "error",
        message: err instanceof Error ? err.message : String(err),
      });
    }
  }, [status]);

  return {
    appVersion,
    status,
    lastChecked,
    checkForUpdate,
    downloadAndInstall,
  };
}

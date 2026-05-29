import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// Tauri expects a known port and to know if it's iOS/Android.
// See https://v2.tauri.app/start/frontend/vite/
const host = process.env.TAURI_DEV_HOST;
const devPort = Number(process.env.CELLAR_DEV_PORT ?? 1430);

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: devPort,
    strictPort: !!process.env.CELLAR_DEV_PORT,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: devPort + 1 }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    target:
      process.env.TAURI_ENV_PLATFORM === "windows"
        ? "chrome105"
        : "safari13",
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
});

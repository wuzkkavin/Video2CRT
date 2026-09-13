import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri 2 dev server contract: fixed port 1420, no host header check.
//
// Build-time timestamp injection: every `vite build` writes the current
// Unix timestamp into the global `__VIDEO2CRT_BUILD_TIME__`. App.tsx reads
// it via `(window as any).__VIDEO2CRT_BUILD_TIME__` and renders it in the
// footer, so the user can visually confirm that the running .exe matches
// the latest build (not a stale binary from a previous cargo run).
// This was added 2026-09-13 after the user reported the footer version
// string never changed across builds — they had no way to tell whether
// a code fix had actually been compiled into the .exe they were running.
export default defineConfig(async () => ({
  define: {
    // Local-time build stamp so the footer reads as the user's wall-
    // clock (e.g. "2026-09-13 16:50:00") rather than UTC ("2026-09-13
    // 08:50:00Z"). Intl.DateTimeFormat with the user's resolved
    // timezone gives a YYYY-MM-DD HH:MM:SS string that matches what
    // the user sees on their task tray. We append " (local)" so the
    // footer makes the timezone explicit and the user doesn't think
    // we're back to the old UTC-with-Z format.
    __VIDEO2CRT_BUILD_TIME__: JSON.stringify(
      // YYYY-MM-DD HH:MM:SS in the user's local timezone (Intl
      // resolves tz from the OS, same source Windows tray uses).
      // We use the 'en-CA' locale because it formats dates as
      // YYYY-MM-DD by default, then strip the comma that
      // en-CA-style 24h time inserts. " (local)" suffix tells the
      // user explicitly that this is local-time not UTC.
      new Intl.DateTimeFormat("en-CA", {
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
        hour12: false,
      })
        .format(new Date())
        .replace(", ", " ") + " (local)",
    ),
    __VIDEO2CRT_BUILD_EPOCH__: JSON.stringify(Date.now()),
  },
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "localhost",
    hmr: { protocol: "ws", host: "localhost", port: 1421 },
    watch: { ignored: ["**/src-tauri/**"] },
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2021",
    minify: "esbuild",
    sourcemap: false,
  },
}));
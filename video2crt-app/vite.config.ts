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
    __VIDEO2CRT_BUILD_TIME__: JSON.stringify(new Date().toISOString()),
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
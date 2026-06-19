import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri-aware Vite config. See https://v2.tauri.app/start/frontend/vite/
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],

  // Prevent Vite from obscuring Rust errors.
  clearScreen: false,
  server: {
    // Tauri expects a fixed port; fail if it is taken instead of auto-picking.
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: {
      // Don't watch the Rust side; cargo handles that.
      ignored: ["**/src-tauri/**"],
    },
  },

  // Env vars starting with these are exposed to the frontend.
  envPrefix: ["VITE_", "TAURI_ENV_*"],

  build: {
    // Tauri uses a modern webview, so target the engines it ships with.
    target:
      process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari13",
    // Don't minify for debug builds.
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
});

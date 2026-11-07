/// <reference types="vitest/config" />

import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import babel from "@rolldown/plugin-babel";
import tailwindcss from "@tailwindcss/vite";
import react, { reactCompilerPreset } from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import { snapshotPlugin } from "./snapshot-plugin.ts";

const root = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [react(), babel({ presets: [reactCompilerPreset()] }), tailwindcss(), snapshotPlugin()],
  clearScreen: false,
  resolve: {
    alias: { "@": resolve(root, "src") },
  },
  server: {
    host: "127.0.0.1",
    port: 1421,
    strictPort: true,
    watch: {
      usePolling: true,
      interval: 400,
      ignored: ["**/node_modules/**", /[\\/]sed\w{6}$/, /\.tmp$/, /~$/],
    },
  },
  build: {
    outDir: "../../crates/median-app/dist",
    emptyOutDir: true,
    rollupOptions: {
      input: {
        main: resolve(root, "index.html"),
        launcher: resolve(root, "launcher.html"),
        overlay: resolve(root, "overlay.html"),
      },
    },
  },
  test: {
    environment: "jsdom",
  },
});

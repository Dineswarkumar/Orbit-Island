import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const siteRoot = fileURLToPath(new URL(".", import.meta.url));
const repoRoot = fileURLToPath(new URL("..", import.meta.url));

export default defineConfig({
  plugins: [react()],
  root: siteRoot,
  resolve: {
    alias: [
      {
        find: /^(\.\.\/)+core\/ipc\/client$/,
        replacement: `${siteRoot}src/mocks/ipc.ts`,
      },
      {
        find: /^@tauri-apps\/api\/event$/,
        replacement: `${siteRoot}src/mocks/tauri-event.ts`,
      },
      {
        find: /^@tauri-apps\/api(\/.*)?$/,
        replacement: `${siteRoot}src/mocks/tauri-event.ts`,
      },
      {
        find: "@app",
        replacement: `${repoRoot}src`,
      },
    ],
  },
  server: {
    fs: {
      allow: [repoRoot],
    },
  },
});

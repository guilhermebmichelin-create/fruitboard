import process from "node:process";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const host = process.env["TAURI_DEV_HOST"];
const debug = Boolean(process.env["TAURI_ENV_DEBUG"]);

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    host: host ?? "127.0.0.1",
    port: 1420,
    strictPort: true,
    ...(host
      ? {
          hmr: {
            protocol: "ws",
            host,
            port: 1421,
          },
        }
      : {}),
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    target:
      process.env["TAURI_ENV_PLATFORM"] === "windows"
        ? "chrome105"
        : "safari13",
    minify: debug ? false : "oxc",
    sourcemap: debug,
  },
});

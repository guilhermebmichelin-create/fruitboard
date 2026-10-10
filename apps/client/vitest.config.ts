import process from "node:process";
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // Windows fork workers can stall before running tests on the pinned host.
    // Threads preserve per-file isolation without that process startup path.
    pool: process.platform === "win32" ? "threads" : "forks",
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
  },
});

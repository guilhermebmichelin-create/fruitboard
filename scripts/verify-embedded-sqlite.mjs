#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import process from "node:process";

const probe = spawnSync(
  "cargo",
  [
    "run",
    "--quiet",
    "--locked",
    "-p",
    "fruitboard-storage",
    "--example",
    "sqlite-version",
  ],
  { encoding: "utf8", shell: false },
);
if (probe.status !== 0 || !/^\d+\.\d+\.\d+$/.test(probe.stdout?.trim() ?? "")) {
  console.error("BLOCK: the embedded SQLite runtime probe failed");
  process.exitCode = 1;
} else {
  const gate = spawnSync(
    process.execPath,
    ["scripts/verify-sqlite-version.mjs", probe.stdout.trim()],
    { stdio: "inherit", shell: false },
  );
  process.exitCode = gate.status ?? 1;
}

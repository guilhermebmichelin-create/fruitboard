#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import process from "node:process";
import { parseVersion, readToolchainPolicy } from "./lib/toolchain-policy.mjs";

const policy = readToolchainPolicy();
const failures = [];
const observed = {};

function run(command, args = []) {
  return execFileSync(command, args, {
    cwd: process.cwd(),
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    windowsHide: true,
  }).trim();
}

function expectVersion(label, output, expected) {
  const actual = parseVersion(output, label).normalized;
  observed[label] = actual;
  if (actual !== expected) {
    failures.push(`${label}: expected ${expected}, observed ${actual}`);
  }
}

function checkCommand(label, command, args, expected) {
  try {
    expectVersion(label, run(command, args), expected);
  } catch (error) {
    failures.push(`${label}: ${error.message}`);
  }
}

expectVersion("node", process.versions.node, policy.node);
if (process.env.npm_execpath) {
  checkCommand(
    "pnpm",
    process.execPath,
    [process.env.npm_execpath, "--version"],
    policy.pnpm,
  );
} else {
  failures.push(
    "pnpm: npm_execpath is unavailable; run verification through pnpm",
  );
}

const corepackScript = join(
  process.cwd(),
  "node_modules",
  "corepack",
  "dist",
  "corepack.js",
);
if (existsSync(corepackScript)) {
  checkCommand(
    "corepack",
    process.execPath,
    [corepackScript, "--version"],
    policy.corepack,
  );
} else {
  failures.push(`corepack: pinned executable missing at ${corepackScript}`);
}
checkCommand("rust", "rustc", ["--version"], policy.rust);
checkCommand("uv", "uv", ["--version"], policy.uv);

const pythonPath =
  process.platform === "win32"
    ? join(process.cwd(), ".venv", "Scripts", "python.exe")
    : join(process.cwd(), ".venv", "bin", "python");

if (!existsSync(pythonPath)) {
  failures.push(`python: isolated environment missing at ${pythonPath}`);
} else {
  try {
    expectVersion("python", run(pythonPath, ["--version"]), policy.python);
  } catch (error) {
    failures.push(`python: ${error.message}`);
  }
}

try {
  const installedComponents = run("rustup", [
    "component",
    "list",
    "--installed",
  ]);
  for (const component of policy.rustComponents) {
    if (
      !installedComponents
        .split(/\r?\n/)
        .some((line) => line.startsWith(component))
    ) {
      failures.push(`rust component missing: ${component}`);
    }
  }

  if (process.platform === "win32") {
    const installedTargets = run("rustup", ["target", "list", "--installed"]);
    if (!installedTargets.split(/\r?\n/).includes(policy.windowsRustTarget)) {
      failures.push(`Rust target missing: ${policy.windowsRustTarget}`);
    }
  }
} catch (error) {
  failures.push(`rustup: ${error.message}`);
}

const nodeVersionFile = readFileSync(
  new URL("../.node-version", import.meta.url),
  "utf8",
).trim();
const pythonVersionFile = readFileSync(
  new URL("../.python-version", import.meta.url),
  "utf8",
).trim();

if (nodeVersionFile !== policy.node) {
  failures.push(
    `.node-version: expected ${policy.node}, observed ${nodeVersionFile}`,
  );
}
if (pythonVersionFile !== policy.python) {
  failures.push(
    `.python-version: expected ${policy.python}, observed ${pythonVersionFile}`,
  );
}

if (failures.length > 0) {
  console.error("Toolchain verification failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exitCode = 1;
} else {
  console.log("Toolchain verification passed:");
  for (const [name, version] of Object.entries(observed)) {
    console.log(`- ${name} ${version}`);
  }
}

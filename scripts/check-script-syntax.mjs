#!/usr/bin/env node
// Syntax-check every first-party script and test with the pinned Node
// runtime. This replaces the hand-maintained `lint:scripts` file list: new
// scripts and tests are covered automatically instead of drifting out of
// the declared gate.
import { spawnSync } from "node:child_process";
import { readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = dirname(dirname(fileURLToPath(import.meta.url)));

const collect = (directory, targets) => {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const full = join(directory, entry.name);
    if (entry.isDirectory()) {
      collect(full, targets);
    } else if (entry.name.endsWith(".mjs")) {
      targets.push(full);
    }
  }
};

const targets = [];
for (const directory of ["scripts", "tests"]) {
  collect(join(repositoryRoot, directory), targets);
}
targets.sort();

let failed = 0;
for (const target of targets) {
  const result = spawnSync(process.execPath, ["--check", target], {
    encoding: "utf8",
  });
  if (result.status !== 0) {
    failed += 1;
    process.stderr.write(result.stderr);
    process.stderr.write(`syntax check failed: ${target}\n`);
  }
}

console.log(`syntax-checked ${targets.length} script/test files`);
if (failed > 0) {
  process.exit(1);
}

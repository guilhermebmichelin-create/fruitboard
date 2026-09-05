#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import process from "node:process";
import {
  formatPrivacyViolation,
  inspectRepositoryEntries,
} from "./lib/repository-privacy.mjs";

const repositoryPaths = execFileSync(
  "git",
  ["ls-files", "--cached", "--others", "--exclude-standard", "-z"],
  {
    cwd: process.cwd(),
    encoding: "utf8",
  },
)
  .split("\0")
  .filter(Boolean);

const entries = repositoryPaths.filter(existsSync).map((path) => {
  const contents = readFileSync(path);
  const sample = contents.subarray(0, 8192);
  const binary = sample.includes(0);
  return { path, text: binary ? null : contents.toString("utf8") };
});

const violations = inspectRepositoryEntries(entries);

if (violations.length === 0) {
  console.log(
    `Repository privacy verification passed (${entries.length} files).`,
  );
} else {
  console.error("Repository privacy verification failed:");
  for (const violation of violations) {
    console.error(`- ${formatPrivacyViolation(violation)}`);
  }
  process.exitCode = 1;
}

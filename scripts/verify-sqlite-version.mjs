#!/usr/bin/env node

import process from "node:process";
import {
  evaluateSqliteWalSafety,
  readToolchainPolicy,
} from "./lib/toolchain-policy.mjs";

const candidate =
  process.argv.slice(2).find((argument) => argument !== "--") ??
  process.env.FRUITBOARD_SQLITE_VERSION;

if (!candidate) {
  console.error(
    "Usage: pnpm verify:sqlite -- <runtime-version> (or set FRUITBOARD_SQLITE_VERSION)",
  );
  process.exitCode = 2;
} else {
  try {
    const result = evaluateSqliteWalSafety(
      candidate,
      readToolchainPolicy().sqlite,
    );
    const marker = result.safe ? "PASS" : "BLOCK";
    console.log(`${marker}: ${result.reason}`);
    process.exitCode = result.safe ? 0 : 1;
  } catch (error) {
    console.error(`BLOCK: ${error.message}`);
    process.exitCode = 2;
  }
}

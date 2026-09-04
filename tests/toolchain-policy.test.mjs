import assert from "node:assert/strict";
import test from "node:test";
import {
  compareVersions,
  evaluateSqliteWalSafety,
  parseVersion,
  readToolchainPolicy,
} from "../scripts/lib/toolchain-policy.mjs";

const policy = readToolchainPolicy();

test("extracts normalized semantic versions from tool output", () => {
  assert.equal(parseVersion("rustc 1.98.1 (commit)").normalized, "1.98.1");
  assert.equal(parseVersion("v24.20.0").normalized, "24.20.0");
  assert.throws(() => parseVersion("not-a-version"), /numeric x\.y\.z/);
});

test("compares numeric versions without lexical ordering errors", () => {
  assert.equal(compareVersions("3.51.3", "3.51.3"), 0);
  assert.equal(compareVersions("3.52.0", "3.51.3"), 1);
  assert.equal(compareVersions("3.9.0", "3.51.3"), -1);
});

test("blocks SQLite versions below the WAL-safe minimum", () => {
  assert.equal(evaluateSqliteWalSafety("3.51.2", policy.sqlite).safe, false);
  assert.equal(evaluateSqliteWalSafety("3.51.3", policy.sqlite).safe, true);
  assert.equal(evaluateSqliteWalSafety("3.53.0", policy.sqlite).safe, true);
});

test("allows only explicitly approved fixed backports below the minimum", () => {
  const withBackport = {
    ...policy.sqlite,
    approvedFixedBackports: ["3.50.7"],
  };

  assert.equal(evaluateSqliteWalSafety("3.50.7", withBackport).safe, true);
  assert.equal(evaluateSqliteWalSafety("3.50.6", withBackport).safe, false);
});

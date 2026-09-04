import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { readToolchainPolicy } from "../scripts/lib/toolchain-policy.mjs";

const policy = readToolchainPolicy();
const readRootFile = (path) =>
  readFileSync(new URL(`../${path}`, import.meta.url), "utf8");

test("JavaScript manifests agree with the central toolchain policy", () => {
  const packageJson = JSON.parse(readRootFile("package.json"));
  assert.equal(packageJson.engines.node, policy.node);
  assert.equal(packageJson.engines.pnpm, policy.pnpm);
  assert.match(
    packageJson.packageManager,
    new RegExp(`^pnpm@${policy.pnpm}\\+sha512\\.`),
  );
  assert.equal(packageJson.devDependencies.corepack, policy.corepack);
});

test("Rust and Python pins agree with the central policy", () => {
  assert.equal(readRootFile(".node-version").trim(), policy.node);
  assert.equal(readRootFile(".python-version").trim(), policy.python);
  assert.match(
    readRootFile("rust-toolchain.toml"),
    new RegExp(`channel = "${policy.rust}"`),
  );
  assert.match(
    readRootFile("pyproject.toml"),
    /requires-python = "==3\.11\.\*"/,
  );
});

test("research environment contains no PyFLP dependency", () => {
  const dependencyFiles = `${readRootFile("pyproject.toml")}\n${readRootFile("uv.lock")}`;
  assert.doesNotMatch(dependencyFiles, /pyflp/i);
});

test("privacy and generated-output ignore rules remain present", () => {
  const gitignore = readRootFile(".gitignore");
  for (const pattern of [
    "*.flp",
    "*.fst",
    "*.wav",
    "*.mp3",
    "*.flac",
    ".env",
    "node_modules/",
    "target/",
    ".venv/",
  ]) {
    assert.match(
      gitignore,
      new RegExp(`^${pattern.replaceAll("*", "\\*")}$`, "m"),
    );
  }
});

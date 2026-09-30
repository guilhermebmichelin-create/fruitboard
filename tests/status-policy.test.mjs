import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const readRootFile = (path) =>
  readFileSync(new URL(`../${path}`, import.meta.url), "utf8");

test("phase state claims match the Phase 2 acceptance record", () => {
  const readme = readRootFile("README.md");
  const development = readRootFile("DEVELOPMENT.md");
  const acceptance = readRootFile(
    "docs/review/phase-2-integration/acceptance-2026-09-14.md",
  );

  assert.match(acceptance, /accept/i);
  assert.match(readme, /## Current status/);
  assert.match(readme, /Phase 2 accepted with known gaps/);
  assert.match(development, /Phase 0-2 accepted/);

  // Superseded narratives must not present themselves as current state.
  assert.doesNotMatch(readme, /Phase 2 proceeds under epic #33/);
  assert.doesNotMatch(readme, /still has no parser/);
  assert.doesNotMatch(development, /Phase 2 \(epic #33\) underway/);
});

test("parser selection claims match ADR-002", () => {
  const readme = readRootFile("README.md");
  const dataModel = readRootFile("DATA_MODEL.md");
  const architecture = readRootFile("ARCHITECTURE.md");
  const adr002 = readRootFile("docs/adr/002-flp-parser-process.md");
  const adrIndex = readRootFile("docs/adr/README.md");
  const spikeProposal = readRootFile(
    "docs/research/rust-parser-spike-proposal.md",
  );
  const manifest = readRootFile("fixtures/parser-corpus/manifest.md");

  // ADR-002 is the authoritative selection record.
  assert.match(adr002, /Rust selected/i);
  assert.match(adrIndex, /Rust selected 2026-09-28/);
  assert.match(manifest, /Rust is selected/i);

  for (const document of [readme, dataModel, spikeProposal]) {
    assert.doesNotMatch(
      document,
      /no parser (?:exists|selected)|no production parser has been selected/i,
    );
  }
  assert.match(dataModel, /crates\/flp-parser/);

  // The architecture tree must reflect the real crate layout.
  const tree =
    architecture.match(/```text\nfruitboard\/([\s\S]*?)```/)?.[1] ?? "";
  assert.match(tree, /flp-parser/);
  assert.match(tree, /storage-sqlite/);
  assert.doesNotMatch(tree, /services\//);
  assert.doesNotMatch(tree, /parser-protocol/);
});

test("the dependency table tracks the pinned Tauri versions", () => {
  const development = readRootFile("DEVELOPMENT.md");
  const cargoToml = readRootFile("Cargo.toml");

  const tauriVersion = cargoToml.match(/tauri = \{ version = "=([^"]+)"/)?.[1];
  assert.ok(tauriVersion, "Cargo.toml must pin tauri");
  assert.match(
    development,
    new RegExp(`Tauri Rust / build\\s+\\| ${tauriVersion}`),
    "the dependency table must record the pinned tauri version",
  );

  const dialogVersion = cargoToml.match(
    /tauri-plugin-dialog = \{ version = "=([^"]+)"/,
  )?.[1];
  assert.ok(dialogVersion, "Cargo.toml must pin tauri-plugin-dialog");
  assert.match(
    development,
    new RegExp(`Tauri dialog plugin\\s+\\| ${dialogVersion}`),
    "the dependency table must record the pinned dialog plugin",
  );
});

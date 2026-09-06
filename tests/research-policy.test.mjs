import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const readRootFile = (path) =>
  readFileSync(new URL(`../${path}`, import.meta.url), "utf8");

const spikeReports = [
  "docs/research/p0-d-drivefs-watcher.md",
  "docs/research/p0-e-file-identity.md",
];

test("spike reports state question, limits, decisions, and reproduction", () => {
  for (const report of spikeReports) {
    const content = readRootFile(report);
    assert.match(content, /^## Question$/m);
    assert.match(content, /^## Limitations$/m);
    assert.match(content, /^## Decisions/m);
    assert.match(content, /^## Reproduction$/m);
    assert.match(content, /scripts\/research-fs-probe\.ps1/);
  }
});

test("spike reports never present absence without an authoritative scan", () => {
  for (const report of spikeReports) {
    const content = readRootFile(report);
    assert.match(content, /unseen[\s\S]{0,20}files missing/);
  }
});

test("hardlink aliases keep per-path presence records", () => {
  const identity = readRootFile("docs/research/p0-e-file-identity.md");
  const model = readRootFile("DATA_MODEL.md");

  assert.match(identity, /preserve one[\s\S]*presence record per path/);
  assert.doesNotMatch(identity, /deduplicate locations by identity/);
  assert.match(identity, /one disappears, the other remains available/);
  assert.match(
    model,
    /keep one location row[\s\S]*each: availability is tracked per path/,
  );
});

test("the research probe is disposable, path-free, and self-cleaning", () => {
  const probe = readRootFile("scripts/research-fs-probe.ps1");

  assert.match(probe, /Set-StrictMode -Version Latest/);
  assert.match(probe, /\$env:TEMP/);
  assert.doesNotMatch(probe, /C:\\Users/);
  assert.match(probe, /ValidateSet\("Identity", "Watcher", "All"\)/);
  assert.match(probe, /Remove-Item -LiteralPath \$root -Recurse -Force/);
  assert.match(probe, /all assertions passed/);
});

test("unverified environments stay tracked as open work", () => {
  const roadmap = readRootFile("ROADMAP.md");

  assert.match(roadmap, /#47 and #48/);
  assert.match(roadmap, /A closed spike is not qualification/);
});

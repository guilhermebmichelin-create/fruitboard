import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { buildPlan, writePlan } from "../scripts/generate-synthetic-tree.mjs";
import {
  buildEnvironmentReport,
  detectScannerIntegration,
  MEASURED_ITERATIONS,
  parseDriverLines,
  runCli,
  WARM_UP_RUNS,
} from "../scripts/run-benchmark.mjs";

const TINY = {
  sizeLabel: "custom",
  seed: "p2-11-test",
  fileCount: 50,
  leafDirectoryCount: 5,
};

const makeTempDir = (label) =>
  mkdtemp(path.join(os.tmpdir(), `fruitboard-benchmark-${label}-`));
const cleanup = (dir) => rm(dir, { recursive: true, force: true });

const fakeIntegrationRoot = async () => {
  const root = await makeTempDir("integrated");
  await mkdir(path.join(root, "crates", "scan-execution"), { recursive: true });
  await writeFile(
    path.join(root, "crates", "scan-execution", "Cargo.toml"),
    '[package]\nname = "placeholder"\n',
    "utf8",
  );
  return root;
};

const generateFixture = async (dir) => {
  await writePlan(buildPlan(TINY), dir);
  return path.join(dir, "manifest.json");
};

test("the methodology contract stays pinned to the accepted protocol", () => {
  assert.equal(WARM_UP_RUNS, 1);
  assert.equal(MEASURED_ITERATIONS, 10);
});

test("driver protocol sanitization drops unbounded diagnostic text", () => {
  const lines = parseDriverLines(
    [
      JSON.stringify({
        phase: "error",
        elapsed_ms: 1,
        message: "C:\\private\\file.flp",
        error_code: "native_error",
      }),
      "C:\\private\\unparseable native output",
      JSON.stringify({
        phase: "scan_finished",
        status: "Failed",
        outcome: "Partial",
        authoritative: false,
        partial_class: "partial_directory_not_found",
        private_path: "C:\\private\\file.flp",
      }),
    ].join("\n"),
  );

  assert.deepEqual(lines[0], {
    phase: "error",
    elapsed_ms: 1,
    error_code: null,
  });
  assert.deepEqual(lines[1], { phase: "unparseable" });
  assert.equal(lines[2].partial_class, "partial_directory_not_found");
  assert.equal("private_path" in lines[2], false);
});

test("the scanner integration marker is present in this repository", () => {
  const detected = detectScannerIntegration();
  assert.equal(detected.present, true);
  assert.equal(detected.marker, "crates/scan-execution/Cargo.toml");
});

test("the scaffold refuses to run without scanner integration", async (t) => {
  const fixtureDir = await makeTempDir("fixture");
  const reportDir = await makeTempDir("report");
  const emptyRoot = await makeTempDir("no-integration");
  t.after(() =>
    Promise.all([cleanup(fixtureDir), cleanup(reportDir), cleanup(emptyRoot)]),
  );
  const manifestPath = await generateFixture(fixtureDir);
  const reportPath = path.join(reportDir, "report.json");

  const result = await runCli(
    ["--manifest", manifestPath, "--out", reportPath],
    { repoRoot: emptyRoot },
  );
  assert.equal(result.code, 1);
  assert.match(
    result.stderr.join("\n"),
    /scanner benchmark integration is not present/,
  );
  assert.equal(existsSync(reportPath), false, "no report without integration");
});

test("the scaffold requires a fixture manifest argument", async () => {
  const result = await runCli([]);
  assert.equal(result.code, 1);
  assert.match(result.stderr.join("\n"), /--manifest is required/);
});

test("the scaffold validates the fixture manifest before reporting", async (t) => {
  const fixtureDir = await makeTempDir("tampered");
  const integrationRoot = await fakeIntegrationRoot();
  t.after(() => Promise.all([cleanup(fixtureDir), cleanup(integrationRoot)]));

  const document = buildPlan(TINY);
  document.hardlinkSupport = "pending";
  const manifestPath = path.join(fixtureDir, "manifest.json");
  await writeFile(manifestPath, JSON.stringify(document, null, 2), "utf8");

  const result = await runCli(["--manifest", manifestPath], {
    repoRoot: integrationRoot,
  });
  assert.equal(result.code, 1);
  assert.match(
    result.stderr.join("\n"),
    /pending is not a finished fixture manifest/,
  );
});

test("the scaffold captures a sanitized environment report", async (t) => {
  const fixtureDir = await makeTempDir("fixture");
  const reportDir = await makeTempDir("report");
  const integrationRoot = await fakeIntegrationRoot();
  t.after(() =>
    Promise.all([
      cleanup(fixtureDir),
      cleanup(reportDir),
      cleanup(integrationRoot),
    ]),
  );
  const manifestPath = await generateFixture(fixtureDir);
  const reportPath = path.join(reportDir, "report.json");

  const result = await runCli(
    ["--manifest", manifestPath, "--out", reportPath],
    { repoRoot: integrationRoot },
  );
  assert.equal(result.code, 0, result.stderr.join("; "));

  const text = await readFile(reportPath, "utf8");
  const report = JSON.parse(text);
  assert.equal(report.schema, "fruitboard/benchmark-environment/1");
  assert.equal(report.measurement.status, "pending-integrated-scanner");
  assert.equal(report.measurement.warmUpRuns, WARM_UP_RUNS);
  assert.equal(report.measurement.measuredIterations, MEASURED_ITERATIONS);
  assert.deepEqual(report.measurement.statistics, [
    "median",
    "max",
    "nearest-rank-p95",
  ]);
  assert.equal(report.measurement.firstRunReportedSeparately, true);
  assert.equal(report.releaseBuild.captured, false);
  assert.match(report.fixture.manifestSha256, /^[0-9a-f]{64}$/u);
  assert.equal(report.fixture.seed, TINY.seed);
  assert.ok(report.cpu.cores >= 1);
  assert.equal(typeof report.memoryTotalBytes, "number");
  assert.equal(typeof report.powerMode, "string");

  assert.doesNotMatch(text, /[A-Za-z]:[\\/]/u, "no drive-letter paths");
  assert.doesNotMatch(text, /\/(?:Users|home)\//u, "no POSIX home paths");
  assert.doesNotMatch(text, /\\\\/u, "no windows separators");
});

test("the environment report captures the required machine profile fields", () => {
  const report = buildEnvironmentReport({
    fixture: {
      sizeLabel: "baseline",
      seed: "p2-11",
      manifestSha256: "0".repeat(64),
      counts: { flpFiles: 10_000 },
    },
    snapshot: {
      platform: "testplatform",
      release: "10.0",
      arch: "x64",
      machine: "test-machine-class",
      cpuModel: "Test CPU Model",
      cpuCores: 8,
      memoryTotalBytes: 16_000_000_000,
    },
    volume: { totalBytes: 1_000_000_000, freeBytes: 500_000_000 },
    powerMode: "Balanced",
    capturedAtIso: "2026-09-07T00:00:00.000Z",
  });

  assert.equal(report.cpu.model, "Test CPU Model");
  assert.equal(report.cpu.cores, 8);
  assert.equal(report.memoryTotalBytes, 16_000_000_000);
  assert.equal(report.powerMode, "Balanced");
  assert.equal(report.machine.platform, "testplatform");
  assert.deepEqual(report.fixtureVolume, {
    totalBytes: 1_000_000_000,
    freeBytes: 500_000_000,
  });
  assert.equal(report.fixture.sizeLabel, "baseline");
});

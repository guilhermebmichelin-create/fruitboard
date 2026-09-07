#!/usr/bin/env node
//
// Scanner benchmark scaffold for #41 / P2-11 prep.
//
// This scaffold implements the measurement methodology accepted in
// docs/PHASE_2_EXECUTION_PLAN.md ("Proposed targets accepted as provisional
// budgets"): capture the reference machine profile, require a pinned release
// build, run one warm-up plus 10 measured iterations, and report median,
// maximum and nearest-rank p95 with the first run reported separately.
//
// Scope guard: the integrated scanner worker does not exist yet, so this
// scaffold runs nothing scanner-related. It exits with a clear error when the
// integration marker is absent. When the integration exists, it still only
// validates the fixture manifest and captures the environment report; the
// measured-run harness lands with the integrating PR and must follow
// scripts/benchmark-scan.md exactly. No benchmark numbers are produced or
// claimed here.
//
// Usage:
//   node scripts/run-benchmark.mjs --manifest <fixture>/manifest.json [--out <report.json>]

import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { mkdir, statfs } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  computeManifestHash,
  validateSyntheticManifest,
} from "./generate-synthetic-tree.mjs";

export const ENVIRONMENT_REPORT_SCHEMA = "fruitboard/benchmark-environment/1";
export const INTEGRATION_MARKER = "crates/scan-worker/Cargo.toml";
export const MEASURED_ITERATIONS = 10;
export const WARM_UP_RUNS = 1;

export function repositoryRoot() {
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
}

export function detectScannerIntegration(repoRoot = repositoryRoot()) {
  const markerPath = path.join(repoRoot, ...INTEGRATION_MARKER.split("/"));
  return { present: existsSync(markerPath), marker: INTEGRATION_MARKER };
}

export function captureMachineSnapshot() {
  const cpus = os.cpus();
  return {
    platform: process.platform,
    release: os.release(),
    arch: os.arch(),
    machine: os.machine(),
    cpuModel: cpus[0]?.model ?? "unknown",
    cpuCores: cpus.length,
    memoryTotalBytes: os.totalmem(),
  };
}

export function detectPowerMode() {
  if (process.platform !== "win32") {
    return "unknown";
  }
  try {
    const result = spawnSync("powercfg", ["/getactivescheme"], {
      encoding: "utf8",
      windowsHide: true,
    });
    const match = result.stdout?.match(/\(([^()]+)\)\s*$/m);
    return match ? match[1].trim() : "unknown";
  } catch {
    return "unknown";
  }
}

async function captureFixtureVolume(fixtureDirectory) {
  try {
    const stats = await statfs(fixtureDirectory);
    return {
      totalBytes: Number(stats.blocks * stats.bsize),
      freeBytes: Number(stats.bfree * stats.bsize),
    };
  } catch {
    return null;
  }
}

export function buildEnvironmentReport({
  fixture,
  snapshot = captureMachineSnapshot(),
  volume = null,
  powerMode = detectPowerMode(),
  capturedAtIso = new Date().toISOString(),
}) {
  return {
    schema: ENVIRONMENT_REPORT_SCHEMA,
    role: "P2-11 scaffold environment capture; no measurements are included",
    capturedAtIso,
    machine: {
      platform: snapshot.platform,
      release: snapshot.release,
      arch: snapshot.arch,
      machine: snapshot.machine,
    },
    cpu: { model: snapshot.cpuModel, cores: snapshot.cpuCores },
    memoryTotalBytes: snapshot.memoryTotalBytes,
    powerMode,
    fixtureVolume: volume,
    fixture,
    releaseBuild: {
      captured: false,
      requirement:
        "measurements must use a pinned release build; record the commit, toolchain and build command in the report notes before measuring",
    },
    measurement: {
      status: "pending-integrated-scanner",
      warmUpRuns: WARM_UP_RUNS,
      measuredIterations: MEASURED_ITERATIONS,
      statistics: ["median", "max", "nearest-rank-p95"],
      firstRunReportedSeparately: true,
    },
    notes: [],
  };
}

export function parseBenchmarkArgs(argv) {
  const usage = [
    "Usage:",
    "  node scripts/run-benchmark.mjs --manifest <fixture>/manifest.json [--out <report.json>]",
    "",
    "Options:",
    "  --manifest <path>  manifest.json of a generated synthetic fixture tree",
    "  --out <path>       environment report destination (default: a file in the OS temp directory)",
    "  --help             show this help",
  ].join("\n");
  const parsed = { manifest: null, out: null, help: false };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    const equals = argument.indexOf("=");
    const flag = equals === -1 ? argument : argument.slice(0, equals);
    const inlineValue = equals === -1 ? undefined : argument.slice(equals + 1);
    const next = () => {
      if (inlineValue !== undefined) {
        return inlineValue;
      }
      index += 1;
      if (index >= argv.length) {
        throw new Error(`missing value for ${flag}\n${usage}`);
      }
      return argv[index];
    };
    switch (flag) {
      case "--manifest":
        parsed.manifest = next();
        break;
      case "--out":
        parsed.out = next();
        break;
      case "--help":
      case "-h":
        parsed.help = true;
        break;
      default:
        throw new Error(`unknown argument: ${argument}\n${usage}`);
    }
  }
  if (parsed.help) {
    return parsed;
  }
  if (parsed.manifest === null) {
    throw new Error(`--manifest is required\n${usage}`);
  }
  return parsed;
}

export async function runCli(argv, { repoRoot = repositoryRoot() } = {}) {
  const stdout = [];
  const stderr = [];
  let parsed;
  try {
    parsed = parseBenchmarkArgs(argv);
  } catch (error) {
    return { code: 1, stdout, stderr: [...stderr, error.message] };
  }
  if (parsed.help) {
    return {
      code: 0,
      stdout: [
        "Usage: node scripts/run-benchmark.mjs --manifest <fixture>/manifest.json [--out <report.json>]",
      ],
      stderr,
    };
  }

  const integration = detectScannerIntegration(repoRoot);
  if (!integration.present) {
    return {
      code: 1,
      stdout,
      stderr: [
        `scanner benchmark integration is not present (missing ${integration.marker}).`,
        "This scaffold runs nothing scanner-related; it exits instead of measuring.",
        "The measured-run harness lands with the integrating PR and must follow scripts/benchmark-scan.md.",
      ],
    };
  }

  let document;
  try {
    document = JSON.parse(readFileSync(parsed.manifest, "utf8"));
  } catch (error) {
    return {
      code: 1,
      stdout,
      stderr: [
        ...stderr,
        `fixture manifest is not readable JSON: ${error.message}`,
      ],
    };
  }
  const validated = validateSyntheticManifest(document, {
    allowPending: false,
  });
  if (!validated.ok) {
    return {
      code: 1,
      stdout,
      stderr: [
        "fixture manifest failed validation:",
        ...validated.errors.map((error) => `- ${error}`),
      ],
    };
  }

  const manifestSha256 = computeManifestHash(document);
  const volume = await captureFixtureVolume(
    path.dirname(path.resolve(parsed.manifest)),
  );
  const report = buildEnvironmentReport({
    fixture: {
      sizeLabel: document.sizeLabel,
      seed: document.seed,
      manifestSha256,
      counts: document.counts,
    },
    volume,
  });

  const outPath = path.resolve(
    parsed.out ??
      path.join(os.tmpdir(), "fruitboard-benchmark-environment.json"),
  );
  await mkdir(path.dirname(outPath), { recursive: true });
  writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");

  stdout.push(
    `fixture manifest valid: ${document.counts.flpFiles} .flp-named files`,
  );
  stdout.push(`fixture manifest sha-256: ${manifestSha256}`);
  stdout.push(
    `environment report written: ${outPath} (no measurements; scanner integration pending)`,
  );
  return { code: 0, stdout, stderr };
}

export async function main(argv = process.argv.slice(2)) {
  const result = await runCli(argv);
  for (const line of result.stdout) {
    console.log(line);
  }
  for (const line of result.stderr) {
    console.error(line);
  }
  return result.code;
}

const invokedDirectly =
  process.argv[1] !== undefined &&
  import.meta.url === pathToFileURL(process.argv[1]).href;
if (invokedDirectly) {
  process.exitCode = await main();
}

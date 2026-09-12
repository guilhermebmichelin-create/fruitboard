#!/usr/bin/env node
//
// Scanner benchmark harness for #41 / P2-11.
//
// Implements the measurement methodology accepted in
// docs/PHASE_2_EXECUTION_PLAN.md ("Proposed targets accepted as provisional
// budgets") and scripts/benchmark-scan.md against the integrated scanner
// worker: build the pinned release benchmark driver, generate the fixture,
// run one warm-up plus 10 measured iterations (fresh process per iteration
// against one committed database), sample the driver's working set during
// each run, measure cooperative cancellation stop latency, and report
// median / maximum / nearest-rank p95 with the first run reported separately.
//
// Honesty rules (never relaxed): no safety check is skipped to make an
// iteration faster; a non-authoritative iteration is reported as a failure,
// not discarded; budget misses are recorded as findings with the measured
// numbers, never silently accepted and never "fixed" by loosening checks.
//
// When the repository is not a full cargo workspace (for example a fixture
// root without the workspace manifests), the harness falls back to the
// environment-only capture so the scaffold stays usable for validation.
//
// Usage:
//   node scripts/run-benchmark.mjs --manifest <fixture>/manifest.json [--out <report.json>]
//   node scripts/run-benchmark.mjs --size baseline --seed 0 [--out <report.json>]
//   node scripts/run-benchmark.mjs --size custom --files 9995 [--out <report.json>]

import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { mkdir, mkdtemp, rm, statfs } from "node:fs/promises";
import { spawn, spawnSync } from "node:child_process";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  buildPlan,
  computeManifestHash,
  validateSyntheticManifest,
  writePlan,
} from "./generate-synthetic-tree.mjs";

export const ENVIRONMENT_REPORT_SCHEMA = "fruitboard/benchmark-environment/1";
export const MEASURED_REPORT_SCHEMA = "fruitboard/benchmark-report/1";
export const INTEGRATION_MARKER = "crates/scan-execution/Cargo.toml";
export const MEASURED_ITERATIONS = 10;
export const WARM_UP_RUNS = 1;
export const CANCEL_ITERATIONS = 3;
export const CANCEL_AFTER_MS = 250;
export const SETTLE_MS = 300;
export const MEMORY_SAMPLING_INTERVAL_MS = 100;

// Accepted provisional budgets from docs/PHASE_2_EXECUTION_PLAN.md. They are
// starting targets, not promises; misses are findings with evidence.
export const BUDGETS = Object.freeze([
  {
    id: "first-discovery",
    area: "NTFS scan latency",
    metric: "baseline first discovery",
    unit: "ms",
    targetMs: 30_000,
  },
  {
    id: "warm-p95",
    area: "NTFS scan latency",
    metric: "unchanged warm reconciliation p95",
    unit: "ms",
    targetMs: 10_000,
  },
  {
    id: "cancel-stop",
    area: "Cancellation",
    metric:
      "cooperative worker stop p95 (UI ack budget does not bind the worker)",
    unit: "ms",
    targetMs: 1_000,
  },
  {
    id: "working-memory",
    area: "Working memory",
    metric:
      "incremental private memory (measured on the baseline set, not the 100k set)",
    unit: "MiB",
    targetMiB: 128,
  },
]);

export function repositoryRoot() {
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
}

export function detectScannerIntegration(repoRoot = repositoryRoot()) {
  const markerPath = path.join(repoRoot, ...INTEGRATION_MARKER.split("/"));
  return { present: existsSync(markerPath), marker: INTEGRATION_MARKER };
}

function canExecuteProtocol(repoRoot) {
  return (
    existsSync(path.join(repoRoot, "Cargo.toml")) &&
    existsSync(path.join(repoRoot, "rust-toolchain.toml"))
  );
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

function median(values) {
  const sorted = [...values].sort((left, right) => left - right);
  const middle = Math.floor(sorted.length / 2);
  if (sorted.length % 2 === 1) {
    return sorted[middle];
  }
  return (sorted[middle - 1] + sorted[middle]) / 2;
}

function nearestRankP95(values) {
  const sorted = [...values].sort((left, right) => left - right);
  const rank = Math.max(1, Math.ceil(0.95 * sorted.length));
  return sorted[rank - 1];
}

function statistics(values) {
  if (values.length === 0) {
    return null;
  }
  return {
    medianMs: median(values),
    maxMs: Math.max(...values),
    nearestRankP95Ms: nearestRankP95(values),
    sampleCount: values.length,
  };
}

function round(value, digits = 1) {
  const factor = 10 ** digits;
  return Math.round(value * factor) / factor;
}

async function workingSetKb(pid) {
  if (process.platform !== "win32") {
    return null;
  }
  try {
    const result = spawnSync(
      "tasklist",
      ["/FI", `PID eq ${pid}`, "/FO", "CSV", "/NH"],
      { encoding: "utf8", windowsHide: true, timeout: 2_000 },
    );
    const fields = result.stdout?.trim().match(/"([^"]*)"/g) ?? [];
    const usage = fields.pop()?.replace(/"/g, "");
    // The usage column is locale-formatted ("6.024 K" is 6024 KB in pt-BR;
    // "12,345 K" in en-US). Strip every digit separator and parse KB.
    const kilobytes = Number.parseInt((usage ?? "").replace(/[,.]/g, ""), 10);
    return Number.isFinite(kilobytes) && kilobytes > 0 ? kilobytes : null;
  } catch {
    return null;
  }
}

function startMemorySampler(pid, intervalMs) {
  const samples = [];
  let inFlight = false;
  let timer = null;
  if (intervalMs > 0) {
    timer = setInterval(() => {
      if (inFlight) {
        return;
      }
      inFlight = true;
      const observedAt = Date.now();
      workingSetKb(pid)
        .then((kilobytes) => {
          if (kilobytes !== null) {
            samples.push({ observedAt, kilobytes });
          }
        })
        .finally(() => {
          inFlight = false;
        });
    }, intervalMs);
    timer.unref();
  }
  return {
    samples,
    stop() {
      if (timer !== null) {
        clearInterval(timer);
        timer = null;
      }
    },
  };
}

function summarizeMemory(
  samples,
  spawnStartedAt,
  scanStartedMs,
  scanFinishedMs,
) {
  if (samples.length === 0) {
    return null;
  }
  const idleWindow = [];
  const scanWindow = [];
  for (const sample of samples) {
    const processRelativeMs = sample.observedAt - spawnStartedAt;
    if (scanStartedMs !== null && processRelativeMs < scanStartedMs) {
      idleWindow.push(sample.kilobytes);
    }
    if (
      scanStartedMs !== null &&
      scanFinishedMs !== null &&
      processRelativeMs >= scanStartedMs &&
      processRelativeMs < scanFinishedMs
    ) {
      scanWindow.push(sample.kilobytes);
    }
  }
  const idleKb = idleWindow.length > 0 ? Math.max(...idleWindow) : null;
  const fallbackIdleKb = idleKb ?? Math.min(...samples.map((s) => s.kilobytes));
  const peakKb =
    scanWindow.length > 0
      ? Math.max(...scanWindow)
      : Math.max(...samples.map((s) => s.kilobytes));
  return {
    idleMb: round(fallbackIdleKb / 1024),
    peakMb: round(peakKb / 1024),
    incrementalMb: round(Math.max(0, peakKb - fallbackIdleKb) / 1024),
    sampleCount: samples.length,
    idleWindowSamples: idleWindow.length,
  };
}

const DRIVER_STATUSES = new Set([
  "Published",
  "Failed",
  "Cancelled",
  "Interrupted",
  "Fenced",
]);

const DRIVER_OUTCOMES = new Set([
  "Complete",
  "Partial",
  "Denied",
  "RootUnavailable",
  "Cancelled",
  "ResourceLimit",
  "SinkFailed",
  "Invalid",
  "UnsupportedFilesystem",
]);

const PARTIAL_CLASSES = new Set([
  "partial_root_identity_unavailable",
  "partial_directory_not_found",
  "partial_directory_read",
  "partial_directory_changed",
  "partial_directory_identity_unavailable",
  "partial_entry_disappeared",
  "partial_metadata_read",
  "partial_timestamp_out_of_range",
  "partial_byte_size_out_of_range",
  "partial_locator_key_too_long",
  "partial_multiple",
  "partial_unknown",
]);

const DIAGNOSTIC_CODES = new Set([
  "access_denied",
  "unsupported",
  "resource_limit",
  "unavailable",
  "cancelled",
  "worker_failed",
  "driver_argument",
  "driver_database_open",
  "driver_worker_config",
  "driver_session_start",
  "driver_root_register",
  "driver_root_list",
  "driver_no_execution",
  "driver_poll",
  "driver_request_scan",
  "storage_io_failed",
  "storage_busy",
  "storage_unsafe_location",
  "storage_unsupported_sqlite",
  "storage_unsupported_journal",
  "storage_newer_schema",
  "storage_invalid_schema",
  "storage_migration_failed",
  "storage_invalid_backup",
  "storage_conflict",
  "storage_staging_rejected",
  "storage_not_found",
  "storage_invalid_cursor",
  "storage_stale_cursor",
  "storage_database_failed",
  ...PARTIAL_CLASSES,
]);

function safeInteger(value) {
  return Number.isSafeInteger(value) ? value : null;
}

function safeDuration(value) {
  return Number.isSafeInteger(value) && value >= 0 ? value : null;
}

function safeIdentifier(value) {
  return typeof value === "string" && /^[0-9a-f-]{1,128}$/i.test(value)
    ? value
    : null;
}

function safeCode(value) {
  return typeof value === "string" && DIAGNOSTIC_CODES.has(value)
    ? value
    : null;
}

function safeEnum(value, values) {
  return typeof value === "string" && values.has(value) ? value : null;
}

function sanitizeDriverLine(value) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    return { phase: "invalid_protocol" };
  }
  switch (value.phase) {
    case "ready": {
      const line = { phase: "ready" };
      const elapsed = safeDuration(value.elapsed_ms);
      const rootId = safeIdentifier(value.root_id);
      const sessionId = safeIdentifier(value.session_id);
      if (elapsed !== null) line.elapsed_ms = elapsed;
      if (rootId !== null) line.root_id = rootId;
      if (sessionId !== null) line.session_id = sessionId;
      if (value.worker_config === "default") line.worker_config = "default";
      return line;
    }
    case "scan_started":
    case "cancellation_requested": {
      const line = { phase: value.phase };
      const elapsed = safeDuration(value.elapsed_ms);
      if (elapsed !== null) line.elapsed_ms = elapsed;
      return line;
    }
    case "scan_finished": {
      const line = { phase: "scan_finished" };
      const integerFields = [
        "elapsed_ms",
        "scan_ms",
        "location_count",
        "generation",
        "changes_added",
        "changes_modified",
        "changes_replaced",
        "changes_identity_uncertain",
        "changes_missing",
        "changes_restored",
        "changes_renames",
      ];
      for (const field of integerFields) {
        const fieldValue =
          field === "elapsed_ms" || field === "scan_ms"
            ? safeDuration(value[field])
            : safeInteger(value[field]);
        if (fieldValue !== null) line[field] = fieldValue;
      }
      const status = safeEnum(value.status, DRIVER_STATUSES);
      const outcome = safeEnum(value.outcome, DRIVER_OUTCOMES);
      const runId = safeIdentifier(value.run_id);
      const jobId = safeIdentifier(value.job_id);
      const errorCode = safeCode(value.error_code);
      const partialClass = safeEnum(value.partial_class, PARTIAL_CLASSES);
      if (status !== null) line.status = status;
      if (outcome !== null) line.outcome = outcome;
      if (typeof value.authoritative === "boolean") {
        line.authoritative = value.authoritative;
      }
      if (runId !== null) line.run_id = runId;
      if (jobId !== null) line.job_id = jobId;
      line.error_code = errorCode;
      if (typeof value.changes_computed === "boolean") {
        line.changes_computed = value.changes_computed;
      }
      line.partial_class = partialClass;
      return line;
    }
    case "error": {
      const line = { phase: "error" };
      const elapsed = safeDuration(value.elapsed_ms);
      if (elapsed !== null) line.elapsed_ms = elapsed;
      line.error_code = safeCode(value.error_code);
      return line;
    }
    default:
      return { phase: "unknown_protocol" };
  }
}

export function parseDriverLines(stdout) {
  const lines = [];
  for (const raw of stdout.split("\n")) {
    const line = raw.trim();
    if (line.length === 0) {
      continue;
    }
    try {
      lines.push(sanitizeDriverLine(JSON.parse(line)));
    } catch {
      // Never retain arbitrary driver output: it may contain a private path,
      // file content, or a native error string. The phase is enough to flag a
      // protocol violation; the raw line is not benchmark evidence.
      lines.push({ phase: "unparseable" });
    }
  }
  return lines;
}

function runRecord({
  label,
  spawnStartedAt,
  spawnFinishedAt,
  child,
  lines,
  memory,
  cancelRequestedMs,
}) {
  const started = lines.find((line) => line.phase === "scan_started") ?? null;
  const finished = lines.find((line) => line.phase === "scan_finished") ?? null;
  const failure = lines.find((line) => line.phase === "error") ?? null;
  const record = {
    label,
    exitCode: child.code,
    totalMs: spawnFinishedAt - spawnStartedAt,
    scanMs:
      started !== null && finished !== null ? (finished.scan_ms ?? null) : null,
    status: finished?.status ?? null,
    authoritative: finished?.authoritative ?? false,
    outcome: finished?.outcome ?? null,
    locationCount: finished?.location_count ?? null,
    generation: finished?.generation ?? null,
    changesComputed: finished?.changes_computed ?? null,
    errorCode: finished?.error_code ?? null,
    failureCode: failure?.error_code ?? null,
    failureMessage:
      failure === null ? null : (failure.error_code ?? "driver_error"),
    partialClass: finished?.partial_class ?? null,
    stderrObserved: child.stderrObserved,
    memory,
    lines,
  };
  if (cancelRequestedMs !== null && finished !== null) {
    record.cancellationRequestedMs = cancelRequestedMs;
    record.stopLatencyMs =
      safeDuration(finished.elapsed_ms) !== null &&
      finished.elapsed_ms >= cancelRequestedMs
        ? finished.elapsed_ms - cancelRequestedMs
        : null;
  }
  return record;
}

async function runDriverProcess({
  bin,
  root,
  db,
  cancelAfterMs,
  settleMs,
  memoryIntervalMs,
}) {
  const args = ["--root", root, "--db", db, "--settle-ms", String(settleMs)];
  if (cancelAfterMs !== null) {
    args.push("--cancel-after-ms", String(cancelAfterMs));
  }
  const spawnStartedAt = Date.now();
  const child = spawn(bin, args, {
    windowsHide: true,
    stdio: ["ignore", "pipe", "pipe"],
  });
  const sampler = startMemorySampler(child.pid, memoryIntervalMs);
  let stdout = "";
  let stderr = "";
  child.stdout.on("data", (chunk) => {
    stdout += chunk;
  });
  child.stderr.on("data", (chunk) => {
    stderr += chunk;
  });
  const exitCode = await new Promise((resolve) => {
    child.on("close", resolve);
    child.on("error", () => {
      // Keep the observation, not the arbitrary OS/native message.
      stderr += "spawn_error\n";
      resolve(-1);
    });
  });
  sampler.stop();
  const spawnFinishedAt = Date.now();
  const lines = parseDriverLines(stdout);
  const cancelLine =
    lines.find((line) => line.phase === "cancellation_requested") ?? null;
  const startedLine =
    lines.find((line) => line.phase === "scan_started") ?? null;
  const finishedLine =
    lines.find((line) => line.phase === "scan_finished") ?? null;
  const memory = summarizeMemory(
    sampler.samples,
    spawnStartedAt,
    startedLine?.elapsed_ms ?? null,
    finishedLine?.elapsed_ms ?? null,
  );
  return {
    child: { code: exitCode, stderrObserved: stderr.trim().length > 0 },
    lines,
    memory,
    cancelRequestedMs: cancelLine?.elapsed_ms ?? null,
    record: runRecord({
      label: "run",
      spawnStartedAt,
      spawnFinishedAt,
      child: { code: exitCode, stderrObserved: stderr.trim().length > 0 },
      lines,
      memory,
      cancelRequestedMs: cancelLine?.elapsed_ms ?? null,
    }),
  };
}

async function buildDriver(repoRoot, cargoCommand) {
  const startedAt = Date.now();
  const result = spawnSync(
    cargoCommand,
    [
      "build",
      "--release",
      "-p",
      "fruitboard-scan-execution",
      "--example",
      "benchmark",
      "--locked",
    ],
    { cwd: repoRoot, encoding: "utf8", windowsHide: true, timeout: 1_200_000 },
  );
  const buildMs = Date.now() - startedAt;
  if (result.status !== 0) {
    const tail = (result.stderr ?? result.stdout ?? "")
      .trim()
      .split("\n")
      .slice(-12)
      .join("\n");
    throw new Error(
      `cargo build failed (exit ${result.status ?? "spawn"}):\n${tail}`,
    );
  }
  let commit = "unknown";
  try {
    const git = spawnSync("git", ["rev-parse", "--short", "HEAD"], {
      cwd: repoRoot,
      encoding: "utf8",
      windowsHide: true,
    });
    if (git.status === 0 && git.stdout?.trim().length > 0) {
      commit = git.stdout.trim();
    }
  } catch {
    // The build is still valid; the commit is recorded in the report notes.
  }
  return {
    bin: path.join(repoRoot, "target", "release", "examples", "benchmark.exe"),
    buildMs,
    commit,
  };
}

async function generateFixture({ size, files, seed, destination }) {
  const preset = {
    baseline: [10_000, 1_000],
    qualification: [100_000, 10_000],
  };
  let plan;
  if (size === "custom") {
    if (!Number.isInteger(files) || files < 1) {
      throw new Error("--files must be a positive integer with --size custom");
    }
    plan = buildPlan({
      sizeLabel: `custom-${files}`,
      seed,
      fileCount: files,
      leafDirectoryCount: 1_000,
    });
  } else {
    const [fileCount, leafDirectoryCount] = preset[size];
    plan = buildPlan({
      sizeLabel: size,
      seed,
      fileCount,
      leafDirectoryCount,
    });
  }
  const { hash } = await writePlan(plan, destination);
  return { hash };
}

function buildFixtureSummary(document) {
  return {
    sizeLabel: document.sizeLabel,
    seed: document.seed,
    manifestSha256: computeManifestHash(document),
    counts: document.counts,
    hardlinkSupport: document.hardlinkSupport,
  };
}

function isAuthoritativeMeasurement(iteration) {
  return (
    iteration.authoritative === true &&
    iteration.status === "Published" &&
    iteration.outcome === "Complete" &&
    safeDuration(iteration.scanMs) !== null
  );
}

export function budgetResults({ warmUp, iterations, cancellation }) {
  const authoritative = iterations.filter(isAuthoritativeMeasurement);
  const scanTimes = authoritative
    .map((iteration) => iteration.scanMs)
    .filter((value) => safeDuration(value) !== null);
  const firstDiscoveryMs =
    warmUp !== null && isAuthoritativeMeasurement(warmUp)
      ? warmUp.scanMs
      : null;
  const warmStats = statistics(scanTimes);
  const cancelTimes = (cancellation ?? [])
    .map((iteration) => iteration.stopLatencyMs)
    .filter((value) => safeDuration(value) !== null);
  const cancelStats = statistics(cancelTimes);
  const memoryIncrements = (iterations ?? [])
    .map((iteration) => iteration.memory?.incrementalMb)
    .filter((value) => value !== null && value !== undefined);
  const peakIncrementalMb =
    memoryIncrements.length > 0 ? Math.max(...memoryIncrements) : null;

  const rows = [];
  for (const budget of BUDGETS) {
    let measured = null;
    let pass = null;
    let note = null;
    if (budget.id === "first-discovery") {
      measured = firstDiscoveryMs;
      pass = measured !== null ? measured <= budget.targetMs : false;
      note =
        measured === null
          ? "no authoritative first-discovery run (see findings)"
          : null;
    } else if (budget.id === "warm-p95") {
      measured = warmStats?.nearestRankP95Ms ?? null;
      pass = measured !== null ? measured <= budget.targetMs : false;
      note =
        measured === null
          ? `no authoritative iterations (${iterations.filter((i) => !i.authoritative).length} of ${iterations.length} non-authoritative)`
          : null;
    } else if (budget.id === "cancel-stop") {
      measured = cancelStats?.nearestRankP95Ms ?? null;
      pass = measured !== null ? measured <= budget.targetMs : false;
      note =
        measured === null
          ? "no cancellation measurements completed"
          : "UI acknowledgement budget (<=250 ms) does not bind the worker; stop latency reported honestly";
    } else if (budget.id === "working-memory") {
      measured = peakIncrementalMb;
      pass = measured !== null ? measured <= budget.targetMiB : null;
      note =
        "measured on the baseline (10,000-location) set; the accepted 100,000-entry qualification set cannot complete under the 10,000-record staging quota (see findings)";
    }
    rows.push({
      id: budget.id,
      area: budget.area,
      metric: budget.metric,
      unit: budget.unit,
      target: budget.targetMs ?? budget.targetMiB,
      measured,
      pass,
      note,
    });
  }
  return { rows, warmStats, cancelStats };
}

export function parseBenchmarkArgs(argv) {
  const usage = [
    "Usage:",
    "  node scripts/run-benchmark.mjs --manifest <fixture>/manifest.json [--out <report.json>]",
    "  node scripts/run-benchmark.mjs --size <baseline|qualification|custom> [--seed <seed>] [--files <n>]",
    "",
    "Options:",
    "  --manifest <path>        manifest.json of a generated synthetic fixture tree",
    "  --size <preset>          generate the fixture instead of using --manifest",
    "  --files <n>              total .flp-named files for --size custom",
    "  --seed <seed>            deterministic generator seed (default 0)",
    "  --iterations <n>         measured iterations (default 10)",
    "  --warmup <n>             warm-up runs (default 1)",
    "  --cancel-iterations <n>  cancellation measurements (default 3)",
    "  --cancel-after-ms <n>    cancellation trigger delay (default 250)",
    "  --settle-ms <n>          idle settle window for memory sampling (default 300)",
    "  --memory-interval-ms <n> working-set sampling interval (default 100)",
    "  --cargo <path>           cargo executable (default: $CARGO or cargo on PATH)",
    "  --bin <path>             use an existing release benchmark binary, skip the build",
    "  --keep-fixture           do not remove the generated temp fixture",
    "  --out <path>             report destination (default: a file in the OS temp directory)",
    "  --help                   show this help",
  ].join("\n");
  const parsed = {
    manifest: null,
    size: null,
    files: null,
    seed: "0",
    iterations: MEASURED_ITERATIONS,
    warmup: WARM_UP_RUNS,
    cancelIterations: CANCEL_ITERATIONS,
    cancelAfterMs: CANCEL_AFTER_MS,
    settleMs: SETTLE_MS,
    memoryIntervalMs: MEMORY_SAMPLING_INTERVAL_MS,
    cargo: null,
    bin: null,
    out: null,
    keepFixture: false,
    help: false,
  };
  const readNumber = (flag, raw) => {
    const value = Number.parseInt(raw, 10);
    if (!Number.isInteger(value) || value < 0) {
      throw new Error(`invalid number for ${flag}: ${raw}\n${usage}`);
    }
    return value;
  };
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
      case "--size":
        parsed.size = next();
        break;
      case "--files":
        parsed.files = readNumber(flag, next());
        break;
      case "--seed":
        parsed.seed = next();
        break;
      case "--iterations":
        parsed.iterations = readNumber(flag, next());
        break;
      case "--warmup":
        parsed.warmup = readNumber(flag, next());
        break;
      case "--cancel-iterations":
        parsed.cancelIterations = readNumber(flag, next());
        break;
      case "--cancel-after-ms":
        parsed.cancelAfterMs = readNumber(flag, next());
        break;
      case "--settle-ms":
        parsed.settleMs = readNumber(flag, next());
        break;
      case "--memory-interval-ms":
        parsed.memoryIntervalMs = readNumber(flag, next());
        break;
      case "--cargo":
        parsed.cargo = next();
        break;
      case "--bin":
        parsed.bin = next();
        break;
      case "--keep-fixture":
        parsed.keepFixture = true;
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
  if (parsed.manifest === null && parsed.size === null) {
    throw new Error(
      `--manifest is required (or --size to generate a fixture)\n${usage}`,
    );
  }
  if (parsed.manifest !== null && parsed.size !== null) {
    throw new Error("--manifest and --size are mutually exclusive");
  }
  if (
    parsed.size !== null &&
    !["baseline", "qualification", "custom"].includes(parsed.size)
  ) {
    throw new Error(
      `--size must be baseline, qualification or custom\n${usage}`,
    );
  }
  if (parsed.size === "custom" && parsed.files === null) {
    throw new Error(`--files is required with --size custom\n${usage}`);
  }
  if (parsed.size !== "custom" && parsed.files !== null) {
    throw new Error("--files applies only to --size custom");
  }
  return parsed;
}

async function captureEnvironment({ fixtureDirectory, document }) {
  const snapshot = captureMachineSnapshot();
  const volume = await captureFixtureVolume(fixtureDirectory);
  return {
    schema: MEASURED_REPORT_SCHEMA,
    role: "P2-11 measured scanner benchmark report (scripts/benchmark-scan.md protocol)",
    capturedAtIso: new Date().toISOString(),
    machine: {
      platform: snapshot.platform,
      release: snapshot.release,
      arch: snapshot.arch,
      machine: snapshot.machine,
    },
    cpu: { model: snapshot.cpuModel, cores: snapshot.cpuCores },
    memoryTotalBytes: snapshot.memoryTotalBytes,
    powerMode: detectPowerMode(),
    fixtureVolume: volume,
    fixture: buildFixtureSummary(document),
    build: null,
    measurement: {
      status: "measured",
      warmUpRuns: WARM_UP_RUNS,
      measuredIterations: MEASURED_ITERATIONS,
      statistics: ["median", "max", "nearest-rank-p95"],
      firstRunReportedSeparately: true,
      sampleIntervalMs: null,
      cancelAfterMs: null,
    },
    warmUp: null,
    iterations: [],
    statistics: null,
    cancellation: [],
    cancellationStatistics: null,
    memory: null,
    budgets: [],
    notes: [],
  };
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
        "This harness runs nothing scanner-related; it exits instead of measuring.",
      ],
    };
  }

  let fixtureDocument;
  let fixtureDirectory;
  let fixtureRoot;
  let tempBase = await mkdtemp(path.join(os.tmpdir(), "fruitboard-benchmark-"));
  if (parsed.manifest !== null) {
    try {
      fixtureDocument = JSON.parse(readFileSync(parsed.manifest, "utf8"));
    } catch (error) {
      await rm(tempBase, { recursive: true, force: true });
      return {
        code: 1,
        stdout,
        stderr: [
          ...stderr,
          `fixture manifest is not readable JSON: ${error.message}`,
        ],
      };
    }
    const validated = validateSyntheticManifest(fixtureDocument, {
      allowPending: false,
    });
    if (!validated.ok) {
      await rm(tempBase, { recursive: true, force: true });
      return {
        code: 1,
        stdout,
        stderr: [
          "fixture manifest failed validation:",
          ...validated.errors.map((error) => `- ${error}`),
        ],
      };
    }
    fixtureDirectory = path.dirname(path.resolve(parsed.manifest));
    fixtureRoot = fixtureDirectory;
  } else {
    fixtureDirectory = path.join(tempBase, "fixture");
    try {
      const { hash } = await generateFixture({
        size: parsed.size,
        files: parsed.files,
        seed: parsed.seed,
        destination: fixtureDirectory,
      });
      fixtureDocument = JSON.parse(
        readFileSync(path.join(fixtureDirectory, "manifest.json"), "utf8"),
      );
      if (computeManifestHash(fixtureDocument) !== hash) {
        throw new Error("generated fixture manifest hash mismatch");
      }
    } catch (error) {
      await rm(tempBase, { recursive: true, force: true });
      return {
        code: 1,
        stdout,
        stderr: [...stderr, `fixture generation failed: ${error.message}`],
      };
    }
    fixtureRoot = fixtureDirectory;
  }

  // Environment-only fallback when the repository is not a complete cargo
  // workspace: the scaffold validates the fixture and captures the machine
  // profile, but never claims measurements it cannot execute.
  if (!canExecuteProtocol(repoRoot)) {
    const volume = await captureFixtureVolume(fixtureRoot);
    const report = buildEnvironmentReport({
      fixture: {
        sizeLabel: fixtureDocument.sizeLabel,
        seed: fixtureDocument.seed,
        manifestSha256: computeManifestHash(fixtureDocument),
        counts: fixtureDocument.counts,
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
      `fixture manifest valid: ${fixtureDocument.counts.flpFiles} .flp-named files`,
    );
    stdout.push(`fixture manifest sha-256: ${report.fixture.manifestSha256}`);
    stdout.push(
      `environment report written: ${outPath} (no measurements; the harness needs the full cargo workspace)`,
    );
    if (!parsed.keepFixture) {
      await rm(tempBase, { recursive: true, force: true });
    }
    return { code: 0, stdout, stderr };
  }

  const environment = await captureEnvironment({
    fixtureDirectory: fixtureRoot,
    document: fixtureDocument,
  });
  environment.measurement.sampleIntervalMs = parsed.memoryIntervalMs;
  environment.measurement.cancelAfterMs = parsed.cancelAfterMs;
  environment.measurement.settleMs = parsed.settleMs;

  // Pinned release build (reported separately from the measurements).
  let driver;
  if (parsed.bin !== null) {
    driver = { bin: path.resolve(parsed.bin), buildMs: null };
    environment.build = {
      captured: true,
      prebuilt: true,
      command: "prebuilt benchmark binary (--bin)",
      toolchain: "see report notes",
    };
  } else {
    const cargoCommand = parsed.cargo ?? process.env.CARGO ?? "cargo";
    try {
      const built = await buildDriver(repoRoot, cargoCommand);
      driver = { bin: built.bin, buildMs: built.buildMs };
      environment.build = {
        captured: true,
        prebuilt: false,
        command:
          "cargo build --release -p fruitboard-scan-execution --example benchmark --locked",
        toolchain: "pinned via rust-toolchain.toml (1.98.1)",
        buildMs: built.buildMs,
        commit: built.commit,
      };
    } catch (error) {
      if (!parsed.keepFixture) {
        await rm(tempBase, { recursive: true, force: true });
      }
      return { code: 1, stdout, stderr: [...stderr, error.message] };
    }
  }

  // The committed database is shared by the warm-up and the measured
  // iterations so every iteration starts from the committed state.
  const dbDirectory = path.join(tempBase, "db");
  await mkdir(dbDirectory, { recursive: true });

  // Warm-up: the first discovery against an empty committed state. Reported
  // separately; never labelled cold-cache (OS cache state is not controlled).
  const warmUp = await runDriverProcess({
    bin: driver.bin,
    root: fixtureRoot,
    db: dbDirectory,
    cancelAfterMs: null,
    settleMs: parsed.settleMs,
    memoryIntervalMs: parsed.memoryIntervalMs,
  });
  warmUp.record.label = "warmup";
  environment.warmUp = warmUp.record;
  if (!warmUp.record.authoritative) {
    environment.notes.push(
      `warm-up run was non-authoritative (${warmUp.record.outcome ?? "unknown"}); measured iterations continue against the committed state`,
    );
  }

  // Measured iterations: a fresh process per iteration against the same
  // committed database.
  for (let index = 1; index <= parsed.iterations; index += 1) {
    const run = await runDriverProcess({
      bin: driver.bin,
      root: fixtureRoot,
      db: dbDirectory,
      cancelAfterMs: null,
      settleMs: parsed.settleMs,
      memoryIntervalMs: parsed.memoryIntervalMs,
    });
    run.record.label = `iteration-${index}`;
    environment.iterations.push(run.record);
    if (run.child.code !== 0 && run.child.stderrObserved) {
      environment.notes.push(`iteration ${index} stderr_observed`);
    }
  }

  // Cancellation: fresh committed state per measurement, cancel fired
  // mid-run; observed stop latency is reported honestly.
  for (let index = 1; index <= parsed.cancelIterations; index += 1) {
    const cancelDb = path.join(tempBase, `db-cancel-${index}`);
    await mkdir(cancelDb, { recursive: true });
    const run = await runDriverProcess({
      bin: driver.bin,
      root: fixtureRoot,
      db: cancelDb,
      cancelAfterMs: parsed.cancelAfterMs,
      settleMs: parsed.settleMs,
      memoryIntervalMs: parsed.memoryIntervalMs,
    });
    run.record.label = `cancel-${index}`;
    environment.cancellation.push(run.record);
  }

  const results = budgetResults({
    warmUp: environment.warmUp,
    iterations: environment.iterations,
    cancellation: environment.cancellation,
  });
  environment.statistics = results.warmStats;
  environment.cancellationStatistics = results.cancelStats;
  environment.budgets = results.rows;
  environment.memory = {
    samplingIntervalMs: parsed.memoryIntervalMs,
    warmUp: environment.warmUp.memory,
    iterations: environment.iterations.map((iteration) => ({
      label: iteration.label,
      memory: iteration.memory,
    })),
  };

  const outPath = path.resolve(
    parsed.out ?? path.join(os.tmpdir(), "fruitboard-benchmark-report.json"),
  );
  await mkdir(path.dirname(outPath), { recursive: true });
  writeFileSync(outPath, `${JSON.stringify(environment, null, 2)}\n`, "utf8");

  stdout.push(
    `fixture: ${fixtureDocument.sizeLabel}, seed ${fixtureDocument.seed}, sha-256 ${environment.fixture.manifestSha256}`,
  );
  stdout.push(
    `scan window: ${warmUp.record.scanMs ?? "n/a"} ms first run (warm-up, ${warmUp.record.status ?? "no terminal"}); ${environment.iterations.length} measured iterations`,
  );
  if (results.warmStats !== null) {
    stdout.push(
      `statistics (scan ms): median ${results.warmStats.medianMs}, max ${results.warmStats.maxMs}, nearest-rank p95 ${results.warmStats.nearestRankP95Ms} (n=${results.warmStats.sampleCount})`,
    );
  }
  if (results.cancelStats !== null) {
    stdout.push(
      `cancellation stop latency (ms): median ${results.cancelStats.medianMs}, max ${results.cancelStats.maxMs} (n=${results.cancelStats.sampleCount})`,
    );
  }
  stdout.push("budgets:");
  for (const row of results.rows) {
    const verdict =
      row.pass === null ? "not-measured" : row.pass ? "PASS" : "FAIL";
    const measured =
      row.measured === null ? "n/a" : `${row.measured} ${row.unit}`;
    stdout.push(
      `  [${verdict}] ${row.id}: ${row.metric} -> ${measured} (target ${row.target} ${row.unit})${row.note ? `; ${row.note}` : ""}`,
    );
  }
  stdout.push(`report written: ${outPath}`);

  if (!parsed.keepFixture) {
    await rm(tempBase, { recursive: true, force: true });
    stdout.push("temporary fixture removed after the run");
  }

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

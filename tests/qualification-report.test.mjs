import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import test from "node:test";

const repositoryRoot = fileURLToPath(new URL("../", import.meta.url));
const validatorPath = fileURLToPath(
  new URL("../scripts/validate-qualification-report.mjs", import.meta.url),
);
const expectedManifest =
  "a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a";

const statistics = (values) => {
  const sorted = [...values].sort((left, right) => left - right);
  const middle = Math.floor(sorted.length / 2);
  return {
    sampleCount: sorted.length,
    medianMs:
      sorted.length % 2 === 1
        ? sorted[middle]
        : (sorted[middle - 1] + sorted[middle]) / 2,
    maxMs: sorted[sorted.length - 1],
    nearestRankP95Ms: sorted[Math.max(1, Math.ceil(0.95 * sorted.length)) - 1],
  };
};

const memory = () => ({
  sampleCount: 2,
  idleWindowSamples: 1,
  idleMb: 1,
  peakMb: 2,
  incrementalMb: 1,
});

const attempt = ({
  label,
  scanMs = 100,
  cancellation = false,
  stopLatencyMs = 100,
  exitCode = 0,
} = {}) => ({
  label,
  exitCode,
  totalMs: 100,
  scanMs,
  status: cancellation ? "Cancelled" : "Published",
  authoritative: !cancellation,
  outcome: cancellation ? "Cancelled" : "Complete",
  locationCount: 10_000,
  generation: 1,
  changesComputed: true,
  errorCode: null,
  failureCode: null,
  failureMessage: null,
  partialClass: null,
  stderrObserved: false,
  lines: [{ phase: "scan_finished" }],
  memory: memory(),
  ...(cancellation ? { cancellationRequestedMs: 10, stopLatencyMs } : {}),
});

const makeReport = ({
  scanTimes = Array(10).fill(100),
  stopTimes = [100, 100, 100],
} = {}) => ({
  schema: "fruitboard/benchmark-report/1",
  role: "synthetic",
  capturedAtIso: "2026-09-13T00:00:00.000Z",
  machine: {},
  cpu: { cores: 1 },
  memoryTotalBytes: 100,
  powerMode: "AC",
  fixtureVolume: null,
  fixture: {
    sizeLabel: "custom-9995",
    seed: "0",
    manifestSha256: expectedManifest,
    hardlinkSupport: "created",
    counts: {
      flpFiles: 9995,
      aliasLocations: 5,
      otherFiles: 4,
      leafDirectories: 1000,
      directories: 1025,
      emptyDirectories: 10,
      hardlinkGroups: 5,
    },
  },
  build: {
    captured: true,
    prebuilt: true,
    command: "synthetic",
    toolchain: "synthetic",
  },
  measurement: {
    status: "measured",
    warmUpRuns: 1,
    measuredIterations: 10,
    statistics: ["median", "max", "nearest-rank-p95"],
    firstRunReportedSeparately: true,
    sampleIntervalMs: 100,
    cancelAfterMs: 250,
    settleMs: 300,
  },
  warmUp: attempt({ label: "warmup" }),
  iterations: scanTimes.map((scanMs, index) =>
    attempt({ label: `iteration-${index}`, scanMs }),
  ),
  statistics: statistics(scanTimes),
  cancellation: stopTimes.map((stopLatencyMs, index) =>
    attempt({
      label: `cancel-${index}`,
      cancellation: true,
      stopLatencyMs,
    }),
  ),
  cancellationStatistics: statistics(stopTimes),
  memory: {
    samplingIntervalMs: 100,
    warmUp: memory(),
    iterations: [],
  },
  budgets: [],
  notes: [],
});

async function runValidator(report) {
  const temporaryDirectory = await mkdtemp(
    path.join(os.tmpdir(), "fruitboard-qualification-report-test-"),
  );
  const reportPath = path.join(temporaryDirectory, "report.json");
  const summaryPath = path.join(temporaryDirectory, "summary.json");
  try {
    await writeFile(reportPath, `${JSON.stringify(report)}\n`, "utf8");
    const result = spawnSync(
      process.execPath,
      [
        validatorPath,
        "--report",
        reportPath,
        "--summary",
        summaryPath,
        "--expected-fixture",
        "custom-9995",
        "--expected-seed",
        "0",
        "--expected-manifest",
        expectedManifest,
        "--expected-locations",
        "10000",
        "--expected-iterations",
        "10",
        "--expected-cancellations",
        "3",
        "--harness-exit",
        "0",
      ],
      {
        cwd: repositoryRoot,
        encoding: "utf8",
        maxBuffer: 4 * 1024 * 1024,
      },
    );
    assert.equal(result.error, undefined, result.error?.message);
    return {
      exitCode: result.status,
      summary: JSON.parse(await readFile(summaryPath, "utf8")),
    };
  } finally {
    await rm(temporaryDirectory, { recursive: true, force: true });
  }
}

const nonQualifying = (result) => {
  assert.equal(result.exitCode, 1);
  assert.equal(result.summary.disposition, "non-qualifying");
  return result.summary.validationFailures.join(" | ");
};

test("accepts a valid report and exposes derived attempt statistics", async () => {
  const result = await runValidator(makeReport());

  assert.equal(result.exitCode, 0);
  assert.equal(result.summary.disposition, "qualified");
  assert.deepEqual(
    result.summary.scanStatistics,
    statistics(Array(10).fill(100)),
  );
  assert.deepEqual(
    result.summary.cancellationStatistics,
    statistics([100, 100, 100]),
  );
  assert.equal(result.summary.attempts[0].exitCode, 0);
  assert.equal(result.summary.attemptFailures.length, 0);
});

test("rejects an understated measured scan summary and budgets the observed scan", async () => {
  const report = makeReport();
  report.iterations[9].scanMs = 20_000;

  const result = await runValidator(report);
  const failures = nonQualifying(result);

  assert.match(failures, /statistics\.maxMs does not match/);
  assert.match(failures, /statistics\.nearestRankP95Ms does not match/);
  assert.match(failures, /warm-reconciliation target missed/);
  assert.equal(result.summary.scanStatistics.nearestRankP95Ms, 20_000);
});

test("rejects an understated cancellation summary and budgets stop latency", async () => {
  const report = makeReport();
  report.cancellation[2].stopLatencyMs = 2_000;

  const result = await runValidator(report);
  const failures = nonQualifying(result);

  assert.match(failures, /cancellationStatistics\.maxMs does not match/);
  assert.match(
    failures,
    /cancellationStatistics\.nearestRankP95Ms does not match/,
  );
  assert.match(failures, /cooperative-stop target missed/);
  assert.equal(result.summary.cancellationStatistics.nearestRankP95Ms, 2_000);
});

test("uses cooperative-stop latency rather than cancellation scan duration", async () => {
  const report = makeReport();
  report.cancellation[0].scanMs = 20_000;

  const result = await runValidator(report);

  assert.equal(result.exitCode, 0);
  assert.equal(result.summary.cancellationStatistics.nearestRankP95Ms, 100);
});

for (const field of ["medianMs", "maxMs", "nearestRankP95Ms", "sampleCount"]) {
  test(`rejects an incorrect reported ${field}`, async () => {
    const report = makeReport();
    report.statistics[field] = field === "sampleCount" ? 9 : 101;

    const result = await runValidator(report);
    const failures = nonQualifying(result);

    assert.match(failures, new RegExp(`statistics\\.${field}`));
    assert.match(failures, /validated individual records/);
  });
}

for (const location of ["warmUp", "measured", "cancellation"]) {
  test(`rejects a nonzero ${location} driver exit`, async () => {
    const report = makeReport();
    const record =
      location === "warmUp"
        ? report.warmUp
        : location === "measured"
          ? report.iterations[0]
          : report.cancellation[0];
    record.exitCode = 7;

    const result = await runValidator(report);
    const failures = nonQualifying(result);

    assert.match(failures, /driver exitCode is not 0/);
    assert.equal(
      result.summary.failedAttemptLabels.includes(record.label),
      true,
    );
    assert.equal(
      result.summary.attempts.find(({ label }) => label === record.label)
        .exitCode,
      7,
    );
  });
}

test("rejects missing and non-authoritative measured attempts", async (t) => {
  await t.test("missing attempt", async () => {
    const report = makeReport();
    report.iterations.pop();

    const failures = nonQualifying(await runValidator(report));
    assert.match(failures, /required measured iteration count is not 10/);
    assert.match(failures, /statistics\.sampleCount/);
  });

  await t.test("non-authoritative attempt", async () => {
    const report = makeReport();
    report.iterations[0].authoritative = false;

    const result = await runValidator(report);
    const failures = nonQualifying(result);
    assert.match(
      failures,
      /incomplete or non-authoritative measured observations/,
    );
    assert.match(failures, /statistics\.sampleCount/);
    assert.equal(
      result.summary.failedAttemptLabels.includes("iteration-0"),
      true,
    );
  });
});

for (const malformed of [
  [
    "scanMs",
    (report) => {
      report.iterations[0].scanMs = "100";
    },
  ],
  [
    "memory",
    (report) => {
      report.iterations[0].memory.peakMb = "2";
    },
  ],
]) {
  test(`rejects malformed numeric ${malformed[0]}`, async () => {
    const report = makeReport();
    malformed[1](report);

    const failures = nonQualifying(await runValidator(report));
    assert.match(failures, /invalid/);
  });
}

test("rejects a fractional target miss without rounding", async () => {
  const scanTimes = Array(10).fill(10_000.5);
  const result = await runValidator(makeReport({ scanTimes }));
  const failures = nonQualifying(result);

  assert.match(failures, /warm-reconciliation target missed/);
  assert.equal(result.summary.scanStatistics.nearestRankP95Ms, 10_000.5);
});

for (const missing of [
  [
    "stop latency",
    (report) => {
      delete report.cancellation[0].stopLatencyMs;
    },
  ],
  [
    "cancellation request time",
    (report) => {
      delete report.cancellation[0].cancellationRequestedMs;
    },
  ],
  [
    "working-set metric",
    (report) => {
      delete report.iterations[0].memory.peakMb;
    },
  ],
]) {
  test(`rejects a missing required ${missing[0]}`, async () => {
    const report = makeReport();
    missing[1](report);

    const failures = nonQualifying(await runValidator(report));
    assert.match(failures, /missing or invalid|missing or malformed/);
  });
}

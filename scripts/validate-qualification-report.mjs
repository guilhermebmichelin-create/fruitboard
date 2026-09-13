#!/usr/bin/env node

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";

const defaults = {
  fixture: "custom-9995",
  seed: "0",
  manifest: "a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a",
  locations: 10_000,
  iterations: 10,
  cancellations: 3,
};
const statuses = new Set(
  "Published Failed Cancelled Interrupted Fenced".split(" "),
);
const outcomes = new Set(
  "Complete Partial Denied RootUnavailable Cancelled ResourceLimit SinkFailed Invalid UnsupportedFilesystem".split(
    " ",
  ),
);
const partialClasses = new Set(
  `partial_root_identity_unavailable partial_directory_not_found
   partial_directory_read partial_directory_changed
   partial_directory_identity_unavailable partial_entry_disappeared
   partial_metadata_read partial_timestamp_out_of_range
   partial_byte_size_out_of_range partial_locator_key_too_long
   partial_multiple partial_unknown`
    .trim()
    .split(/\s+/u),
);
const diagnosticCodes = new Set(
  `access_denied unsupported resource_limit unavailable cancelled worker_failed
   driver_argument driver_database_open driver_worker_config driver_session_start
   driver_root_register driver_root_list driver_no_execution driver_poll
   driver_request_scan storage_io_failed storage_busy storage_unsafe_location
   storage_unsupported_sqlite storage_unsupported_journal storage_newer_schema
   storage_invalid_schema storage_migration_failed storage_invalid_backup
   storage_conflict storage_staging_rejected storage_not_found storage_invalid_cursor
   storage_stale_cursor storage_database_failed`
    .trim()
    .split(/\s+/u),
);
for (const value of partialClasses) diagnosticCodes.add(value);
const linePhases = new Set(
  "ready scan_started cancellation_requested scan_finished error invalid_protocol unknown_protocol unparseable".split(
    " ",
  ),
);

const isObject = (value) =>
  value !== null && typeof value === "object" && !Array.isArray(value);
const isArray = Array.isArray;
const has = (value, key) =>
  isObject(value) && Object.prototype.hasOwnProperty.call(value, key);
const get = (value, key) => (has(value, key) ? value[key] : undefined);
const string = (value) => typeof value === "string" && value.trim() !== "";
const integer = (value) =>
  typeof value === "number" && Number.isSafeInteger(value);
const nonNegativeInteger = (value) => integer(value) && value >= 0;
const duration = (value) =>
  typeof value === "number" && Number.isFinite(value) && value >= 0;
const nullable = (value, predicate) => value === null || predicate(value);
const oneOf = (value, allowed) =>
  value === null || (typeof value === "string" && allowed.has(value));
const identifier = (value) =>
  typeof value === "string" && /^[0-9a-f-]{1,128}$/iu.test(value);

function parseArgs(argv) {
  const options = { ...defaults, report: null, summary: null, harnessExit: 0 };
  const valueFlags = new Map([
    ["--report", "report"],
    ["--summary", "summary"],
    ["--expected-fixture", "fixture"],
    ["--expected-seed", "seed"],
    ["--expected-manifest", "manifest"],
    ["--expected-locations", "locations"],
    ["--expected-iterations", "iterations"],
    ["--expected-cancellations", "cancellations"],
    ["--harness-exit", "harnessExit"],
  ]);
  const numericFlags = new Set([
    "--expected-locations",
    "--expected-iterations",
    "--expected-cancellations",
    "--harness-exit",
  ]);
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    const equals = argument.indexOf("=");
    const flag = equals < 0 ? argument : argument.slice(0, equals);
    if (!valueFlags.has(flag)) throw new Error(`unknown argument: ${flag}`);
    const raw =
      equals < 0
        ? (argv[++index] ??
          (() => {
            throw new Error(`missing value for ${flag}`);
          })())
        : argument.slice(equals + 1);
    options[valueFlags.get(flag)] = numericFlags.has(flag) ? Number(raw) : raw;
  }
  if (options.report === null || options.summary === null) {
    throw new Error("--report and --summary are required");
  }
  for (const field of ["locations", "iterations", "cancellations"]) {
    if (!nonNegativeInteger(options[field]))
      throw new Error(`invalid ${field}`);
  }
  if (!integer(options.harnessExit))
    throw new Error("invalid harness exit code");
  if (!/^[0-9a-f]{64}$/iu.test(options.manifest)) {
    throw new Error("expected manifest must be a SHA-256 hex string");
  }
  return options;
}

function requireFields(value, fields, label, fail) {
  for (const field of fields) {
    if (!has(value, field)) fail(`${label} is missing ${field}`);
  }
}

function validateMemory(memory, label, fail) {
  if (!isObject(memory)) {
    fail(`${label} is missing or not an object`);
    return;
  }
  for (const field of ["sampleCount", "idleWindowSamples"]) {
    if (!has(memory, field) || !nonNegativeInteger(get(memory, field))) {
      fail(`${label}.${field} is missing or invalid`);
    }
  }
  if (get(memory, "sampleCount") < 1) fail(`${label}.sampleCount is invalid`);
  for (const field of ["idleMb", "peakMb", "incrementalMb"]) {
    if (!has(memory, field) || !duration(get(memory, field))) {
      fail(`${label}.${field} is missing or invalid`);
    }
  }
}

function validateLine(line, label, fail) {
  if (!isObject(line)) {
    fail(`${label} is not an object`);
    return;
  }
  if (!linePhases.has(get(line, "phase"))) fail(`${label}.phase is invalid`);
  for (const field of ["elapsed_ms", "scan_ms"]) {
    if (has(line, field) && !duration(get(line, field)))
      fail(`${label}.${field} is invalid`);
  }
  for (const field of [
    "location_count",
    "generation",
    "changes_added",
    "changes_modified",
    "changes_replaced",
    "changes_identity_uncertain",
    "changes_missing",
    "changes_restored",
    "changes_renames",
  ]) {
    if (has(line, field) && !nonNegativeInteger(get(line, field)))
      fail(`${label}.${field} is invalid`);
  }
  if (has(line, "status") && !oneOf(get(line, "status"), statuses))
    fail(`${label}.status is invalid`);
  if (has(line, "outcome") && !oneOf(get(line, "outcome"), outcomes))
    fail(`${label}.outcome is invalid`);
  for (const field of ["authoritative", "changes_computed"]) {
    if (
      has(line, field) &&
      !nullable(get(line, field), (value) => typeof value === "boolean")
    )
      fail(`${label}.${field} is invalid`);
  }
  if (
    has(line, "error_code") &&
    !oneOf(get(line, "error_code"), diagnosticCodes)
  )
    fail(`${label}.error_code is invalid`);
  if (
    has(line, "partial_class") &&
    !oneOf(get(line, "partial_class"), partialClasses)
  )
    fail(`${label}.partial_class is invalid`);
  for (const field of ["root_id", "session_id", "run_id", "job_id"]) {
    if (has(line, field) && !identifier(get(line, field)))
      fail(`${label}.${field} is invalid`);
  }
  if (has(line, "worker_config") && get(line, "worker_config") !== "default")
    fail(`${label}.worker_config is invalid`);
}

function validateRecord(record, label, cancellation, fail) {
  if (!isObject(record)) {
    fail(`${label} is not an object`);
    return;
  }
  requireFields(
    record,
    `label exitCode totalMs scanMs status authoritative outcome locationCount generation changesComputed errorCode failureCode failureMessage partialClass stderrObserved lines memory`.split(
      " ",
    ),
    label,
    fail,
  );
  if (!string(get(record, "label"))) fail(`${label}.label is invalid`);
  if (!integer(get(record, "exitCode"))) fail(`${label}.exitCode is invalid`);
  if (!duration(get(record, "totalMs"))) fail(`${label}.totalMs is invalid`);
  if (!nullable(get(record, "scanMs"), duration))
    fail(`${label}.scanMs is invalid`);
  if (!oneOf(get(record, "status"), statuses))
    fail(`${label}.status is invalid`);
  if (typeof get(record, "authoritative") !== "boolean")
    fail(`${label}.authoritative is invalid`);
  if (!oneOf(get(record, "outcome"), outcomes))
    fail(`${label}.outcome is invalid`);
  for (const field of ["locationCount", "generation"]) {
    if (!nullable(get(record, field), nonNegativeInteger))
      fail(`${label}.${field} is invalid`);
  }
  if (
    !nullable(
      get(record, "changesComputed"),
      (value) => typeof value === "boolean",
    )
  )
    fail(`${label}.changesComputed is invalid`);
  for (const field of ["errorCode", "failureCode"]) {
    if (!oneOf(get(record, field), diagnosticCodes))
      fail(`${label}.${field} is invalid`);
  }
  const failureMessage = get(record, "failureMessage");
  if (
    failureMessage !== null &&
    failureMessage !== "driver_error" &&
    !diagnosticCodes.has(failureMessage)
  ) {
    fail(`${label}.failureMessage is invalid`);
  }
  if (!oneOf(get(record, "partialClass"), partialClasses))
    fail(`${label}.partialClass is invalid`);
  if (
    get(record, "outcome") === "Partial" &&
    !partialClasses.has(get(record, "partialClass"))
  ) {
    fail(`${label} Partial outcome has no bounded partialClass`);
  }
  if (typeof get(record, "stderrObserved") !== "boolean")
    fail(`${label}.stderrObserved is invalid`);
  const lines = get(record, "lines");
  if (
    !isArray(lines) ||
    lines.length < 1 ||
    lines.some((line) => !isObject(line))
  ) {
    fail(`${label}.lines is missing, empty, or malformed`);
  } else {
    lines.forEach((line, index) =>
      validateLine(line, `${label}.lines[${index}]`, fail),
    );
  }
  validateMemory(get(record, "memory"), `${label}.memory`, fail);
  const diagnostic = ["errorCode", "failureCode", "failureMessage"].some(
    (field) => get(record, field) !== null && get(record, field) !== "",
  );
  if (
    (get(record, "status") === "Failed" ||
      [
        "Denied",
        "ResourceLimit",
        "UnsupportedFilesystem",
        "RootUnavailable",
        "SinkFailed",
        "Invalid",
      ].includes(get(record, "outcome"))) &&
    !diagnostic &&
    get(record, "outcome") !== "Partial"
  ) {
    fail(`${label} failed outcome has no fixed diagnostic`);
  }
  if (cancellation) {
    for (const field of ["cancellationRequestedMs", "stopLatencyMs"]) {
      if (!has(record, field) || !duration(get(record, field)))
        fail(`${label}.${field} is missing or invalid`);
    }
  }
}

function validateStatistics(statistics, label, expected, fail) {
  if (!isObject(statistics)) {
    fail(`${label} is missing or not an object`);
    return;
  }
  if (
    !nonNegativeInteger(get(statistics, "sampleCount")) ||
    get(statistics, "sampleCount") !== expected
  ) {
    fail(`${label}.sampleCount is not ${expected}`);
  }
  for (const field of ["medianMs", "maxMs", "nearestRankP95Ms"]) {
    if (!duration(get(statistics, field)))
      fail(`${label}.${field} is missing or invalid`);
  }
}

function validateReport(options) {
  const failures = [];
  const fail = (message) => failures.push(message);
  let report;
  try {
    report = JSON.parse(readFileSync(options.report, "utf8"));
  } catch {
    fail("report is not readable JSON");
  }
  let warmUp = null;
  let iterations = [];
  let cancellation = [];
  let statistics = null;
  let cancellationStatistics = null;
  if (!isObject(report)) {
    fail("report root is missing or not an object");
  } else {
    requireFields(
      report,
      "schema role capturedAtIso machine cpu memoryTotalBytes powerMode fixtureVolume fixture build measurement warmUp iterations statistics cancellation cancellationStatistics memory budgets notes".split(
        " ",
      ),
      "report",
      fail,
    );
    if (get(report, "schema") !== "fruitboard/benchmark-report/1")
      fail("report schema is unexpected");
    for (const field of ["role", "capturedAtIso"])
      if (!string(get(report, field))) fail(`report.${field} is invalid`);
    if (!nonNegativeInteger(get(report, "memoryTotalBytes")))
      fail("report.memoryTotalBytes is invalid");
    for (const field of ["machine", "cpu"])
      if (!isObject(get(report, field))) fail(`report.${field} is invalid`);
    if (
      isObject(get(report, "cpu")) &&
      !nonNegativeInteger(get(get(report, "cpu"), "cores"))
    )
      fail("report.cpu.cores is invalid");
    const volume = get(report, "fixtureVolume");
    if (volume !== null && !isObject(volume))
      fail("report.fixtureVolume is invalid");
    if (isObject(volume))
      for (const field of ["totalBytes", "freeBytes"])
        if (!nonNegativeInteger(get(volume, field)))
          fail(`report.fixtureVolume.${field} is invalid`);

    const measurement = get(report, "measurement");
    if (!isObject(measurement)) fail("report.measurement is invalid");
    else {
      if (get(measurement, "status") !== "measured")
        fail("report.measurement.status is not measured");
      if (get(measurement, "warmUpRuns") !== 1)
        fail("report.measurement.warmUpRuns is not 1");
      if (get(measurement, "measuredIterations") !== options.iterations)
        fail("report.measurement.measuredIterations is not selected");
      if (
        JSON.stringify(get(measurement, "statistics")) !==
        JSON.stringify(["median", "max", "nearest-rank-p95"])
      )
        fail("report.measurement.statistics is not the harness list");
      if (get(measurement, "firstRunReportedSeparately") !== true)
        fail("report.measurement.firstRunReportedSeparately is not true");
      for (const [field, expected] of Object.entries({
        sampleIntervalMs: 100,
        cancelAfterMs: 250,
        settleMs: 300,
      })) {
        if (
          !duration(get(measurement, field)) ||
          get(measurement, field) !== expected
        )
          fail(`report.measurement.${field} is not selected`);
      }
    }

    const build = get(report, "build");
    if (
      !isObject(build) ||
      get(build, "captured") !== true ||
      get(build, "prebuilt") !== true ||
      !string(get(build, "command")) ||
      !string(get(build, "toolchain"))
    )
      fail("report.build does not record the selected --bin build");
    const fixture = get(report, "fixture");
    if (!isObject(fixture)) fail("report.fixture is invalid");
    else {
      if (
        get(fixture, "sizeLabel") !== options.fixture ||
        get(fixture, "seed") !== options.seed ||
        get(fixture, "manifestSha256") !== options.manifest
      )
        fail("report.fixture identity mismatch");
      if (get(fixture, "hardlinkSupport") !== "created")
        fail("report.fixture.hardlinkSupport is not created");
      const counts = get(fixture, "counts");
      if (!isObject(counts)) fail("report.fixture.counts is invalid");
      else {
        const fields =
          "flpFiles aliasLocations otherFiles leafDirectories directories emptyDirectories hardlinkGroups".split(
            " ",
          );
        for (const field of fields)
          if (!nonNegativeInteger(get(counts, field)))
            fail(`report.fixture.counts.${field} is invalid`);
        const expected = {
          flpFiles: 9995,
          aliasLocations: 5,
          otherFiles: 4,
          leafDirectories: 1000,
          directories: 1025,
          emptyDirectories: 10,
          hardlinkGroups: 5,
        };
        if (
          fields.some((field) => get(counts, field) !== expected[field]) ||
          get(counts, "flpFiles") + get(counts, "aliasLocations") !==
            options.locations
        )
          fail(
            "report.fixture accounting does not match the selected observation count",
          );
      }
    }

    warmUp = get(report, "warmUp");
    const measured = get(report, "iterations");
    const cancelled = get(report, "cancellation");
    if (!isArray(measured))
      fail("report.iterations is missing or not an array");
    else {
      iterations = measured;
      if (iterations.length !== options.iterations)
        fail(`required measured iteration count is not ${options.iterations}`);
    }
    if (!isArray(cancelled))
      fail("report.cancellation is missing or not an array");
    else {
      cancellation = cancelled;
      if (cancellation.length !== options.cancellations)
        fail(`required cancellation count is not ${options.cancellations}`);
    }
    statistics = get(report, "statistics");
    cancellationStatistics = get(report, "cancellationStatistics");
    validateStatistics(statistics, "statistics", options.iterations, fail);
    validateStatistics(
      cancellationStatistics,
      "cancellationStatistics",
      options.cancellations,
      fail,
    );
    const memory = get(report, "memory");
    if (
      !isObject(memory) ||
      !duration(get(memory, "samplingIntervalMs")) ||
      !isObject(get(memory, "warmUp")) ||
      !isArray(get(memory, "iterations"))
    )
      fail("report.memory is missing or malformed");
    if (!isArray(get(report, "budgets")))
      fail("report.budgets is missing or not an array");
    if (!isArray(get(report, "notes")))
      fail("report.notes is missing or not an array");
  }

  if (warmUp !== null) validateRecord(warmUp, "warmUp", false, fail);
  iterations.forEach((record, index) =>
    validateRecord(record, `iteration-${index + 1}`, false, fail),
  );
  cancellation.forEach((record, index) =>
    validateRecord(record, `cancel-${index + 1}`, true, fail),
  );
  const successful = iterations.filter(
    (record) =>
      isObject(record) &&
      get(record, "authoritative") === true &&
      get(record, "status") === "Published" &&
      get(record, "outcome") === "Complete" &&
      duration(get(record, "scanMs")) &&
      nonNegativeInteger(get(record, "locationCount")) &&
      get(record, "locationCount") === options.locations,
  );
  const failed = iterations.filter((record) => !successful.includes(record));
  if (successful.length !== options.iterations)
    fail("incomplete or non-authoritative measured observations");
  const warmScan = get(warmUp, "scanMs");
  if (!duration(warmScan)) fail("warm-up timing is missing or invalid");
  else if (
    get(warmUp, "authoritative") !== true ||
    get(warmUp, "status") !== "Published" ||
    get(warmUp, "outcome") !== "Complete" ||
    get(warmUp, "locationCount") !== options.locations
  )
    fail(
      "warm-up was not authoritative Complete with the selected location count",
    );
  else if (warmScan > 30_000)
    fail("first-discovery target missed: warm-up exceeds 30,000 ms");
  const warmP95 = get(statistics, "nearestRankP95Ms");
  if (duration(warmP95) && warmP95 > 10_000)
    fail(
      "warm-reconciliation target missed: nearest-rank p95 exceeds 10,000 ms",
    );
  const cancelP95 = get(cancellationStatistics, "nearestRankP95Ms");
  if (duration(cancelP95) && cancelP95 > 1_000)
    fail("cooperative-stop target missed: cancellation p95 exceeds 1,000 ms");
  cancellation.forEach((record) => {
    if (
      get(record, "status") !== "Cancelled" ||
      get(record, "outcome") !== "Cancelled"
    )
      fail(
        `cancellation record ${String(get(record, "label"))} is not terminal Cancelled`,
      );
  });
  if (options.harnessExit !== 0)
    fail(`benchmark harness exited with code ${options.harnessExit}`);

  const attempts = [warmUp, ...iterations, ...cancellation].filter(
    (value) => value !== null && value !== undefined,
  );
  return {
    disposition: failures.length === 0 ? "qualified" : "non-qualifying",
    harnessExit: options.harnessExit,
    measuredAttempts: iterations.length,
    successfulAuthoritative: successful.length,
    authoritativeSampleCount: get(statistics, "sampleCount"),
    failedOrNonAuthoritative: failed.length,
    failedLabels: failed.map((record) => get(record, "label")),
    validationFailures: failures,
    warmUp,
    scanStatistics: statistics,
    cancellationStatistics,
    budgetRows: get(report, "budgets"),
    attempts: attempts.map((record) => ({
      label: get(record, "label"),
      status: get(record, "status"),
      authoritative: get(record, "authoritative"),
      outcome: get(record, "outcome"),
      locationCount: get(record, "locationCount"),
      scanMs: get(record, "scanMs"),
      stopLatencyMs: get(record, "stopLatencyMs"),
      errorCode: get(record, "errorCode"),
      failureCode: get(record, "failureCode"),
      partialClass: get(record, "partialClass"),
      stderrObserved: get(record, "stderrObserved"),
      memory: get(record, "memory"),
    })),
  };
}

function writeSummary(summary, outputPath) {
  mkdirSync(path.dirname(path.resolve(outputPath)), { recursive: true });
  writeFileSync(outputPath, `${JSON.stringify(summary, null, 2)}\n`, "utf8");
  console.log(JSON.stringify(summary, null, 2));
}

let options;
try {
  options = parseArgs(process.argv.slice(2));
  const summary = validateReport(options);
  writeSummary(summary, options.summary);
  process.exitCode = summary.disposition === "qualified" ? 0 : 1;
} catch (error) {
  const summary = {
    disposition: "non-qualifying",
    validationFailures: [error.message],
  };
  if (options?.summary) writeSummary(summary, options.summary);
  else console.error(error.message);
  process.exitCode = 1;
}

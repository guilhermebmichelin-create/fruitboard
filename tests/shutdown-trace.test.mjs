import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import {
  MAX_SHUTDOWN_TRACE_BYTES,
  validateShutdownTraceText,
} from "../scripts/validate-shutdown-trace.mjs";

const STANDARD_STAGES = [
  "shutdown_start",
  "sample_checks_start",
  "sample_checks_end",
  "scan_console_start",
  "scan_console_end",
  "shutdown_end",
];

const ANALYSIS_STAGES = [
  "shutdown_start",
  "sample_checks_start",
  "sample_checks_end",
  "analysis_host_start",
  "analysis_host_end",
  "scan_console_start",
  "scan_console_end",
  "shutdown_end",
];

const traceText = (stages, elapsed = stages.map((_, index) => index * 3)) =>
  stages
    .map((stage, index) => JSON.stringify({ stage, elapsedMs: elapsed[index] }))
    .join("\n") + "\n";

function assertInvalid(text, options) {
  assert.throws(
    () => validateShutdownTraceText(text, options),
    (error) => error?.code === "shutdown_trace_invalid",
  );
}

test("validates the six-stage trace and returns fixed numeric timings", () => {
  assert.deepEqual(validateShutdownTraceText(traceText(STANDARD_STAGES)), {
    stageCount: 6,
    shutdownMs: 15,
    sampleChecksMs: 3,
    scanConsoleMs: 3,
  });
});

test("validates the eight-stage analysis trace and includes its timing", () => {
  assert.deepEqual(
    validateShutdownTraceText(traceText(ANALYSIS_STAGES), {
      analysisJobs: true,
    }),
    {
      stageCount: 8,
      shutdownMs: 21,
      sampleChecksMs: 3,
      analysisHostMs: 3,
      scanConsoleMs: 3,
    },
  );
});

test("accepts JSON whitespace and either field order while rejecting non-JSON whitespace", () => {
  const validJsonWhitespace =
    STANDARD_STAGES.map((stage, index) => {
      const elapsedMs = index * 3;
      return index % 2 === 0
        ? `\t{ "stage" : "${stage}", "elapsedMs" : ${elapsedMs} }\r`
        : ` { "elapsedMs" : ${elapsedMs}, "stage" : "${stage}" } `;
    }).join("\n") + "\n";
  const invalidWhitespace = `\v${traceText(STANDARD_STAGES)}`;

  assert.equal(validateShutdownTraceText(validJsonWhitespace).shutdownMs, 15);
  assertInvalid(invalidWhitespace);
});

test("rejects missing, empty, truncated, blank, and appended traces", () => {
  const valid = traceText(STANDARD_STAGES);
  const records = valid.trimEnd().split("\n");
  const blankLine = [...records.slice(0, 1), "", ...records.slice(1), ""].join(
    "\n",
  );
  assertInvalid("");
  assertInvalid(valid.slice(0, -1));
  assertInvalid(blankLine);
  assertInvalid(valid + valid);
});

test("rejects duplicate, reordered, unknown, or feature-inconsistent stages", () => {
  const duplicate = [...STANDARD_STAGES];
  duplicate[3] = "sample_checks_end";
  const reordered = [...STANDARD_STAGES];
  [reordered[3], reordered[4]] = [reordered[4], reordered[3]];
  const unknown = [...STANDARD_STAGES];
  unknown[3] = "private_path_stage";

  assertInvalid(traceText(duplicate));
  assertInvalid(traceText(reordered));
  assertInvalid(traceText(unknown));
  assertInvalid(traceText(ANALYSIS_STAGES));
  assertInvalid(traceText(STANDARD_STAGES), { analysisJobs: true });
});

test("rejects extra or duplicate fields and malformed elapsed values", () => {
  const valid = traceText(STANDARD_STAGES);
  const extraField = valid.replace(
    '"stage":"shutdown_start","elapsedMs":0',
    '"stage":"shutdown_start","elapsedMs":0,"path":"private"',
  );
  const duplicateField = valid.replace(
    '"stage":"shutdown_start","elapsedMs":0',
    '"stage":"shutdown_start","stage":"shutdown_start","elapsedMs":0',
  );
  const negative = valid.replace('"elapsedMs":0', '"elapsedMs":-1');
  const fractional = valid.replace('"elapsedMs":0', '"elapsedMs":0.5');
  const stringValue = valid.replace('"elapsedMs":0', '"elapsedMs":"0"');
  const unsafeInteger = valid.replace(
    '"elapsedMs":0',
    '"elapsedMs":9007199254740992',
  );

  assertInvalid(extraField);
  assertInvalid(duplicateField);
  assertInvalid(negative);
  assertInvalid(fractional);
  assertInvalid(stringValue);
  assertInvalid(unsafeInteger);
});

test("rejects decreasing or oversized elapsed data", () => {
  const decreasing = traceText(STANDARD_STAGES, [0, 4, 3, 5, 6, 7]);
  assertInvalid(decreasing);
  assertInvalid("x".repeat(MAX_SHUTDOWN_TRACE_BYTES + 1));
});

test("the CLI validates bounded files and keeps failures path- and content-free", () => {
  const directory = mkdtempSync(join(tmpdir(), "fruitboard-shutdown-trace-"));
  const validPath = join(directory, "valid.shutdown-stages.jsonl");
  const analysisPath = join(directory, "analysis.shutdown-stages.jsonl");
  const invalidPath = join(directory, "private-name.shutdown-stages.jsonl");
  const oversizedPath = join(directory, "oversized.shutdown-stages.jsonl");
  const cliPath = fileURLToPath(
    new URL("../scripts/validate-shutdown-trace.mjs", import.meta.url),
  );
  const privateText = `private marker C:\\Users\\Person\\session\n`;

  try {
    writeFileSync(validPath, traceText(STANDARD_STAGES), "utf8");
    writeFileSync(analysisPath, traceText(ANALYSIS_STAGES), "utf8");
    writeFileSync(invalidPath, privateText, "utf8");
    writeFileSync(oversizedPath, "x".repeat(MAX_SHUTDOWN_TRACE_BYTES + 1));

    const valid = spawnSync(process.execPath, [cliPath, validPath], {
      encoding: "utf8",
    });
    assert.equal(valid.status, 0);
    assert.equal(valid.stderr, "");
    assert.deepEqual(JSON.parse(valid.stdout), {
      stageCount: 6,
      shutdownMs: 15,
      sampleChecksMs: 3,
      scanConsoleMs: 3,
    });

    const analysis = spawnSync(
      process.execPath,
      [cliPath, analysisPath, "--analysis-jobs"],
      { encoding: "utf8" },
    );
    assert.equal(analysis.status, 0);
    assert.equal(analysis.stderr, "");
    assert.deepEqual(JSON.parse(analysis.stdout), {
      stageCount: 8,
      shutdownMs: 21,
      sampleChecksMs: 3,
      scanConsoleMs: 3,
      analysisHostMs: 3,
    });

    const oversized = spawnSync(process.execPath, [cliPath, oversizedPath], {
      encoding: "utf8",
    });
    assert.equal(oversized.status, 1);
    assert.equal(oversized.stderr, "Shutdown trace is invalid.\n");
    assert.equal(oversized.stdout, "");

    const invalid = spawnSync(process.execPath, [cliPath, invalidPath], {
      encoding: "utf8",
    });
    assert.equal(invalid.status, 1);
    assert.equal(invalid.stderr, "Shutdown trace is invalid.\n");
    assert.equal(invalid.stdout, "");
    assert.doesNotMatch(
      invalid.stderr + invalid.stdout,
      /private marker|Person|session/,
    );
    assert.doesNotMatch(
      invalid.stderr + invalid.stdout,
      /invalidPath|private-name/,
    );

    const unavailable = spawnSync(
      process.execPath,
      [cliPath, join(directory, "missing.jsonl")],
      { encoding: "utf8" },
    );
    assert.equal(unavailable.status, 1);
    assert.equal(unavailable.stderr, "Could not read shutdown trace.\n");
    assert.doesNotMatch(unavailable.stderr, /missing\.jsonl/);

    const misuse = spawnSync(
      process.execPath,
      [cliPath, invalidPath, "--unknown"],
      {
        encoding: "utf8",
      },
    );
    assert.equal(misuse.status, 2);
    assert.match(
      misuse.stderr,
      /^Usage: node scripts\/validate-shutdown-trace\.mjs <trace-path> \[--analysis-jobs\]\n$/,
    );
    assert.doesNotMatch(misuse.stderr, /private-name|invalidPath/);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

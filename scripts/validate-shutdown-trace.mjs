import {
  closeSync,
  constants as fsConstants,
  fstatSync,
  lstatSync,
  openSync,
  readSync,
} from "node:fs";
import process from "node:process";
import { pathToFileURL } from "node:url";

export const MAX_SHUTDOWN_TRACE_BYTES = 4096;

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

const TRACE_INVALID = "shutdown_trace_invalid";
const TRACE_UNAVAILABLE = "shutdown_trace_unavailable";
const USAGE =
  "Usage: node scripts/validate-shutdown-trace.mjs <trace-path> [--analysis-jobs]";

function fail(code = TRACE_INVALID) {
  const error = new Error(code);
  error.code = code;
  throw error;
}

function parseRecord(line) {
  // This intentionally accepts only the two primitive fields in the native
  // trace contract. It also rejects duplicate keys, escaped substitutions,
  // nested values, and any additional content without echoing it.
  const stageFirst =
    /^[ \t\r]*\{[ \t\r]*"stage"[ \t\r]*:[ \t\r]*"([a-z_]+)"[ \t\r]*,[ \t\r]*"elapsedMs"[ \t\r]*:[ \t\r]*(0|[1-9][0-9]*)[ \t\r]*\}[ \t\r]*$/;
  const elapsedFirst =
    /^[ \t\r]*\{[ \t\r]*"elapsedMs"[ \t\r]*:[ \t\r]*(0|[1-9][0-9]*)[ \t\r]*,[ \t\r]*"stage"[ \t\r]*:[ \t\r]*"([a-z_]+)"[ \t\r]*\}[ \t\r]*$/;
  const match = stageFirst.exec(line);
  const reversed = match ? null : elapsedFirst.exec(line);
  if (!match && !reversed) fail();

  const stage = match ? match[1] : reversed[2];
  const elapsedMs = Number(match ? match[2] : reversed[1]);
  if (!Number.isSafeInteger(elapsedMs) || elapsedMs < 0) fail();
  return { stage, elapsedMs };
}

export function validateShutdownTraceText(text, { analysisJobs = false } = {}) {
  if (typeof text !== "string" || typeof analysisJobs !== "boolean") fail();
  if (Buffer.byteLength(text, "utf8") > MAX_SHUTDOWN_TRACE_BYTES) fail();
  if (text.length === 0 || !text.endsWith("\n")) fail();

  const lines = text.slice(0, -1).split("\n");
  const expected = analysisJobs ? ANALYSIS_STAGES : STANDARD_STAGES;
  if (lines.length !== expected.length) fail();

  const records = lines.map(parseRecord);
  let previousElapsedMs = -1;
  for (let index = 0; index < records.length; index += 1) {
    const record = records[index];
    if (record.stage !== expected[index]) fail();
    if (record.elapsedMs < previousElapsedMs) fail();
    previousElapsedMs = record.elapsedMs;
  }
  if (records.at(-1)?.stage !== "shutdown_end") fail();

  const elapsed = (start, end) => {
    const startRecord = records.find((record) => record.stage === start);
    const endRecord = records.find((record) => record.stage === end);
    return endRecord.elapsedMs - startRecord.elapsedMs;
  };

  const summary = {
    stageCount: records.length,
    shutdownMs: elapsed("shutdown_start", "shutdown_end"),
    sampleChecksMs: elapsed("sample_checks_start", "sample_checks_end"),
    scanConsoleMs: elapsed("scan_console_start", "scan_console_end"),
  };
  if (analysisJobs) {
    summary.analysisHostMs = elapsed(
      "analysis_host_start",
      "analysis_host_end",
    );
  }
  return summary;
}

export function validateShutdownTraceFile(tracePath, options = {}) {
  if (typeof tracePath !== "string" || tracePath.length === 0) {
    fail(TRACE_UNAVAILABLE);
  }

  let descriptor;
  try {
    const linkInfo = lstatSync(tracePath);
    if (linkInfo.isSymbolicLink() || !linkInfo.isFile()) fail();

    const noFollow = fsConstants.O_NOFOLLOW ?? 0;
    descriptor = openSync(tracePath, fsConstants.O_RDONLY | noFollow);
    const before = fstatSync(descriptor);
    if (
      !before.isFile() ||
      before.size <= 0 ||
      before.size > MAX_SHUTDOWN_TRACE_BYTES
    ) {
      fail();
    }

    const buffer = Buffer.alloc(MAX_SHUTDOWN_TRACE_BYTES + 1);
    let bytesRead = 0;
    while (bytesRead < buffer.length) {
      const count = readSync(
        descriptor,
        buffer,
        bytesRead,
        buffer.length - bytesRead,
        bytesRead,
      );
      if (count === 0) break;
      bytesRead += count;
    }

    const after = fstatSync(descriptor);
    if (
      bytesRead > MAX_SHUTDOWN_TRACE_BYTES ||
      bytesRead !== before.size ||
      after.size !== before.size
    ) {
      fail();
    }

    let text;
    try {
      text = new TextDecoder("utf-8", { fatal: true }).decode(
        buffer.subarray(0, bytesRead),
      );
    } catch {
      fail();
    }
    return validateShutdownTraceText(text, options);
  } catch (error) {
    if (error?.code === TRACE_INVALID || error?.code === TRACE_UNAVAILABLE)
      throw error;
    fail(TRACE_UNAVAILABLE);
  } finally {
    if (descriptor !== undefined) {
      try {
        closeSync(descriptor);
      } catch {
        // A close failure cannot make an already bounded read unsafe.
      }
    }
  }
}

export function runCli(
  args,
  { stdout = process.stdout, stderr = process.stderr } = {},
) {
  if (
    !Array.isArray(args) ||
    (args.length !== 1 && args.length !== 2) ||
    (args.length === 2 && args[1] !== "--analysis-jobs")
  ) {
    stderr.write(`${USAGE}\n`);
    return 2;
  }

  try {
    const summary = validateShutdownTraceFile(args[0], {
      analysisJobs: args.length === 2,
    });
    stdout.write(`${JSON.stringify(summary)}\n`);
    return 0;
  } catch (error) {
    const message =
      error?.code === TRACE_UNAVAILABLE
        ? "Could not read shutdown trace."
        : "Shutdown trace is invalid.";
    stderr.write(`${message}\n`);
    return 1;
  }
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  process.exitCode = runCli(process.argv.slice(2));
}

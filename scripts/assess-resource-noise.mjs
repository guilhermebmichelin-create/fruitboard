#!/usr/bin/env node
import { readFileSync, statSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const RESOURCE_NOISE_LIMITS = Object.freeze({
  initialProcessorLoadPercent: 20,
  unrelatedProcessCpuSeconds: 0.5,
});

const invalid = () => {
  throw new TypeError("Invalid resource noise receipt.");
};
const record = (value) =>
  value !== null && typeof value === "object" && !Array.isArray(value);
const identity = (value) => {
  if (
    !record(value) ||
    !Number.isSafeInteger(value.pid) ||
    value.pid <= 0 ||
    typeof value.startedUtc !== "string" ||
    !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,7})?Z$/u.test(
      value.startedUtc,
    ) ||
    !Number.isFinite(Date.parse(value.startedUtc))
  ) {
    invalid();
  }
  return `${value.pid}:${value.startedUtc}`;
};
const identities = (values) => {
  if (!Array.isArray(values) || values.length > 10_000) invalid();
  const keys = values.map(identity);
  if (new Set(keys).size !== keys.length) invalid();
  return new Set(keys);
};

// Every observed process is subject to the same limit. Names never determine
// eligibility. Exclusions require a recorded PID AND its start identity.
// This evaluates observations; boundary snapshots cannot certify processes
// that terminated or were inaccessible to the collector.
export function assessResourceNoise(receipt) {
  if (!record(receipt) || receipt.schemaVersion !== 1) invalid();
  const loads = receipt.initialProcessorLoadPercent;
  if (
    !Array.isArray(loads) ||
    loads.length === 0 ||
    loads.length > 256 ||
    !loads.every((load) => Number.isFinite(load) && load >= 0 && load <= 100)
  ) {
    invalid();
  }
  const instrumentation = identities(receipt.instrumentationIdentities);
  const measured = identities(receipt.measuredIdentities);
  if ([...instrumentation].some((key) => measured.has(key))) invalid();
  if (!Array.isArray(receipt.cpuDeltas) || receipt.cpuDeltas.length > 10_000) {
    invalid();
  }
  const seen = new Set();
  const violations = [];
  const excluded = [];
  for (const process of receipt.cpuDeltas) {
    const key = identity(process);
    if (
      seen.has(key) ||
      typeof process.name !== "string" ||
      process.name.length === 0 ||
      process.name.length > 256 ||
      !Number.isFinite(process.cpuSeconds) ||
      process.cpuSeconds < 0
    ) {
      invalid();
    }
    seen.add(key);
    const observation = {
      pid: process.pid,
      startedUtc: process.startedUtc,
      name: process.name,
      cpuSeconds: process.cpuSeconds,
    };
    if (instrumentation.has(key) || measured.has(key)) {
      excluded.push({
        ...observation,
        role: instrumentation.has(key) ? "instrumentation" : "measured",
      });
    } else if (
      process.cpuSeconds > RESOURCE_NOISE_LIMITS.unrelatedProcessCpuSeconds
    ) {
      violations.push(observation);
    }
  }
  const initialPreconditionPassed = loads.every(
    (load) => load <= RESOURCE_NOISE_LIMITS.initialProcessorLoadPercent,
  );
  const noiseGuardPassed = initialPreconditionPassed && violations.length === 0;
  return {
    schemaVersion: 1,
    limits: RESOURCE_NOISE_LIMITS,
    initialPreconditionPassed,
    noiseGuardPassed,
    classification: noiseGuardPassed
      ? "guarded observation; budget approval pending"
      : "characterization; quiet-host guard failed",
    violations,
    excluded,
    limitations: [
      "The collector must retain source observations and exact exclusion identities.",
      "Boundary snapshots omit terminated or inaccessible processes.",
      "All observed non-owned work is counted conservatively, including system/service processes.",
      "A guard pass does not approve budgets or establish a timing/memory target pass.",
    ],
  };
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  try {
    if (process.argv.length !== 4 || process.argv[2] !== "--input") invalid();
    const file = process.argv[3];
    if (statSync(file).size > 16 * 1024 * 1024) invalid();
    const receipt = JSON.parse(
      readFileSync(file, "utf8").replace(/^\uFEFF/u, ""),
    );
    process.stdout.write(`${JSON.stringify(assessResourceNoise(receipt))}\n`);
  } catch {
    process.stderr.write("Invalid resource noise receipt or arguments.\n");
    process.exitCode = 1;
  }
}

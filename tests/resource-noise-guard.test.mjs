import assert from "node:assert/strict";
import test from "node:test";
import { assessResourceNoise } from "../scripts/assess-resource-noise.mjs";

const startedUtc = "2026-10-11T00:00:00.0000000Z";
const process = (pid, name, cpuSeconds, overrides = {}) => ({
  pid,
  name,
  cpuSeconds,
  startedUtc,
  ...overrides,
});
const receipt = (cpuDeltas = [], overrides = {}) => ({
  schemaVersion: 1,
  initialProcessorLoadPercent: [5],
  instrumentationIdentities: [],
  measuredIdentities: [],
  cpuDeltas,
  ...overrides,
});

test("FL Studio and unfamiliar applications cannot escape the CPU guard", () => {
  const input = receipt([
    process(1, "FL64", 31.484375),
    process(2, "Unfamiliar audio application", 0.515625),
    process(3, "audiodg", 1),
  ]);
  const before = structuredClone(input);
  const result = assessResourceNoise(input);
  assert.equal(result.noiseGuardPassed, false);
  assert.deepEqual(
    result.violations.map((p) => p.pid),
    [1, 2, 3],
  );
  assert.deepEqual(input, before, "assessment must preserve source evidence");
});

test("CPU and initial-load boundaries keep the predeclared limits", () => {
  assert.equal(
    assessResourceNoise(
      receipt([process(1, "FL64", 0.5)], {
        initialProcessorLoadPercent: [20],
      }),
    ).noiseGuardPassed,
    true,
  );
  assert.equal(
    assessResourceNoise(receipt([process(1, "FL64", 0.500001)]))
      .noiseGuardPassed,
    false,
  );
  const busy = assessResourceNoise(
    receipt([], {
      initialProcessorLoadPercent: [5, 20.01],
    }),
  );
  assert.equal(busy.noiseGuardPassed, false);
  assert.equal(busy.initialPreconditionPassed, false);
});

test("exclusions match both PID and start identity, rather than process name", () => {
  const result = assessResourceNoise(
    receipt(
      [
        process(1, "node", 2),
        process(2, "fruitboard-desktop", 4),
        process(3, "fruitboard-desktop", 1),
        process(1, "FL64", 1, { startedUtc: "2026-10-11T01:00:00Z" }),
      ],
      {
        instrumentationIdentities: [{ pid: 1, startedUtc }],
        measuredIdentities: [{ pid: 2, startedUtc }],
      },
    ),
  );
  assert.deepEqual(
    result.excluded.map((p) => p.role),
    ["instrumentation", "measured"],
  );
  assert.deepEqual(
    result.violations.map((p) => p.name),
    ["fruitboard-desktop", "FL64"],
  );
});

test("untrusted legacy flags alone never exempt an observed process", () => {
  const result = assessResourceNoise(
    receipt([
      process(1, "FL64", 1, { Instrumentation: true, ReviewOwned: true }),
    ]),
  );
  assert.equal(result.noiseGuardPassed, false);
  assert.equal(result.excluded.length, 0);
});

test("malformed observations and ambiguous ownership fail closed", () => {
  const invalid = [
    receipt([], { initialProcessorLoadPercent: [] }),
    receipt([], { initialProcessorLoadPercent: [NaN] }),
    receipt([], { initialProcessorLoadPercent: [101] }),
    receipt([process(1, "FL64", -1)]),
    receipt([process(1, "FL64", Infinity)]),
    receipt([process(1, "FL64", "1")]),
    receipt([process(1, "FL64", 1, { startedUtc: "unknown" })]),
    receipt([process(1, "FL64", 1), process(1, "FL64", 1)]),
    receipt([], { instrumentationIdentities: [{ pid: 1 }] }),
    receipt([], {
      instrumentationIdentities: [{ pid: 1, startedUtc }],
      measuredIdentities: [{ pid: 1, startedUtc }],
    }),
  ];
  for (const value of invalid) assert.throws(() => assessResourceNoise(value));
});

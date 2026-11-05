import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";
import {
  compareResearchFields,
  readResearchReply,
} from "../scripts/lib/cross-build-research.mjs";

const expected = {
  savedVersion: { status: "extracted", value: "26.1.0.5530" },
  channelNames: {
    status: "inferred",
    value: ["Sampler"],
    method: "sampler-label-for-known-build",
    confidence: "high",
  },
  sampleReferences: {
    status: "unavailable",
    reason: "SAMPLE_REFERENCE_NOT_STORED",
  },
};
const classify = (actual) =>
  compareResearchFields(expected, actual).map((f) => f.classification);

test("GUI value agreement preserves inferred and missing field meanings", () => {
  assert.deepEqual(classify(structuredClone(expected)), [
    "matched_extracted",
    "matched_inferred",
    "matched_unavailable",
  ]);
  const fakeExtraction = structuredClone(expected);
  fakeExtraction.channelNames.status = "extracted";
  assert.equal(classify(fakeExtraction)[1], "mismatched");
  const wrongInference = structuredClone(expected);
  wrongInference.channelNames.method = "guessed-default";
  assert.equal(classify(wrongInference)[1], "mismatched");
  const inventedZero = structuredClone(expected);
  inventedZero.sampleReferences.value = [];
  assert.equal(classify(inventedZero)[2], "mismatched");
});

test("successful parse cannot hide a wrong value, array order or missing field", () => {
  const wrong = structuredClone(expected);
  wrong.savedVersion.value = "26.1.0.9999";
  wrong.channelNames.value = ["Sampler", "Other"];
  delete wrong.sampleReferences;
  assert.deepEqual(classify(wrong), ["mismatched", "mismatched", "failed"]);
  const channels = { channelNames: { status: "extracted", value: ["A", "B"] } };
  assert.equal(
    compareResearchFields(channels, {
      channelNames: { status: "extracted", value: ["B", "A"] },
    })[0].classification,
    "mismatched",
  );
});

test("version policy refusal is distinct from unsupported field layout", () => {
  const oracle = { baseTempoBpm: { status: "extracted", value: 130 } };
  for (const [reason, wanted] of [
    ["UNSUPPORTED_SAVED_VERSION", "policy_blocked"],
    ["UNVERIFIED_LAYOUT", "unsupported"],
  ]) {
    assert.equal(
      compareResearchFields(oracle, {
        baseTempoBpm: { status: "unsupported", reason },
      })[0].classification,
      wanted,
    );
  }
});

test("failed and unexpected states never count as a matching oracle", () => {
  for (const actual of [
    null,
    {},
    { status: "complete" },
    { status: "failed" },
  ]) {
    assert.equal(
      compareResearchFields(
        { field: { status: "extracted", value: 1 } },
        {
          field: actual,
        },
      )[0].classification,
      "failed",
    );
  }
});

test("process-supplied values and reasons cannot leak into the report", () => {
  const secret = "private process text";
  const report = compareResearchFields(expected, {
    savedVersion: { status: "extracted", value: secret },
    channelNames: { status: "unsupported", reason: secret },
    sampleReferences: { status: "failed", reason: secret },
  });
  assert.equal(JSON.stringify(report).includes(secret), false);
});

test("reply requires the matching protocol, schema, request and result", () => {
  const valid = {
    protocolVersion: 1,
    schemaVersion: 2,
    id: "request",
    result: { outcome: "complete" },
  };
  assert.deepEqual(readResearchReply(JSON.stringify(valid), "request"), {
    outcome: "complete",
  });
  for (const change of [
    { protocolVersion: 2 },
    { schemaVersion: 1 },
    { id: "other" },
    { error: { code: "failure" } },
    { result: null },
    { result: [] },
    { result: { outcome: "invented" } },
  ])
    assert.throws(() =>
      readResearchReply(JSON.stringify({ ...valid, ...change }), "request"),
    );
  for (const text of ["", "null", "{}\n{}", "not json"])
    assert.throws(() => readResearchReply(text, "request"));
});

test("CLI rejects arbitrary input paths and an unpinned executable privately", () => {
  const script = new URL(
    "../scripts/research-cross-build-baseline.mjs",
    import.meta.url,
  );
  for (const args of [
    ["--input", "private-project.flp"],
    ["--parser", process.execPath, "--sha256", "0".repeat(64)],
  ]) {
    const child = spawnSync(
      process.execPath,
      [fileURLToPath(script), ...args],
      {
        encoding: "utf8",
        timeout: 10000,
      },
    );
    assert.equal(child.status, 1);
    assert.equal(child.stdout, "");
    assert.equal(
      child.stderr.trim(),
      "Cross-build baseline failed; no qualifying report produced.",
    );
  }
});

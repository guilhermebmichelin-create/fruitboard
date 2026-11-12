import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { compareFixedProbe } from "../scripts/lib/cross-build-probe.mjs";

const specification = JSON.parse(
  readFileSync(
    new URL("../research/cross-build-controls.json", import.meta.url),
  ),
);
const script = fileURLToPath(
  new URL("../scripts/research-cross-build-probe.mjs", import.meta.url),
);
const pinnedHash = createHash("sha256")
  .update(readFileSync(script))
  .digest("hex");

// Constructed observations test report behavior; they are not save evidence.
function reply() {
  return {
    researchOnly: true,
    qualified: false,
    sourceHashesUnchanged: true,
    controls: specification.controls.map((control) => ({
      id: control.id,
      sha256: control.sha256,
      observations: [1, 3, 4].map((width) => ({
        researchOnly: true,
        qualified: false,
        profile: {
          exactSavedBuild: control.fields.savedVersion.value,
          event172Bytes: width,
        },
        decoderOutcome: "complete",
        decoderCode: null,
        fieldObservations: structuredClone(control.fields),
        observedEvents: 1,
        framingPrefix: [
          { id: 199, eventOffset: 22, payloadOffset: 24, payloadBytes: 12 },
        ],
        framingPrefixTruncated: false,
      })),
    })),
  };
}
const compare = (value) =>
  compareFixedProbe(JSON.stringify(value), specification);

test("all fields and explicit hypotheses are compared without choosing a width", () => {
  const report = compare(reply());
  assert.equal(report.qualified, false);
  assert.equal(report.compatibilityExpanded, false);
  assert.equal(report.controls.length, 4);
  for (const control of report.controls) {
    assert.equal(control.selectedWidth, null);
    assert.equal(control.hypotheses.length, 3);
    assert.ok(
      control.hypotheses.every((h) => h.fields.length === 5 && h.fieldsMatch),
    );
    assert.equal(control.matchingPairs.length, 3);
    assert.ok(
      control.matchingPairs.every(
        (p) => p.framingEvidence === "indistinguishable_in_captured_evidence",
      ),
    );
  }
  assert.ok(
    report.additionalBuilds.every(
      (b) => b.state === "not_tested" && b.exactBuild === null,
    ),
  );
  assert.equal(
    report.controls[1].hypotheses[0].fields[3].classification,
    "matched_inferred",
  );
  assert.equal(
    report.controls[1].hypotheses[0].fields[4].classification,
    "matched_unavailable",
  );
});

test("identical pilot values can expose distinct event counts or captured boundaries", () => {
  const value = reply();
  const observations = value.controls[2].observations;
  observations[0].framingPrefix[0].payloadBytes = 10;
  observations[2].observedEvents = 0;
  observations[2].framingPrefix = [];
  const result = compare(value).controls[2];
  assert.ok(result.hypotheses.every((h) => h.fieldsMatch));
  assert.ok(
    result.matchingPairs.every((p) => p.framingEvidence === "different"),
  );
  assert.equal(result.selectedWidth, null);
});

test("a complete decoder outcome cannot hide an incorrect value or provenance", () => {
  for (const change of [
    (fields) => {
      fields.baseTempoBpm.value += 1;
    },
    (fields) => {
      fields.channelNames.status = "extracted";
    },
    (fields) => {
      fields.sampleReferences.value = [];
    },
    (fields) => {
      delete fields.channelCount;
    },
  ]) {
    const value = reply();
    change(value.controls[1].observations[0].fieldObservations);
    const control = compare(value).controls[1];
    assert.equal(control.hypotheses[0].fieldsMatch, false);
    assert.deepEqual(
      control.matchingPairs.map((p) => p.event172Bytes),
      [[3, 4]],
    );
  }
});

test("failures and policy/layout refusals remain observations, not framing conclusions", () => {
  const value = reply();
  const observation = value.controls[0].observations[2];
  observation.decoderOutcome = "failed";
  observation.fieldObservations.savedVersion = {
    status: "failed",
    reason: "private text",
  };
  observation.fieldObservations.baseTempoBpm = {
    status: "unsupported",
    reason: "UNSUPPORTED_SAVED_VERSION",
  };
  observation.fieldObservations.channelCount = {
    status: "unsupported",
    reason: "UNKNOWN_LAYOUT",
  };
  const hypothesis = compare(value).controls[0].hypotheses[2];
  assert.equal(hypothesis.fieldsMatch, false);
  assert.deepEqual(
    hypothesis.fields.slice(0, 3).map((f) => f.classification),
    ["failed", "policy_blocked", "unsupported"],
  );
});

test("process text, extra properties, reasons and arbitrary values stay private", () => {
  const value = reply();
  const secret = "PRIVATE_PROCESS_TEXT";
  value.secret = secret;
  const observation = value.controls[0].observations[0];
  observation.decoderCode = secret;
  observation.profile.secret = secret;
  observation.framingPrefix[0].payload = secret;
  observation.fieldObservations.savedVersion.value = secret;
  observation.fieldObservations.channelNames = {
    status: "unsupported",
    reason: secret,
  };
  assert.equal(JSON.stringify(compare(value)).includes(secret), false);
});

test("partial or malformed probe identity and hypothesis sets fail closed", () => {
  for (const change of [
    (v) => {
      v.researchOnly = false;
    },
    (v) => {
      v.qualified = true;
    },
    (v) => {
      delete v.sourceHashesUnchanged;
    },
    (v) => {
      v.controls.pop();
    },
    (v) => {
      v.controls.push(v.controls[0]);
    },
    (v) => {
      v.controls[1] = v.controls[0];
    },
    (v) => {
      v.controls[0].sha256 = "0".repeat(64);
    },
    (v) => {
      v.controls[0].observations.pop();
    },
    (v) => {
      v.controls[0].observations[0].profile.event172Bytes = 3;
    },
    (v) => {
      v.controls[0].observations[0].profile.exactSavedBuild = "26.1.1.9999";
    },
    (v) => {
      v.controls[0].observations[0].researchOnly = false;
    },
    (v) => {
      v.controls[0].observations[0].qualified = true;
    },
    (v) => {
      v.controls[0].observations[0].decoderOutcome = "success";
    },
    (v) => {
      v.controls[0].observations[0].fieldObservations = [];
    },
  ]) {
    const value = reply();
    change(value);
    assert.throws(() => compare(value));
  }
  for (const text of ["null", "{}\n{}", "not json", " ".repeat(256 * 1024 + 1)])
    assert.throws(() => compareFixedProbe(text, specification));
});

test("framing evidence must be bounded, contiguous and explicitly truncated", () => {
  for (const change of [
    (o) => {
      o.observedEvents = 100001;
    },
    (o) => {
      o.observedEvents = 0.5;
    },
    (o) => {
      o.observedEvents = 33;
    },
    (o) => {
      o.framingPrefixTruncated = true;
    },
    (o) => {
      o.framingPrefix[0].id = "private text";
    },
    (o) => {
      o.framingPrefix[0].eventOffset = 21;
    },
    (o) => {
      o.framingPrefix[0].payloadOffset = 22;
    },
    (o) => {
      o.framingPrefix[0].payloadBytes = 1000000;
    },
    (o) => {
      o.framingPrefix.push(o.framingPrefix[0]);
      o.observedEvents = 2;
    },
  ]) {
    const value = reply();
    change(value.controls[0].observations[0]);
    assert.throws(() => compare(value));
  }
  const value = reply();
  const observation = value.controls[0].observations[0];
  observation.observedEvents = 33;
  observation.framingPrefixTruncated = true;
  observation.framingPrefix = Array.from({ length: 32 }, (_, i) => ({
    id: 1,
    eventOffset: 22 + i * 2,
    payloadOffset: 23 + i * 2,
    payloadBytes: 1,
  }));
  assert.equal(compare(value).controls[0].hypotheses[0].prefixTruncated, true);
});

function cli(args, childResult, changedIdentity = false) {
  const nodeArgs = [];
  if (childResult) {
    // Mock only the subprocess in a fresh Node process. The CLI still checks
    // the real fixed corpus and pinned script bytes before and after the call.
    const source = `import cp from 'node:child_process'; import {syncBuiltinESMExports} from 'node:module';
      import fs from 'node:fs'; const result = JSON.parse(fs.readFileSync(0, 'utf8'));
      let changed = false; const originalStat = fs.fstatSync;
      fs.fstatSync = (fd) => { const stat = originalStat(fd); if (changed) stat.ino += 1; return stat; };
      cp.spawnSync = (path, args, options) => {
        if (args.length || options.timeout !== 10000 || options.maxBuffer !== 262144 ||
            options.shell !== false || options.windowsHide !== true) throw Error('bounds');
        changed = ${changedIdentity}; return result;
      }; syncBuiltinESMExports();`;
    nodeArgs.push(
      "--import",
      `data:text/javascript;base64,${Buffer.from(source).toString("base64")}`,
    );
  }
  return spawnSync(process.execPath, [...nodeArgs, script, ...args], {
    encoding: "utf8",
    input: childResult ? JSON.stringify(childResult) : undefined,
    timeout: 15000,
    windowsHide: true,
  });
}
const pinnedArgs = ["--probe", script, "--sha256", pinnedHash];

test("supervisor compares bounded process output after real fixed-input checks", () => {
  const child = cli(pinnedArgs, {
    status: 0,
    stderr: "",
    stdout: JSON.stringify(reply()),
  });
  assert.equal(child.status, 0, child.stderr);
  const report = JSON.parse(child.stdout);
  assert.equal(report.probeSha256, pinnedHash);
  assert.equal(report.sourceHashesUnchanged, true);
  assert.equal(report.controls.length, 4);
  assert.equal(report.qualified, false);
});

test("supervisor rejects identity changes even when pinned bytes still match", () => {
  const child = cli(
    pinnedArgs,
    {
      status: 0,
      stderr: "",
      stdout: JSON.stringify(reply()),
    },
    true,
  );
  assert.equal(child.status, 1);
  assert.equal(child.stdout, "");
  assert.equal(
    child.stderr.trim(),
    "Cross-build probe failed; no comparison report produced.",
  );
});

test("CLI failures produce only a fixed diagnostic, never partial results or process text", () => {
  const secret = "PRIVATE_PROCESS_TEXT";
  for (const result of [
    { status: 1, stderr: secret, stdout: JSON.stringify(reply()) },
    {
      status: null,
      error: { code: "ETIMEDOUT", message: secret },
      stderr: "",
      stdout: "",
    },
    { status: null, error: { code: "ENOBUFS" }, stderr: "", stdout: "" },
    { status: 0, stderr: "x".repeat(65537), stdout: "{}" },
    { status: 0, stderr: secret, stdout: "bad json" },
  ]) {
    const child = cli(pinnedArgs, result);
    assert.equal(child.status, 1);
    assert.equal(child.stdout, "");
    assert.equal(
      child.stderr.trim(),
      "Cross-build probe failed; no comparison report produced.",
    );
  }
  for (const args of [
    ["--input", secret],
    ["--probe", script, "--sha256", "0".repeat(64)],
  ]) {
    const child = cli(args);
    assert.equal(child.status, 1);
    assert.equal(child.stdout, "");
    assert.equal(
      child.stderr.trim(),
      "Cross-build probe failed; no comparison report produced.",
    );
  }
});

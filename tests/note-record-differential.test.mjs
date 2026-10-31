import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, rmdirSync, unlinkSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { tmpdir } from "node:os";
import test from "node:test";
import { compareNoteRecordCandidates as compare } from "../scripts/lib/note-record-differential.mjs";
import { inspectNoteRecordCandidates as inspect } from "../scripts/lib/note-record-research.mjs";

const fixture = fileURLToPath(
  new URL("../fixtures/parser-corpus/FIX-FL2026-NOTES.flp", import.meta.url),
);
const bytes = readFileSync(fixture);
const digest = createHash("sha256").update(bytes).digest("hex");
const offset = inspect(bytes).patterns[0].records[0].fileOffset;
const script = fileURLToPath(
  new URL("../scripts/compare-note-records.mjs", import.meta.url),
);

test("identical approved bytes retain all records without a semantic claim", () => {
  const result = compare(bytes, bytes);
  assert.equal(result.recordCount, 9);
  assert.equal(result.musicalFieldsQualified, false);
  assert.equal(result.baselineSha256, digest);
  assert.equal(result.candidateSha256, digest);
  assert.deepEqual(
    result.patterns.map((p) => p.records.length),
    [3, 2, 4],
  );
  for (const record of result.patterns.flatMap((p) => p.records)) {
    assert.deepEqual(record.changedBytes, []);
    assert.deepEqual(record.changedCandidateWords, []);
  }
});

for (const [field, byteOffset] of [
  ["positionWord", 0],
  ["flagsWord", 4],
  ["channelWord", 6],
  ["lengthWord", 8],
  ["keyWord", 12],
  ["byte21", 21],
]) {
  test(`constructed ${field} mutation is numeric evidence only`, () => {
    const candidate = Buffer.from(bytes);
    candidate[offset + byteOffset] ^= 1;
    const record = compare(bytes, candidate).patterns[0].records[0];
    assert.deepEqual(record.changedBytes, [
      {
        offset: byteOffset,
        baseline: bytes[offset + byteOffset],
        candidate: candidate[offset + byteOffset],
      },
    ]);
    assert.equal(record.changedCandidateWords.length, 1);
    assert.equal(record.changedCandidateWords[0].field, field);
    assert.equal(createHash("sha256").update(bytes).digest("hex"), digest);
  });
}

test("uninterpreted record bytes cannot disappear from differential evidence", () => {
  const candidate = Buffer.from(bytes);
  candidate[offset + 23] ^= 1;
  const record = compare(bytes, candidate).patterns[0].records[0];
  assert.equal(record.changedBytes[0].offset, 23);
  assert.deepEqual(record.changedCandidateWords, []);
});

test("all 24 record bytes are compared, including unknown fields", () => {
  const candidate = Buffer.from(bytes);
  for (let i = 0; i < 24; i++) candidate[offset + i] ^= 255;
  const record = compare(bytes, candidate).patterns[0].records[0];
  assert.deepEqual(
    record.changedBytes.map((b) => b.offset),
    Array.from({ length: 24 }, (_, i) => i),
  );
});

test("unchanged note bytes never certify unchanged whole-project content", () => {
  const candidate = Buffer.from(bytes);
  candidate[10] ^= 1; // Header channel count is outside this research comparison.
  const result = compare(bytes, candidate);
  assert.notEqual(result.baselineSha256, result.candidateSha256);
  assert.ok(
    result.patterns.every((p) =>
      p.records.every((r) => !r.changedBytes.length),
    ),
  );
  assert.match(result.scope, /other FLP content is not compared/);
  assert.equal(result.musicalFieldsQualified, false);
});

test("comparison does not infer identity across PPQ or pattern changes", () => {
  const ppq = Buffer.from(bytes);
  ppq.writeUInt16LE(192, 12);
  assert.throws(() => compare(bytes, ppq), /COMPARISON_LAYOUT/);
  const id = Buffer.from(bytes);
  // The two-byte pattern marker precedes the event-224 ID/length.
  id.writeUInt16LE(50, inspect(bytes).patterns[0].eventOffset - 2);
  assert.throws(() => compare(bytes, id), /COMPARISON_LAYOUT/);
});

test("missing payload is not an empty comparable sequence", () => {
  const empty = readFileSync(
    new URL(
      "../fixtures/parser-corpus/FIX-FL2026-NAMED-EMPTY.flp",
      import.meta.url,
    ),
  );
  assert.throws(() => compare(bytes, empty), /COMPARISON_LAYOUT/);
  assert.throws(() => compare(empty, empty), /COMPARISON_NO_RECORDS/);
});

test("malformed inputs retain the observer's refusal", () => {
  assert.throws(() => compare(bytes, bytes.subarray(0, 30)), /HEADER_OR_SIZE/);
  const candidate = Buffer.from(bytes);
  candidate[offset - 1] = 25;
  assert.throws(() => compare(bytes, candidate), /NOTE_LAYOUT/);
});

function constructed(recordCount, patternId = 1) {
  const version = Buffer.concat([
    Buffer.from([199, 12]),
    Buffer.from("26.1.0.5530\0"),
  ]);
  const marker = Buffer.from([65, patternId, 0]);
  const records = Buffer.alloc(24 * recordCount);
  // All records intentionally identical: comparisons must retain duplicates.
  let length = records.length;
  const encoded = [];
  do {
    const part = length & 127;
    length = Math.floor(length / 128);
    encoded.push(part | (length ? 128 : 0));
  } while (length);
  const body = Buffer.concat([
    version,
    marker,
    Buffer.from([224, ...encoded]),
    records,
  ]);
  const header = Buffer.alloc(22);
  header.write("FLhd");
  header.writeUInt32LE(6, 4);
  header.writeUInt16LE(96, 12);
  header.write("FLdt", 14);
  header.writeUInt32LE(body.length, 18);
  return Buffer.concat([header, body]);
}

test("all 32 source ordinals survive, including identical records", () => {
  const baseline = constructed(32);
  const candidate = Buffer.from(baseline);
  const lastOffset = inspect(candidate).patterns[0].records[31].fileOffset;
  candidate[lastOffset + 23] = 255;
  const result = compare(baseline, candidate);
  assert.equal(result.recordCount, 32);
  assert.deepEqual(
    result.patterns[0].records.map((r) => r.ordinal),
    Array.from({ length: 32 }, (_, i) => i),
  );
  assert.deepEqual(result.patterns[0].records[31].changedBytes, [
    { offset: 23, baseline: 0, candidate: 255 },
  ]);
  assert.throws(() => compare(baseline, constructed(33)), /RECORD_LIMIT/);
});

test("record cardinality changes refuse positional pairing", () => {
  assert.throws(
    () => compare(constructed(1), constructed(2)),
    /COMPARISON_LAYOUT/,
  );
});

test("explicit hash-pinned CLI works outside the checkout and never writes inputs", () => {
  const run = spawnSync(
    process.execPath,
    [script, fixture, digest, fixture, digest],
    { cwd: tmpdir(), encoding: "utf8", timeout: 10000 },
  );
  assert.equal(run.status, 0, run.stderr);
  const result = JSON.parse(run.stdout);
  assert.equal(result.recordCount, 9);
  assert.equal(result.musicalFieldsQualified, false);
  assert.doesNotMatch(run.stdout, /FIX-FL2026|[A-Z]:\\/);
  assert.equal(
    createHash("sha256").update(readFileSync(fixture)).digest("hex"),
    digest,
  );
});

test("CLI rejects wrong pins, missing files and invalid arguments privately", () => {
  for (const args of [
    [],
    [fixture, digest],
    [fixture, "f".repeat(64), fixture, digest],
    ["private-song.flp", digest, fixture, digest],
    [fixture, digest, fixture, digest, "private-extra"],
    [fixture, "not-a-digest", fixture, digest],
    [
      fileURLToPath(new URL("../fixtures", import.meta.url)),
      digest,
      fixture,
      digest,
    ],
  ]) {
    const run = spawnSync(process.execPath, [script, ...args], {
      encoding: "utf8",
      timeout: 10000,
    });
    assert.equal(run.status, 1);
    assert.equal(run.stdout, "");
    assert.equal(
      run.stderr.trim(),
      "Note-record comparison failed; no report produced.",
    );
  }
});

test(
  "CLI refuses Linux FIFO inputs without waiting for a writer",
  { skip: process.platform !== "linux" },
  () => {
    const directory = mkdtempSync(join(tmpdir(), "fruitboard-note-fifo-"));
    const fifo = join(directory, "input");
    let created = false;
    try {
      const setup = spawnSync("mkfifo", [fifo], {
        encoding: "utf8",
        timeout: 10000,
      });
      assert.equal(setup.status, 0, setup.stderr);
      created = true;
      for (const args of [
        [fifo, digest, fixture, digest],
        [fixture, digest, fifo, digest],
      ]) {
        const run = spawnSync(process.execPath, [script, ...args], {
          encoding: "utf8",
          timeout: 10000,
        });
        assert.equal(run.status, 1, String(run.error ?? run.stderr));
        assert.equal(run.stdout, "");
        assert.equal(
          run.stderr.trim(),
          "Note-record comparison failed; no report produced.",
        );
      }
    } finally {
      if (created) unlinkSync(fifo);
      rmdirSync(directory);
    }
  },
);

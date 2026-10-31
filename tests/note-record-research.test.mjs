import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { tmpdir } from "node:os";
import test from "node:test";
import { inspectNoteRecordCandidates as inspect } from "../scripts/lib/note-record-research.mjs";

const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
const version = Buffer.concat([
  Buffer.from([199, 12]),
  Buffer.from("26.1.0.5530\0"),
]);
function project(...events) {
  const body = Buffer.concat([version, ...events]);
  const header = Buffer.alloc(22);
  header.write("FLhd");
  header.writeUInt32LE(6, 4);
  header.writeUInt16LE(96, 12);
  header.write("FLdt", 14);
  header.writeUInt32LE(body.length, 18);
  return Buffer.concat([header, body]);
}
function pattern(id = 1) {
  const marker = Buffer.from([65, 0, 0]);
  marker.writeUInt16LE(id, 1);
  return marker;
}
function payload(size = 24) {
  assert.ok(size < 128);
  return Buffer.concat([Buffer.from([224, size]), Buffer.alloc(size)]);
}

test("approved F16 has 3/2/4 records with candidate words and unchanged bytes", () => {
  const bytes = readFileSync(
    new URL("../fixtures/parser-corpus/FIX-FL2026-NOTES.flp", import.meta.url),
  );
  const before = hash(bytes);
  assert.equal(
    before,
    "be58afee914e10ab95f73f55405d8ea72f607430016a4264ecd1d5a488ef041f",
  );
  const result = inspect(bytes);
  assert.deepEqual(
    result.patterns.map((p) => [p.patternId, p.records.length]),
    [
      [1, 3],
      [2, 2],
      [3, 4],
    ],
  );
  assert.deepEqual(
    result.patterns[0].records.map((r) => [
      r.positionWord,
      r.channelWord,
      r.lengthWord,
      r.keyWord,
    ]),
    [
      [0, 0, 48, 60],
      [96, 1, 48, 64],
      [192, 0, 48, 62],
    ],
  );
  assert.equal(result.patterns[1].records[1].flagsWord, 0x4008);
  assert.deepEqual(
    result.patterns[2].records.map((r) => r.lengthWord),
    [0, 0, 0, 0],
  );
  assert.equal(hash(bytes), before);
});
test("F17 missing payload is an observation, not an extracted zero MIDI sequence", () => {
  const bytes = readFileSync(
    new URL(
      "../fixtures/parser-corpus/FIX-FL2026-NAMED-EMPTY.flp",
      import.meta.url,
    ),
  );
  const result = inspect(bytes);
  assert.deepEqual(result.patterns, []);
  assert.match(result.interpretation, /Unqualified/);
  assert.equal("midiNotes" in result, false);
});
test("truncated input fails", () =>
  assert.throws(() => inspect(project().subarray(0, 25)), /HEADER_OR_SIZE/));
test("chunk signatures require exact bytes, including their high bits", () => {
  const valid = project(pattern(), payload());
  assert.equal(inspect(valid).recordCount, 1);
  for (const offset of [0, 1, 2, 3, 14, 15, 16, 17]) {
    for (const changedBits of [1, 128]) {
      const corrupted = Buffer.from(valid);
      corrupted[offset] ^= changedBits;
      assert.throws(
        () => inspect(corrupted),
        /RESEARCH_HEADER_OR_SIZE/,
        `corrupted signature byte ${offset} must be refused`,
      );
    }
  }
});

test("absent version fails", () => {
  const bytes = project().subarray(0, 22);
  bytes.writeUInt32LE(0, 18);
  assert.throws(() => inspect(bytes), /VERSION_MISSING/);
});
test("duplicate version fails", () =>
  assert.throws(() => inspect(project(version)), /DUPLICATE_VERSION/));
test("other saved build fails", () => {
  const bytes = project();
  bytes[26] = 53;
  assert.throws(() => inspect(bytes), /UNVERIFIED_BUILD/);
});
test("unbound or reopened content section fails", () => {
  assert.throws(() => inspect(project(payload())), /NOTE_BINDING/);
  assert.throws(
    () =>
      inspect(
        project(
          pattern(),
          payload(),
          Buffer.from([64, 0, 0]),
          pattern(2),
          payload(),
        ),
      ),
    /NOTE_BINDING/,
  );
});
test("consecutive unresolved pattern IDs fail", () =>
  assert.throws(
    () => inspect(project(pattern(), pattern(2), payload())),
    /NOTE_BINDING/,
  ));
test("duplicate pattern content fails", () =>
  assert.throws(
    () => inspect(project(pattern(), payload(), pattern(), payload())),
    /DUPLICATE_PATTERN/,
  ));
test("unknown preamble cannot gain note authority", () =>
  assert.throws(
    () => inspect(project(Buffer.from([63, 0]), pattern(), payload())),
    /NOTE_BINDING/,
  ));
test("empty and fractional record layouts fail", () => {
  for (const n of [0, 1, 23, 25])
    assert.throws(() => inspect(project(pattern(), payload(n))), /NOTE_LAYOUT/);
});
test("malformed length and payload overrun fail with fixed codes", () => {
  assert.throws(
    () => inspect(project(Buffer.from([224, 128, 128, 128, 128, 128, 128]))),
    /EVENT_LENGTH/,
  );
  assert.throws(
    () => inspect(project(Buffer.from([224, 24, 0]))),
    /EVENT_BOUNDS/,
  );
});
test("research record cap charges all patterns", () => {
  const events = [];
  for (let id = 1; id <= 9; id++) events.push(pattern(id), payload(96));
  assert.throws(() => inspect(project(...events)), /RECORD_LIMIT/);
});
test("CLI runs from another cwd with only approved fixed fixtures", () => {
  const script = new URL(
    "../scripts/research-note-records.mjs",
    import.meta.url,
  );
  const run = spawnSync(process.execPath, [fileURLToPath(script)], {
    cwd: tmpdir(),
    encoding: "utf8",
  });
  assert.equal(run.status, 0, run.stderr);
  const result = JSON.parse(run.stdout);
  assert.equal(result.researchOnly, true);
  assert.deepEqual(
    result.results.map((r) => r.recordCount),
    [9, 0],
  );
});
test("CLI rejects arbitrary input paths without echoing them", () => {
  const script = new URL(
    "../scripts/research-note-records.mjs",
    import.meta.url,
  );
  const run = spawnSync(
    process.execPath,
    [fileURLToPath(script), "private-file.flp"],
    { encoding: "utf8" },
  );
  assert.equal(run.status, 1);
  assert.equal(run.stdout, "");
  assert.doesNotMatch(run.stderr, /private-file/);
});

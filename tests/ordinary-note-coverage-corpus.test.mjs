import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import test from "node:test";
import { inspectNoteRecordCandidates } from "../scripts/lib/note-record-research.mjs";
import { compareNoteRecordCandidates } from "../scripts/lib/note-record-differential.mjs";
import {
  inspectApprovedFixtureBinary,
  inspectTrackedPath,
} from "../scripts/lib/repository-privacy.mjs";

const registeredBytes = readFileSync(
  new URL(
    "../fixtures/parser-corpus/ordinary-note-coverage-expectations.json",
    import.meta.url,
  ),
);
const registered = JSON.parse(registeredBytes.toString("utf8"));
const read = (entry) =>
  readFileSync(
    new URL(`../fixtures/parser-corpus/${entry.file}`, import.meta.url),
  );
const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
const baseline = read({ file: "FIX-FL2026-NOTE-BASELINE.flp" });
const entry = (id) => registered.cases.find((item) => item.case === id);

// Framing for these exact hash-pinned saves only, including build26.1's
// three-byte event172. Never accept an arbitrary project or interpret byte19.
function framedEvents(bytes) {
  assert.equal(bytes.subarray(24, 36).toString("ascii"), "26.1.0.5530\0");
  const events = [];
  let cursor = 22;
  while (cursor < bytes.length) {
    assert.ok(events.length < 100000);
    const offset = cursor;
    const id = bytes[cursor++];
    let length;
    if (id < 64) length = 1;
    else if (id < 128) length = 2;
    else if (id < 192) length = id === 172 ? 3 : 4;
    else {
      length = 0;
      for (let shift = 0; ; shift += 7) {
        assert.ok(shift <= 28 && cursor < bytes.length);
        const part = bytes[cursor++];
        length += (part & 127) * 2 ** shift;
        if (part < 128) break;
      }
    }
    assert.ok(length <= bytes.length - cursor);
    events.push({
      id,
      offset,
      payload: bytes.subarray(cursor, cursor + length),
    });
    cursor += length;
  }
  return events;
}

test("F31-F38 retain their approved thirteen complete records read-only", () => {
  assert.equal(registered.savedBuild, "26.1.0.5530");
  assert.deepEqual(
    registered.cases.map((item) => item.slot),
    ["F31", "F32", "F33", "F34", "F35", "F36", "F37", "F38"],
  );
  assert.equal(
    registered.cases.reduce((count, item) => count + item.savedRecordCount, 0),
    13,
  );
  for (const item of registered.cases) {
    const bytes = read(item);
    assert.equal(bytes.length, item.bytes);
    assert.equal(hash(bytes), item.sha256);
    const observed = inspectNoteRecordCandidates(bytes);
    assert.equal(observed.ppqHeader, 96);
    assert.equal(observed.savedBuild, registered.savedBuild);
    assert.equal(observed.recordCount, item.savedRecordCount);
    assert.equal(observed.patterns.length, 1);
    assert.equal(observed.patterns[0].patternId, 1);
    assert.deepEqual(
      observed.patterns[0].records.map((record) => record.recordBytes),
      item.records.map((record) => record.recordBytes),
    );
    for (const [index, record] of observed.patterns[0].records.entries()) {
      const expected = item.records[index];
      assert.deepEqual(
        [
          record.ordinal,
          record.positionWord,
          record.channelWord,
          record.lengthWord,
          record.keyWord,
          record.byte21,
          record.flagsWord,
        ],
        [
          expected.ordinal,
          expected.position,
          expected.savedChannelId,
          expected.storedLength,
          expected.savedKey,
          expected.storedVelocityByte,
          expected.flags,
        ],
      );
    }
    // Exact declared offsets only; this is not a general channel decoder.
    for (const declaration of item.channelDeclarations) {
      assert.equal(bytes[declaration.eventOffset], 64);
      assert.equal(
        bytes.readUInt16LE(declaration.eventOffset + 1),
        declaration.savedId,
      );
      if (declaration.storedName !== null) {
        const events = framedEvents(bytes);
        const start = events.findIndex(
          (event) => event.offset === declaration.eventOffset,
        );
        const next = events.findIndex(
          (event, index) =>
            index > start && [64, 65, 100, 233, 236].includes(event.id),
        );
        const names = events
          .slice(start + 1, next < 0 ? events.length : next)
          .filter((event) => event.id === 203);
        assert.equal(names.length, 1);
        assert.equal(
          names[0].payload.toString("utf16le").replace(/\0+$/, ""),
          declaration.storedName,
        );
      }
    }
    assert.equal(hash(read(item)), item.sha256);
  }
});

test("only exact approved paths and bytes bypass the fixture privacy block", () => {
  // Pin independently of the per-fixture allowlist and expectation values.
  assert.equal(
    hash(registeredBytes),
    "9e9942a6b4864b0ace136e7d9a0258ae8681d5d36aa57fc2f3c335e17ffca5b9",
  );
  for (const item of registered.cases) {
    const path = `fixtures/parser-corpus/${item.file}`;
    const bytes = read(item);
    assert.deepEqual(inspectTrackedPath(path, hash(bytes)), []);
    assert.deepEqual(inspectApprovedFixtureBinary(path, bytes), []);
    assert.ok(
      framedEvents(bytes).every((event) => event.id !== 200),
      item.case,
    );
    for (const signature of [
      Buffer.from("RIFF"),
      Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
      Buffer.from([255, 216, 255]),
      Buffer.from("GIF87a"),
      Buffer.from("GIF89a"),
    ]) {
      assert.equal(bytes.includes(signature), false, item.case);
    }
    const texts = [
      bytes.toString("latin1"),
      ...[0, 1].map((offset) =>
        bytes
          .subarray(
            offset,
            offset + Math.floor((bytes.length - offset) / 2) * 2,
          )
          .toString("utf16le"),
      ),
    ];
    assert.ok(
      texts.every(
        (text) => !/[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}/i.test(text),
      ),
      item.case,
    );
    const changed = Buffer.from(bytes);
    changed[changed.length - 1] ^= 1;
    assert.equal(
      inspectTrackedPath(path, hash(changed))[0].rule,
      "fixture-hash-mismatch",
    );
    assert.equal(
      inspectTrackedPath(`other/${item.file}`, hash(bytes))[0].rule,
      "blocked-extension",
    );
  }
});

test("registered single-note controls retain both clean changes and the failed pitch control", () => {
  for (const id of ["N09", "N10", "N11", "N12", "N16"]) {
    const item = entry(id);
    const comparison = compareNoteRecordCandidates(baseline, read(item));
    assert.equal(comparison.musicalFieldsQualified, false);
    assert.deepEqual(
      comparison.patterns[0].records[0].changedBytes.map(
        (change) => change.offset,
      ),
      item.baselineChangedOffsets[0],
    );
  }
  assert.deepEqual(entry("N09").baselineChangedOffsets, [[12, 21]]);
  assert.equal(entry("N09").qualification, "negative control");
  assert.equal(entry("N09").independentGui.notes[0].velocityPercent, 70);
  assert.equal(entry("N09").records[0].storedVelocityByte, 90);
  assert.equal(
    inspectNoteRecordCandidates(baseline).patterns[0].records[0].byte21,
    89,
  );
  assert.deepEqual(entry("N10").baselineChangedOffsets, [[0]]);
  assert.deepEqual(entry("N11").baselineChangedOffsets, [[8]]);
  assert.deepEqual(entry("N12").baselineChangedOffsets, [[21]]);
  assert.equal(entry("N12").records[0].storedVelocityByte, 128);
});

test("melody and chord preserve cardinality and saved source order without invented alignment", () => {
  assert.deepEqual(
    entry("N13").records.map((note) => [
      note.position,
      note.storedLength,
      note.savedKey,
    ]),
    [
      [0, 48, 60],
      [48, 48, 62],
      [96, 96, 64],
    ],
  );
  assert.deepEqual(
    entry("N14").records.map((note) => [
      note.position,
      note.storedLength,
      note.savedKey,
    ]),
    [
      [0, 48, 67],
      [0, 48, 64],
      [0, 48, 60],
    ],
  );
  for (const id of ["N13", "N14", "N15"]) {
    assert.throws(
      () => compareNoteRecordCandidates(baseline, read(entry(id))),
      /RESEARCH_COMPARISON_LAYOUT/,
    );
  }
});

test("the duplicate attempt keeps two records and its unexplained byte difference", () => {
  const item = entry("N15");
  assert.equal(item.qualification, "negative control");
  assert.equal(item.independentGui.retainedNoteCount, 2);
  const [first, second] = inspectNoteRecordCandidates(read(item)).patterns[0]
    .records;
  assert.deepEqual(
    first.recordBytes.flatMap((value, offset) =>
      value === second.recordBytes[offset]
        ? []
        : [{ offset, first: value, second: second.recordBytes[offset] }],
    ),
    [{ offset: 19, first: 64, second: 0 }],
  );
  assert.deepEqual(item.baselineChangedOffsets, [[19], []]);
  assert.equal(item.currentParser.patternStatus, "unsupported");
});

test("deleting the empty preceding channel preserves NoteTwo ownership after renumbering", () => {
  const item = entry("N16");
  assert.deepEqual(item.independentGui.beforeDeletionOrder, [
    "DeleteMe",
    "NoteTwo",
    "Reserve",
  ]);
  assert.deepEqual(
    item.channelDeclarations.map((channel) => [
      channel.savedId,
      channel.storedName,
    ]),
    [
      [0, "NoteTwo"],
      [1, "Reserve"],
    ],
  );
  assert.equal(item.records[0].savedChannelId, 0);
  assert.equal(item.independentGui.otherChannelEmpty, true);
  assert.equal(item.currentParser.patternStatus, "extracted");
});

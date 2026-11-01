import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import test from "node:test";
import { inspectNoteRecordCandidates } from "../scripts/lib/note-record-research.mjs";
import { compareNoteRecordCandidates } from "../scripts/lib/note-record-differential.mjs";

const registered = JSON.parse(
  readFileSync(
    new URL(
      "../fixtures/parser-corpus/note-sequence-expectations.json",
      import.meta.url,
    ),
    "utf8",
  ),
);
const read = (entry) =>
  readFileSync(
    new URL(`../fixtures/parser-corpus/${entry.file}`, import.meta.url),
  );
const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");

test("approved genuine saves preserve the registered ordinary-note observations", () => {
  for (const entry of registered.cases) {
    const bytes = read(entry);
    assert.equal(bytes.length, entry.bytes);
    assert.equal(hash(bytes), entry.sha256);
    const observed = inspectNoteRecordCandidates(bytes);
    assert.equal(observed.savedBuild, registered.savedBuild);
    assert.equal(observed.ppqHeader, registered.ppqHeader);
    assert.equal(observed.recordCount, entry.savedRecordCount);
    assert.equal(observed.patterns.length, 1);
    assert.equal(observed.patterns[0].patternId, entry.patternId);
    const record = observed.patterns[0].records[0];
    assert.deepEqual(record.recordBytes, entry.recordBytes, entry.case);
    const expected = entry.expectedNote;
    assert.deepEqual(
      [
        record.positionWord,
        record.flagsWord,
        record.channelWord,
        record.lengthWord,
        record.keyWord,
        record.byte21,
      ],
      [
        expected.position,
        expected.flags,
        expected.savedChannelId,
        expected.storedLength,
        expected.savedKey,
        expected.storedVelocityByte,
      ],
      entry.case,
    );
    // These are pinned declaration offsets, not a new general channel decoder.
    for (const channel of entry.channelDeclarations) {
      assert.equal(bytes[channel.eventOffset], 64);
      assert.equal(
        bytes.readUInt16LE(channel.eventOffset + 1),
        channel.savedId,
      );
    }
    assert.equal(hash(read(entry)), entry.sha256);
  }
});

test("each controlled genuine-save comparison changes only its registered note bytes", () => {
  const baseline = read(registered.cases[0]);
  for (const entry of registered.cases.slice(1)) {
    const result = compareNoteRecordCandidates(baseline, read(entry));
    assert.deepEqual(
      result.patterns[0].records[0].changedBytes,
      entry.noteByteChangesFromBaseline,
      entry.case,
    );
    assert.equal(result.recordCount, 1);
    // Independent GUI evidence supplies semantics; this byte tool stays numeric.
    assert.equal(result.musicalFieldsQualified, false);
  }
});

test("the channel reorder renumbers the saved reference while NoteTwo retains its note", () => {
  const [second, reordered] = registered.cases.slice(5);
  const result = compareNoteRecordCandidates(read(second), read(reordered));
  assert.deepEqual(result.patterns[0].records[0].changedBytes, [
    { offset: 6, baseline: 1, candidate: 0 },
  ]);
  for (const entry of [second, reordered]) {
    const owners = entry.channelDeclarations.filter(
      ({ savedId }) => savedId === entry.expectedNote.savedChannelId,
    );
    assert.equal(owners.length, 1);
    assert.equal(owners[0].storedName, "NoteTwo");
    assert.equal(entry.independentGui.channel, "NoteTwo");
    assert.equal(entry.independentGui.otherChannelEmpty, true);
  }
  assert.deepEqual(second.independentGui.channelOrder, ["NoteOne", "NoteTwo"]);
  assert.deepEqual(reordered.independentGui.channelOrder, [
    "NoteTwo",
    "NoteOne",
  ]);
});

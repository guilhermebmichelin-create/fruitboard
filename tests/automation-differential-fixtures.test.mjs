import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import test from "node:test";
import {
  inspectApprovedFixtureBinary,
  inspectTrackedPath,
} from "../scripts/lib/repository-privacy.mjs";

const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
const read = (file) =>
  readFileSync(new URL(`../fixtures/parser-corpus/${file}`, import.meta.url));
const registered = read("automation-differential-expectations.json");
assert.equal(registered.length, 5977);
assert.equal(
  hash(registered),
  "a09f62bbe3a432dcec27649c4f469ce073f253734a7995b5e33ce1f68ed81162",
);
const expected = JSON.parse(registered);

test("automation differentials admit only approved bytes and paths; preparation status is preserved", () => {
  assert.equal(
    expected.status,
    "Prepared for exact-byte approval; not approved for public fixture inclusion",
  );
  // The manifest records the owner's later 2026-11-14 approval. Never rewrite
  // the exact approved expectations merely to change preparation-time wording.
  assert.deepEqual(
    expected.cases.map((c) => c.proposedSlot),
    ["F39", "F40", "F41", "F42"],
  );
  for (const entry of expected.cases) {
    const path = `fixtures/parser-corpus/${entry.proposedFile}`;
    const bytes = read(entry.proposedFile);
    assert.equal(bytes.length, entry.bytes);
    assert.equal(hash(bytes), entry.sha256);
    assert.deepEqual(inspectTrackedPath(path, entry.sha256), []);
    assert.deepEqual(inspectApprovedFixtureBinary(path, bytes), []);
    const changed = Buffer.from(bytes);
    changed[changed.length - 1] ^= 1;
    assert.equal(
      inspectTrackedPath(path, hash(changed))[0].rule,
      "fixture-hash-mismatch",
    );
    assert.equal(inspectTrackedPath(path)[0].rule, "fixture-hash-mismatch");
    assert.equal(
      inspectTrackedPath(`other/${entry.proposedFile}`, entry.sha256)[0].rule,
      "blocked-extension",
    );
    for (const encoding of ["latin1", "utf16le"]) {
      for (const prefix of [Buffer.alloc(0), Buffer.from([0])]) {
        assert.ok(
          inspectApprovedFixtureBinary(
            path,
            Buffer.concat([
              prefix,
              Buffer.from("C:\\Users\\person\\Documents\\", encoding),
            ]),
          ).some(({ rule }) => rule === "fixture-personal-home-path"),
        );
      }
    }
  }
});

// Independent fixed-input research walk, not a product decoder or a new API.
// Hash admission above restricts its use to these approved exact-build bytes.
function framedRecords(bytes) {
  assert.ok(bytes.length >= 22 && bytes.length <= 128 * 1024);
  assert.equal(bytes.toString("ascii", 0, 4), "FLhd");
  assert.equal(bytes.readUInt32LE(4), 6);
  assert.equal(bytes.toString("ascii", 14, 18), "FLdt");
  assert.equal(bytes.readUInt32LE(18), bytes.length - 22);
  const result = [];
  let offset = 22;
  let savedVersion;
  while (offset < bytes.length) {
    assert.ok(result.length < 100000);
    const id = bytes[offset++];
    let length;
    if (id < 64) length = 1;
    else if (id < 128) length = 2;
    else if (id < 192) {
      if (id === 172) assert.equal(savedVersion, "26.1.0.5530");
      length = id === 172 ? 3 : 4;
    } else {
      length = 0;
      let shift = 0;
      while (true) {
        assert.ok(offset < bytes.length && shift <= 28);
        const value = bytes[offset++];
        length += (value & 127) * 2 ** shift;
        if (value < 128) break;
        shift += 7;
      }
    }
    assert.ok(Number.isSafeInteger(length) && length <= bytes.length - offset);
    const payload = bytes.subarray(offset, offset + length);
    if (id === 199) {
      savedVersion = payload.toString("latin1").replace(/\0+$/, "");
      assert.equal(savedVersion, "26.1.0.5530");
    }
    result.push({ id, payload });
    offset += length;
  }
  assert.equal(savedVersion, "26.1.0.5530");
  assert.equal(offset, bytes.length);
  return result;
}

test("GUI point differentials distinguish the observed count from placements without promoting a grammar", () => {
  const baselineFile = "FIX-FL2026-AUTOMATION.flp";
  const baseline = read(baselineFile);
  assert.equal(
    hash(baseline),
    "23b764e4220fc5e48ffbd4a99c602bda092c032ef35b9b2d5b785250b622cf7f",
  );
  const one = (records, id) => {
    const found = records.filter((event) => event.id === id);
    assert.equal(found.length, 1);
    return found[0].payload;
  };
  const baseRecords = framedRecords(baseline);
  const baseCurve = one(baseRecords, 234);
  const basePlaylist = one(baseRecords, 233);
  assert.equal(baseCurve.length, 205);
  assert.equal(baseCurve.readUInt32LE(17), 3);
  for (const entry of expected.cases) {
    const before = read(entry.proposedFile);
    assert.equal(hash(before), entry.sha256);
    const records = framedRecords(before);
    const curve = one(records, 234);
    const playlist = one(records, 233);
    const points = entry.gui.editableControlPoints;
    assert.equal(curve.length, entry.byteObservations.event234Bytes);
    assert.equal(playlist.length, entry.byteObservations.event233Bytes);
    assert.equal(curve.readUInt32LE(13), 3); // Falsified as the changing count.
    assert.equal(curve.readUInt32LE(17), points);
    assert.equal(curve.length, 21 + 24 * points + 112);
    assert.deepEqual(curve.subarray(0, 17), baseCurve.subarray(0, 17));
    assert.deepEqual(
      curve.subarray(21 + 24 * points),
      baseCurve.subarray(21 + 24 * 3),
    );
    assert.equal(playlist.length, 88 * entry.gui.playlistPlacements);
    if (["A03", "A04"].includes(entry.case)) assert.deepEqual(curve, baseCurve);
    if (entry.case === "A01") assert.deepEqual(playlist, basePlaylist);
    if (entry.case === "A02") {
      // Preserve the unexplained placement-record change; no token meaning is inferred.
      assert.deepEqual(
        [...playlist.keys()].filter((i) => playlist[i] !== basePlaylist[i]),
        [32],
      );
    }
    assert.deepEqual(read(entry.proposedFile), before);
  }
  assert.deepEqual(read(baselineFile), baseline);
});

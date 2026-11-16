import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import test from "node:test";
import {
  inspectApprovedFixtureBinary,
  inspectTrackedPath,
  inspectTrackedText,
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

const objectRegistration = read("automation-object-expectations.json");
assert.equal(objectRegistration.length, 6777);
assert.equal(
  hash(objectRegistration),
  "c206b82e18abdaf11aa8fbc32c3a695e603824bd0c69f4b84421f43f4bc6dfd7",
);
const objectExamples = JSON.parse(objectRegistration);

const enumerationRegistration = read(
  "automation-enumeration-expectations.json",
);
assert.equal(enumerationRegistration.length, 5942);
assert.equal(
  hash(enumerationRegistration),
  "1e9978b757073e885e55659a371a0ea15a1e9ff30525d3733ccb33ad0988831c",
);
const enumerationExamples = JSON.parse(enumerationRegistration);

test("approved enumeration controls require their exact bytes, paths and GUI expectations", () => {
  assert.deepEqual(
    enumerationExamples.cases.map((c) => c.proposedSlot),
    ["F46", "F47", "F48"],
  );
  assert.deepEqual(
    inspectTrackedText(
      "fixtures/parser-corpus/automation-enumeration-expectations.json",
      enumerationRegistration.toString("utf8"),
    ),
    [],
  );
  for (const entry of enumerationExamples.cases) {
    const path = `fixtures/parser-corpus/${entry.file}`;
    const bytes = read(entry.file);
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
      inspectTrackedPath(`other/${entry.file}`, entry.sha256)[0].rule,
      "blocked-extension",
    );
  }
});

test("reorder/removal controls associate named curve bodies while IDs are reassigned; empty set stays a fixed-input observation", () => {
  // Research only: every input is hash-admitted above. This is not a runtime
  // recognizer, target decoder or an absence-as-zero rule for other layouts.
  const declarations = (records) => {
    const channels = [],
      orphanCurves = [],
      playlists = [];
    let current;
    for (const { id, payload } of records) {
      if ([64, 65, 98, 100, 233, 236].includes(id)) current = undefined;
      if (id === 64) {
        assert.equal(payload.length, 2);
        current = {
          savedId: payload.readUInt16LE(),
          types: [],
          names: [],
          curves: [],
        };
        channels.push(current);
      }
      if (id === 21 && current) current.types.push(payload[0]);
      if (id === 203 && current) {
        assert.equal(payload.length % 2, 0);
        current.names.push(payload.toString("utf16le").replace(/\0+$/, ""));
      }
      if (id === 234) (current ? current.curves : orphanCurves).push(payload);
      if (id === 233) playlists.push(payload);
    }
    assert.deepEqual(orphanCurves, []);
    assert.equal(playlists.length, 1);
    assert.equal(new Set(channels.map((c) => c.savedId)).size, channels.length);
    for (const channel of channels) {
      assert.equal(channel.types.length, 1);
      assert.ok([0, 5].includes(channel.types[0]));
      assert.equal(channel.curves.length, channel.types[0] === 5 ? 1 : 0);
      assert.equal(channel.names.length, channel.types[0] === 5 ? 1 : 0);
    }
    return { channels, playlist: playlists[0] };
  };
  const source = enumerationExamples.sourceApprovedFixture;
  const baseline = read(source.file);
  assert.equal(hash(baseline), source.sha256);
  const base = declarations(framedRecords(baseline));
  const baseObjects = new Map(
    base.channels.filter((c) => c.types[0] === 5).map((c) => [c.names[0], c]),
  );
  for (const entry of enumerationExamples.cases) {
    const before = read(entry.file);
    assert.equal(hash(before), entry.sha256);
    const { channels, playlist } = declarations(framedRecords(before));
    assert.equal(before.readUInt16LE(10), channels.length);
    assert.equal(channels.length, entry.independentGui.channelRows.length);
    const objects = channels.filter((c) => c.types[0] === 5);
    assert.equal(
      objects.length,
      entry.independentGui.nativeAutomationClipObjects,
    );
    assert.deepEqual(
      objects.map((c) => c.names[0]),
      entry.independentGui.objects.map((o) => o.name),
    );
    const counts = objects.map((c) => c.curves[0].readUInt32LE(17));
    assert.deepEqual(
      counts,
      entry.independentGui.objects.map((o) => o.editablePoints),
    );
    assert.equal(
      counts.reduce((n, value) => n + value, 0),
      entry.independentGui.editableMainCurvePoints,
    );
    assert.equal(
      playlist.length,
      88 * entry.independentGui.playlistOccurrences,
    );
    for (const [i, channel] of objects.entries()) {
      const original = baseObjects.get(channel.names[0]);
      const curve = channel.curves[0],
        old = original.curves[0];
      assert.equal(curve.length, 21 + 24 * counts[i] + 112);
      assert.equal(curve.readUInt32LE(13), 3);
      assert.deepEqual(curve.subarray(0, 17), old.subarray(0, 17));
      assert.deepEqual(curve.subarray(-112), old.subarray(-112));
      assert.equal(curve.length, old.length);
      const offsets = [...curve.keys()].filter((j) => curve[j] !== old[j]);
      if (entry.case === "C01") {
        assert.notEqual(channel.savedId, original.savedId);
        assert.equal(channel.savedId, i + 1);
        assert.deepEqual(offsets, i === 0 ? [] : [67]);
      } else {
        assert.equal(original.savedId, 2);
        assert.equal(channel.savedId, 1);
        assert.deepEqual(offsets, [115]); // Meaning remains unqualified.
      }
    }
    if (entry.case === "C03") {
      assert.deepEqual(
        channels.map((c) => c.types),
        [[0]],
      );
      assert.equal(objects.length, 0);
      assert.equal(playlist.length, 0);
      assert.ok(
        entry.independentGui.scope.includes(
          "not a claim about event automation",
        ),
      );
    }
    assert.deepEqual(read(entry.file), before);
  }
  assert.deepEqual(read(source.file), baseline);
});

test("frozen expectation prose exception cannot admit another file or changed personal paths", () => {
  const path = "fixtures/parser-corpus/automation-object-expectations.json";
  const text = objectRegistration.toString("utf8");
  assert.deepEqual(inspectTrackedText(path, text), []);
  assert.ok(
    inspectTrackedText(`other/${path}`, text).some(
      ({ rule }) => rule === "personal-home-path",
    ),
  );
  assert.ok(
    inspectTrackedText(path, text + "\n").some(
      ({ rule }) => rule === "personal-home-path",
    ),
  );
  assert.ok(
    inspectTrackedText(path, text + "/home/" + "someone/project/").some(
      ({ rule }) => rule === "personal-home-path",
    ),
  );
  const credential = "ghp_" + "A".repeat(36);
  assert.ok(
    inspectTrackedText(path, text + credential).some(
      ({ rule }) => rule === "github-token",
    ),
  );
});

test("approved object examples require exact bytes, paths and frozen GUI expectations", () => {
  assert.deepEqual(
    objectExamples.cases.map((c) => c.proposedSlot),
    ["F43", "F44", "F45"],
  );
  for (const entry of objectExamples.cases) {
    const path = `fixtures/parser-corpus/${entry.file}`;
    const bytes = read(entry.file);
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
      inspectTrackedPath(`other/${entry.file}`, entry.sha256)[0].rule,
      "blocked-extension",
    );
  }
});

test("same-count, multiple-object and one-point saves retain bounded observations without zero or binding inference", () => {
  const baseline = read("FIX-FL2026-AUTOMATION.flp");
  const twoPoint = read("FIX-FL2026-AUTO-2POINT.flp");
  const curves = (records) =>
    records.filter((event) => event.id === 234).map((event) => event.payload);
  const playlist = (records) => {
    const matches = records.filter((event) => event.id === 233);
    assert.equal(matches.length, 1);
    return matches[0].payload;
  };
  const differentOffsets = (left, right) => {
    assert.equal(left.length, right.length);
    return [...left.keys()].filter((i) => left[i] !== right[i]);
  };
  const baseRecords = framedRecords(baseline);
  const baseCurve = curves(baseRecords)[0];
  const basePlaylist = playlist(baseRecords);
  for (const entry of objectExamples.cases) {
    const before = read(entry.file);
    assert.equal(hash(before), entry.sha256);
    const records = framedRecords(before);
    const pointLists = curves(records);
    const counts = pointLists.map((payload) => payload.readUInt32LE(17));
    assert.equal(pointLists.length, entry.gui.uniqueNativeAutomationObjects);
    assert.deepEqual(
      counts,
      entry.gui.objects.map((object) => object.editablePoints),
    );
    assert.equal(
      counts.reduce((total, n) => total + n, 0),
      entry.gui.editableControlPoints,
    );
    for (const [i, payload] of pointLists.entries()) {
      assert.equal(payload.readUInt32LE(13), 3);
      assert.equal(payload.length, 21 + 24 * counts[i] + 112);
      assert.deepEqual(payload.subarray(0, 17), baseCurve.subarray(0, 17));
      assert.deepEqual(payload.subarray(-112), baseCurve.subarray(-112));
    }
    const placements = playlist(records);
    if (entry.case === "B01") {
      assert.deepEqual(
        differentOffsets(baseCurve, pointLists[0]),
        [56, 57, 58, 59, 67],
      );
      assert.deepEqual(placements, basePlaylist);
    } else if (entry.case === "B02") {
      assert.deepEqual(
        differentOffsets(curves(framedRecords(twoPoint))[0], pointLists[0]),
        [67], // Meaning remains unqualified, even with the same GUI shape.
      );
      assert.equal(placements.length, 176);
      assert.equal(
        entry.gui.objects[1].pointObservations[1].position,
        "001:05:001",
      );
      assert.ok(entry.savedTimingObservation.includes("001:05:000"));
    } else {
      assert.equal(entry.gui.editableControlPoints, 1);
      assert.equal(entry.gui.visibleNonemptyPlaylistOccurrences, 0);
      assert.equal(entry.gui.playlistPlacementCount, null);
      assert.equal(placements.length, 88); // Not an absent placement record.
      assert.deepEqual(differentOffsets(basePlaylist, placements), [8, 9, 32]);
      assert.ok(entry.authoringLimitation.includes("empty curve not achieved"));
    }
    assert.deepEqual(read(entry.file), before);
  }
  assert.deepEqual(read("FIX-FL2026-AUTOMATION.flp"), baseline);
  assert.deepEqual(read("FIX-FL2026-AUTO-2POINT.flp"), twoPoint);
});

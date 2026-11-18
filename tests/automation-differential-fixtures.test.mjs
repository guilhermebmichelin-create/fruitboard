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

const channelSectionEnd = new Set([65, 98, 100, 233, 236]);
const samplerRecords =
  "64,21,201,212,155,128,41,0,209,138,139,89,97,48,69,86,71,83,74,75,76,85,131,70,104,50,219,229,221,215,132,144,145,32,251,228,228,218,218,218,218,218,143,20,170,51";
const automationRecords =
  "64,21,201,212,203,155,128,41,0,209,138,139,89,97,48,69,86,71,83,74,75,76,85,131,70,104,50,219,229,221,215,132,144,145,234,32,228,228,218,218,218,218,218,143,20,170,51";
const lastChannelRecords = ",99,241,36";
const emptyGeneratorName = Buffer.alloc(2);
const curvePrefix = Buffer.from("0100000040000000000400000003000000", "hex");
const curveSuffix = Buffer.from(
  "01000000ffffffffffffffffffffffffffffffff80000000800000000000000080000000" +
    "0500000003000000010000000000000000000000000000000000f03f0000000000000000" +
    "0100000000000000fffffffffffffffffffffffffbb2000000000000000000000000000000000000",
  "hex",
);

function observedCurvePointCount(payload) {
  // This range is the demonstrated fixture range, not an FL Studio limit.
  if (payload.length < 21) return null;
  const count = payload.readUInt32LE(17);
  if (count < 1 || count > 4 || payload.length !== 21 + 24 * count + 112) {
    return null;
  }
  if (
    !payload.subarray(0, 17).equals(curvePrefix) ||
    !payload.subarray(21 + 24 * count).equals(curveSuffix)
  ) {
    return null;
  }
  return count;
}

// Deliberately narrow hypothesis evaluated only against approved fixed inputs
// and in-memory variants. No parser, native command, or app uses this function.
function recognizeAutomationStudyLayout(header, records) {
  const channelCount = header.readUInt16LE(10);
  if (
    header.readUInt16LE(8) !== 0 ||
    header.readUInt16LE(12) !== 96 ||
    channelCount < 1 ||
    channelCount > 3
  ) {
    return { reason: "header" };
  }

  const versions = records.filter(({ id }) => id === 199);
  const firstChannel = records.findIndex(({ id }) => id === 64);
  if (
    versions.length !== 1 ||
    versions[0].payload.toString("latin1") !== "26.1.0.5530\0" ||
    records.indexOf(versions[0]) >= firstChannel
  ) {
    return { reason: "version" };
  }

  const channels = [];
  const seenIds = new Set();
  let current;
  let playlist;
  for (const record of records) {
    const { id, payload } = record;
    if (channelSectionEnd.has(id)) {
      if (id === 233) {
        if (playlist || !current) return { reason: "section" };
        playlist = payload;
      } else if (!playlist) {
        return { reason: "section" };
      }
      current = undefined;
      continue;
    }
    if (id === 64) {
      const savedId = payload.readUInt16LE();
      if (playlist || channels.length >= channelCount || seenIds.has(savedId)) {
        return { reason: "declaration" };
      }
      seenIds.add(savedId);
      current = [];
      channels.push(current);
    }
    if (current) {
      current.push(record);
    } else if (id === 21 || id === 201 || id === 234) {
      return { reason: "orphan" };
    }
  }
  if (!playlist || channels.length !== channelCount) {
    return { reason: "incomplete" };
  }

  const pointCounts = [];
  for (const [index, channel] of channels.entries()) {
    const typeRecord = channel[1];
    const type = typeRecord?.id === 21 ? typeRecord.payload[0] : undefined;
    const expectedRecords =
      (index === 0 ? samplerRecords : automationRecords) +
      (index === channels.length - 1 ? lastChannelRecords : "");
    if (
      type !== (index === 0 ? 0 : 5) ||
      channel.map(({ id }) => id).join(",") !== expectedRecords ||
      !channel[2].payload.equals(emptyGeneratorName)
    ) {
      return { reason: "channel-layout" };
    }
    if (type === 5) {
      const { payload } = channel.find(({ id }) => id === 234);
      const count = observedCurvePointCount(payload);
      if (count === null) return { reason: "curve-layout" };
      pointCounts.push(count);
    }
  }

  const pointCount = pointCounts.reduce((total, count) => total + count, 0);
  if (pointCount > 6) return { reason: "study-aggregate" };
  if (pointCounts.length === 0 && playlist.length !== 0) {
    return { reason: "empty-set" };
  }
  return { objectCount: pointCounts.length, pointCount, pointCounts };
}

test("strict automation hypothesis matches the independent fixed-input counts and object order", () => {
  const approvals = [
    {
      file: "FIX-FL2026-MIN.flp",
      sha256:
        "e0471d55032b7dfaa8bbcd496c487147149b9ce18e0b250882cc6f67f47480ee",
      points: [],
    },
    {
      file: "FIX-FL2026-AUTOMATION.flp",
      sha256:
        "23b764e4220fc5e48ffbd4a99c602bda092c032ef35b9b2d5b785250b622cf7f",
      points: [3],
    },
    ...expected.cases.map((entry) => ({
      file: entry.proposedFile,
      sha256: entry.sha256,
      points: [entry.gui.editableControlPoints],
    })),
    ...objectExamples.cases.map((entry) => ({
      file: entry.file,
      sha256: entry.sha256,
      points: entry.gui.objects.map((object) => object.editablePoints),
    })),
    ...enumerationExamples.cases.map((entry) => ({
      file: entry.file,
      sha256: entry.sha256,
      points: entry.independentGui.objects.map(
        (object) => object.editablePoints,
      ),
    })),
  ];
  assert.equal(approvals.length, 12);
  for (const { file, sha256, points } of approvals) {
    const input = read(file);
    assert.equal(hash(input), sha256);
    assert.deepEqual(
      recognizeAutomationStudyLayout(input, framedRecords(input)),
      {
        objectCount: points.length,
        pointCount: points.reduce((total, count) => total + count, 0),
        pointCounts: points,
      },
      file,
    );
    assert.equal(hash(input), sha256);
    assert.equal(hash(read(file)), sha256);
  }
});

test("strict automation hypothesis refuses incomplete counts, unknown signatures, and ambiguous scopes", () => {
  const input = read("FIX-FL2026-AUTO-TWO-OBJECTS.flp");
  assert.equal(hash(input), enumerationExamples.sourceApprovedFixture.sha256);
  const records = framedRecords(input);
  const curves = records.flatMap(({ id }, index) =>
    id === 234 ? [index] : [],
  );
  const [firstCurve, secondCurve] = curves;
  const declarations = records.flatMap(({ id }, index) =>
    id === 64 ? [index] : [],
  );
  const [firstId, secondId] = declarations;
  const playlist = records.findIndex(({ id }) => id === 233);
  const version = records.findIndex(({ id }) => id === 199);
  const headerWith = (offset, value) => {
    const header = Buffer.from(input.subarray(0, 22));
    header.writeUInt16LE(value, offset);
    return header;
  };
  const withRecord = (index, record) => records.with(index, record);
  const withBody = (index, payload) =>
    withRecord(index, { id: records[index].id, payload });
  const without = (index) => records.toSpliced(index, 1);
  const insert = (index, record) => records.toSpliced(index, 0, record);
  const editCurve = (edit) => {
    const payload = Buffer.from(records[firstCurve].payload);
    edit(payload);
    return withBody(firstCurve, payload);
  };

  const cases = [
    ["wrong PPQ", "header", records, headerWith(12, 192)],
    ["oversized channel count", "header", records, headerWith(10, 65535)],
    [
      "header overcounts declarations",
      "incomplete",
      records.toSpliced(secondId, declarations[2] - secondId),
    ],
    [
      "header undercounts declarations",
      "declaration",
      records,
      headerWith(10, 2),
    ],
    [
      "duplicate declaration ID",
      "declaration",
      withBody(secondId, records[firstId].payload),
    ],
    ["missing channel type", "channel-layout", without(secondId + 1)],
    [
      "duplicate channel type",
      "channel-layout",
      insert(secondId + 1, records[secondId + 1]),
    ],
    [
      "unknown channel type",
      "channel-layout",
      withBody(secondId + 1, Buffer.from([6])),
    ],
    [
      "unexpected generator name on a clip",
      "channel-layout",
      withBody(secondId + 2, Buffer.from("3x Osc\0", "utf16le")),
    ],
    ["missing curve", "channel-layout", without(firstCurve)],
    [
      "duplicate curve",
      "channel-layout",
      insert(firstCurve, records[firstCurve]),
    ],
    ["orphan curve", "orphan", insert(playlist + 1, records[firstCurve])],
    [
      "section before curve",
      "section",
      insert(firstCurve, { id: 65, payload: Buffer.alloc(2) }),
    ],
    ["duplicate Playlist", "section", insert(playlist, records[playlist])],
    [
      "wrong saved version",
      "version",
      withBody(version, Buffer.from("26.1.1.5547\0")),
    ],
    ["duplicate saved version", "version", insert(version, records[version])],
    [
      "missing curve prefix bytes",
      "curve-layout",
      withBody(firstCurve, Buffer.alloc(20)),
    ],
    [
      "contradictory curve width",
      "curve-layout",
      withBody(firstCurve, records[firstCurve].payload.subarray(0, -1)),
    ],
    [
      "unknown curve prefix",
      "curve-layout",
      editCurve((payload) => (payload[0] ^= 1)),
    ],
    [
      "unknown curve suffix",
      "curve-layout",
      editCurve((payload) => (payload[payload.length - 1] ^= 1)),
    ],
    [
      "zero points on a present object",
      "curve-layout",
      editCurve((payload) => payload.writeUInt32LE(0, 17)),
    ],
    [
      "count outside study bound",
      "curve-layout",
      editCurve((payload) => payload.writeUInt32LE(0xffffffff, 17)),
    ],
    [
      "aggregate outside study bound",
      "study-aggregate",
      withBody(firstCurve, records[secondCurve].payload),
    ],
  ];
  for (const [name, reason, observed, header = input] of cases) {
    assert.deepEqual(
      recognizeAutomationStudyLayout(header, observed),
      { reason },
      name,
    );
  }

  const empty = read("FIX-FL2026-AUTO-REMOVED.flp");
  assert.equal(hash(empty), enumerationExamples.cases[2].sha256);
  const emptyRecords = framedRecords(empty);
  const emptyPlaylist = emptyRecords.findIndex(({ id }) => id === 233);
  const samplerGenerator = emptyRecords.findIndex(({ id }) => id === 201);
  const namedGenerator = {
    id: 201,
    payload: Buffer.from("3x Osc\0", "utf16le"),
  };
  assert.deepEqual(
    recognizeAutomationStudyLayout(
      empty,
      emptyRecords.with(samplerGenerator, namedGenerator),
    ),
    { reason: "channel-layout" },
    "a contradictory generator name must not qualify as a zero-object Sampler",
  );
  assert.deepEqual(
    recognizeAutomationStudyLayout(
      empty,
      emptyRecords.toSpliced(emptyPlaylist + 1, 0, namedGenerator),
    ),
    { reason: "orphan" },
    "an unscoped generator name must not qualify as a zero-object save",
  );
  assert.deepEqual(
    recognizeAutomationStudyLayout(
      empty,
      emptyRecords.with(emptyPlaylist, records[playlist]),
    ),
    { reason: "empty-set" },
  );
  assert.equal(hash(input), enumerationExamples.sourceApprovedFixture.sha256);
  assert.equal(hash(empty), enumerationExamples.cases[2].sha256);
});

test("the study does not promote familiar but unqualified saves to zero", () => {
  const examples = [
    ["FIX-FL2026-SAMPLE.flp", "channel-layout"],
    ["FIX-FL2026-PATTERNS.flp", "section"],
    ["FIX-FL2026-3XOSC.flp", "channel-layout"],
    ["FIX-FL2026-MULTISAMPLER.flp", "channel-layout"],
  ];
  for (const [file, reason] of examples) {
    const input = read(file);
    const before = hash(input);
    assert.deepEqual(
      inspectTrackedPath(`fixtures/parser-corpus/${file}`, before),
      [],
    );
    assert.deepEqual(
      recognizeAutomationStudyLayout(input, framedRecords(input)),
      { reason },
      file,
    );
    assert.equal(hash(read(file)), before);
  }
});

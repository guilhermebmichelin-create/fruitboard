import { describe, expect, it } from "vitest";
import { parseProjectDetails } from "./projectDetails";
import { identity } from "./projectDetails.fixture";
import { sequenceDetails } from "./noteSequences.fixture";
import { parseProjectNoteSequences } from "./projectNoteSequences";

const decode = (value: unknown, details = sequenceDetails()) =>
  parseProjectNoteSequences(
    value,
    "26.1.0.5530",
    details.patterns,
    details.patternNoteCounts,
    details.channels.length,
  );
describe("saved note sequence decoder", () => {
  it("selects complete source-ordered raw records and duplicate notes", () => {
    const details = sequenceDetails(3);
    expect(parseProjectDetails(details, identity)).toEqual(details);
    expect(decode(details.patternNoteSequences, details)).toEqual(
      details.patternNoteSequences,
    );
    expect(details.patternNoteSequences.items[0]?.sequence).toMatchObject({
      records: [
        { ordinal: 0, channelIndex: 0, startTick: 48 },
        { ordinal: 1, channelIndex: 1, startTick: 0 },
        { ordinal: 2, channelIndex: 0, startTick: 48 },
      ],
    });
  });
  it("rejects malformed records and relationships rather than hiding notes", () => {
    for (const [key, value] of [
      ["ordinal", 1],
      ["channelIndex", 2],
      ["savedKey", 61],
      ["startTick", -1],
      ["durationTick", 0],
      ["storedVelocityByte", 89.5],
      ["savedKey", "60"],
      ["startTick", NaN],
      ["durationTick", Infinity],
      ["private", "discard"],
    ] as const) {
      const raw = structuredClone(sequenceDetails().patternNoteSequences);
      const record = (
        raw.items[0]!.sequence as unknown as {
          records: Record<string, unknown>[];
        }
      ).records[0]!;
      record[key] = value;
      expect(() => decode(raw)).toThrow();
    }
    const raw = sequenceDetails().patternNoteSequences;
    for (const bad of [
      null,
      { ...raw, ppq: 192 },
      { ...raw, coverage: "song" },
      { ...raw, coordinateSystem: "song-ticks" },
      { ...raw, items: [] },
      { ...raw, items: [{ patternId: 2, sequence: raw.items[0]!.sequence }] },
      {
        ...raw,
        items: [
          { patternId: 1, sequence: { status: "extracted", records: [] } },
        ],
      },
      {
        ...raw,
        items: [
          {
            patternId: 1,
            sequence: { status: "unsupported", reason: "limit_exceeded" },
          },
        ],
      },
    ])
      expect(() => decode(bad)).toThrow();
  });
  it("keeps older, missing, unsupported and explicit detail-limit results separate", () => {
    expect(decode({ state: "unsupported", reason: "not_saved" })).toEqual({
      state: "unsupported",
      reason: "not_saved",
    });
    const missing = {
      ...sequenceDetails(),
      patterns: { state: "unavailable", reason: "no_stored_patterns" } as const,
      patternNoteCounts: {
        state: "unavailable",
        reason: "no_stored_patterns",
      } as const,
    };
    expect(
      decode({ state: "unavailable", reason: "no_stored_patterns" }, missing),
    ).toEqual({ state: "unavailable", reason: "no_stored_patterns" });
    for (const reason of [
      "unverified_ppq",
      "unverified_channels",
      "limit_exceeded",
    ] as const) {
      expect(decode({ state: "unsupported", reason })).toEqual({
        state: "unsupported",
        reason,
      });
    }
    for (const bad of [
      { state: "available", items: [] },
      { state: "unavailable", reason: "not_saved" },
      { state: "unsupported", reason: "unknown" },
      { state: "unsupported", reason: "unverified_build" },
      { state: "unsupported", reason: "not_saved", records: [] },
    ])
      expect(() => decode(bad)).toThrow();
  });
  it("enforces 512 per pattern and charges refused records against the 1024 total", () => {
    const boundary = sequenceDetails(512);
    expect(decode(boundary.patternNoteSequences, boundary)).toEqual(
      boundary.patternNoteSequences,
    );
    const over = sequenceDetails(513);
    expect(() => decode(over.patternNoteSequences, over)).toThrow();
    const refused = {
      ...over.patternNoteSequences,
      items: [
        {
          patternId: 1,
          sequence: {
            status: "unsupported",
            reason: "limit_exceeded",
          } as const,
        },
      ],
    };
    expect(decode(refused, over)).toEqual(refused);
    const patterns = {
      state: "available",
      count: 3,
      items: [1, 2, 3].map((patternId) => ({
        patternId,
        name: { status: "unavailable", value: null } as const,
      })),
    } as const;
    const counts = {
      state: "available",
      items: [1, 2, 3].map((patternId) => ({
        patternId,
        noteCount: { status: "extracted", value: 512 } as const,
      })),
    } as const;
    const raw = {
      ...refused,
      items: [1, 2, 3].map((patternId) => ({
        patternId,
        sequence: { status: "unsupported", reason: "unqualified_record" },
      })),
    };
    expect(() =>
      parseProjectNoteSequences(raw, "26.1.0.5530", patterns, counts, 2),
    ).toThrow();
  });
  it("rejects details on disabled/no-current replies and null present extensions", () => {
    for (const state of ["disabled", "no_current"])
      expect(() =>
        parseProjectDetails(
          {
            ...identity,
            state,
            patternNoteSequences: sequenceDetails().patternNoteSequences,
          },
          identity,
        ),
      ).toThrow();
    expect(() =>
      parseProjectDetails(
        { ...sequenceDetails(), patternNoteSequences: null },
        identity,
      ),
    ).toThrow();
    const details = sequenceDetails();
    const older: Record<string, unknown> = { ...details };
    delete older["patternNoteSequences"];
    expect(parseProjectDetails(older, identity)).toEqual(older);
  });
  it("keeps qualified siblings alongside missing and unverified note payloads", () => {
    const base = sequenceDetails();
    const patterns = {
      state: "available",
      count: 3,
      items: [1, 2, 3].map((patternId) => ({
        patternId,
        name: { status: "unavailable", value: null } as const,
      })),
    } as const;
    const counts = {
      state: "available",
      items: [
        { patternId: 1, noteCount: { status: "extracted", value: 1 } },
        {
          patternId: 2,
          noteCount: { status: "unavailable", reason: "no_stored_notes" },
        },
        {
          patternId: 3,
          noteCount: { status: "unsupported", reason: "unverified_layout" },
        },
      ],
    } as const;
    const raw = {
      ...base.patternNoteSequences,
      items: [
        base.patternNoteSequences.items[0],
        {
          patternId: 2,
          sequence: { status: "unavailable", reason: "no_stored_notes" },
        },
        {
          patternId: 3,
          sequence: { status: "unsupported", reason: "unverified_layout" },
        },
      ],
    };
    expect(
      parseProjectNoteSequences(raw, "26.1.0.5530", patterns, counts, 2),
    ).toEqual(raw);
    expect(
      parseProjectNoteSequences(
        { state: "unsupported", reason: "unverified_build" },
        "25.1.3.4922",
        { state: "unsupported", reason: "unverified_build" },
        { state: "unsupported", reason: "unverified_build" },
        1,
      ),
    ).toEqual({ state: "unsupported", reason: "unverified_build" });
  });
});

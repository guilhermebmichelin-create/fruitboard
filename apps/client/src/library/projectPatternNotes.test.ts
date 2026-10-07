import { describe, expect, it } from "vitest";
import { parseProjectPatternNotes } from "./projectPatternNotes";
import type { ProjectPatterns } from "./projectPatterns";
import { parseProjectDetails } from "./projectDetails";
import { identity, savedDetails } from "./projectDetails.fixture";

const build = "26.1.0.5530";
const patterns: ProjectPatterns = {
  state: "available",
  count: 3,
  items: [1, 2, 65535].map((patternId) => ({
    patternId,
    name: { status: "unavailable", value: null },
  })),
};
const notes = () => ({
  state: "available",
  items: [
    { patternId: 1, noteCount: { status: "extracted", value: 3 } },
    {
      patternId: 2,
      noteCount: { status: "unavailable", reason: "no_stored_notes" },
    },
    {
      patternId: 65535,
      noteCount: { status: "unsupported", reason: "unverified_layout" },
    },
  ],
});
describe("saved pattern note relationships", () => {
  it("selects mixed states and sparse IDs from the matching snapshot", () => {
    expect(parseProjectPatternNotes(notes(), build, patterns)).toEqual(notes());
    expect(
      parseProjectDetails(
        { ...savedDetails(), patterns, patternNoteCounts: notes() },
        identity,
      ),
    ).toMatchObject({ patterns, patternNoteCounts: notes() });
  });
  it("rejects malformed counts, extra note data, foreign, duplicate and reordered IDs", () => {
    for (const value of [0, -1, 1.5, 65537, "3", NaN, Infinity]) {
      const raw = notes();
      raw.items[0]!.noteCount.value = value as number;
      expect(() => parseProjectPatternNotes(raw, build, patterns)).toThrow();
    }
    for (const raw of [
      null,
      { ...notes(), rawNotes: [60] },
      { state: "available", items: [] },
      { state: "available", items: notes().items.reverse() },
      {
        state: "available",
        items: [notes().items[0], notes().items[0], notes().items[2]],
      },
      {
        state: "available",
        items: [
          {
            patternId: 1,
            noteCount: { status: "extracted", value: 3, pitches: [60] },
          },
          ...notes().items.slice(1),
        ],
      },
      {
        state: "available",
        items: [
          {
            patternId: 1,
            noteCount: { status: "unavailable", reason: "private error" },
          },
          ...notes().items.slice(1),
        ],
      },
    ]) {
      expect(() => parseProjectPatternNotes(raw, build, patterns)).toThrow();
    }
    expect(() =>
      parseProjectPatternNotes(notes(), "99.1.0.1", patterns),
    ).toThrow();
    expect(() => parseProjectPatternNotes(notes(), build)).toThrow();
  });
  it("enforces the total limit independently of the per-pattern ceiling", () => {
    const many: ProjectPatterns = {
      state: "available",
      count: 5,
      items: Array.from({ length: 5 }, (_, i) => ({
        patternId: i + 1,
        name: { status: "unavailable", value: null },
      })),
    };
    const raw = {
      state: "available",
      items: many.items.map((p) => ({
        patternId: p.patternId,
        noteCount: { status: "extracted", value: 65536 },
      })),
    };
    expect(() => parseProjectPatternNotes(raw, build, many)).toThrow();
    expect(
      parseProjectPatternNotes(
        { ...raw, items: raw.items.slice(0, 4) },
        build,
        { ...many, count: 4, items: many.items.slice(0, 4) },
      ).state,
    ).toBe("available");
  });
  it("keeps old snapshots readable and never accepts counts on unavailable details", () => {
    expect(parseProjectDetails(savedDetails(), identity).state).toBe(
      "available",
    );
    for (const state of ["disabled", "no_current"])
      expect(() =>
        parseProjectDetails(
          { ...identity, state, patternNoteCounts: notes() },
          identity,
        ),
      ).toThrow();
    expect(() =>
      parseProjectDetails(
        { ...savedDetails(), patterns, patternNoteCounts: null },
        identity,
      ),
    ).toThrow();
    for (const reason of [
      "not_saved",
      "not_advertised",
      "limit_exceeded",
      "unverified_binding",
    ])
      expect(
        parseProjectPatternNotes(
          { state: "unsupported", reason },
          build,
          patterns,
        ),
      ).toEqual({ state: "unsupported", reason });
    expect(
      parseProjectPatternNotes(
        { state: "unavailable", reason: "no_stored_patterns" },
        build,
        { state: "unavailable", reason: "no_stored_patterns" },
      ).state,
    ).toBe("unavailable");
    expect(() =>
      parseProjectPatternNotes(
        { state: "unavailable", reason: "no_stored_patterns" },
        build,
        patterns,
      ),
    ).toThrow();
  });
});

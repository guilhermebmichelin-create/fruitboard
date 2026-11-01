import { savedDetails } from "./projectDetails.fixture";
import type { ProjectDetails } from "./projectDetails";
import type { ProjectNoteSequences } from "./projectNoteSequences";

/** Constructed display/defense data; no additional FL Studio qualification. */
export function sequenceDetails(count = 1): Extract<
  ProjectDetails,
  { state: "available" }
> & {
  readonly patternNoteSequences: Extract<
    ProjectNoteSequences,
    { state: "available" }
  >;
} {
  const base = savedDetails();
  const sampler = base.channels[1]!;
  return {
    ...base,
    facts: base.facts.map((fact) =>
      fact.key === "channelCount" ? { ...fact, value: "2" } : fact,
    ),
    channels: [1, 2].map((position) => ({
      ...sampler,
      position,
      name: {
        status: "extracted",
        value: position === 1 ? "NoteOne" : "NoteTwo",
        explanation: null,
      },
    })),
    patterns: {
      state: "available",
      count: 1,
      items: [
        {
          patternId: 1,
          name: { status: "extracted", value: "Ordinary notes" },
        },
      ],
    },
    patternNoteCounts: {
      state: "available",
      items: [
        { patternId: 1, noteCount: { status: "extracted", value: count } },
      ],
    },
    patternNoteSequences: {
      state: "available",
      coverage: "stored-pattern-note-records",
      coordinateSystem: "pattern-local-ticks",
      ppq: 96,
      items: [
        {
          patternId: 1,
          sequence: {
            status: "extracted",
            records: Array.from({ length: count }, (_, ordinal) => ({
              ordinal,
              channelIndex: ordinal % 2,
              savedKey: ordinal % 2 === 0 ? 60 : 64,
              startTick: ordinal % 2 === 0 ? 48 : 0,
              durationTick: 48,
              storedVelocityByte: 89,
            })),
          },
        },
      ],
    },
  };
}

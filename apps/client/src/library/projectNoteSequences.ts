import { LibraryAdapterError } from "./contracts";
import type { ProjectPatterns } from "./projectPatterns";
import type { ProjectPatternNotes } from "./projectPatternNotes";

export interface SavedNoteRecord {
  readonly ordinal: number;
  readonly channelIndex: number;
  readonly savedKey: number;
  readonly startTick: number;
  readonly durationTick: number;
  readonly storedVelocityByte: number;
}
export type SavedNoteSequence =
  | {
      readonly status: "extracted";
      readonly records: readonly SavedNoteRecord[];
    }
  | { readonly status: "unavailable"; readonly reason: "no_stored_notes" }
  | {
      readonly status: "unsupported";
      readonly reason:
        | "unverified_layout"
        | "multiple_payloads"
        | "limit_exceeded"
        | "unqualified_record";
    };
export type ProjectNoteSequences =
  | {
      readonly state: "available";
      readonly coverage: "stored-pattern-note-records";
      readonly coordinateSystem: "pattern-local-ticks";
      readonly ppq: 96;
      readonly items: readonly {
        readonly patternId: number;
        readonly sequence: SavedNoteSequence;
      }[];
    }
  | { readonly state: "unavailable"; readonly reason: "no_stored_patterns" }
  | {
      readonly state: "unsupported";
      readonly reason:
        | "not_saved"
        | "unverified_build"
        | "unverified_binding"
        | "limit_exceeded"
        | "unverified_ppq"
        | "unverified_channels";
    };

const record = (v: unknown): v is Record<string, unknown> =>
  typeof v === "object" && v !== null && !Array.isArray(v);
const fail = (): never => {
  throw new LibraryAdapterError("internal");
};
function shape(
  value: unknown,
  keys: readonly string[],
): asserts value is Record<string, unknown> {
  if (
    !record(value) ||
    Object.keys(value).length !== keys.length ||
    keys.some((key) => !Object.hasOwn(value, key))
  )
    fail();
}
function integer(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0)
    return fail();
  return value;
}

/** Same-result pattern/count/channel relationships; raw saved values only. */
export function parseProjectNoteSequences(
  value: unknown,
  build: string,
  patterns: ProjectPatterns | undefined,
  counts: ProjectPatternNotes | undefined,
  channelCount: number,
): ProjectNoteSequences {
  if (!record(value)) return fail();
  const state = value["state"];
  const qualified = build === "26.1.0.5530";
  if (state !== "available") {
    shape(value, ["state", "reason"]);
    const reason = value["reason"];
    if (
      state === "unavailable" &&
      reason === "no_stored_patterns" &&
      qualified &&
      patterns?.state === "unavailable" &&
      counts?.state === "unavailable"
    )
      return { state, reason };
    if (
      state === "unsupported" &&
      (reason === "not_saved" ||
        (reason === "unverified_build" &&
          !qualified &&
          counts?.state === "unsupported" &&
          counts.reason === reason) ||
        (qualified && reason === "limit_exceeded") ||
        (qualified &&
          reason === "unverified_binding" &&
          counts?.state === "unsupported" &&
          counts.reason === reason) ||
        (qualified &&
          (reason === "unverified_ppq" || reason === "unverified_channels") &&
          counts?.state === "available"))
    )
      return { state, reason };
    return fail();
  }
  shape(value, ["state", "coverage", "coordinateSystem", "ppq", "items"]);
  const items = value["items"];
  if (
    !qualified ||
    value["coverage"] !== "stored-pattern-note-records" ||
    value["coordinateSystem"] !== "pattern-local-ticks" ||
    value["ppq"] !== 96 ||
    !Number.isInteger(channelCount) ||
    channelCount < 1 ||
    channelCount > 2 ||
    patterns?.state !== "available" ||
    counts?.state !== "available" ||
    !Array.isArray(items) ||
    items.length < 1 ||
    items.length > 1024 ||
    items.length !== patterns.count ||
    items.length !== counts.items.length
  )
    return fail();
  let total = 0;
  const selected = items.map((item: unknown, index) => {
    shape(item, ["patternId", "sequence"]);
    const patternId = integer(item["patternId"]);
    const count = counts.items[index];
    if (
      patternId !== patterns.items[index]?.patternId ||
      patternId !== count?.patternId
    )
      return fail();
    const sequence = item["sequence"];
    if (!record(sequence)) return fail();
    let chosen: SavedNoteSequence;
    const noteCount = count.noteCount;
    if (noteCount.status === "unavailable") {
      shape(sequence, ["status", "reason"]);
      if (
        sequence["status"] !== "unavailable" ||
        sequence["reason"] !== "no_stored_notes"
      )
        return fail();
      chosen = { status: "unavailable", reason: "no_stored_notes" };
    } else if (noteCount.status === "unsupported") {
      shape(sequence, ["status", "reason"]);
      if (
        sequence["status"] !== "unsupported" ||
        sequence["reason"] !== noteCount.reason
      )
        return fail();
      chosen = { status: "unsupported", reason: noteCount.reason };
    } else {
      // Refused candidates still charge the complete saved count.
      total += noteCount.value;
      if (total > 1024) return fail();
      if (sequence["status"] === "unsupported") {
        shape(sequence, ["status", "reason"]);
        const reason =
          noteCount.value > 512 ? "limit_exceeded" : "unqualified_record";
        if (sequence["reason"] !== reason) return fail();
        chosen = { status: "unsupported", reason };
      } else {
        shape(sequence, ["status", "records"]);
        const records = sequence["records"];
        if (
          sequence["status"] !== "extracted" ||
          !Array.isArray(records) ||
          records.length !== noteCount.value ||
          records.length < 1 ||
          records.length > 512
        )
          return fail();
        const selectedRecords = records.map((note: unknown, ordinal) => {
          shape(note, [
            "ordinal",
            "channelIndex",
            "savedKey",
            "startTick",
            "durationTick",
            "storedVelocityByte",
          ]);
          const sourceOrdinal = integer(note["ordinal"]);
          const channelIndex = integer(note["channelIndex"]);
          const savedKey = integer(note["savedKey"]);
          const startTick = integer(note["startTick"]);
          const durationTick = integer(note["durationTick"]);
          const storedVelocityByte = integer(note["storedVelocityByte"]);
          if (
            sourceOrdinal !== ordinal ||
            channelIndex >= channelCount ||
            ![60, 64].includes(savedKey) ||
            ![0, 48].includes(startTick) ||
            ![48, 96].includes(durationTick) ||
            ![74, 89].includes(storedVelocityByte) ||
            startTick + durationTick > 0xffffffff
          )
            return fail();
          return {
            ordinal,
            channelIndex,
            savedKey,
            startTick,
            durationTick,
            storedVelocityByte,
          };
        });
        chosen = { status: "extracted", records: selectedRecords };
      }
    }
    return { patternId, sequence: chosen };
  });
  const result: ProjectNoteSequences = {
    state: "available",
    coverage: "stored-pattern-note-records",
    coordinateSystem: "pattern-local-ticks",
    ppq: 96,
    items: selected,
  };
  // All selected sequence keys/reasons are ASCII and values are bounded integers.
  if (JSON.stringify(result).length > 128 * 1024) return fail();
  return result;
}

export const NOTE_SEQUENCE_REASONS = {
  not_saved:
    "Note sequence details were not saved with this result. Analyze the project again to add them.",
  no_stored_patterns:
    "Note sequences are unknown because no pattern data was stored.",
  no_stored_notes:
    "No note payload was stored for this pattern. This does not prove it has zero notes.",
  unverified_build:
    "Note sequences are not verified for this saved FL Studio build.",
  unverified_binding:
    "Saved note data could not be safely matched to patterns.",
  unverified_layout:
    "The saved note layout is not verified, so no sequence is shown.",
  multiple_payloads:
    "Multiple note payloads were stored for this pattern, so no sequence is shown.",
  limit_exceeded:
    "Note sequence details exceeded the supported detail limit. No shortened sequence is shown.",
  unverified_ppq:
    "The saved tick resolution is not verified for note sequences.",
  unverified_channels:
    "The saved channel layout is not verified for note sequences.",
  unqualified_record:
    "This pattern contains an unqualified note record. The complete sequence is unavailable; its saved note count remains separate.",
} as const;

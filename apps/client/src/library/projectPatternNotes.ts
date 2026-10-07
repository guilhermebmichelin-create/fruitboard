import { LibraryAdapterError } from "./contracts";
import type { ProjectPatterns } from "./projectPatterns";

export type PatternNoteCount =
  | { readonly status: "extracted"; readonly value: number }
  | { readonly status: "unavailable"; readonly reason: "no_stored_notes" }
  | {
      readonly status: "unsupported";
      readonly reason: "unverified_layout" | "multiple_payloads";
    };
export type ProjectPatternNotes =
  | {
      readonly state: "available";
      readonly items: readonly {
        readonly patternId: number;
        readonly noteCount: PatternNoteCount;
      }[];
    }
  | { readonly state: "unavailable"; readonly reason: "no_stored_patterns" }
  | {
      readonly state: "unsupported";
      readonly reason:
        | "not_saved"
        | "not_advertised"
        | "unverified_build"
        | "unverified_binding"
        | "limit_exceeded";
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

/** Counts belong to exactly the patterns in this current saved snapshot. */
export function parseProjectPatternNotes(
  value: unknown,
  build: string,
  patterns?: ProjectPatterns,
): ProjectPatternNotes {
  if (!record(value)) return fail();
  const state = value["state"];
  if (state !== "available") {
    shape(value, ["state", "reason"]);
    const reason = value["reason"];
    if (
      state === "unavailable" &&
      reason === "no_stored_patterns" &&
      build === "26.1.0.5530" &&
      patterns?.state === "unavailable"
    )
      return { state, reason };
    if (
      state === "unsupported" &&
      (reason === "not_saved" ||
        reason === "not_advertised" ||
        (reason === "unverified_build" && build !== "26.1.0.5530") ||
        ((reason === "unverified_binding" || reason === "limit_exceeded") &&
          build === "26.1.0.5530"))
    )
      return { state, reason };
    return fail();
  }
  shape(value, ["state", "items"]);
  const items = value["items"];
  if (
    build !== "26.1.0.5530" ||
    patterns?.state !== "available" ||
    !Array.isArray(items) ||
    items.length !== patterns.count
  )
    return fail();
  let total = 0;
  const selected = items.map((item: unknown, index) => {
    shape(item, ["patternId", "noteCount"]);
    const patternId = item["patternId"];
    if (
      patternId !== patterns.items[index]?.patternId ||
      typeof patternId !== "number"
    )
      return fail();
    const count = item["noteCount"];
    if (!record(count)) return fail();
    let noteCount: PatternNoteCount;
    if (count["status"] === "extracted") {
      shape(count, ["status", "value"]);
      const number = count["value"];
      if (
        typeof number !== "number" ||
        !Number.isInteger(number) ||
        number < 1 ||
        number > 65536
      )
        return fail();
      total += number;
      if (total > 262144) return fail();
      noteCount = { status: "extracted", value: number };
    } else {
      shape(count, ["status", "reason"]);
      const reason = count["reason"];
      if (count["status"] === "unavailable" && reason === "no_stored_notes")
        noteCount = { status: "unavailable", reason };
      else if (
        count["status"] === "unsupported" &&
        (reason === "unverified_layout" || reason === "multiple_payloads")
      )
        noteCount = { status: "unsupported", reason };
      else return fail();
    }
    return { patternId, noteCount };
  });
  return { state: "available", items: selected };
}

export const NOTE_COUNT_REASONS = {
  not_saved:
    "Note counts were not saved with this result. A new analysis is needed to add them.",
  not_advertised: "The parser did not include note counts in this result.",
  no_stored_patterns:
    "Saved note count is unknown; no pattern data was stored.",
  no_stored_notes:
    "Saved note count is unknown; no note data was stored for this pattern.",
  unverified_build:
    "Note counts are not verified for this saved FL Studio build.",
  unverified_layout:
    "Saved note count is unavailable because the note layout is not verified.",
  multiple_payloads:
    "Saved note count is unavailable because multiple note payloads were stored for this pattern.",
  unverified_binding:
    "Saved note counts are unavailable because the note data could not be safely matched to patterns.",
  limit_exceeded:
    "Saved note counts are unavailable because the supported note-record limit was exceeded.",
} as const;

import { LibraryAdapterError } from "./contracts";

export type MixerInsertName =
  | { readonly status: "extracted"; readonly value: string }
  | { readonly status: "unavailable"; readonly reason: "name_not_saved" };
export type ProjectMixerInserts =
  | {
      readonly state: "available";
      readonly count: number;
      readonly items: readonly {
        readonly savedInsertId: number;
        readonly name: MixerInsertName;
      }[];
    }
  | { readonly state: "unavailable"; readonly reason: "no_saved_section" }
  | {
      readonly state: "unsupported";
      readonly reason:
        | "not_saved"
        | "not_advertised"
        | "unverified_build"
        | "unverified_layout"
        | "unverified_binding"
        | "ambiguous_records"
        | "limit_exceeded";
    };
const record = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);
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

/** Only the approved complete layout's ordinary positions 1–16 are qualified. */
export function parseProjectMixerInserts(
  value: unknown,
  build: string,
): ProjectMixerInserts {
  if (!record(value)) return fail();
  const state = value["state"];
  if (state !== "available") {
    shape(value, ["state", "reason"]);
    const reason = value["reason"];
    if (
      state === "unavailable" &&
      reason === "no_saved_section" &&
      build === "26.1.0.5530"
    )
      return { state, reason };
    if (
      state === "unsupported" &&
      (reason === "not_saved" ||
        reason === "not_advertised" ||
        (reason === "unverified_build" && build !== "26.1.0.5530") ||
        (build === "26.1.0.5530" &&
          (reason === "unverified_layout" ||
            reason === "unverified_binding" ||
            reason === "ambiguous_records" ||
            reason === "limit_exceeded")))
    )
      return { state, reason };
    return fail();
  }
  shape(value, ["state", "count", "items"]);
  const items = value["items"];
  if (
    build !== "26.1.0.5530" ||
    value["count"] !== 16 ||
    !Array.isArray(items) ||
    items.length !== 16
  )
    return fail();
  let bytes = 0;
  const selected = items.map((item: unknown, index) => {
    shape(item, ["savedInsertId", "name"]);
    if (item["savedInsertId"] !== index + 1) return fail();
    const name = item["name"];
    if (!record(name)) return fail();
    let selectedName: MixerInsertName;
    if (name["status"] === "extracted") {
      shape(name, ["status", "value"]);
      const text = name["value"];
      if (
        typeof text !== "string" ||
        text.length === 0 ||
        text.length > 4095 ||
        text.includes("\0") ||
        /[\uD800-\uDFFF]/u.test(text)
      )
        return fail();
      const length = new TextEncoder().encode(text).length;
      bytes += length;
      if (length > 12285 || bytes > 256 * 1024) return fail();
      selectedName = { status: "extracted", value: text };
    } else {
      shape(name, ["status", "reason"]);
      if (
        name["status"] !== "unavailable" ||
        name["reason"] !== "name_not_saved"
      )
        return fail();
      selectedName = { status: "unavailable", reason: "name_not_saved" };
    }
    return { savedInsertId: index + 1, name: selectedName };
  });
  return { state, count: 16, items: selected };
}

export const MIXER_REASONS = {
  not_saved:
    "Mixer details were not saved in this result. Run a new analysis to read them.",
  not_advertised: "This parser did not provide saved mixer insert details.",
  no_saved_section: "No qualified saved mixer insert section was found.",
  unverified_build:
    "Saved mixer inserts are not verified for this FL Studio build.",
  unverified_layout: "This saved mixer layout is not verified.",
  unverified_binding: "Saved mixer insert identities could not be verified.",
  ambiguous_records: "Saved mixer insert records are ambiguous.",
  limit_exceeded: "Saved mixer insert details exceed the supported limits.",
} as const;

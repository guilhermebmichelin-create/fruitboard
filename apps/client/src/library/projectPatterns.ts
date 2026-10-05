import { LibraryAdapterError } from "./contracts";

export interface ProjectPattern {
  readonly patternId: number;
  readonly name:
    | { readonly status: "extracted"; readonly value: string }
    | { readonly status: "unavailable"; readonly value: null };
}
export type ProjectPatterns =
  | {
      readonly state: "available";
      readonly count: number;
      readonly items: readonly ProjectPattern[];
    }
  | { readonly state: "unavailable"; readonly reason: "no_stored_patterns" }
  | {
      readonly state: "unsupported";
      readonly reason: "not_saved" | "not_advertised" | "unverified_build";
    };

const record = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);
const fail = (): never => {
  throw new LibraryAdapterError("internal");
};

/** Select bounded current display data; discard arbitrary response extensions. */
export function parseProjectPatterns(
  value: unknown,
  build: string,
): ProjectPatterns {
  if (!record(value)) return fail();
  if (value["state"] !== "available") {
    if (value["count"] !== undefined || value["items"] !== undefined)
      return fail();
    const reason = value["reason"];
    if (
      value["state"] === "unavailable" &&
      reason === "no_stored_patterns" &&
      build === "26.1.0.5530"
    )
      return { state: "unavailable", reason };
    if (
      value["state"] === "unsupported" &&
      (reason === "not_saved" ||
        reason === "not_advertised" ||
        (reason === "unverified_build" && build !== "26.1.0.5530"))
    )
      return { state: "unsupported", reason };
    return fail();
  }
  const count = value["count"];
  const items = value["items"];
  if (
    build !== "26.1.0.5530" ||
    value["reason"] !== undefined ||
    typeof count !== "number" ||
    !Number.isInteger(count) ||
    count < 1 ||
    count > 1024 ||
    !Array.isArray(items) ||
    items.length !== count
  )
    return fail();
  let previous = 0;
  const selected = items.map((item: unknown): ProjectPattern => {
    if (!record(item)) return fail();
    const patternId = item["patternId"];
    const name = item["name"];
    if (
      typeof patternId !== "number" ||
      !Number.isInteger(patternId) ||
      patternId <= previous ||
      patternId > 65535 ||
      !record(name)
    )
      return fail();
    previous = patternId;
    if (name["status"] === "unavailable" && name["value"] === null)
      return { patternId, name: { status: "unavailable", value: null } };
    const text = name["value"];
    if (
      name["status"] !== "extracted" ||
      typeof text !== "string" ||
      text.length > 4095 ||
      new TextEncoder().encode(text).length > 12285 ||
      text.includes("\0")
    )
      return fail();
    return { patternId, name: { status: "extracted", value: text } };
  });
  return { state: "available", count, items: selected };
}

export const PATTERN_REASONS = {
  not_saved:
    "Pattern details were not saved with this result. A new analysis is needed to add them.",
  not_advertised: "The parser did not include pattern details in this result.",
  unverified_build:
    "Pattern details are not verified for this saved FL Studio build.",
  no_stored_patterns:
    "No pattern data was stored. FL Studio may show a default empty pattern; its saved count is unknown.",
} as const;

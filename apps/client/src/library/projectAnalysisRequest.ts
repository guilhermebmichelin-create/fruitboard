import { LibraryAdapterError, type PublishedFileLocation } from "./contracts";

export const ANALYSIS_REQUEST_BLOCKS = [
  "pending",
  "attempt_limit",
  "unsupported",
  "source_changed",
  "unqualified_source",
  "too_large",
  "queue_full",
  "runtime_unavailable",
] as const;
export type AnalysisRequestBlock = (typeof ANALYSIS_REQUEST_BLOCKS)[number];
export type ProjectAnalysisRequest =
  | { readonly state: "ready"; readonly requestKey: string }
  | { readonly state: AnalysisRequestBlock; readonly requestKey: null };
export type ProjectAnalysisRequestResult = {
  readonly rootId: string;
  readonly locationId: string;
} & (
  | { readonly state: "queued" | "disabled"; readonly reason: null }
  | { readonly state: "blocked"; readonly reason: AnalysisRequestBlock }
);
const record = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);
const fail = (): never => {
  throw new LibraryAdapterError("internal");
};
export const validAnalysisRequestKey = (key: unknown): key is string =>
  typeof key === "string" && /^[0-9a-f]{64}$/.test(key);
export function parseProjectAnalysisRequest(
  value: unknown,
): ProjectAnalysisRequest {
  if (!record(value)) return fail();
  const state = value["state"];
  if (state === "ready" && validAnalysisRequestKey(value["requestKey"]))
    return { state, requestKey: value["requestKey"] };
  if (
    !ANALYSIS_REQUEST_BLOCKS.includes(state as AnalysisRequestBlock) ||
    value["requestKey"] !== null
  )
    return fail();
  return { state: state as AnalysisRequestBlock, requestKey: null };
}
export function parseProjectAnalysisRequestResult(
  value: unknown,
  file: Pick<PublishedFileLocation, "rootId" | "locationId">,
): ProjectAnalysisRequestResult {
  if (
    !record(value) ||
    value["rootId"] !== file.rootId ||
    value["locationId"] !== file.locationId
  )
    return fail();
  const identity = { rootId: file.rootId, locationId: file.locationId };
  const state = value["state"];
  if (state === "queued" || state === "disabled") {
    if (value["reason"] !== null) return fail();
    return { ...identity, state, reason: null };
  }
  if (
    state !== "blocked" ||
    !ANALYSIS_REQUEST_BLOCKS.includes(value["reason"] as AnalysisRequestBlock)
  )
    return fail();
  return {
    ...identity,
    state,
    reason: value["reason"] as AnalysisRequestBlock,
  };
}
export const ANALYSIS_REQUEST_NOTES: Readonly<
  Record<AnalysisRequestBlock, string>
> = {
  pending:
    "Analysis is already waiting or reading. Refresh details to check for a result.",
  attempt_limit:
    "All three attempts for this unchanged file have been started. Another request cannot reset that limit.",
  unsupported:
    "This saved build is unsupported. Another attempt with the same file and reader cannot add support.",
  source_changed:
    "The source changed or is no longer eligible. Refresh the Library before requesting analysis.",
  unqualified_source:
    "This file has no verified local identity. Scan its approved local folder before requesting analysis.",
  too_large: "This file exceeds the 64 MiB analysis limit (67,108,864 bytes).",
  queue_full:
    "The analysis queue is full. Refresh details after other work finishes.",
  runtime_unavailable:
    "The analysis worker is unavailable in this session. Reopen the development app before requesting analysis.",
};

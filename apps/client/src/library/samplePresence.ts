import {
  LibraryAdapterError,
  extractModifiedAtFraction,
  validateByteSizeDecimal,
} from "./contracts";
import type { ProjectSampleReference } from "./sampleReferences";

export const UNCHECKED_REASONS = [
  "outside_root",
  "unsupported_path_syntax",
  "relative_reference",
  "unresolved_placeholder",
  "unqualified_filesystem",
  "unsupported_case_mode",
  "reparse_or_offline",
  "access_denied",
  "not_regular_file",
  "limit_reached",
  "deadline",
  "cancelled",
] as const;
export type UncheckedReason = (typeof UNCHECKED_REASONS)[number];
export const UNCHECKED_COPY: Readonly<Record<UncheckedReason, string>> = {
  outside_root: "Outside the selected folder.",
  unsupported_path_syntax: "This saved path format is not supported.",
  relative_reference: "Relative references cannot be checked yet.",
  unresolved_placeholder: "This reference contains an unresolved placeholder.",
  unqualified_filesystem:
    "The source folder could not be qualified as local NTFS.",
  unsupported_case_mode: "This path's letter case could not be matched safely.",
  reparse_or_offline: "A linked or offline location cannot be checked.",
  access_denied: "Metadata access was denied.",
  not_regular_file: "This saved path does not name an ordinary file.",
  limit_reached: "The check reached its work limit.",
  deadline: "The check reached its time limit.",
  cancelled: "The check was cancelled.",
};
export interface SampleCheckRequest {
  readonly schemaVersion: 1;
  readonly rootId: string;
  readonly locationId: string;
  readonly snapshotId: string;
  readonly requestId: string;
  readonly expectedByteSize: string;
  readonly expectedModifiedAt: string;
}
export type SampleOutcome = { readonly position: number } & (
  | { readonly status: "present" | "not_found" | "no_saved_reference" }
  | { readonly status: "not_checked"; readonly reason: UncheckedReason }
);
export interface SampleReport {
  readonly checkedAt: number;
  readonly channels: readonly SampleOutcome[];
}
export type SampleCheckResult =
  | { readonly state: "complete"; readonly report: SampleReport }
  | { readonly state: "unavailable"; readonly reason: UncheckedReason }
  | {
      readonly state:
        | "disabled"
        | "busy"
        | "stale"
        | "cancelled"
        | "deadline"
        | "failed"
        | "references_unavailable"
        | "no_references";
    };

const fail = (): never => {
  throw new LibraryAdapterError("internal");
};
const id = (value: unknown): value is string =>
  typeof value === "string" && /^[A-Za-z0-9_-]{1,128}$/.test(value);
function object(
  value: unknown,
  keys: readonly string[],
): Record<string, unknown> {
  if (
    typeof value !== "object" ||
    value === null ||
    Array.isArray(value) ||
    Object.keys(value).length !== keys.length ||
    keys.some((key) => !Object.hasOwn(value, key))
  )
    return fail();
  return value as Record<string, unknown>;
}
function reason(value: unknown): UncheckedReason {
  if (typeof value !== "string" || !UNCHECKED_REASONS.some((r) => r === value))
    return fail();
  return value as UncheckedReason;
}
const REQUEST_KEYS = [
  "schemaVersion",
  "rootId",
  "locationId",
  "snapshotId",
  "requestId",
  "expectedByteSize",
  "expectedModifiedAt",
] as const;
export function sampleCheckArguments(request: SampleCheckRequest) {
  const value = object(request, REQUEST_KEYS);
  if (
    request.schemaVersion !== 1 ||
    !id(request.rootId) ||
    !id(request.locationId) ||
    !id(request.snapshotId) ||
    !id(request.requestId) ||
    !validateByteSizeDecimal(request.expectedByteSize).ok ||
    extractModifiedAtFraction(request.expectedModifiedAt) === null
  )
    return fail();
  // A field-by-field copy grants no authority to runtime extension properties.
  return {
    request: Object.fromEntries(REQUEST_KEYS.map((key) => [key, value[key]])),
  };
}
export function parseSampleCheckResult(
  raw: unknown,
  request: SampleCheckRequest,
  samples: readonly ProjectSampleReference[],
  cancel = false,
): SampleCheckResult {
  const value = object(raw, ["request", "state", "reason", "report"]);
  const echo = object(value["request"], REQUEST_KEYS);
  if (REQUEST_KEYS.some((key) => echo[key] !== request[key])) return fail();
  const state = value["state"];
  if (cancel && state !== "cancelled" && state !== "disabled") return fail();
  if (state !== "complete") {
    if (value["report"] !== null) return fail();
    if (state === "unavailable")
      return { state, reason: reason(value["reason"]) };
    if (
      value["reason"] !== null ||
      ![
        "disabled",
        "busy",
        "stale",
        "cancelled",
        "deadline",
        "failed",
        "references_unavailable",
        "no_references",
      ].some((s) => s === state)
    )
      return fail();
    if (
      state === "no_references" &&
      samples.some((s) => s.status === "extracted")
    )
      return fail();
    return {
      state: state as Exclude<
        SampleCheckResult["state"],
        "complete" | "unavailable"
      >,
    };
  }
  if (value["reason"] !== null || samples.length > 256) return fail();
  const report = object(value["report"], [
    "version",
    "context",
    "checked_at_unix_ms",
    "channels",
  ]);
  const context = object(report["context"], [
    "request_id",
    "root_id",
    "location_id",
    "snapshot_id",
    "session_id",
  ]);
  if (
    report["version"] !== 1 ||
    context["request_id"] !== request.requestId ||
    context["root_id"] !== request.rootId ||
    context["location_id"] !== request.locationId ||
    context["snapshot_id"] !== request.snapshotId ||
    !id(context["session_id"])
  )
    return fail();
  const timestamp = report["checked_at_unix_ms"];
  if (
    typeof timestamp !== "number" ||
    !Number.isSafeInteger(timestamp) ||
    timestamp < 0 ||
    timestamp > 253402300799999
  )
    return fail();
  const items = report["channels"];
  if (!Array.isArray(items) || items.length !== samples.length) return fail();
  const channels = items.map((item: unknown, index): SampleOutcome => {
    if (typeof item !== "object" || item === null) return fail();
    const status = (item as Record<string, unknown>)["status"];
    const entry = object(
      item,
      status === "not_checked"
        ? ["position", "status", "reason"]
        : ["position", "status"],
    );
    if (
      entry["position"] !== index + 1 ||
      samples[index]?.position !== index + 1
    )
      return fail();
    if (status === "no_saved_reference") {
      if (
        samples[index]?.status !== "unavailable" ||
        samples[index]?.value !== null
      )
        return fail();
      return { position: index + 1, status };
    }
    if (samples[index]?.status !== "extracted") return fail();
    if (status === "present" || status === "not_found")
      return { position: index + 1, status };
    if (status === "not_checked")
      return { position: index + 1, status, reason: reason(entry["reason"]) };
    return fail();
  });
  return { state: "complete", report: { checkedAt: timestamp, channels } };
}

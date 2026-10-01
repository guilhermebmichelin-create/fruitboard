import { LibraryAdapterError } from "./contracts";

const STATES = [
  "not_reported",
  "not_current",
  "queued",
  "running",
  "complete",
  "unsupported",
  "failed",
  "cancelled",
  "stale",
] as const;
const REASONS = [
  "interrupted",
  "source_unavailable",
  "source_changed",
  "parser_unavailable",
  "invalid_result",
  "cancelled",
  "unsupported_version",
  "resource_limit",
  "invalid_file",
  "unclassified",
] as const;
export interface ProjectAnalysis {
  readonly state: (typeof STATES)[number];
  readonly attempts: number | null;
  readonly reason: (typeof REASONS)[number] | null;
}
export const NO_REPORTED_ANALYSIS: ProjectAnalysis = {
  state: "not_reported",
  attempts: null,
  reason: null,
};

/** Only fixed categories cross the display boundary; never raw parser text. */
export function parseProjectAnalysis(value: unknown): ProjectAnalysis {
  const fail = (): never => {
    throw new LibraryAdapterError("internal");
  };
  if (typeof value !== "object" || value === null || Array.isArray(value))
    return fail();
  const entry = value as Record<string, unknown>;
  const state = entry["state"];
  const attempts = entry["attempts"];
  const reason = entry["reason"];
  if (
    !STATES.includes(state as ProjectAnalysis["state"]) ||
    (reason !== null &&
      !REASONS.includes(reason as NonNullable<ProjectAnalysis["reason"]>))
  )
    return fail();
  if (state === "not_reported" || state === "not_current") {
    if (attempts !== null || reason !== null) return fail();
  } else if (
    typeof attempts !== "number" ||
    !Number.isInteger(attempts) ||
    attempts < 0 ||
    attempts > 3 ||
    (["running", "complete", "unsupported"].includes(String(state)) &&
      attempts === 0)
  )
    return fail();
  return {
    state: state as ProjectAnalysis["state"],
    attempts,
    reason: reason as ProjectAnalysis["reason"],
  };
}

export const ANALYSIS_LABELS: Readonly<
  Record<ProjectAnalysis["state"], string>
> = {
  not_reported: "No analysis attempt reported",
  not_current: "No status for this scanned file",
  queued: "Waiting for analysis",
  running: "Reading project",
  complete: "Analysis finished",
  unsupported: "Saved build unsupported",
  failed: "Analysis failed",
  cancelled: "Analysis cancelled",
  stale: "Source changed",
};
export const ANALYSIS_NOTES: Readonly<
  Record<ProjectAnalysis["state"], string>
> = {
  not_reported:
    "No matching attempt was recorded for this file. Saved facts may have been read by an older version of Fruitboard or through another file location.",
  not_current:
    "The file is missing, changed, or no longer available through this folder. Refresh the Library before checking again.",
  queued:
    "An attempt is waiting. This saved state does not confirm that analysis is currently active.",
  running:
    "An attempt was recorded as reading this file. This saved state does not confirm that it is still running.",
  complete:
    "The recorded attempt finished. Only supported fields can be shown.",
  unsupported:
    "The saved FL Studio build is outside the parser's verified support. No new project facts were published by this attempt.",
  failed: "The recorded attempt could not produce new project facts.",
  cancelled:
    "The recorded attempt was cancelled before publishing new project facts.",
  stale:
    "The file changed during analysis. That attempt cannot provide current project facts.",
};
export const ANALYSIS_REASONS: Readonly<
  Record<NonNullable<ProjectAnalysis["reason"]>, string>
> = {
  interrupted:
    "Analysis was interrupted. Its started attempts still count toward the limit.",
  source_unavailable:
    "The file could not be read through its approved local folder.",
  source_changed: "The file changed while it was being read.",
  parser_unavailable: "The project reader could not complete its response.",
  invalid_result:
    "The project reader returned a result that could not be accepted safely.",
  cancelled: "Cancellation was recorded for this attempt.",
  unsupported_version:
    "This saved FL Studio build has not been verified for project analysis.",
  resource_limit: "The project exceeds a supported analysis limit.",
  invalid_file: "The saved project data could not be understood safely.",
  unclassified: "A problem was recorded without a supported explanation.",
};

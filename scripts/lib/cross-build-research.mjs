// Research result comparison only. No parser policy, path access or app wiring.
import { isDeepStrictEqual } from "node:util";

const statuses = new Set([
  "extracted",
  "inferred",
  "unavailable",
  "unsupported",
  "failed",
]);

export function readResearchReply(text, id) {
  const reply = JSON.parse(text);
  if (
    reply?.protocolVersion !== 1 ||
    reply?.schemaVersion !== 2 ||
    reply?.id !== id ||
    Object.hasOwn(reply, "error") ||
    !reply.result ||
    typeof reply.result !== "object" ||
    Array.isArray(reply.result) ||
    !["complete", "partial", "unsupported", "failed"].includes(
      reply.result.outcome,
    )
  ) {
    throw new Error("RESEARCH_INVALID_REPLY");
  }
  return reply.result;
}

export function compareResearchFields(expected, result) {
  return Object.entries(expected).map(([field, oracle]) => {
    const actual = result[field];
    let classification = "failed";
    if (actual && statuses.has(actual.status)) {
      if (actual.status === "unsupported") {
        classification =
          actual.reason === "UNSUPPORTED_SAVED_VERSION"
            ? "policy_blocked"
            : "unsupported";
      } else if (actual.status === "failed") {
        classification = "failed";
      } else {
        const matches =
          Object.entries(oracle).every(([key, value]) =>
            isDeepStrictEqual(actual[key], value),
          ) &&
          (Object.hasOwn(oracle, "value") || !Object.hasOwn(actual, "value"));
        classification = matches ? `matched_${oracle.status}` : "mismatched";
      }
    }
    // Arbitrary values/reasons returned by a process may contain private data.
    // Report known expectations and an allowlisted classification, never echo it.
    return { field, expected: oracle, classification };
  });
}

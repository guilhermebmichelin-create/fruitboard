// Fixed research observations only; never a product metadata envelope.
import { isDeepStrictEqual } from "node:util";
import { compareResearchFields } from "./cross-build-research.mjs";

const widths = [1, 3, 4];
const outcomes = new Set(["complete", "partial", "unsupported", "failed"]);
const record = (value) =>
  value !== null && typeof value === "object" && !Array.isArray(value);
const integer = (value, min, max) =>
  Number.isSafeInteger(value) && value >= min && value <= max;

function invalid() {
  throw new Error("RESEARCH_INVALID_PROBE");
}

function checkFraming(observation, bytes) {
  const { observedEvents, framingPrefix, framingPrefixTruncated } = observation;
  if (
    !integer(observedEvents, 0, 100000) ||
    !Array.isArray(framingPrefix) ||
    framingPrefix.length !== Math.min(observedEvents, 32) ||
    framingPrefixTruncated !== observedEvents > 32
  )
    invalid();
  let nextOffset = 22;
  for (const event of framingPrefix) {
    if (
      !record(event) ||
      !integer(event.id, 0, 255) ||
      event.eventOffset !== nextOffset ||
      !integer(event.payloadOffset, nextOffset + 1, nextOffset + 6) ||
      !integer(event.payloadBytes, 0, bytes - event.payloadOffset)
    )
      invalid();
    nextOffset = event.payloadOffset + event.payloadBytes;
  }
}

export function compareFixedProbe(text, specification) {
  if (typeof text !== "string" || Buffer.byteLength(text) > 256 * 1024)
    invalid();
  const reply = JSON.parse(text);
  if (
    !record(reply) ||
    reply.researchOnly !== true ||
    reply.qualified !== false ||
    reply.sourceHashesUnchanged !== true ||
    !Array.isArray(reply.controls) ||
    reply.controls.length !== specification.controls.length
  )
    invalid();

  const controls = specification.controls.map((control, index) => {
    const actual = reply.controls[index];
    if (
      !record(actual) ||
      actual.id !== control.id ||
      actual.sha256 !== control.sha256 ||
      !Array.isArray(actual.observations) ||
      actual.observations.length !== widths.length
    )
      invalid();
    const hypotheses = actual.observations.map((observation, i) => {
      if (
        !record(observation) ||
        observation.researchOnly !== true ||
        observation.qualified !== false ||
        !record(observation.profile) ||
        observation.profile.exactSavedBuild !==
          control.fields.savedVersion.value ||
        observation.profile.event172Bytes !== widths[i] ||
        !outcomes.has(observation.decoderOutcome) ||
        !record(observation.fieldObservations)
      )
        invalid();
      checkFraming(observation, control.bytes);
      const fields = compareResearchFields(
        control.fields,
        observation.fieldObservations,
      );
      return {
        event172Bytes: widths[i],
        decoderOutcome: observation.decoderOutcome,
        fieldsMatch:
          observation.decoderOutcome === "complete" &&
          fields.every((field) => field.classification.startsWith("matched_")),
        fields,
        observedEvents: observation.observedEvents,
        prefixEvents: observation.framingPrefix.length,
        prefixTruncated: observation.framingPrefixTruncated,
      };
    });
    const matchingPairs = [];
    for (let a = 0; a < widths.length; a += 1) {
      for (let b = a + 1; b < widths.length; b += 1) {
        if (!hypotheses[a].fieldsMatch || !hypotheses[b].fieldsMatch) continue;
        const left = actual.observations[a];
        const right = actual.observations[b];
        // Compare only captured evidence. Identical prefixes do not establish
        // identical full-file framing, and no successful pair picks a winner.
        const framing = (observation) =>
          observation.framingPrefix.map((event) => [
            event.id,
            event.eventOffset,
            event.payloadOffset,
            event.payloadBytes,
          ]);
        matchingPairs.push({
          event172Bytes: [widths[a], widths[b]],
          framingEvidence:
            left.observedEvents !== right.observedEvents ||
            !isDeepStrictEqual(framing(left), framing(right))
              ? "different"
              : "indistinguishable_in_captured_evidence",
        });
      }
    }
    return {
      id: control.id,
      sha256: control.sha256,
      hypotheses,
      matchingPairs,
      selectedWidth: null,
    };
  });
  return {
    researchOnly: true,
    qualified: false,
    compatibilityExpanded: false,
    controls,
    additionalBuilds: [
      { role: "adjacent", state: "not_tested", exactBuild: null },
      { role: "format_transition", state: "not_tested", exactBuild: null },
    ],
  };
}

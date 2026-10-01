import { PROJECT_FACT_KEYS, type ProjectDetails } from "./projectDetails";

/** Synthetic review/test data; no compatibility or real-file evidence. */
export const identity = { rootId: "root-test", locationId: "location-test" };
export function savedDetails(): Extract<
  ProjectDetails,
  { state: "available" }
> {
  const values = [
    "26.1.0.5530",
    "120",
    "3",
    "2026-01-02T03:04:05.123",
    "1767312000000",
    "3661000",
    "768",
    "2",
    "4",
  ];
  return {
    ...identity,
    state: "available",
    snapshotId: "12345678-1234-1234-1234-123456789abc",
    outcome: "partial",
    facts: PROJECT_FACT_KEYS.map((key, index) => ({
      key,
      status: index >= 7 ? "inferred" : "extracted",
      value: values[index] ?? "0",
      explanation:
        index === 7
          ? "Low confidence. Pattern clip span calculated using the verified meter; excludes unverified clip kinds."
          : index === 8
            ? "Low confidence. Assumes tempo remains at base BPM and only verified pattern clips define span; excludes tempo automation and unverified clip kinds."
            : null,
    })),
  };
}

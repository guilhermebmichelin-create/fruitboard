import { LibraryAdapterError } from "./contracts";

export type ProjectSampleReference =
  | {
      readonly position: number;
      readonly status: "extracted";
      readonly value: string;
    }
  | {
      readonly position: number;
      readonly status: "unavailable";
      readonly value: null;
    };

export function parseProjectSamples(
  value: unknown,
  channelCount: number,
): readonly ProjectSampleReference[] {
  const fail = (): never => {
    throw new LibraryAdapterError("internal");
  };
  if (
    !Array.isArray(value) ||
    value.length > 256 ||
    value.length !== channelCount
  )
    return fail();
  return value.map((entry: unknown, index) => {
    if (typeof entry !== "object" || entry === null || Array.isArray(entry))
      return fail();
    const item = entry as Record<string, unknown>;
    if (item["position"] !== index + 1) return fail();
    if (item["status"] === "extracted") {
      const text = item["value"];
      if (
        typeof text !== "string" ||
        text.length === 0 ||
        text.length > 4095 ||
        text.includes("\0")
      )
        return fail();
      return { position: index + 1, status: "extracted", value: text };
    }
    if (item["status"] !== "unavailable" || item["value"] !== null)
      return fail();
    return { position: index + 1, status: "unavailable", value: null };
  });
}

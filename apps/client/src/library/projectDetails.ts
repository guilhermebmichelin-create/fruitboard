import { LibraryAdapterError, type PublishedFileLocation } from "./contracts";
import { parseProjectAnalysis, type ProjectAnalysis } from "./projectAnalysis";

export const PROJECT_FACT_KEYS = [
  "savedVersion",
  "baseTempoBpm",
  "channelCount",
  "projectCreatedLocal",
  "filesystemCreatedAtMs",
  "flStudioTimeSpentMs",
  "playlistPatternEndTick",
  "playlistPatternSpanBars",
  "playlistPatternNominalSeconds",
] as const;
export type ProjectFactKey = (typeof PROJECT_FACT_KEYS)[number];
export interface ProjectFact {
  readonly key: ProjectFactKey;
  readonly status: "extracted" | "inferred" | "unavailable" | "unsupported";
  /** Bounded scalar display data; large integers remain decimal strings. */
  readonly value: string | null;
  readonly explanation: string | null;
}
export interface ChannelDetail {
  readonly status: "extracted" | "inferred" | "unavailable" | "unsupported";
  readonly value: string | null;
  readonly explanation: string | null;
}
export interface ProjectChannel {
  /** One-based parser order, not FL Studio's saved numeric channel ID. */
  readonly position: number;
  readonly name: ChannelDetail;
  readonly instrument: ChannelDetail;
}
export type ProjectDetails = {
  readonly rootId: string;
  readonly locationId: string;
} & (
  | { readonly state: "disabled" }
  | { readonly state: "no_current"; readonly analysis?: ProjectAnalysis }
  | {
      readonly state: "available";
      readonly snapshotId: string;
      readonly outcome: "complete" | "partial";
      readonly facts: readonly ProjectFact[];
      readonly channels: readonly ProjectChannel[];
      readonly analysis?: ProjectAnalysis;
      readonly warnings?: readonly "unverified_events"[];
    }
);

const record = (v: unknown): v is Record<string, unknown> =>
  typeof v === "object" && v !== null && !Array.isArray(v);
const fail = (): never => {
  throw new LibraryAdapterError("internal");
};

/** Allowlisted display projection; never returns raw JSON or extra properties. */
export function parseProjectDetails(
  value: unknown,
  file: Pick<PublishedFileLocation, "rootId" | "locationId">,
): ProjectDetails {
  if (
    !record(value) ||
    value["rootId"] !== file.rootId ||
    value["locationId"] !== file.locationId
  )
    return fail();
  const identity = { rootId: file.rootId, locationId: file.locationId };
  const state = value["state"];
  if (state === "disabled" || state === "no_current") {
    if (
      value["facts"] !== undefined ||
      value["snapshotId"] !== undefined ||
      value["channels"] !== undefined ||
      value["warnings"] !== undefined ||
      (state === "disabled" && value["analysis"] !== undefined)
    )
      return fail();
    return state === "no_current" && value["analysis"] !== undefined
      ? {
          ...identity,
          state,
          analysis: parseProjectAnalysis(value["analysis"]),
        }
      : { ...identity, state };
  }
  if (
    state !== "available" ||
    typeof value["snapshotId"] !== "string" ||
    !/^[0-9a-f-]{36}$/.test(value["snapshotId"]) ||
    !["complete", "partial"].includes(String(value["outcome"])) ||
    !Array.isArray(value["facts"]) ||
    value["facts"].length !== PROJECT_FACT_KEYS.length
  )
    return fail();
  const facts = value["facts"].map(
    (entry: unknown, index: number): ProjectFact => {
      if (!record(entry)) return fail();
      const key = PROJECT_FACT_KEYS[index];
      if (key === undefined || entry["key"] !== key) return fail();
      const status = entry["status"];
      const text = entry["value"];
      const explanation = entry["explanation"];
      if (
        explanation !== null &&
        (typeof explanation !== "string" ||
          explanation.length === 0 ||
          explanation.length > 320 ||
          [...explanation].some(
            (char) => char.charCodeAt(0) < 32 || char.charCodeAt(0) === 127,
          ))
      )
        return fail();
      if (status === "unavailable" || status === "unsupported") {
        if (
          text !== null ||
          explanation === null ||
          key === "savedVersion" ||
          key === "channelCount"
        )
          return fail();
        return { key, status, value: null, explanation };
      }
      if (
        (status !== "extracted" && status !== "inferred") ||
        typeof text !== "string" ||
        text.length > 64 ||
        text.length === 0
      )
        return fail();
      const estimate =
        key === "playlistPatternSpanBars" ||
        key === "playlistPatternNominalSeconds";
      if (
        (status === "inferred") !== estimate ||
        (status === "inferred" ? explanation === null : explanation !== null)
      )
        return fail();
      if (key === "savedVersion") {
        if (!/^\d+\.\d+\.\d+\.\d+$/.test(text)) return fail();
      } else if (key === "projectCreatedLocal") {
        if (
          !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}$/.test(text) ||
          !validLocalDate(text)
        )
          return fail();
      } else if (
        [
          "filesystemCreatedAtMs",
          "flStudioTimeSpentMs",
          "playlistPatternEndTick",
          "channelCount",
        ].includes(key)
      ) {
        if (!/^(0|[1-9]\d*)$/.test(text)) return fail();
        const maximum =
          key === "channelCount"
            ? 256n
            : key === "playlistPatternEndTick"
              ? 4294967295n
              : key === "flStudioTimeSpentMs"
                ? 255611462399999n
                : 18446744073709551615n;
        if (BigInt(text) > maximum) return fail();
      } else {
        if (!/^(0|[1-9]\d*)(\.\d+)?(e[+-]?\d+)?$/.test(text)) return fail();
        const number = Number(text);
        const maximum =
          key === "baseTempoBpm"
            ? 999
            : key === "playlistPatternSpanBars"
              ? 4294967295
              : 4294967295 * 60;
        if (
          !Number.isFinite(number) ||
          number < (key === "baseTempoBpm" ? 1 : 0) ||
          number > maximum
        )
          return fail();
      }
      return {
        key,
        status,
        value: text,
        explanation,
      };
    },
  );
  const channels = value["channels"];
  const count = Number(
    facts.find((fact) => fact.key === "channelCount")?.value,
  );
  const build = facts.find((fact) => fact.key === "savedVersion")?.value;
  if (!Array.isArray(channels) || channels.length !== count) return fail();
  let inferredNames = 0;
  const selectedChannels = channels.map(
    (entry: unknown, index: number): ProjectChannel => {
      if (!record(entry) || entry["position"] !== index + 1) return fail();
      const name = parseChannelDetail(entry["name"], false);
      const instrument = parseChannelDetail(entry["instrument"], true);
      if (name.status === "inferred") {
        inferredNames++;
        const expected =
          inferredNames === 1 ? "Sampler" : `Sampler ${inferredNames}`;
        if (
          !["25.1.3.4922", "26.1.0.5530"].includes(build ?? "") ||
          name.value !== expected
        )
          return fail();
      }
      if (instrument.status !== "unsupported" && build !== "26.1.0.5530")
        return fail();
      return { position: index + 1, name, instrument };
    },
  );
  const warnings = value["warnings"];
  if (
    warnings !== undefined &&
    (!Array.isArray(warnings) ||
      warnings.length > 1 ||
      warnings.some((v: unknown) => v !== "unverified_events"))
  )
    return fail();
  return {
    ...identity,
    state,
    snapshotId: value["snapshotId"],
    outcome: value["outcome"] as "complete" | "partial",
    facts,
    channels: selectedChannels,
    ...(value["analysis"] === undefined
      ? {}
      : { analysis: parseProjectAnalysis(value["analysis"]) }),
    ...(warnings === undefined
      ? {}
      : { warnings: warnings as "unverified_events"[] }),
  };
}

function parseChannelDetail(
  value: unknown,
  instrument: boolean,
): ChannelDetail {
  if (!record(value)) return fail();
  const status = value["status"];
  const text = value["value"];
  const explanation = value["explanation"];
  if (
    explanation !== null &&
    (typeof explanation !== "string" ||
      explanation.length === 0 ||
      explanation.length > 320 ||
      [...explanation].some(displayControl))
  )
    return fail();
  if (status === "unavailable" || status === "unsupported") {
    if (
      (instrument ? status !== "unsupported" : status !== "unavailable") ||
      text !== null ||
      explanation === null
    )
      return fail();
    return { status, value: null, explanation };
  }
  if (
    (status !== "extracted" && status !== "inferred") ||
    typeof text !== "string" ||
    text.length > 4095 ||
    new TextEncoder().encode(text).length > 12285 ||
    text.includes("\0") ||
    (status === "inferred" ? explanation === null : explanation !== null)
  )
    return fail();
  if (
    instrument &&
    (status === "extracted" ? text !== "3x Osc" : text !== "Sampler")
  )
    return fail();
  return { status, value: text, explanation };
}

/** Keep control/bidi characters visible rather than letting them alter labels. */
export function channelLabel(value: string): string {
  if (value.length === 0) return "Empty saved label";
  return [...value]
    .map((char) =>
      displayControl(char)
        ? `\\u${char.charCodeAt(0).toString(16).padStart(4, "0")}`
        : char,
    )
    .join("");
}

function displayControl(char: string): boolean {
  const code = char.charCodeAt(0);
  return (
    code < 32 ||
    (code >= 127 && code <= 159) ||
    code === 1564 ||
    code === 8206 ||
    code === 8207 ||
    (code >= 8232 && code <= 8238) ||
    (code >= 8294 && code <= 8297)
  );
}

function validLocalDate(value: string): boolean {
  const date = new Date(`${value}Z`);
  return (
    Number(value.slice(0, 4)) >= 1900 &&
    Number.isFinite(date.valueOf()) &&
    date.toISOString().slice(0, 23) === value
  );
}

export const FACT_LABELS: Readonly<Record<ProjectFactKey, string>> = {
  savedVersion: "Saved FL Studio version",
  baseTempoBpm: "Base tempo",
  channelCount: "Channels",
  projectCreatedLocal: "Project created",
  filesystemCreatedAtMs: "File created",
  flStudioTimeSpentMs: "FL Studio saved time counter",
  playlistPatternEndTick: "Pattern clip endpoint",
  playlistPatternSpanBars: "Pattern clip span",
  playlistPatternNominalSeconds: "Pattern clip time estimate",
};
export const FACT_NOTES: Partial<Readonly<Record<ProjectFactKey, string>>> = {
  baseTempoBpm: "Saved base tempo; tempo automation may change playback.",
  projectCreatedLocal: "Saved local date and time; timezone unspecified.",
  filesystemCreatedAtMs:
    "Filesystem creation time; copying a file can change it.",
  flStudioTimeSpentMs:
    "FL Studio’s saved counter; Fruitboard has not measured your work time.",
  playlistPatternEndTick: "Latest verified pattern clip endpoint, in ticks.",
  playlistPatternSpanBars:
    "Verified pattern clips only; excludes unverified clip kinds.",
  playlistPatternNominalSeconds:
    "An estimate for verified pattern clips, not the finished song’s length.",
};

export function formatProjectFact(fact: ProjectFact): string {
  if (fact.value === null)
    return fact.status === "unsupported" ? "Unsupported" : "Unavailable";
  const text = fact.value;
  switch (fact.key) {
    case "baseTempoBpm":
      return `${text} BPM`;
    case "projectCreatedLocal":
      return `${text.replace("T", " ")} (local)`;
    case "filesystemCreatedAtMs": {
      if (BigInt(text) > 8640000000000000n)
        return "Outside the supported date range";
      return `${new Date(Number(text)).toISOString().replace("T", " ").replace(".000Z", " UTC").replace(/Z$/, " UTC")}`;
    }
    case "flStudioTimeSpentMs": {
      const ms = BigInt(text);
      return `${ms / 3600000n} h ${(ms / 60000n) % 60n} min ${(ms / 1000n) % 60n} s`;
    }
    case "playlistPatternEndTick":
      return `${text} ticks`;
    case "playlistPatternSpanBars":
      return `${text} bars`;
    case "playlistPatternNominalSeconds":
      return `${text} seconds`;
    default:
      return text;
  }
}

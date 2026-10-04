import { LibraryAdapterError, type PublishedFileLocation } from "./contracts";
import {
  parseProjectPlugins,
  type ProjectPluginReference,
} from "./pluginReferences";

export type PluginIdentity = Pick<
  ProjectPluginReference,
  "name" | "className" | "vendor"
>;
export type ExplorerMetadataState =
  "current" | "missing" | "stale" | "no_analysis" | "older" | "unavailable";
export interface PluginExplorerEntry extends PublishedFileLocation {
  readonly metadataState: ExplorerMetadataState;
  readonly unnamedReferenceCount: number;
}
export interface PluginExplorerGroup {
  readonly plugin: PluginIdentity;
  readonly referenceCount: number;
  readonly matches: readonly {
    readonly locationId: string;
    readonly referenceCount: number;
  }[];
}
export type PluginExplorerResult = { readonly rootId: string } & (
  | { readonly state: "disabled" | "unavailable" | "limited" }
  | {
      readonly state: "ready";
      readonly entries: readonly PluginExplorerEntry[];
      readonly groups: readonly PluginExplorerGroup[];
      readonly referenceCount: number;
      readonly unnamedReferenceCount: number;
    }
);

const record = (v: unknown): v is Record<string, unknown> =>
  typeof v === "object" && v !== null && !Array.isArray(v);
const fail = (): never => {
  throw new LibraryAdapterError("internal");
};
const count = (v: unknown, max: number): number =>
  typeof v === "number" && Number.isInteger(v) && v >= 0 && v <= max
    ? v
    : fail();
const metadataStates = new Set<unknown>([
  "current",
  "missing",
  "stale",
  "no_analysis",
  "older",
  "unavailable",
]);

export function parsePluginExplorer(
  value: unknown,
  rootId: string,
  parseLocation: (value: unknown) => PublishedFileLocation,
): PluginExplorerResult {
  if (
    !record(value) ||
    value["rootId"] !== rootId ||
    rootId.length === 0 ||
    rootId.length > 128 ||
    rootId.includes("\0")
  )
    return fail();
  if (new TextEncoder().encode(JSON.stringify(value)).length > 2 * 1024 * 1024)
    return fail();
  const state = value["state"];
  if (state === "disabled" || state === "unavailable" || state === "limited") {
    if (
      ["entries", "groups", "referenceCount", "unnamedReferenceCount"].some(
        (key) => value[key] !== undefined,
      )
    )
      return fail();
    return { rootId, state };
  }
  const rawEntries = value["entries"],
    rawGroups = value["groups"];
  if (
    state !== "ready" ||
    !Array.isArray(rawEntries) ||
    rawEntries.length > 2000 ||
    !Array.isArray(rawGroups) ||
    rawGroups.length > 1024
  )
    return fail();
  const entryById = new Map<string, PluginExplorerEntry>();
  const referencesById = new Map<string, number>();
  const entries = rawEntries.map((raw: unknown): PluginExplorerEntry => {
    if (!record(raw) || !metadataStates.has(raw["metadataState"]))
      return fail();
    const location = parseLocation(raw);
    const metadataState = raw["metadataState"] as ExplorerMetadataState;
    const unnamedReferenceCount = count(raw["unnamedReferenceCount"], 1024);
    if (
      location.rootId !== rootId ||
      entryById.has(location.locationId) ||
      (metadataState === "missing") !== (location.presence === "missing") ||
      (metadataState !== "current" && unnamedReferenceCount !== 0)
    )
      return fail();
    const entry = { ...location, metadataState, unnamedReferenceCount };
    entryById.set(entry.locationId, entry);
    referencesById.set(entry.locationId, unnamedReferenceCount);
    return entry;
  });
  const identities = new Set<string>();
  const groups = rawGroups.map((raw: unknown): PluginExplorerGroup => {
    if (
      !record(raw) ||
      !record(raw["plugin"]) ||
      !Array.isArray(raw["matches"]) ||
      raw["matches"].length === 0 ||
      raw["matches"].length > entries.length
    )
      return fail();
    const rawPlugin = raw["plugin"];
    // Native validation checks each saved build. This shared parser checks the
    // aggregate's exact allowlisted field/inference shapes and relationships.
    const item = parseProjectPlugins(
      {
        coverage: "top-level-saved-references",
        items: [{ ...rawPlugin, position: 1 }],
      },
      "26.1.0.5530",
    ).items[0];
    if (!item || item.name.value === null) return fail();
    const plugin = {
      name: item.name,
      className: item.className,
      vendor: item.vendor,
    };
    const identity = JSON.stringify(plugin);
    if (identities.has(identity)) return fail();
    identities.add(identity);
    const locations = new Set<string>();
    const matches = raw["matches"].map((match: unknown) => {
      if (!record(match) || typeof match["locationId"] !== "string")
        return fail();
      const locationId = match["locationId"];
      if (
        entryById.get(locationId)?.metadataState !== "current" ||
        locations.has(locationId)
      )
        return fail();
      locations.add(locationId);
      const referenceCount = count(match["referenceCount"], 1024);
      if (referenceCount === 0) return fail();
      const sum = (referencesById.get(locationId) ?? 0) + referenceCount;
      if (sum > 1024) return fail();
      referencesById.set(locationId, sum);
      return { locationId, referenceCount };
    });
    const referenceCount = count(raw["referenceCount"], 8192);
    if (
      referenceCount !==
      matches.reduce((sum, match) => sum + match.referenceCount, 0)
    )
      return fail();
    return { plugin, matches, referenceCount };
  });
  const referenceCount = count(value["referenceCount"], 8192);
  const unnamedReferenceCount = count(value["unnamedReferenceCount"], 8192);
  if (
    unnamedReferenceCount !==
      entries.reduce((sum, entry) => sum + entry.unnamedReferenceCount, 0) ||
    referenceCount !==
      unnamedReferenceCount +
        groups.reduce((sum, group) => sum + group.referenceCount, 0)
  )
    return fail();
  return {
    rootId,
    state,
    entries,
    groups,
    referenceCount,
    unnamedReferenceCount,
  };
}

export const EXPLORER_GAP_COPY: Readonly<
  Record<ExplorerMetadataState, string>
> = {
  current: "Current saved references",
  missing: "File is missing",
  stale: "No current result; saved history or the source is outdated",
  no_analysis: "No current saved plugin analysis",
  older: "Older result did not report plugin references",
  unavailable: "Saved plugin details could not be validated",
};

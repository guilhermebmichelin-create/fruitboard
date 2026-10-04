import type {
  PluginExplorerEntry,
  PluginExplorerResult,
} from "./pluginExplorer";
import { builtinPlugin, savedPlugins } from "./pluginReferences.fixture";

/** Synthetic display data only; not saved-build compatibility evidence. */
export function explorerFixture(
  rootId = "root-test",
): Extract<PluginExplorerResult, { state: "ready" }> {
  const entry = (
    locationId: string,
    relativePath: string,
  ): PluginExplorerEntry => ({
    rootId,
    locationId,
    rootDisplayName: "Projects",
    rootCanonicalPath: "C:\\Synthetic\\Projects",
    fileName: relativePath.split("/").at(-1) ?? relativePath,
    relativePath,
    byteSize: "1024",
    modifiedAt: "2026-01-02T00:00:00Z",
    presence: "present",
    metadataState: "current",
    unnamedReferenceCount: 0,
  });
  const identity = (item: ReturnType<typeof builtinPlugin>) => ({
    name: item.name,
    className: item.className,
    vendor: item.vendor,
  });
  const builtin = identity(builtinPlugin());
  const sampler = identity(savedPlugins().items[1]!);
  const wrapper = identity(savedPlugins().items[2]!);
  return {
    rootId,
    state: "ready",
    referenceCount: 8,
    unnamedReferenceCount: 1,
    entries: [
      { ...entry("entry-a", "Beats/First.flp"), unnamedReferenceCount: 1 },
      entry("entry-b", "Copies/First.flp"),
      {
        ...entry("entry-missing", "Missing.flp"),
        presence: "missing",
        metadataState: "missing",
      },
      { ...entry("entry-older", "Older.flp"), metadataState: "older" },
      {
        ...entry("entry-pending", "Pending.flp"),
        metadataState: "no_analysis",
      },
    ],
    groups: [
      {
        plugin: builtin,
        referenceCount: 3,
        matches: [
          { locationId: "entry-a", referenceCount: 2 },
          { locationId: "entry-b", referenceCount: 1 },
        ],
      },
      {
        plugin: identity(builtinPlugin(1, "3X Osc")),
        referenceCount: 1,
        matches: [{ locationId: "entry-b", referenceCount: 1 }],
      },
      {
        plugin: sampler,
        referenceCount: 1,
        matches: [{ locationId: "entry-a", referenceCount: 1 }],
      },
      {
        plugin: wrapper,
        referenceCount: 1,
        matches: [{ locationId: "entry-a", referenceCount: 1 }],
      },
      {
        plugin: {
          ...wrapper,
          vendor: { status: "extracted", value: "Other Vendor" },
        },
        referenceCount: 1,
        matches: [{ locationId: "entry-b", referenceCount: 1 }],
      },
    ],
  };
}

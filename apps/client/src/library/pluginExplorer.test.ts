import { describe, expect, it, vi } from "vitest";
import { LibraryAdapterError } from "./contracts";
import { createNativeLibraryScanAdapter, parseRecord } from "./native";
import { parsePluginExplorer } from "./pluginExplorer";
import { explorerFixture } from "./pluginExplorer.fixture";

describe("root-scoped Plugin Explorer contract", () => {
  it("preserves exact spellings, provenance, distinct entries and duplicate reference counts", () => {
    const fixture = explorerFixture();
    expect(parsePluginExplorer(fixture, fixture.rootId, parseRecord)).toEqual(
      fixture,
    );
    expect(fixture.groups[0]?.matches).toHaveLength(2);
    expect(fixture.groups[0]?.referenceCount).toBe(3);
    expect(fixture.groups[2]?.plugin.name.status).toBe("inferred");
  });

  it("projects only allowlisted fields and keeps saved text inert", () => {
    const fixture = explorerFixture();
    const raw = {
      ...fixture,
      rawReply: "private extension",
      entries: fixture.entries.map((entry) => ({
        ...entry,
        hidden: "private extension",
      })),
      groups: fixture.groups.map((group) => ({
        ...group,
        hidden: "private extension",
      })),
    };
    const result = parsePluginExplorer(raw, fixture.rootId, parseRecord);
    expect(JSON.stringify(result)).not.toContain("private extension");
  });

  it.each(["disabled", "unavailable", "limited"] as const)(
    "accepts %s only without partial aggregate claims",
    (state) => {
      expect(
        parsePluginExplorer(
          { rootId: "root-test", state },
          "root-test",
          parseRecord,
        ),
      ).toEqual({ rootId: "root-test", state });
      expect(() =>
        parsePluginExplorer(
          { ...explorerFixture(), state },
          "root-test",
          parseRecord,
        ),
      ).toThrow(LibraryAdapterError);
    },
  );

  it("rejects mixed roots, dangling or duplicated matches, forged provenance and inconsistent totals", () => {
    const fixture = explorerFixture();
    const group = fixture.groups[0]!;
    const entry = fixture.entries[0]!;
    const invalid: unknown[] = [
      { ...fixture, rootId: "another" },
      {
        ...fixture,
        entries: [{ ...entry, rootId: "another" }, ...fixture.entries.slice(1)],
      },
      { ...fixture, entries: [...fixture.entries, entry] },
      { ...fixture, referenceCount: 9 },
      { ...fixture, unnamedReferenceCount: 0 },
      {
        ...fixture,
        groups: [{ ...group, referenceCount: 99 }, ...fixture.groups.slice(1)],
      },
      {
        ...fixture,
        groups: [
          { ...group, matches: [...group.matches, group.matches[0]!] },
          ...fixture.groups.slice(1),
        ],
      },
      {
        ...fixture,
        groups: [
          { ...group, matches: [{ locationId: "unknown", referenceCount: 3 }] },
          ...fixture.groups.slice(1),
        ],
      },
      {
        ...fixture,
        groups: [
          {
            ...group,
            matches: [{ locationId: "entry-missing", referenceCount: 3 }],
          },
          ...fixture.groups.slice(1),
        ],
      },
      { ...fixture, groups: [...fixture.groups, group] },
      {
        ...fixture,
        entries: [
          { ...entry, unnamedReferenceCount: 1025 },
          ...fixture.entries.slice(1),
        ],
      },
      {
        ...fixture,
        groups: [
          {
            ...group,
            plugin: {
              ...group.plugin,
              name: {
                status: "inferred",
                value: "guessed",
                method: "guess",
                confidence: "high",
              },
            },
          },
          ...fixture.groups.slice(1),
        ],
      },
      {
        ...fixture,
        groups: [
          {
            ...group,
            plugin: {
              ...group.plugin,
              vendor: { status: "extracted", value: "invented" },
            },
          },
          ...fixture.groups.slice(1),
        ],
      },
      {
        ...fixture,
        groups: [
          {
            ...group,
            plugin: {
              ...group.plugin,
              name: { status: "extracted", value: "different" },
            },
          },
          ...fixture.groups.slice(1),
        ],
      },
      { ...fixture, hidden: "x".repeat(2 * 1024 * 1024) },
    ];
    for (const value of invalid)
      expect(() =>
        parsePluginExplorer(value, "root-test", parseRecord),
      ).toThrow(LibraryAdapterError);
  });

  it("invokes only the opaque root read command and rejects wrong-root responses", async () => {
    const invoke = vi.fn(() =>
      Promise.resolve({
        schemaVersion: 1,
        correlationId: "correlation_00000000000000000000000000000001",
        status: "ok",
        data: explorerFixture(),
      }),
    );
    const adapter = createNativeLibraryScanAdapter({
      invoke,
      listen: () => () => undefined,
    });
    await expect(adapter.getPluginExplorer!("root-test")).resolves.toEqual(
      explorerFixture(),
    );
    expect(invoke).toHaveBeenCalledWith("get_plugin_explorer", {
      request: { schemaVersion: 1, rootId: "root-test" },
    });
    await expect(adapter.getPluginExplorer!("other")).rejects.toBeInstanceOf(
      LibraryAdapterError,
    );
    await expect(adapter.getPluginExplorer!("root\0")).rejects.toBeInstanceOf(
      LibraryAdapterError,
    );
    expect(JSON.stringify(invoke.mock.calls)).not.toContain("Synthetic");
  });
});

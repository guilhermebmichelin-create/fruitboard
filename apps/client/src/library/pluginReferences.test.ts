import { describe, expect, it, vi } from "vitest";
import { parseProjectPlugins } from "./pluginReferences";
import { builtinPlugin, savedPlugins } from "./pluginReferences.fixture";
import { identity, savedDetails } from "./projectDetails.fixture";
import { parseProjectDetails } from "./projectDetails";
import { createNativeLibraryScanAdapter } from "./native";

const build = "26.1.0.5530";
describe("typed saved plugin reference projection", () => {
  it("keeps ordered duplicates and field provenance while dropping extensions", () => {
    const raw = savedPlugins();
    expect(
      parseProjectPlugins(
        {
          ...raw,
          installation: "private",
          items: raw.items.map((item) => ({
            ...item,
            path: "private.dll",
            name: { ...item.name, secret: "discard" },
          })),
        },
        build,
      ),
    ).toEqual(raw);
    expect(
      parseProjectDetails(
        { ...savedDetails(), pluginReferences: raw },
        identity,
      ),
    ).toEqual({ ...savedDetails(), pluginReferences: raw });
    expect(parseProjectDetails(savedDetails(), identity)).toEqual(
      savedDetails(),
    );
    expect(
      parseProjectPlugins({ coverage: raw.coverage, items: [] }, build).items,
    ).toEqual([]);
  });
  it.each([
    null,
    [],
    {},
    { coverage: "installed", items: [] },
    { ...savedPlugins(), status: "extracted" },
    { ...savedPlugins(), items: null },
    { ...savedPlugins(), items: [builtinPlugin(2)] },
    { ...savedPlugins(), items: [null] },
    {
      ...savedPlugins(),
      items: [
        { ...builtinPlugin(), name: { status: "extracted", value: "other" } },
      ],
    },
    {
      ...savedPlugins(),
      items: Array.from({ length: 1025 }, (_, i) => builtinPlugin(i + 1)),
    },
  ])("rejects invalid containers/positions/relationships %#", (raw) => {
    expect(() => parseProjectPlugins(raw, build)).toThrow("internal");
  });
  it.each([
    { status: "extracted", value: "" },
    { status: "extracted", value: "x\0y" },
    { status: "extracted", value: "x".repeat(4096) },
    { status: "extracted", value: "3x Osc", method: "guess" },
    { status: "extracted", value: "3x Osc", reason: null },
    {
      status: "unavailable",
      value: "guessed",
      reason: "PLUGIN_NAME_NOT_STORED",
    },
    { status: "unavailable", value: null, reason: "private diagnostic" },
    {
      status: "unsupported",
      value: null,
      reason: "VST_METADATA_UNSUPPORTED",
      confidence: "high",
    },
    { status: "resolved", value: "3x Osc" },
  ])("rejects invalid field shapes without exposing text %#", (name) => {
    expect(() =>
      parseProjectPlugins(
        { ...savedPlugins(), items: [{ ...builtinPlugin(), name }] },
        build,
      ),
    ).toThrow("internal");
  });
  it("accepts only the verified Sampler inference with consistent absent class/vendor", () => {
    const raw = savedPlugins();
    expect(() => parseProjectPlugins(raw, "20.0.0.1")).toThrow("internal");
    expect(parseProjectPlugins(raw, "25.1.3.4922")).toEqual(raw);
    for (const name of [
      { ...raw.items[1]!.name, value: "Sampler 2" },
      { ...raw.items[1]!.name, confidence: "medium" },
      { ...raw.items[1]!.name, method: "guess" },
    ])
      expect(() =>
        parseProjectPlugins(
          { ...raw, items: [{ ...raw.items[1], position: 1, name }] },
          build,
        ),
      ).toThrow("internal");
  });
  it("checks constructed wrapper missing and unsupported record consistency", () => {
    const item = savedPlugins().items[2]!;
    for (const reason of [
      "VST_METADATA_UNSUPPORTED",
      "MULTIPLE_VST_METADATA_RECORDS",
    ] as const) {
      const field = { status: "unsupported", value: null, reason } as const;
      const raw = {
        coverage: "top-level-saved-references",
        items: [{ ...item, position: 1, name: field, vendor: field }],
      };
      expect(parseProjectPlugins(raw, build).items[0]?.name).toEqual(field);
      expect(() =>
        parseProjectPlugins(
          {
            ...raw,
            items: [
              {
                ...raw.items[0],
                vendor: {
                  ...field,
                  reason: "PLUGIN_NAME_ENCODING_UNSUPPORTED",
                },
              },
            ],
          },
          build,
        ),
      ).toThrow("internal");
    }
    const name = {
      status: "unavailable",
      value: null,
      reason: "PLUGIN_NAME_NOT_STORED",
    } as const;
    expect(
      parseProjectPlugins(
        {
          coverage: "top-level-saved-references",
          items: [{ ...item, position: 1, name }],
        },
        build,
      ).items[0]?.name,
    ).toEqual(name);
  });
  it("enforces the entry and 256 KiB UTF-8 projection budgets", () => {
    expect(
      parseProjectPlugins(
        {
          coverage: "top-level-saved-references",
          items: Array.from({ length: 1024 }, (_, i) => builtinPlugin(i + 1)),
        },
        build,
      ).items,
    ).toHaveLength(1024);
    const text = "🎵".repeat(1900);
    expect(() =>
      parseProjectPlugins(
        {
          coverage: "top-level-saved-references",
          items: Array.from({ length: 40 }, (_, i) =>
            builtinPlugin(i + 1, text),
          ),
        },
        build,
      ),
    ).toThrow("internal");
  });
  it("forbids references in disabled/no-current and mismatched-source responses", () => {
    for (const state of ["disabled", "no_current"])
      expect(() =>
        parseProjectDetails(
          { ...identity, state, pluginReferences: savedPlugins() },
          identity,
        ),
      ).toThrow("internal");
    expect(() =>
      parseProjectDetails(
        {
          ...savedDetails(),
          rootId: "other",
          pluginReferences: savedPlugins(),
        },
        identity,
      ),
    ).toThrow("internal");
  });
  it("uses only the existing read request and never sends names/vendors as arguments", async () => {
    const invoke = vi.fn().mockResolvedValue({
      schemaVersion: 1,
      status: "ok",
      correlationId: "correlation_00000000000000000000000000000001",
      data: { ...savedDetails(), pluginReferences: savedPlugins() },
    });
    const adapter = createNativeLibraryScanAdapter({ invoke, listen: vi.fn() });
    const file = {
      ...identity,
      byteSize: "1024",
      modifiedAt: "2026-01-02T00:00:00Z",
    };
    expect((await adapter.getProjectDetails!(file)).state).toBe("available");
    expect(invoke).toHaveBeenCalledExactlyOnceWith("get_project_details", {
      request: {
        schemaVersion: 1,
        rootId: file.rootId,
        locationId: file.locationId,
        expectedByteSize: file.byteSize,
        expectedModifiedAt: file.modifiedAt,
      },
    });
  });
});

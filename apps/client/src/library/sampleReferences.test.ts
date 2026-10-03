import { describe, expect, it, vi } from "vitest";
import { parseProjectDetails } from "./projectDetails";
import { identity, savedDetails } from "./projectDetails.fixture";
import { createNativeLibraryScanAdapter } from "./native";
import { parseProjectSamples } from "./sampleReferences";

const samples = [
  { position: 1, status: "extracted", value: "..\\Samples\\Kick.wav" },
  { position: 2, status: "unavailable", value: null },
  { position: 3, status: "extracted", value: "..\\Samples\\Kick.wav" },
];
describe("typed saved sample references", () => {
  it("retains order, duplicates, missing entries, and only allowed local fields", () => {
    expect(
      parseProjectSamples(
        samples.map((v) => ({ ...v, resolvedPath: "private", exists: true })),
        3,
      ),
    ).toEqual(samples);
    expect(
      parseProjectDetails(
        { ...savedDetails(), sampleReferences: samples },
        identity,
      ),
    ).toEqual({ ...savedDetails(), sampleReferences: samples });
    expect(parseProjectDetails(savedDetails(), identity)).toEqual(
      savedDetails(),
    );
    expect(parseProjectSamples([], 0)).toEqual([]);
  });
  it.each([
    null,
    {},
    [],
    samples.slice(0, 2),
    samples.map((v) => ({ ...v, position: 1 })),
    [{ ...samples[0], value: "" }, ...samples.slice(1)],
    [{ ...samples[0], value: "x\0y" }, ...samples.slice(1)],
    [{ ...samples[0], value: "x".repeat(4096) }, ...samples.slice(1)],
    [{ ...samples[0], status: "resolved" }, ...samples.slice(1)],
    [{ ...samples[0], status: "inferred" }, ...samples.slice(1)],
    [samples[0], { ...samples[1], value: "guessed.wav" }, samples[2]],
    Array.from({ length: 257 }, (_, i) => ({ ...samples[0], position: i + 1 })),
  ])("rejects malformed sample projection %#", (value) => {
    expect(() =>
      parseProjectDetails(
        { ...savedDetails(), sampleReferences: value },
        identity,
      ),
    ).toThrow("internal");
  });
  it("rejects sample values when no current metadata is authorized", () => {
    for (const state of ["disabled", "no_current"])
      expect(() =>
        parseProjectDetails(
          { ...identity, state, sampleReferences: samples },
          identity,
        ),
      ).toThrow("internal");
  });
  it("reads through the existing command without transmitting or acting on sample text", async () => {
    const invoke = vi.fn().mockResolvedValue({
      schemaVersion: 1,
      status: "ok",
      correlationId: "correlation_00000000000000000000000000000001",
      data: { ...savedDetails(), sampleReferences: samples },
    });
    const adapter = createNativeLibraryScanAdapter({ invoke, listen: vi.fn() });
    const file = {
      ...identity,
      byteSize: "1024",
      modifiedAt: "2026-01-02T00:00:00Z",
    };
    await expect(adapter.getProjectDetails!(file)).resolves.toEqual({
      ...savedDetails(),
      sampleReferences: samples,
    });
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

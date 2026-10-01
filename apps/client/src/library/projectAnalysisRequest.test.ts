import { describe, expect, it, vi } from "vitest";
import { createNativeLibraryScanAdapter } from "./native";
import { parseProjectDetails } from "./projectDetails";
import { identity, savedDetails } from "./projectDetails.fixture";
import {
  ANALYSIS_REQUEST_BLOCKS,
  parseProjectAnalysisRequest,
  parseProjectAnalysisRequestResult,
} from "./projectAnalysisRequest";

const requestKey = "a".repeat(64);
const file = {
  ...identity,
  byteSize: "1024",
  modifiedAt: "2026-01-02T00:00:00Z",
  rootCanonicalPath: "C:\\Synthetic\\Private",
  fileName: "Private.flp",
};

describe("bounded project analysis requests", () => {
  it("projects ready and blocked states without private extensions", () => {
    const ready = { state: "ready", requestKey };
    expect(
      parseProjectAnalysisRequest({ ...ready, privatePath: "private" }),
    ).toEqual(ready);
    for (const state of ANALYSIS_REQUEST_BLOCKS) {
      const blocked = { state, requestKey: null };
      expect(parseProjectAnalysisRequest(blocked)).toEqual(blocked);
      expect(
        parseProjectDetails(
          { ...savedDetails(), analysisRequest: blocked },
          identity,
        ),
      ).toEqual({ ...savedDetails(), analysisRequest: blocked });
    }
    expect(
      parseProjectDetails(
        { ...identity, state: "no_current", analysisRequest: ready },
        identity,
      ),
    ).toEqual({ ...identity, state: "no_current", analysisRequest: ready });
    expect(parseProjectDetails(savedDetails(), identity)).toEqual(
      savedDetails(),
    );
  });

  it.each([
    null,
    [],
    { state: "ready", requestKey: "A".repeat(64) },
    { state: "ready", requestKey: "a".repeat(63) },
    { state: "ready", requestKey: "C:\\Private\\Project.flp" },
    { state: "pending", requestKey },
    { state: "pending" },
    { state: "private_reason", requestKey: null },
  ])("rejects malformed action information %#", (value) => {
    expect(() => parseProjectAnalysisRequest(value)).toThrow("internal");
    expect(() =>
      parseProjectDetails(
        { ...savedDetails(), analysisRequest: value },
        identity,
      ),
    ).toThrow("internal");
  });

  it("checks response identity and limits reasons to fixed categories", () => {
    const queued = { ...identity, state: "queued", reason: null };
    expect(
      parseProjectAnalysisRequestResult(
        { ...queued, leaseToken: "private" },
        identity,
      ),
    ).toEqual(queued);
    for (const reason of ANALYSIS_REQUEST_BLOCKS) {
      const result = { ...identity, state: "blocked", reason };
      expect(parseProjectAnalysisRequestResult(result, identity)).toEqual(
        result,
      );
    }
    for (const invalid of [
      { ...queued, rootId: "other" },
      { ...queued, locationId: "other" },
      { ...queued, reason: "private" },
      { ...queued, state: "running" },
      { ...queued, state: "blocked", reason: "private" },
    ])
      expect(() =>
        parseProjectAnalysisRequestResult(invalid, identity),
      ).toThrow("internal");
  });

  it("sends only the displayed identity, fingerprint, and action key", async () => {
    const invoke = vi.fn().mockResolvedValue({
      schemaVersion: 1,
      correlationId: "correlation_00000000000000000000000000000001",
      status: "ok",
      data: {
        ...identity,
        state: "queued",
        reason: null,
        privatePath: "private",
      },
    });
    const adapter = createNativeLibraryScanAdapter({ invoke, listen: vi.fn() });
    await expect(
      adapter.requestProjectAnalysis!(file, requestKey),
    ).resolves.toEqual({ ...identity, state: "queued", reason: null });
    expect(invoke).toHaveBeenCalledExactlyOnceWith("request_project_analysis", {
      request: {
        schemaVersion: 1,
        rootId: file.rootId,
        locationId: file.locationId,
        expectedByteSize: file.byteSize,
        expectedModifiedAt: file.modifiedAt,
        requestKey,
      },
    });
    await expect(
      adapter.requestProjectAnalysis!(file, "private"),
    ).rejects.toThrow("internal");
    expect(invoke).toHaveBeenCalledTimes(1);
  });
});

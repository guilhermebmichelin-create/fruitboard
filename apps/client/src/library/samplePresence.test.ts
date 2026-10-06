import { describe, it, expect, vi } from "vitest";
import {
  parseSampleCheckResult,
  sampleCheckArguments,
  type SampleCheckRequest,
} from "./samplePresence";
import { createNativeLibraryScanAdapter } from "./native";
import type { ProjectSampleReference } from "./sampleReferences";

const request: SampleCheckRequest = {
  schemaVersion: 1,
  rootId: "root",
  locationId: "location",
  snapshotId: "snapshot",
  requestId: "request",
  expectedByteSize: "123",
  expectedModifiedAt: "2026-01-02T00:00:00Z",
};
const samples: readonly ProjectSampleReference[] = [
  { position: 1, status: "extracted", value: "C:\\Private\\same.wav" },
  { position: 2, status: "extracted", value: "C:\\Private\\same.wav" },
  { position: 3, status: "unavailable", value: null },
];
const raw = () => ({
  request,
  state: "complete",
  reason: null,
  report: {
    version: 1,
    context: {
      request_id: "request",
      root_id: "root",
      location_id: "location",
      snapshot_id: "snapshot",
      session_id: "session",
    },
    checked_at_unix_ms: 1791234567890,
    channels: [
      { position: 1, status: "present" },
      { position: 2, status: "not_found" },
      { position: 3, status: "no_saved_reference" },
    ],
  },
});
describe("strict ephemeral sample results", () => {
  it("keeps duplicates in ordered channel slots and strips no private fields", () => {
    const result = parseSampleCheckResult(raw(), request, samples);
    expect(result.state).toBe("complete");
    expect(JSON.stringify(result)).not.toContain("Private");
  });
  it.each([
    ["request", { ...request, requestId: "old-request" }],
    ["rawPath", "C:\\Private\\sample.wav"],
    ["reason", "access_denied"],
  ])("rejects extra or mismatched top-level data %s", (key, value) => {
    expect(() =>
      parseSampleCheckResult({ ...raw(), [key]: value }, request, samples),
    ).toThrow("internal");
  });
  it("rejects stale contexts, unsupported versions, unsafe timestamps, wrong counts/order/status and raw errors", () => {
    const variants: unknown[] = [
      { ...raw(), report: { ...raw().report, version: 2 } },
      { ...raw(), report: { ...raw().report, checked_at_unix_ms: Infinity } },
      {
        ...raw(),
        report: { ...raw().report, checked_at_unix_ms: 253402300800000 },
      },
      { ...raw(), report: { ...raw().report, checked_at_unix_ms: 1.5 } },
      {
        ...raw(),
        report: {
          ...raw().report,
          context: { ...raw().report.context, snapshot_id: "old" },
        },
      },
      { ...raw(), report: { ...raw().report, channels: [] } },
      {
        ...raw(),
        report: {
          ...raw().report,
          channels: [
            { position: 2, status: "present" },
            ...raw().report.channels.slice(1),
          ],
        },
      },
      {
        ...raw(),
        report: {
          ...raw().report,
          channels: [
            { position: 1, status: "missing" },
            ...raw().report.channels.slice(1),
          ],
        },
      },
      {
        ...raw(),
        report: {
          ...raw().report,
          channels: [
            { position: 1, status: "not_checked", reason: "private OS error" },
            ...raw().report.channels.slice(1),
          ],
        },
      },
      {
        ...raw(),
        report: {
          ...raw().report,
          channels: [
            { position: 1, status: "present", path: "private" },
            ...raw().report.channels.slice(1),
          ],
        },
      },
      {
        ...raw(),
        report: {
          ...raw().report,
          channels: [
            { position: 1, status: "no_saved_reference" },
            ...raw().report.channels.slice(1),
          ],
        },
      },
    ];
    for (const value of variants)
      expect(() => parseSampleCheckResult(value, request, samples)).toThrow(
        "internal",
      );
  });
  it("decodes fixed non-success responses without making an absence claim", () => {
    for (const state of [
      "disabled",
      "busy",
      "stale",
      "cancelled",
      "deadline",
      "failed",
      "references_unavailable",
    ]) {
      expect(
        parseSampleCheckResult(
          { request, state, reason: null, report: null },
          request,
          samples,
        ),
      ).toEqual({ state });
    }
    expect(
      parseSampleCheckResult(
        {
          request,
          state: "unavailable",
          reason: "access_denied",
          report: null,
        },
        request,
        samples,
      ),
    ).toEqual({ state: "unavailable", reason: "access_denied" });
    expect(() =>
      parseSampleCheckResult(
        { request, state: "no_references", reason: null, report: null },
        request,
        samples,
      ),
    ).toThrow("internal");
    expect(() => parseSampleCheckResult(raw(), request, [], true)).toThrow(
      "internal",
    );
  });
  it("sends only version, identities and displayed fingerprint for check/cancel", async () => {
    const invoke = vi
      .fn()
      .mockResolvedValueOnce({
        schemaVersion: 1,
        correlationId: "correlation_00000000000000000000000000000001",
        status: "ok",
        data: raw(),
      })
      .mockResolvedValueOnce({
        schemaVersion: 1,
        correlationId: "correlation_00000000000000000000000000000002",
        status: "ok",
        data: { request, state: "cancelled", reason: null, report: null },
      });
    const adapter = createNativeLibraryScanAdapter({
      invoke,
      listen: () => () => {},
    });
    await adapter.checkSavedSamples?.(request, samples);
    await adapter.cancelSampleCheck?.(request);
    expect(invoke.mock.calls).toEqual([
      ["check_saved_samples", { request }],
      ["cancel_sample_check", { request }],
    ]);
    expect(JSON.stringify(invoke.mock.calls)).not.toContain("Private");
    expect(() =>
      sampleCheckArguments({ ...request, expectedByteSize: "01" }),
    ).toThrow("internal");
    expect(() =>
      sampleCheckArguments({
        ...request,
        path: "private",
      } as SampleCheckRequest),
    ).toThrow("internal");
  });
  it("accepts 256 ordered unchecked references without truncation", () => {
    const many = Array.from({ length: 256 }, (_, i) => ({
      position: i + 1,
      status: "extracted" as const,
      value: "relative.wav",
    }));
    const report = {
      ...raw().report,
      channels: many.map((s) => ({
        position: s.position,
        status: "not_checked",
        reason: "relative_reference",
      })),
    };
    const result = parseSampleCheckResult({ ...raw(), report }, request, many);
    expect(result.state === "complete" && result.report.channels.length).toBe(
      256,
    );
  });
});

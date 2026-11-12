import { describe, expect, it } from "vitest";
import { parseProjectAnalysis } from "./projectAnalysis";
import { parseProjectDetails } from "./projectDetails";
import { identity, savedDetails } from "./projectDetails.fixture";

describe("analysis display boundary", () => {
  it("retains only the numeric saved version of a matching unsupported result", () => {
    const result = {
      state: "unsupported",
      attempts: 1,
      reason: "unsupported_version",
      savedVersion: "25.2.4.4960",
      rawPayload: "private",
    };
    expect(parseProjectAnalysis(result)).toEqual({
      state: "unsupported",
      attempts: 1,
      reason: "unsupported_version",
      savedVersion: "25.2.4.4960",
    });
    expect(
      parseProjectAnalysis({
        state: "unsupported",
        attempts: 1,
        reason: "unsupported_version",
      }),
    ).toEqual({
      state: "unsupported",
      attempts: 1,
      reason: "unsupported_version",
    });
  });
  it.each([
    null,
    "",
    "25.2.4",
    "25..4.4960",
    "25.2.4.4960\n",
    "C:\\Private\\project.flp",
    "123456789012345678901234567890.1.1.1",
    25244960,
  ])("rejects unsafe or malformed saved versions", (savedVersion) => {
    expect(() =>
      parseProjectAnalysis({
        state: "unsupported",
        attempts: 1,
        reason: "unsupported_version",
        savedVersion,
      }),
    ).toThrow("internal");
  });
  it.each([
    "not_current",
    "queued",
    "running",
    "complete",
    "failed",
    "cancelled",
    "stale",
  ])("rejects a version attached to %s", (state) => {
    expect(() =>
      parseProjectAnalysis({
        state,
        attempts: 1,
        reason: "unsupported_version",
        savedVersion: "25.2.4.4960",
      }),
    ).toThrow("internal");
  });
  it("rejects a version without an unsupported-version reason", () => {
    for (const reason of [null, "invalid_file", "unclassified"]) {
      expect(() =>
        parseProjectAnalysis({
          state: "unsupported",
          attempts: 1,
          reason,
          savedVersion: "25.2.4.4960",
        }),
      ).toThrow("internal");
    }
  });
  it.each([
    "queued",
    "running",
    "complete",
    "unsupported",
    "failed",
    "cancelled",
    "stale",
  ])("accepts bounded recorded %s without extra private fields", (state) => {
    const value = {
      state,
      attempts: 1,
      reason: null,
      leaseToken: "private",
      errorCode: "private",
    };
    expect(parseProjectAnalysis(value)).toEqual({
      state,
      attempts: 1,
      reason: null,
    });
  });
  it.each([
    null,
    [],
    {},
    { state: "unknown", attempts: 1, reason: null },
    { state: "queued", attempts: 4, reason: null },
    { state: "queued", attempts: -1, reason: null },
    { state: "queued", attempts: 1.5, reason: null },
    { state: "running", attempts: 0, reason: null },
    { state: "complete", attempts: "1", reason: null },
    { state: "not_reported", attempts: 0, reason: null },
    { state: "not_current", attempts: null, reason: "source_changed" },
    { state: "failed", attempts: 1, reason: "C:\\Private\\secret.flp" },
  ])("rejects untrusted states, counts and reasons", (value) => {
    expect(() => parseProjectAnalysis(value)).toThrow("internal");
  });
  it("keeps older responses readable without inventing an attempt", () => {
    const older = { ...savedDetails() };
    delete older.analysis;
    delete older.warnings;
    expect(parseProjectDetails(older, identity)).toEqual(older);
    expect(
      parseProjectDetails({ ...identity, state: "no_current" }, identity),
    ).toEqual({ ...identity, state: "no_current" });
  });
  it("allows a safe problem alongside valid saved facts and rejects raw warnings", () => {
    const details = {
      ...savedDetails(),
      analysis: { state: "failed", attempts: 3, reason: "parser_unavailable" },
    };
    expect(parseProjectDetails(details, identity)).toEqual(details);
    for (const warnings of [
      ["private"],
      ["unverified_events", "unverified_events"],
      "unverified_events",
      null,
    ]) {
      expect(() =>
        parseProjectDetails({ ...details, warnings }, identity),
      ).toThrow("internal");
    }
    expect(() =>
      parseProjectDetails(
        { ...identity, state: "disabled", analysis: details.analysis },
        identity,
      ),
    ).toThrow("internal");
  });
});

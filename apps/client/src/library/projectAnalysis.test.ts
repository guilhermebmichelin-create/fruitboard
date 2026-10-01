import { describe, expect, it } from "vitest";
import { parseProjectAnalysis } from "./projectAnalysis";
import { parseProjectDetails } from "./projectDetails";
import { identity, savedDetails } from "./projectDetails.fixture";

describe("analysis display boundary", () => {
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

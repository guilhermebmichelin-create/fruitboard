import { describe, expect, it } from "vitest";
import { parseProjectDetails } from "./projectDetails";
import { identity, savedDetails } from "./projectDetails.fixture";
import { parseProjectMixerInserts } from "./projectMixerInserts";

const mixerFixture = () => ({
  state: "available" as const,
  count: 16,
  items: Array.from({ length: 16 }, (_, index) => ({
    savedInsertId: index + 1,
    name:
      index < 2
        ? { status: "extracted" as const, value: "Duplicate label" }
        : { status: "unavailable" as const, reason: "name_not_saved" as const },
  })),
});
const build = "26.1.0.5530";
describe("saved mixer insert decoding", () => {
  it("preserves duplicate names, qualified positions and missing names through the actual details decoder", () => {
    const mixer = mixerFixture();
    const result = parseProjectDetails(
      { ...savedDetails(), mixerInserts: mixer },
      identity,
    );
    expect(result.state === "available" && result.mixerInserts).toEqual(mixer);
    expect(parseProjectDetails(savedDetails(), identity).state).toBe(
      "available",
    );
    expect(
      parseProjectMixerInserts(
        { state: "unsupported", reason: "not_saved" },
        build,
      ),
    ).toEqual({ state: "unsupported", reason: "not_saved" });
  });
  it.each([
    { ...mixerFixture(), count: 0 },
    { ...mixerFixture(), count: 17 },
    { ...mixerFixture(), count: 16.5 },
    { ...mixerFixture(), items: mixerFixture().items.slice(1) },
    { ...mixerFixture(), extra: "raw effects" },
    { state: "unsupported", reason: "unverified_build" },
    { state: "unavailable", reason: "not_saved" },
    { state: "unsupported", reason: "limit_exceeded", items: [] },
    null,
  ])("rejects invalid selected structures and states: %j", (invalid) => {
    expect(() =>
      parseProjectDetails(
        { ...savedDetails(), mixerInserts: invalid },
        identity,
      ),
    ).toThrow();
  });
  it("rejects wrong IDs, forbidden names, inference and malformed text rather than accepting a partial list", () => {
    for (const item of [
      { savedInsertId: 0, name: { status: "extracted", value: "Master" } },
      { savedInsertId: 17, name: { status: "extracted", value: "Current" } },
      {
        savedInsertId: 2,
        name: { status: "extracted", value: "Duplicate ID" },
      },
      { savedInsertId: 1, name: { status: "extracted", value: "" } },
      { savedInsertId: 1, name: { status: "extracted", value: "x\0" } },
      { savedInsertId: 1, name: { status: "extracted", value: "\ud800" } },
      {
        savedInsertId: 1,
        name: { status: "extracted", value: "x".repeat(4096) },
      },
      { savedInsertId: 1, name: { status: "inferred", value: "Insert 1" } },
      { savedInsertId: 1, name: { status: "unavailable", reason: "unknown" } },
      {
        savedInsertId: 1,
        name: { status: "extracted", value: "OK", slots: [] },
      },
    ]) {
      const invalid = mixerFixture();
      expect(() =>
        parseProjectMixerInserts(
          { ...invalid, items: [item, ...invalid.items.slice(1)] },
          build,
        ),
      ).toThrow();
    }
  });
  it("checks build relationships and refuses mixer data outside current details", () => {
    expect(() =>
      parseProjectMixerInserts(mixerFixture(), "25.1.3.4922"),
    ).toThrow();
    expect(
      parseProjectMixerInserts(
        { state: "unsupported", reason: "unverified_build" },
        "25.1.3.4922",
      ),
    ).toEqual({ state: "unsupported", reason: "unverified_build" });
    for (const state of ["disabled", "no_current"])
      expect(() =>
        parseProjectDetails(
          { ...identity, state, mixerInserts: mixerFixture() },
          identity,
        ),
      ).toThrow();
    for (const reason of [
      "no_saved_section",
      "unverified_layout",
      "unverified_binding",
      "ambiguous_records",
      "limit_exceeded",
    ]) {
      const result = parseProjectMixerInserts(
        {
          state: reason === "no_saved_section" ? "unavailable" : "unsupported",
          reason,
        },
        build,
      );
      expect(result.state !== "available" && result.reason).toBe(reason);
    }
  });
});

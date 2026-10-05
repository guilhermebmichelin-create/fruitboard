import { describe, expect, it } from "vitest";
import { parseProjectPatterns } from "./projectPatterns";
import { parseProjectDetails } from "./projectDetails";
import { identity, savedDetails } from "./projectDetails.fixture";

const build = "26.1.0.5530";
const item = (patternId: number, value = "Pattern") => ({
  patternId,
  name: { status: "extracted", value },
});
describe("saved pattern contract", () => {
  it("keeps bounded sparse IDs and missing/empty/Unicode names, dropping unknown extensions", () => {
    const input = {
      state: "available",
      count: 3,
      items: [
        { ...item(1, ""), rawPrivate: "discard" },
        {
          patternId: 200,
          name: { status: "unavailable", value: null, path: "discard" },
        },
        item(65535, "合😀\u202e\n"),
      ],
      path: "discard",
    };
    expect(parseProjectPatterns(input, build)).toEqual({
      state: "available",
      count: 3,
      items: [
        item(1, ""),
        { patternId: 200, name: { status: "unavailable", value: null } },
        item(65535, "合😀\u202e\n"),
      ],
    });
    const patterns = parseProjectPatterns(input, build);
    expect(
      parseProjectDetails({ ...savedDetails(), patterns }, identity),
    ).toEqual({ ...savedDetails(), patterns });
    expect(parseProjectDetails(savedDetails(), identity)).toEqual(
      savedDetails(),
    );
  });
  it.each([
    null,
    {},
    { state: "available", count: 0, items: [] },
    { state: "available", count: 1025, items: [] },
    { state: "available", count: 2, items: [item(1)] },
    { state: "available", count: 1.5, items: [item(1)] },
    ...[0, -1, 1.5, 65536].map((id) => ({
      state: "available",
      count: 1,
      items: [item(id)],
    })),
    { state: "available", count: 2, items: [item(1), item(1)] },
    { state: "available", count: 2, items: [item(2), item(1)] },
    ...["bad\0", "x".repeat(4096), "😀".repeat(2048)].map((name) => ({
      state: "available",
      count: 1,
      items: [item(1, name)],
    })),
    {
      state: "available",
      count: 1,
      items: [{ patternId: 1, name: { status: "inferred", value: "Name" } }],
    },
    {
      state: "available",
      count: 1,
      items: [{ patternId: 1, name: { status: "unavailable", value: "Name" } }],
    },
    { state: "unsupported", reason: "unverified_build" },
    { state: "unavailable", reason: "no_stored_patterns", count: 0 },
    { state: "available", count: 1, items: [item(1)], reason: "private" },
  ])("rejects malformed claims", (value) => {
    expect(() => parseProjectPatterns(value, build)).toThrow("internal");
    expect(() =>
      parseProjectDetails({ ...savedDetails(), patterns: value }, identity),
    ).toThrow("internal");
  });
  it("separates absence, old results and unverified builds without accepting stale fields", () => {
    for (const reason of ["not_saved", "not_advertised"] as const) {
      expect(
        parseProjectPatterns({ state: "unsupported", reason }, build),
      ).toEqual({ state: "unsupported", reason });
    }
    expect(
      parseProjectPatterns(
        { state: "unavailable", reason: "no_stored_patterns" },
        build,
      ).state,
    ).toBe("unavailable");
    expect(
      parseProjectPatterns(
        { state: "unsupported", reason: "unverified_build" },
        "25.1.3.4922",
      ).state,
    ).toBe("unsupported");
    expect(() =>
      parseProjectPatterns(
        { state: "available", count: 1, items: [item(1)] },
        "25.1.3.4922",
      ),
    ).toThrow("internal");
    for (const state of ["disabled", "no_current"]) {
      expect(() =>
        parseProjectDetails(
          {
            ...identity,
            state,
            patterns: { state: "unsupported", reason: "not_saved" },
          },
          identity,
        ),
      ).toThrow("internal");
    }
  });
  it("accepts the exact list and Unicode bounds", () => {
    const items = Array.from({ length: 1024 }, (_, index) =>
      item(index + 1, "界".repeat(4095)),
    );
    expect(
      parseProjectPatterns({ state: "available", count: 1024, items }, build),
    ).toEqual({ state: "available", count: 1024, items });
  });
});

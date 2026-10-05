import { describe, expect, it, vi } from "vitest";
import {
  channelLabel,
  formatProjectFact,
  parseProjectDetails,
} from "./projectDetails";
import { createNativeLibraryScanAdapter } from "./native";
import { identity, savedDetails } from "./projectDetails.fixture";

describe("typed project details", () => {
  it("projects only allowed facts and keeps integers exact", () => {
    const details = savedDetails();
    const input = JSON.parse(JSON.stringify(details)) as Record<
      string,
      unknown
    >;
    const result = parseProjectDetails(
      { ...input, rawPrivateExtension: "private" },
      identity,
    );
    expect(result).toEqual(details);
    expect(
      formatProjectFact({
        key: "flStudioTimeSpentMs",
        status: "extracted",
        value: "3661000",
        explanation: null,
      }),
    ).toBe("1 h 1 min 1 s");
    expect(
      formatProjectFact({
        key: "filesystemCreatedAtMs",
        status: "extracted",
        value: "18446744073709551615",
        explanation: null,
      }),
    ).toBe("Outside the supported date range");
    expect(formatProjectFact(details.facts[3]!)).toBe(
      "2026-01-02 03:04:05.123 (local)",
    );
  });

  it("accepts duplicate default Sampler labels and preserves an explained older numbered label", () => {
    const details = savedDetails();
    const sampler = details.channels[1]!;
    const current = {
      ...details,
      channels: [1, 2, 3].map((position) => ({ ...sampler, position })),
    };
    expect(parseProjectDetails(current, identity)).toEqual(current);
    const legacy = {
      ...current,
      channels: current.channels.map((channel) => ({
        ...channel,
        name: {
          ...channel.name,
          value:
            channel.position === 1 ? "Sampler" : `Sampler ${channel.position}`,
          explanation:
            "Earlier Fruitboard inferred a numbered Sampler label. Analyze this project again to check it with the corrected naming rule.",
        },
      })),
    };
    expect(parseProjectDetails(legacy, identity)).toEqual(legacy);
  });

  it.each([
    (v: Record<string, unknown>) => {
      v["rootId"] = "different-root";
    },
    (v: Record<string, unknown>) => {
      v["locationId"] = "different-file";
    },
    (v: Record<string, unknown>) => {
      v["outcome"] = "failed";
    },
    (v: Record<string, unknown>) => {
      v["state"] = "disabled";
    },
    (v: Record<string, unknown>) => {
      v["facts"] = [];
    },
  ])("rejects malformed or mismatched responses", (change) => {
    const value = { ...savedDetails() };
    change(value);
    expect(() => parseProjectDetails(value, identity)).toThrow("internal");
  });

  it.each([
    ["baseTempoBpm", "0"],
    ["channelCount", "9007199254740993"],
    ["filesystemCreatedAtMs", "18446744073709551616"],
    ["projectCreatedLocal", "2026-02-30T00:00:00.000"],
    ["playlistPatternNominalSeconds", "Infinity"],
    ["flStudioTimeSpentMs", "01"],
  ])("rejects invalid %s values", (key, value) => {
    const details = savedDetails();
    const facts = details.facts.map((fact) =>
      fact.key === key ? { ...fact, value } : fact,
    );
    expect(() => parseProjectDetails({ ...details, facts }, identity)).toThrow(
      "internal",
    );
  });

  it("requires inference explanation and unavailable fields to have no value", () => {
    const details = savedDetails();
    for (const replacement of [
      { explanation: null },
      { status: "extracted", explanation: null },
      { status: "unavailable" },
    ]) {
      expect(() =>
        parseProjectDetails(
          {
            ...details,
            facts: details.facts.map((f) =>
              f.key === "playlistPatternSpanBars"
                ? { ...f, ...replacement }
                : f,
            ),
          },
          identity,
        ),
      ).toThrow("internal");
    }
  });

  it("selects channel values without retaining paths or arbitrary response extensions", () => {
    const details = savedDetails();
    const result = parseProjectDetails(
      {
        ...details,
        channels: details.channels.map((channel) => ({
          ...channel,
          privatePath: "private",
          name: { ...channel.name, raw: "private" },
        })),
      },
      identity,
    );
    expect(result).toEqual(details);
    expect(channelLabel("")).toBe("Empty saved label");
    expect(channelLabel("Lead\n\u202eName")).toBe("Lead\\u000a\\u202eName");
    expect(channelLabel("鼓 / Bass 🎵")).toBe("鼓 / Bass 🎵");
    const empty = {
      ...details,
      facts: details.facts.map((fact) =>
        fact.key === "channelCount" ? { ...fact, value: "0" } : fact,
      ),
      channels: [],
    };
    expect(parseProjectDetails(empty, identity)).toEqual(empty);
  });

  it.each([
    (v: Record<string, unknown>) => {
      v["channels"] = [];
    },
    (v: Record<string, unknown>) => {
      v["channels"] = Array(257).fill(savedDetails().channels[0]);
    },
    (v: Record<string, unknown>) => {
      v["channels"] = savedDetails().channels.map((channel) => ({
        ...channel,
        position: 1,
      }));
    },
    (v: Record<string, unknown>) => {
      v["channels"] = savedDetails().channels.map((channel) => ({
        ...channel,
        name: { ...channel.name, value: "\0" },
      }));
    },
    (v: Record<string, unknown>) => {
      v["channels"] = savedDetails().channels.map((channel) => ({
        ...channel,
        name: { ...channel.name, value: "a".repeat(4096) },
      }));
    },
    (v: Record<string, unknown>) => {
      v["channels"] = savedDetails().channels.map((channel) => ({
        ...channel,
        instrument: {
          status: "extracted",
          value: "Unverified Plugin",
          explanation: null,
        },
      }));
    },
    (v: Record<string, unknown>) => {
      v["channels"] = savedDetails().channels.map((channel) => ({
        ...channel,
        instrument: {
          status: "unsupported",
          value: "3x Osc",
          explanation: "Unknown",
        },
      }));
    },
    (v: Record<string, unknown>) => {
      v["channels"] = savedDetails().channels.map((channel) => ({
        ...channel,
        name: { status: "inferred", value: "Sampler 99", explanation: "Guess" },
      }));
    },
    (v: Record<string, unknown>) => {
      v["channels"] = savedDetails().channels.map((channel) => ({
        ...channel,
        instrument: { status: "inferred", value: "Sampler", explanation: null },
      }));
    },
    (v: Record<string, unknown>) => {
      v["facts"] = savedDetails().facts.map((fact) =>
        fact.key === "savedVersion" ? { ...fact, value: "25.1.3.4922" } : fact,
      );
    },
  ])(
    "rejects malformed channel ordering, bounds and identity claims",
    (change) => {
      const value = { ...savedDetails() };
      change(value);
      expect(() => parseProjectDetails(value, identity)).toThrow("internal");
    },
  );

  it("invokes a bounded read with IDs and the displayed fingerprint only", async () => {
    const invoke = vi.fn().mockResolvedValue({
      schemaVersion: 1,
      correlationId: `correlation_${"0".repeat(32)}`,
      status: "ok",
      data: savedDetails(),
    });
    const adapter = createNativeLibraryScanAdapter({
      invoke,
      listen: () => () => undefined,
    });
    await expect(
      adapter.getProjectDetails?.({
        ...identity,
        byteSize: "1024",
        modifiedAt: "2026-01-02T00:00:00Z",
      }),
    ).resolves.toEqual(savedDetails());
    expect(invoke).toHaveBeenCalledWith("get_project_details", {
      request: {
        schemaVersion: 1,
        ...identity,
        expectedByteSize: "1024",
        expectedModifiedAt: "2026-01-02T00:00:00Z",
      },
    });
  });
});

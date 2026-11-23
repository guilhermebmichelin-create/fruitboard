import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { parseProjectDetails } from "./projectDetails";
import { identity, savedDetails } from "./projectDetails.fixture";
import { SavedAutomationCounts } from "./SavedAutomationCounts";

describe("saved automation counts", () => {
  it("displays an independently labeled zero for both counts", () => {
    const base = savedDetails();
    const details = parseProjectDetails(
      {
        ...base,
        facts: base.facts.map((fact) =>
          fact.key === "channelCount" ? { ...fact, value: "1" } : fact,
        ),
        channels: base.channels.slice(0, 1),
        automationCounts: { state: "available", objectCount: 0, pointCount: 0 },
      },
      identity,
    );
    if (details.state !== "available")
      throw new Error("Expected saved details");
    render(<SavedAutomationCounts counts={details.automationCounts} />);
    const section = within(
      screen.getByRole("region", { name: "Saved automation" }),
    );
    expect(section.getByText("Saved Automation Clips")).toBeTruthy();
    expect(section.getByText("Editable control points")).toBeTruthy();
    expect(section.getAllByText("0")).toHaveLength(2);
  });

  it("shows counts in a mixed channel rack", () => {
    const details = parseProjectDetails(
      {
        ...savedDetails(),
        automationCounts: {
          state: "available",
          objectCount: 2,
          pointCount: 11,
        },
      },
      identity,
    );
    if (details.state !== "available")
      throw new Error("Expected saved details");
    render(<SavedAutomationCounts counts={details.automationCounts} />);
    expect(screen.getByText("2")).toBeTruthy();
    expect(screen.getByText("11")).toBeTruthy();
  });

  it("distinguishes older records from unverified layouts and builds", () => {
    const view = render(<SavedAutomationCounts />);
    expect(screen.getByText(/did not save automation counts/)).toBeTruthy();
    view.rerender(
      <SavedAutomationCounts counts={{ state: "unverified_layout" }} />,
    );
    expect(screen.getByText(/saved project layout/)).toBeTruthy();
    view.rerender(
      <SavedAutomationCounts counts={{ state: "unverified_build" }} />,
    );
    expect(screen.getByText(/saved FL Studio build/)).toBeTruthy();
    expect(screen.queryByText("0")).toBeNull();
  });

  it.each([
    null,
    { state: "unverified_build" },
    { state: "unverified_layout", pointCount: 0 },
    { state: "available", objectCount: 4, pointCount: 4 },
    { state: "available", objectCount: 2, pointCount: 1 },
    { state: "available", objectCount: 2, pointCount: 2.5 },
  ])("rejects an invalid details field (%j)", (automationCounts) => {
    expect(() =>
      parseProjectDetails({ ...savedDetails(), automationCounts }, identity),
    ).toThrow();
  });
});

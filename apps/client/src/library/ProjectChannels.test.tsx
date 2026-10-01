import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import axe from "axe-core";
import { describe, expect, it } from "vitest";
import { ProjectChannels } from "./ProjectChannels";
import { savedDetails } from "./projectDetails.fixture";
import type { ProjectChannel } from "./projectDetails";

describe("saved channel details", () => {
  it("keeps labels and instruments separate and explains partial coverage", async () => {
    const channels = savedDetails().channels.map((channel) =>
      channel.position === 3
        ? {
            ...channel,
            name: {
              status: "unavailable" as const,
              value: null,
              explanation: "No channel label was stored.",
            },
          }
        : channel,
    );
    const view = render(<ProjectChannels channels={channels} />);
    const first = screen
      .getByRole("heading", { name: "Channel 1" })
      .closest("li")!;
    expect(within(first).getByText("Fixture Synth A")).toBeTruthy();
    expect(within(first).getByText("3x Osc")).toBeTruthy();
    const third = screen
      .getByRole("heading", { name: "Channel 3" })
      .closest("li")!;
    expect(within(third).getAllByText("Unavailable")).toHaveLength(2);
    expect(within(third).getAllByText("Unsupported")).toHaveLength(2);
    expect(screen.getByText(/does not identify its instrument/)).toBeTruthy();
    expect(screen.getByText(/High confidence. Built-in Sampler/)).toBeTruthy();
    const result = await axe.run(view.container, {
      rules: { "color-contrast": { enabled: false } },
    });
    expect(result.violations).toEqual([]);
  });
  it("renders 20 at a time with keyboard controls that keep focus and all positions reachable", async () => {
    const user = userEvent.setup();
    const channels: ProjectChannel[] = Array.from(
      { length: 45 },
      (_, index) => ({
        position: index + 1,
        name: {
          status: "extracted",
          value: `Label ${index + 1}`,
          explanation: null,
        },
        instrument: {
          status: "unsupported",
          value: null,
          explanation: "Instrument details were not saved with this result.",
        },
      }),
    );
    render(<ProjectChannels channels={channels} />);
    expect(screen.getAllByRole("listitem")).toHaveLength(20);
    const control = screen.getByRole("button", {
      name: "Show next 20 channels",
    });
    control.focus();
    await user.keyboard("{Enter}");
    expect(screen.getAllByRole("listitem")).toHaveLength(40);
    expect(document.activeElement).toBe(control);
    await user.keyboard("{Enter}");
    expect(screen.getAllByRole("listitem")).toHaveLength(45);
    expect(screen.getByText("Label 45")).toBeTruthy();
    expect(screen.getByRole("status").textContent).toBe(
      "Showing 45 of 45 channels.",
    );
    expect(control.textContent).toBe("Show fewer channels");
    expect(document.activeElement).toBe(control);
    await user.keyboard("{Enter}");
    expect(screen.getAllByRole("listitem")).toHaveLength(20);
    expect(document.activeElement).toBe(control);
  });
  it("shows empty and unusual labels as inert text with visible control characters", () => {
    const channel = savedDetails().channels[0]!;
    const view = render(
      <ProjectChannels
        channels={[
          {
            ...channel,
            name: { status: "extracted", value: "", explanation: null },
          },
          {
            ...channel,
            position: 2,
            name: {
              status: "extracted",
              value: "<img src=x onerror=alert(1)>\u202e\n",
              explanation: null,
            },
          },
        ]}
      />,
    );
    expect(screen.getByText("Empty saved label")).toBeTruthy();
    expect(
      screen.getByText("<img src=x onerror=alert(1)>\\u202e\\u000a"),
    ).toBeTruthy();
    expect(view.container.querySelector("img")).toBeNull();
    expect(screen.getByText(/shown as escape codes/)).toBeTruthy();
    view.rerender(<ProjectChannels channels={[]} />);
    expect(
      screen.getByText("No channels were stored in this result."),
    ).toBeTruthy();
  });
});

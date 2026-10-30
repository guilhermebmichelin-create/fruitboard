import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import axe from "axe-core";
import { describe, expect, it, vi } from "vitest";
import { SavedMixerInserts } from "./SavedMixerInserts";
import { ProjectDetailsPanel } from "./ProjectDetailsPanel";
import { createFakeLibraryScanAdapter } from "./fake";
import { identity, savedDetails } from "./projectDetails.fixture";
import type { ProjectDetails } from "./projectDetails";

const mixer = {
  state: "available" as const,
  count: 16,
  items: Array.from({ length: 16 }, (_, index) => ({
    savedInsertId: index + 1,
    name: {
      status: "extracted" as const,
      value: index < 2 ? "Same name" : "<img src=x>\u202e\n",
    },
  })),
};
describe("saved mixer inserts", () => {
  it("renders names as inert text and distinguishes saved records from audibility", async () => {
    const view = render(<SavedMixerInserts mixer={mixer} />);
    expect(screen.getAllByText("Same name")).toHaveLength(2);
    expect(
      screen.getByText(/count does not show which inserts are used or audible/),
    ).toBeTruthy();
    expect(screen.getByText("Saved insert ID: 16")).toBeTruthy();
    expect(view.container.querySelector("img")).toBeNull();
    expect(view.container.textContent).toContain("\\u202e");
    const result = await axe.run(view.container, {
      rules: { "color-contrast": { enabled: false } },
    });
    expect(result.violations).toEqual([]);
  });
  it("explains missing names and older results without inventing zero or default labels", () => {
    const view = render(
      <SavedMixerInserts
        mixer={{
          ...mixer,
          items: mixer.items.map((item) => ({
            ...item,
            name: { status: "unavailable", reason: "name_not_saved" },
          })),
        }}
      />,
    );
    expect(screen.getAllByText("No saved name")).toHaveLength(16);
    view.rerender(<SavedMixerInserts />);
    expect(screen.getByText(/Run a new analysis/)).toBeTruthy();
    expect(screen.queryByRole("status")).toBeNull();
  });
  it("keeps keyboard focus on disclosure for constructed component-scale data", async () => {
    // Component-only scale case; the selected DTO decoder rejects this layout.
    const user = userEvent.setup();
    render(
      <SavedMixerInserts
        mixer={{
          ...mixer,
          count: 45,
          items: Array.from({ length: 45 }, (_, index) => ({
            savedInsertId: index + 1,
            name: { status: "extracted", value: "Constructed" },
          })),
        }}
      />,
    );
    const region = screen.getByRole("region", { name: "Saved mixer inserts" });
    const more = within(region).getByRole("button");
    more.focus();
    await user.keyboard("{Enter}");
    expect(within(region).getAllByRole("listitem")).toHaveLength(40);
    expect(document.activeElement).toBe(more);
    await user.keyboard("{Enter}{Enter}");
    expect(within(region).getAllByRole("listitem")).toHaveLength(20);
    expect(document.activeElement).toBe(more);
  });
  it("clears mixer values on refresh and discards a reply after closing without starting analysis", async () => {
    const user = userEvent.setup();
    const record = {
      ...identity,
      rootDisplayName: "Projects",
      rootCanonicalPath: "C:\\Synthetic",
      fileName: "Mixer.flp",
      relativePath: "Mixer.flp",
      byteSize: "1024",
      modifiedAt: "2026-01-02T00:00:00Z",
      presence: "present" as const,
    };
    const available = { ...savedDetails(), mixerInserts: mixer };
    let finish!: (value: ProjectDetails) => void;
    const adapter = {
      ...createFakeLibraryScanAdapter(),
      getProjectDetails: vi
        .fn()
        .mockResolvedValueOnce(available)
        .mockImplementationOnce(
          () =>
            new Promise<ProjectDetails>((resolve) => {
              finish = resolve;
            }),
        ),
      requestProjectAnalysis: vi.fn(),
    };
    render(<ProjectDetailsPanel record={record} adapter={adapter} />);
    const toggle = screen.getByRole("button", {
      name: "Project details Mixer.flp",
    });
    await user.click(toggle);
    await screen.findByText("Saved insert ID: 16");
    await user.click(
      screen.getByRole("button", { name: "Refresh details Mixer.flp" }),
    );
    expect(screen.queryByText("Saved insert ID: 16")).toBeNull();
    await user.click(toggle);
    await act(async () => {
      finish(available);
      await Promise.resolve();
    });
    expect(
      screen.queryByRole("region", { name: "Saved mixer inserts" }),
    ).toBeNull();
    expect(adapter.requestProjectAnalysis).not.toHaveBeenCalled();
  });
});

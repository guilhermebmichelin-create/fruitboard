import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import axe from "axe-core";
import { describe, expect, it } from "vitest";
import { ProjectPatterns } from "./SavedPatterns";
import type { ProjectPatterns as Patterns } from "./projectPatterns";

describe("saved patterns", () => {
  it("shows counts separately from placements and names as inert, escaped text", async () => {
    const view = render(
      <ProjectPatterns
        patterns={{
          state: "available",
          count: 3,
          items: [
            { patternId: 1, name: { status: "extracted", value: "" } },
            { patternId: 200, name: { status: "unavailable", value: null } },
            {
              patternId: 65535,
              name: {
                status: "extracted",
                value: "<img src=x onerror=alert(1)>\u202e\n",
              },
            },
          ],
        }}
      />,
    );
    expect(screen.getByText("Showing 3 of 3 saved patterns.")).toBeTruthy();
    expect(
      screen.getByText(/distinct stored patterns, not playlist placements/),
    ).toBeTruthy();
    expect(screen.getByText("Empty saved name")).toBeTruthy();
    const missing = screen
      .getByRole("heading", { name: "Pattern 200" })
      .closest("li")!;
    expect(within(missing).getByText("No saved name")).toBeTruthy();
    expect(within(missing).getByText("Unavailable")).toBeTruthy();
    expect(
      screen.getByText("<img src=x onerror=alert(1)>\\u202e\\u000a"),
    ).toBeTruthy();
    expect(view.container.querySelector("img")).toBeNull();
    expect(
      (
        await axe.run(view.container, {
          rules: { "color-contrast": { enabled: false } },
        })
      ).violations,
    ).toEqual([]);
  });
  it("keeps every stored ID reachable with keyboard pagination and focus", async () => {
    const user = userEvent.setup();
    const patterns: Patterns = {
      state: "available",
      count: 45,
      items: Array.from({ length: 45 }, (_, index) => ({
        patternId: index * 2 + 1,
        name: { status: "extracted", value: `Name ${index}` },
      })),
    };
    render(<ProjectPatterns patterns={patterns} />);
    expect(screen.getAllByRole("listitem")).toHaveLength(20);
    const control = screen.getByRole("button", {
      name: "Show next 20 patterns",
    });
    control.focus();
    await user.keyboard("{Enter}");
    expect(screen.getAllByRole("listitem")).toHaveLength(40);
    expect(document.activeElement).toBe(control);
    await user.keyboard("{Enter}");
    expect(screen.getByRole("heading", { name: "Pattern 89" })).toBeTruthy();
    expect(control.textContent).toBe("Show fewer patterns");
    await user.keyboard("{Enter}");
    expect(screen.getAllByRole("listitem")).toHaveLength(20);
    expect(document.activeElement).toBe(control);
  });
  it("explains old results and unknown defaults without inventing zero or one", () => {
    const view = render(<ProjectPatterns />);
    expect(screen.getByText(/new analysis is needed/)).toBeTruthy();
    view.rerender(
      <ProjectPatterns
        patterns={{ state: "unavailable", reason: "no_stored_patterns" }}
      />,
    );
    expect(screen.getByText(/saved count is unknown/)).toBeTruthy();
    expect(screen.queryByRole("list")).toBeNull();
    view.rerender(
      <ProjectPatterns
        patterns={{ state: "unsupported", reason: "unverified_build" }}
      />,
    );
    expect(
      screen.getByText(/not verified for this saved FL Studio build/),
    ).toBeTruthy();
  });
});

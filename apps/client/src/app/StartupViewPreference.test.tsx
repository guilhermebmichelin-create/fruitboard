import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { PlatformPort } from "../platform/contracts";
import { createFakePlatform } from "../platform/fake";
import { StartupViewPreferenceControl } from "./StartupViewPreference";

describe("StartupViewPreferenceControl", () => {
  it("loads the safe Home default without enabling an unnecessary save", async () => {
    render(<StartupViewPreferenceControl platform={createFakePlatform()} />);

    const select = await screen.findByRole("combobox");
    const saveButton = screen.getByRole("button", {
      name: "Save preference",
    });
    if (
      !(select instanceof HTMLSelectElement) ||
      !(saveButton instanceof HTMLButtonElement)
    ) {
      throw new Error("preference controls should use native form elements");
    }
    expect(select.value).toBe("home");
    expect(screen.getByText("Home is the default startup view.")).toBeTruthy();
    expect(saveButton.disabled).toBe(true);
  });

  it("saves a valid selection with the keyboard through the platform port", async () => {
    const user = userEvent.setup();
    const platform = createFakePlatform();
    render(<StartupViewPreferenceControl platform={platform} />);
    const select = await screen.findByRole("combobox", {
      name: "Open Fruitboard on",
    });

    await user.selectOptions(select, "library");
    await user.tab();
    expect(document.activeElement).toBe(
      screen.getByRole("button", { name: "Save preference" }),
    );
    await user.keyboard("{Enter}");

    expect((await screen.findByRole("status")).textContent).toContain(
      "Startup view saved.",
    );
    await expect(platform.getStartupView()).resolves.toEqual({
      startupView: "library",
    });
  });

  it("shows loading and a retryable, diagnostic-free load error", async () => {
    const user = userEvent.setup();
    const getStartupView = vi
      .fn()
      .mockRejectedValueOnce(
        new Error("C:\\Users\\producer\\private-master.flp"),
      )
      .mockResolvedValueOnce({ startupView: "preferences" });
    const platform: PlatformPort = {
      getAppHealth: vi.fn(),
      getStartupView,
      setStartupView: vi.fn(),
    };
    render(<StartupViewPreferenceControl platform={platform} />);

    expect(screen.getByRole("status").textContent).toContain(
      "Loading your preference",
    );
    expect((await screen.findByRole("alert")).textContent).toContain(
      "Your existing preference was not changed.",
    );
    expect(document.body.textContent).not.toContain("private-master.flp");

    await user.click(screen.getByRole("button", { name: "Try again" }));

    const select = await screen.findByRole("combobox");
    if (!(select instanceof HTMLSelectElement)) {
      throw new Error("startup preference should use a native select");
    }
    expect(select.value).toBe("preferences");
    expect(getStartupView).toHaveBeenCalledTimes(2);
  });

  it("keeps the draft selected when saving fails", async () => {
    const user = userEvent.setup();
    const platform: PlatformPort = {
      getAppHealth: vi.fn(),
      getStartupView: () => Promise.resolve({ startupView: "home" }),
      setStartupView: () =>
        Promise.reject(new Error("Bearer private diagnostic")),
    };
    render(<StartupViewPreferenceControl platform={platform} />);
    const select = await screen.findByRole("combobox");

    await user.selectOptions(select, "board");
    await user.click(screen.getByRole("button", { name: "Save preference" }));

    expect((await screen.findByRole("alert")).textContent).toContain(
      "Fruitboard could not save this preference. Try again.",
    );
    expect((select as HTMLSelectElement).value).toBe("board");
    expect(document.body.textContent).not.toContain("Bearer");
  });
});

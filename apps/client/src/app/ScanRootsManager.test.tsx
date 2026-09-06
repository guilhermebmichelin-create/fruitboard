import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { PlatformError } from "../platform/contracts";
import type { PlatformPort } from "../platform/contracts";
import {
  createFakePlatform,
  createFakePlatformWithPickedDirectory,
} from "../platform/fake";
import { ScanRootsManager } from "./ScanRootsManager";

describe("ScanRootsManager", () => {
  it("shows an empty state with folder management copy", async () => {
    render(<ScanRootsManager platform={createFakePlatform()} />);

    expect(
      await screen.findByRole("heading", { level: 2, name: "Scan roots" }),
    ).toBeTruthy();
    expect(screen.getByText("No folders yet.", { exact: false }));
    expect(
      screen.getByText(/never deletes files/, { exact: false }),
    ).toBeTruthy();
  });

  it("adds a picked folder with a derived display name", async () => {
    const user = userEvent.setup();
    const platform = createFakePlatformWithPickedDirectory(
      "C:\\Music\\Projects",
    );
    render(<ScanRootsManager platform={platform} />);

    await screen.findByRole("button", { name: "Add folder" });
    await user.click(screen.getByRole("button", { name: "Add folder" }));

    expect(await screen.findByText("Projects")).toBeTruthy();
    expect(screen.getByText("C:\\Music\\Projects")).toBeTruthy();
    expect(screen.getByText("Folder added.")).toBeTruthy();
  });

  it("treats picker cancellation as an ordinary no-change outcome", async () => {
    const user = userEvent.setup();
    const platform = createFakePlatformWithPickedDirectory(null);
    const listScanRoots = vi.spyOn(platform, "listScanRoots");
    render(<ScanRootsManager platform={platform} />);

    await screen.findByRole("button", { name: "Add folder" });
    const calls = listScanRoots.mock.calls.length;
    await user.click(screen.getByRole("button", { name: "Add folder" }));

    expect(screen.getByText("No folders yet.", { exact: false }));
    expect(screen.queryByRole("alert")).toBeNull();
    expect(listScanRoots.mock.calls.length).toBe(calls);
  });

  it("explains duplicate roots without exposing diagnostics", async () => {
    const user = userEvent.setup();
    const platform = createFakePlatformWithPickedDirectory("C:\\Music");
    const conflict = new PlatformError("conflict");
    const addScanRoot = vi
      .spyOn(platform, "addScanRoot")
      .mockRejectedValue(conflict);
    render(<ScanRootsManager platform={platform} />);

    await screen.findByRole("button", { name: "Add folder" });
    await user.click(screen.getByRole("button", { name: "Add folder" }));

    expect((await screen.findByRole("alert")).textContent).toContain(
      "already tracked or overlaps",
    );
    expect(addScanRoot).toHaveBeenCalledOnce();
  });

  it("removes only after an explicit confirmation step", async () => {
    const user = userEvent.setup();
    const platform = createFakePlatformWithPickedDirectory(
      "C:\\Music\\Projects",
    );
    render(<ScanRootsManager platform={platform} />);
    await screen.findByRole("button", { name: "Add folder" });
    await user.click(screen.getByRole("button", { name: "Add folder" }));
    await screen.findByText("Projects");

    await user.click(screen.getByRole("button", { name: "Remove Projects" }));
    expect(
      screen.getByRole("button", { name: "Confirm removal of Projects" }),
    ).toBeTruthy();
    expect(screen.queryByText("Projects")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Keep Projects" }));
    expect(
      screen.queryByRole("button", { name: "Confirm removal of Projects" }),
    ).toBeNull();
    expect(screen.getByText("Projects")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Remove Projects" }));
    await user.click(
      screen.getByRole("button", { name: "Confirm removal of Projects" }),
    );

    expect(await screen.findByText("Folder removed.")).toBeTruthy();
    expect(screen.queryByText("Projects")).toBeNull();
  });

  it("reports a stale list honestly when refresh fails after adding", async () => {
    const user = userEvent.setup();
    const platform = createFakePlatformWithPickedDirectory(
      "C:\\Music\\Projects",
    );
    render(<ScanRootsManager platform={platform} />);

    await screen.findByRole("button", { name: "Add folder" });
    vi.spyOn(platform, "listScanRoots").mockRejectedValueOnce(
      new Error("storage busy"),
    );
    await user.click(screen.getByRole("button", { name: "Add folder" }));

    expect((await screen.findByRole("alert")).textContent).toContain(
      "Saved, but the list could not be refreshed.",
    );
    expect(
      (await platform.listScanRoots()).map((root) => root.displayName),
    ).toEqual(["Projects"]);
  });

  it("reports a stale list honestly when refresh fails after removing", async () => {
    const user = userEvent.setup();
    const platform = createFakePlatformWithPickedDirectory(
      "C:\\Music\\Projects",
    );
    render(<ScanRootsManager platform={platform} />);
    await screen.findByRole("button", { name: "Add folder" });
    await user.click(screen.getByRole("button", { name: "Add folder" }));
    await screen.findByText("Projects");

    vi.spyOn(platform, "listScanRoots").mockRejectedValueOnce(
      new Error("storage busy"),
    );
    await user.click(screen.getByRole("button", { name: "Remove Projects" }));
    await user.click(
      screen.getByRole("button", { name: "Confirm removal of Projects" }),
    );

    expect((await screen.findByRole("alert")).textContent).toContain(
      "Saved, but the list could not be refreshed.",
    );
    await expect(platform.listScanRoots()).resolves.toEqual([]);
  });

  it("shows a retryable load error without changing folders", async () => {
    const user = userEvent.setup();
    const listScanRoots = vi
      .fn()
      .mockRejectedValueOnce(new Error("C:\\Private\\roots.db"))
      .mockResolvedValueOnce([]);
    const platform: PlatformPort = {
      ...createFakePlatform(),
      listScanRoots,
    };
    render(<ScanRootsManager platform={platform} />);

    expect((await screen.findByRole("alert")).textContent).toContain(
      "Your folders were not changed.",
    );
    expect(document.body.textContent).not.toContain("C:\\Private");

    await user.click(screen.getByRole("button", { name: "Try again" }));
    expect(
      await screen.findByText("No folders yet.", { exact: false }),
    ).toBeTruthy();
    expect(listScanRoots).toHaveBeenCalledTimes(2);
  });

  it("onboards with honest copy that never claims scanning", async () => {
    render(<ScanRootsManager platform={createFakePlatform()} />);

    expect(
      await screen.findByText(/only listed for now/, { exact: false }),
    ).toBeTruthy();
    expect(screen.getByText("Turn folders off anytime without losing them."));
    expect(screen.queryByText(/scan complete|scanning now/i)).toBeNull();
  });

  it("shows availability without implying a completed scan", async () => {
    const user = userEvent.setup();
    const platform = createFakePlatformWithPickedDirectory(
      "C:\\Music\\Projects",
    );
    render(<ScanRootsManager platform={platform} />);
    await screen.findByRole("button", { name: "Add folder" });
    await user.click(screen.getByRole("button", { name: "Add folder" }));

    expect(
      await screen.findByText("Availability not rechecked · Not scanned yet"),
    ).toBeTruthy();
    expect(screen.queryByText(/ready/i)).toBeNull();
  });

  it("renames a root and keeps the new name after reload", async () => {
    const user = userEvent.setup();
    const platform = createFakePlatformWithPickedDirectory(
      "C:\\Music\\Projects",
    );
    render(<ScanRootsManager platform={platform} />);
    await screen.findByRole("button", { name: "Add folder" });
    await user.click(screen.getByRole("button", { name: "Add folder" }));
    await screen.findByText("Projects");

    await user.click(screen.getByRole("button", { name: "Rename Projects" }));
    const input = screen.getByLabelText("Folder name");
    if (!(input instanceof HTMLInputElement)) {
      throw new Error("rename should use a native input");
    }
    expect(input.value).toBe("Projects");
    await user.clear(input);
    await user.type(input, "Released");
    await user.click(screen.getByRole("button", { name: "Save name" }));

    expect(await screen.findByText("Released")).toBeTruthy();
    expect(await screen.findByText("Name saved.")).toBeTruthy();
    expect(screen.queryByText("Projects")).toBeNull();
  });

  it("preserves the rename draft when saving fails", async () => {
    const user = userEvent.setup();
    const platform = createFakePlatformWithPickedDirectory(
      "C:\\Music\\Projects",
    );
    const updateScanRootDisplayName = vi
      .spyOn(platform, "updateScanRootDisplayName")
      .mockRejectedValue(new PlatformError("unavailable"));
    render(<ScanRootsManager platform={platform} />);
    await screen.findByRole("button", { name: "Add folder" });
    await user.click(screen.getByRole("button", { name: "Add folder" }));
    await screen.findByText("Projects");

    await user.click(screen.getByRole("button", { name: "Rename Projects" }));
    const input = screen.getByLabelText("Folder name");
    await user.clear(input);
    await user.type(input, "Draft name");
    await user.click(screen.getByRole("button", { name: "Save name" }));

    expect((await screen.findByRole("alert")).textContent).toContain(
      "could not save that name",
    );
    const draft = screen.getByLabelText("Folder name");
    if (!(draft instanceof HTMLInputElement)) {
      throw new Error("rename should use a native input");
    }
    expect(draft.value).toBe("Draft name");
    expect(updateScanRootDisplayName).toHaveBeenCalledOnce();
  });

  it("toggles a root off and on through the platform port", async () => {
    const user = userEvent.setup();
    const platform = createFakePlatformWithPickedDirectory(
      "C:\\Music\\Projects",
    );
    render(<ScanRootsManager platform={platform} />);
    await screen.findByRole("button", { name: "Add folder" });
    await user.click(screen.getByRole("button", { name: "Add folder" }));
    await screen.findByText("Projects");

    const toggle = screen.getByRole("checkbox", { name: "Projects enabled" });
    if (!(toggle instanceof HTMLInputElement)) {
      throw new Error("enabled should use a native checkbox");
    }
    expect(toggle.checked).toBe(true);

    await user.click(toggle);
    expect(
      await screen.findByRole("checkbox", { name: "Projects enabled" }),
    ).toHaveProperty("checked", false);

    await user.click(
      screen.getByRole("checkbox", { name: "Projects enabled" }),
    );
    expect(
      await screen.findByRole("checkbox", { name: "Projects enabled" }),
    ).toHaveProperty("checked", true);
  });

  it("moves focus to Add folder after a removal", async () => {
    const user = userEvent.setup();
    const platform = createFakePlatformWithPickedDirectory(
      "C:\\Music\\Projects",
    );
    render(<ScanRootsManager platform={platform} />);
    await screen.findByRole("button", { name: "Add folder" });
    await user.click(screen.getByRole("button", { name: "Add folder" }));
    await screen.findByText("Projects");

    await user.click(screen.getByRole("button", { name: "Remove Projects" }));
    await user.click(
      screen.getByRole("button", { name: "Confirm removal of Projects" }),
    );
    await screen.findByText("Folder removed.");

    await waitFor(() => {
      expect(document.activeElement).toBe(
        screen.getByRole("button", { name: "Add folder" }),
      );
    });
  });

  it("reports a stale list when the toggle succeeds but refresh fails", async () => {
    const user = userEvent.setup();
    const platform = createFakePlatformWithPickedDirectory(
      "C:\\Music\\Projects",
    );
    render(<ScanRootsManager platform={platform} />);
    await screen.findByRole("button", { name: "Add folder" });
    await user.click(screen.getByRole("button", { name: "Add folder" }));
    await screen.findByText("Projects");

    vi.spyOn(platform, "listScanRoots").mockRejectedValueOnce(
      new Error("storage busy"),
    );
    await user.click(
      screen.getByRole("checkbox", { name: "Projects enabled" }),
    );

    expect((await screen.findByRole("alert")).textContent).toContain(
      "Saved, but the list could not be refreshed.",
    );
    await expect(platform.listScanRoots()).resolves.toEqual([
      expect.objectContaining({ displayName: "Projects", enabled: false }),
    ]);
  });

  it("reports a toggle mutation failure without changing the checkbox", async () => {
    const user = userEvent.setup();
    const platform = createFakePlatformWithPickedDirectory(
      "C:\\Music\\Projects",
    );
    vi.spyOn(platform, "setScanRootEnabled").mockRejectedValue(
      new PlatformError("unavailable"),
    );
    render(<ScanRootsManager platform={platform} />);
    await screen.findByRole("button", { name: "Add folder" });
    await user.click(screen.getByRole("button", { name: "Add folder" }));
    await screen.findByText("Projects");

    await user.click(
      screen.getByRole("checkbox", { name: "Projects enabled" }),
    );

    expect((await screen.findByRole("alert")).textContent).toContain(
      "could not save that setting",
    );
    expect(
      screen.getByRole("checkbox", { name: "Projects enabled" }),
    ).toHaveProperty("checked", true);
  });

  it("focuses the editor on Rename and restores focus on save and cancel", async () => {
    const user = userEvent.setup();
    const platform = createFakePlatformWithPickedDirectory(
      "C:\\Music\\Projects",
    );
    render(<ScanRootsManager platform={platform} />);
    await screen.findByRole("button", { name: "Add folder" });
    await user.click(screen.getByRole("button", { name: "Add folder" }));
    await screen.findByText("Projects");

    await user.click(screen.getByRole("button", { name: "Rename Projects" }));
    const input = await screen.findByLabelText("Folder name");
    expect(document.activeElement).toBe(input);

    await user.clear(input);
    await user.type(input, "Released");
    await user.click(screen.getByRole("button", { name: "Save name" }));
    await screen.findByText("Released");
    await waitFor(() => {
      expect(document.activeElement).toBe(
        screen.getByRole("button", { name: "Rename Released" }),
      );
    });

    await user.click(screen.getByRole("button", { name: "Rename Released" }));
    await screen.findByLabelText("Folder name");
    await user.keyboard("{Escape}");
    await waitFor(() => {
      expect(document.activeElement).toBe(
        screen.getByRole("button", { name: "Rename Released" }),
      );
    });
  });

  it("gives repeated root controls distinguishable names", async () => {
    const user = userEvent.setup();
    const platform = createFakePlatform();
    await platform.addScanRoot("Projects", "C:\\Music\\Projects");
    await platform.addScanRoot("Loops", "C:\\Music\\Loops");
    render(<ScanRootsManager platform={platform} />);

    expect(
      await screen.findByRole("button", { name: "Remove Projects" }),
    ).toBeTruthy();
    expect(screen.getByRole("button", { name: "Remove Loops" })).toBeTruthy();
    expect(
      screen.getByRole("button", { name: "Rename Projects" }),
    ).toBeTruthy();
    expect(
      screen.getByRole("checkbox", { name: "Loops enabled" }),
    ).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Remove Loops" }));
    await user.click(
      screen.getByRole("button", { name: "Confirm removal of Loops" }),
    );
    expect(await screen.findByText("Folder removed.")).toBeTruthy();
    expect(screen.queryByText("Loops")).toBeNull();
    expect(screen.getByText("Projects")).toBeTruthy();
  });

  it("completes the add flow by keyboard alone", async () => {
    const user = userEvent.setup();
    const platform = createFakePlatformWithPickedDirectory(
      "C:\\Music\\Projects",
    );
    render(<ScanRootsManager platform={platform} />);

    await user.tab();
    expect(document.activeElement).toBe(
      screen.getByRole("button", { name: "Add folder" }),
    );
    await user.keyboard("{Enter}");
    expect(await screen.findByText("Projects")).toBeTruthy();
    expect(await screen.findByText("Folder added.")).toBeTruthy();
    expect(document.activeElement).toBe(
      screen.getByRole("button", { name: "Add folder" }),
    );

    await user.tab({ shift: true });
    expect(document.activeElement).toBe(
      screen.getByRole("button", { name: "Remove Projects" }),
    );
    await user.tab({ shift: true });
    expect(document.activeElement).toBe(
      screen.getByRole("button", { name: "Rename Projects" }),
    );
    await user.tab({ shift: true });
    expect(document.activeElement).toBe(
      screen.getByRole("checkbox", { name: "Projects enabled" }),
    );
  });
});

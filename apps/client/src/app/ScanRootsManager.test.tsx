import { render, screen } from "@testing-library/react";
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
    expect(screen.getByText("No folders yet. Add one to get started."));
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

    expect(screen.getByText("No folders yet. Add one to get started."));
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
    await user.click(screen.getByRole("button", { name: "Add folder" }));
    await screen.findByText("Projects");

    await user.click(screen.getByRole("button", { name: "Remove" }));
    expect(screen.getByRole("button", { name: "Confirm remove" })).toBeTruthy();
    expect(screen.queryByText("Projects")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Keep" }));
    expect(screen.queryByRole("button", { name: "Confirm remove" })).toBeNull();
    expect(screen.getByText("Projects")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Remove" }));
    await user.click(screen.getByRole("button", { name: "Confirm remove" }));

    expect(await screen.findByText("Folder removed.")).toBeTruthy();
    expect(screen.queryByText("Projects")).toBeNull();
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
      await screen.findByText("No folders yet. Add one to get started."),
    ).toBeTruthy();
    expect(listScanRoots).toHaveBeenCalledTimes(2);
  });
});

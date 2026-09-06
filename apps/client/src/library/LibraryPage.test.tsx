import { MemoryRouter } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import type { ScanRoot } from "../platform/contracts";
import { createFakeLibraryScanAdapter } from "./fake";
import { LibraryPage } from "./LibraryPage";
import type { PublishedFileLocation } from "./contracts";

const rootA: ScanRoot = {
  id: "root-a",
  displayName: "Projects",
  canonicalPath: "C:\\Synthetic\\Music\\Projects",
  enabled: true,
  availability: "available",
  lastErrorCode: null,
};

const rootB: ScanRoot = {
  id: "root-b",
  displayName: "Projects",
  canonicalPath: "D:\\Synthetic\\Archive\\Projects",
  enabled: true,
  availability: "available",
  lastErrorCode: null,
};

const makeRecord = (
  root: ScanRoot,
  locationId: string,
  fileName: string,
  relativePath: string,
  presence: PublishedFileLocation["presence"] = "present",
): PublishedFileLocation => ({
  locationId,
  rootId: root.id,
  rootDisplayName: root.displayName,
  rootCanonicalPath: root.canonicalPath,
  fileName,
  relativePath,
  byteSize: 1024,
  modifiedAt: "2026-01-02T03:04:00.000Z",
  presence,
});

function renderLibrary(
  adapter: ReturnType<typeof createFakeLibraryScanAdapter>,
) {
  return render(
    <MemoryRouter>
      <LibraryPage adapter={adapter} />
    </MemoryRouter>,
  );
}

describe("LibraryPage", () => {
  it("renders the initial loading state before an empty committed dataset", async () => {
    const adapter = createFakeLibraryScanAdapter();
    const view = renderLibrary(adapter);

    expect(
      view.container.querySelector('[data-library-state="loading"]'),
    ).toBeTruthy();
    expect(
      screen.getByRole("heading", { name: "Loading your library" }),
    ).toBeTruthy();
    expect(
      await screen.findByRole("heading", { name: "No committed files yet" }),
    ).toBeTruthy();
    expect(
      view.container.querySelector('[data-library-state="empty"]'),
    ).toBeTruthy();
    expect(screen.getByText(/No scan roots are configured/)).toBeTruthy();
  });

  it("loads a bounded, stable page and navigates back without losing focus", async () => {
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA, rootB],
      pageLimit: 2,
      files: [
        makeRecord(rootA, "location-a", "First.flp", "First.flp"),
        makeRecord(rootB, "location-b", "Second.flp", "Second.flp"),
        makeRecord(rootB, "location-c", "Third.flp", "Third.flp", "missing"),
      ],
    });
    renderLibrary(adapter);

    expect(
      await screen.findByRole("heading", { name: "Your FLP library" }),
    ).toBeTruthy();
    expect(screen.getByRole("heading", { name: "First.flp" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Second.flp" })).toBeTruthy();
    expect(screen.getAllByText("Present")).toHaveLength(2);
    expect(screen.getAllByText("1,024 bytes")).toHaveLength(2);
    expect(screen.getAllByText(/Jan 2, 2026/)).toHaveLength(2);
    expect(screen.getAllByText("Relative path")).toHaveLength(2);
    expect(screen.queryByText("Missing")).toBeNull();
    expect(
      screen.getByLabelText("Root Projects (C:\\Synthetic\\Music\\Projects)"),
    ).toBeTruthy();
    expect(
      screen.getByLabelText("Root Projects (D:\\Synthetic\\Archive\\Projects)"),
    ).toBeTruthy();
    expect(adapter.calls.pages.every((limit) => limit <= 200)).toBe(true);

    const next = screen.getByRole("button", { name: "Next library page" });
    await userEvent.setup().click(next);
    expect(
      await screen.findByRole("heading", { name: "Third.flp" }),
    ).toBeTruthy();
    expect(screen.getByText("Missing")).toBeTruthy();
    await waitFor(() => {
      expect(document.activeElement).toBe(
        screen.getByRole("heading", { name: "File locations" }),
      );
    });

    await userEvent
      .setup()
      .click(screen.getByRole("button", { name: "Previous library page" }));
    expect(
      await screen.findByRole("heading", { name: "First.flp" }),
    ).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "Third.flp" })).toBeNull();
    await waitFor(() => {
      expect(document.activeElement).toBe(
        screen.getByRole("heading", { name: "File locations" }),
      );
    });
  });

  it("keeps committed results separate while a fake scan runs and cancels", async () => {
    const user = userEvent.setup();
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA],
      files: [
        makeRecord(rootA, "location-a", "Committed.flp", "Committed.flp"),
      ],
    });
    renderLibrary(adapter);
    await screen.findByRole("heading", { name: "Committed.flp" });

    await user.click(screen.getByRole("button", { name: "Scan now Projects" }));
    expect(await screen.findByText("Queued")).toBeTruthy();
    expect(screen.getByText("Total work unknown")).toBeTruthy();
    expect(
      screen.getByText(
        "Progress is shown as counters; no percentage is estimated.",
      ),
    ).toBeTruthy();
    expect(document.body.textContent).not.toMatch(/\d+%/);
    await waitFor(() => {
      expect(document.activeElement).toBe(
        screen.getByRole("button", { name: "Cancel scan Projects" }),
      );
    });

    adapter.advanceRun(rootA.id);
    expect(await screen.findByText("Running")).toBeTruthy();
    await user.click(
      screen.getByRole("button", { name: "Cancel scan Projects" }),
    );

    expect(await screen.findByText("Cancelled")).toBeTruthy();
    expect(screen.getByText("Showing previous committed results")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Committed.flp" })).toBeTruthy();
    expect(screen.getByText("Present")).toBeTruthy();
    await waitFor(() => {
      expect(document.activeElement).toBe(
        screen.getByRole("button", { name: "Retry scan Projects" }),
      );
    });
    expect(adapter.calls.scanNow).toEqual([rootA.id]);
    expect(adapter.calls.cancelScan).toEqual(["fake-run-1"]);
  });

  it("labels failed and interrupted runs while retaining the old page", async () => {
    const user = userEvent.setup();
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA],
      files: [makeRecord(rootA, "location-a", "Kept.flp", "Kept.flp")],
    });
    renderLibrary(adapter);
    await screen.findByRole("heading", { name: "Kept.flp" });

    await user.click(screen.getByRole("button", { name: "Scan now Projects" }));
    adapter.advanceRun(rootA.id);
    adapter.failScan(rootA.id, "access_denied");
    expect(await screen.findByText("Failed")).toBeTruthy();
    expect(
      screen.getByText(
        "The folder could not be read. Previous committed results were kept.",
      ),
    ).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Kept.flp" })).toBeTruthy();
    expect(screen.getByText("Showing previous committed results")).toBeTruthy();

    await user.click(
      screen.getByRole("button", { name: "Retry scan Projects" }),
    );
    expect(await screen.findByText("Queued")).toBeTruthy();
    expect(
      screen.getByRole("button", { name: "Cancel scan Projects" }),
    ).toBeTruthy();

    adapter.advanceRun(rootA.id);
    adapter.interruptScan(rootA.id);
    expect(await screen.findByText("Interrupted")).toBeTruthy();
    expect(
      screen.getByText(/previous committed results were kept/i),
    ).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Kept.flp" })).toBeTruthy();
  });

  it("shows an unavailable-root state without manufacturing missing files", async () => {
    const user = userEvent.setup();
    const unavailableRoot = { ...rootA, availability: "unavailable" as const };
    const adapter = createFakeLibraryScanAdapter({ roots: [unavailableRoot] });
    renderLibrary(adapter);

    expect(
      await screen.findByRole("heading", {
        name: "A scan root is unavailable",
      }),
    ).toBeTruthy();
    expect(screen.getAllByText("Unavailable")).toHaveLength(2);
    expect(screen.queryByText("Missing")).toBeNull();
    const retry = screen.getByRole("button", { name: "Retry scan Projects" });
    expect(retry).toHaveProperty("disabled", false);
    await user.click(retry);
    expect(
      await screen.findByText(
        "The folder is unavailable. Previous committed results were kept.",
      ),
    ).toBeTruthy();
  });

  it("recovers a page read error through the explicit retry action", async () => {
    const user = userEvent.setup();
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA],
      files: [
        makeRecord(rootA, "location-a", "Recovered.flp", "Recovered.flp"),
      ],
    });
    adapter.setPageError("unavailable");
    renderLibrary(adapter);

    expect(
      await screen.findByRole("heading", {
        name: "Library results unavailable",
      }),
    ).toBeTruthy();
    expect(screen.getByText(/No files were marked missing/)).toBeTruthy();
    adapter.setPageError(null);
    await user.click(screen.getByRole("button", { name: "Try again" }));
    expect(
      await screen.findByRole("heading", { name: "Recovered.flp" }),
    ).toBeTruthy();
  });
});

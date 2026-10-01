import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import axe from "axe-core";
import { describe, expect, it, vi } from "vitest";
import { ProjectDetailsPanel } from "./ProjectDetailsPanel";
import { createFakeLibraryScanAdapter } from "./fake";
import { identity, savedDetails } from "./projectDetails.fixture";
import type { PublishedFileLocation } from "./contracts";
import type { ProjectDetails } from "./projectDetails";
import { MemoryRouter } from "react-router";
import { LibraryPage } from "./LibraryPage";
import type { ScanRoot } from "../platform/contracts";

const record: PublishedFileLocation = {
  ...identity,
  rootDisplayName: "Projects",
  rootCanonicalPath: "C:\\Synthetic",
  fileName: "Example.flp",
  relativePath: "Example.flp",
  byteSize: "1024",
  modifiedAt: "2026-01-02T00:00:00Z",
  presence: "present",
};

describe("Project details panel", () => {
  it("clears displayed facts when the selected root becomes disabled", async () => {
    const user = userEvent.setup();
    const root: ScanRoot = {
      id: identity.rootId,
      displayName: "Projects",
      canonicalPath: record.rootCanonicalPath,
      mode: "localNtfs",
      enabled: true,
      availability: "available",
      lastErrorCode: null,
    };
    const fake = createFakeLibraryScanAdapter({
      roots: [root],
      files: [record],
      projectDetails: { [record.locationId]: savedDetails() },
    });
    let enabled = true;
    const adapter = {
      ...fake,
      listScanStatuses: () =>
        fake.listScanStatuses().then((statuses) =>
          statuses.map((status) => ({
            ...status,
            root: { ...status.root, enabled },
          })),
        ),
    };
    render(
      <MemoryRouter>
        <LibraryPage adapter={adapter} />
      </MemoryRouter>,
    );
    await user.click(
      await screen.findByRole("button", {
        name: "Project details Example.flp",
      }),
    );
    await screen.findByText("120 BPM");
    enabled = false;
    act(() => {
      window.dispatchEvent(new Event("focus"));
    });
    await waitFor(() => expect(screen.queryByText("120 BPM")).toBeNull());
    expect(
      screen
        .getByRole("button", { name: "Project details Example.flp" })
        .getAttribute("aria-expanded"),
    ).toBe("false");
  });
  it("reads on expansion, labels estimates and preserves keyboard focus", async () => {
    const user = userEvent.setup();
    const read = vi.fn().mockResolvedValue(savedDetails());
    const adapter = {
      ...createFakeLibraryScanAdapter(),
      getProjectDetails: read,
    };
    const view = render(
      <ProjectDetailsPanel adapter={adapter} record={record} />,
    );
    const toggle = screen.getByRole("button", {
      name: "Project details Example.flp",
    });
    expect(read).not.toHaveBeenCalled();
    toggle.focus();
    await user.keyboard("{Enter}");
    await screen.findByText("120 BPM");
    expect(document.activeElement).toBe(toggle);
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    expect(screen.getAllByText("Inferred")).toHaveLength(2);
    expect(screen.getByText(/not the finished song/)).toBeTruthy();
    expect(screen.getByText(/has not measured your work time/)).toBeTruthy();
    const accessibility = await axe.run(view.container, {
      rules: { "color-contrast": { enabled: false } },
    });
    expect(accessibility.violations).toEqual([]);
    expect(accessibility.incomplete).toEqual([]);
    await user.keyboard("{Enter}");
    expect(screen.queryByText("120 BPM")).toBeNull();
    expect(document.activeElement).toBe(toggle);
  });

  it.each(["disabled", "no_current"] as const)(
    "explains %s without invented values",
    async (state) => {
      const user = userEvent.setup();
      const adapter = {
        ...createFakeLibraryScanAdapter(),
        getProjectDetails: () =>
          Promise.resolve<ProjectDetails>({ ...identity, state }),
      };
      render(<ProjectDetailsPanel adapter={adapter} record={record} />);
      await user.click(
        screen.getByRole("button", { name: "Project details Example.flp" }),
      );
      await screen.findByText(
        state === "disabled"
          ? /unavailable in this version/
          : /No current saved project details/,
      );
      expect(screen.queryByText("120 BPM")).toBeNull();
    },
  );

  it("contains failed reads, refreshes safely and ignores a late closed response", async () => {
    const user = userEvent.setup();
    let finish!: (v: ProjectDetails) => void;
    const read = vi
      .fn()
      .mockRejectedValueOnce(new Error("private path"))
      .mockImplementationOnce(
        () =>
          new Promise<ProjectDetails>((resolve) => {
            finish = resolve;
          }),
      )
      .mockResolvedValue(savedDetails());
    render(
      <ProjectDetailsPanel
        adapter={{ ...createFakeLibraryScanAdapter(), getProjectDetails: read }}
        record={record}
      />,
    );
    const toggle = screen.getByRole("button", {
      name: "Project details Example.flp",
    });
    await user.click(toggle);
    await screen.findByRole("alert");
    expect(screen.queryByText("private path")).toBeNull();
    await user.click(
      screen.getByRole("button", {
        name: "Refresh project details Example.flp",
      }),
    );
    await screen.findByRole("status");
    await user.click(toggle);
    await act(async () => {
      finish(savedDetails());
      await Promise.resolve();
    });
    expect(screen.queryByText("120 BPM")).toBeNull();
    await user.click(toggle);
    await screen.findByText("120 BPM");
    expect(read).toHaveBeenCalledTimes(3);
  });

  it("does not request metadata for missing files", async () => {
    const user = userEvent.setup();
    const read = vi.fn().mockResolvedValue(savedDetails());
    const adapter = {
      ...createFakeLibraryScanAdapter(),
      getProjectDetails: read,
    };
    render(
      <ProjectDetailsPanel
        adapter={adapter}
        record={{ ...record, presence: "missing" }}
      />,
    );
    await user.click(
      screen.getByRole("button", { name: "Project details Example.flp" }),
    );
    await screen.findByText(/No current saved/);
    expect(read).not.toHaveBeenCalled();
  });

  it("clears a changed Library row and ignores a pending reply for its earlier version", async () => {
    const user = userEvent.setup();
    const root: ScanRoot = {
      id: identity.rootId,
      displayName: "Projects",
      canonicalPath: record.rootCanonicalPath,
      mode: "localNtfs",
      enabled: true,
      availability: "available",
      lastErrorCode: null,
    };
    let currentRecord = record;
    let finish!: (details: ProjectDetails) => void;
    const read = vi
      .fn()
      .mockResolvedValueOnce(savedDetails())
      .mockImplementationOnce(
        () =>
          new Promise<ProjectDetails>((resolve) => {
            finish = resolve;
          }),
      )
      .mockResolvedValue({ ...identity, state: "no_current" });
    const adapter = {
      ...createFakeLibraryScanAdapter({ roots: [root], files: [record] }),
      getLibraryPage: () =>
        Promise.resolve({
          rootId: identity.rootId,
          snapshotId: `snapshot-${currentRecord.byteSize}`,
          records: [currentRecord],
          nextCursor: null,
        }),
      getProjectDetails: read,
    };
    render(
      <MemoryRouter>
        <LibraryPage adapter={adapter} />
      </MemoryRouter>,
    );
    await user.click(
      await screen.findByRole("button", {
        name: "Project details Example.flp",
      }),
    );
    await screen.findByText("120 BPM");
    await user.click(
      screen.getByRole("button", {
        name: "Refresh project details Example.flp",
      }),
    );
    await screen.findByText("Loading saved project details…");
    currentRecord = { ...record, byteSize: "2048" };
    act(() => {
      window.dispatchEvent(new Event("focus"));
    });
    await screen.findByText("2,048 bytes");
    await act(async () => {
      finish(savedDetails());
      await Promise.resolve();
    });
    await waitFor(() => expect(screen.queryByText("120 BPM")).toBeNull());
    expect(
      screen
        .getByRole("button", { name: "Project details Example.flp" })
        .getAttribute("aria-expanded"),
    ).toBe("false");
    await user.click(
      screen.getByRole("button", { name: "Project details Example.flp" }),
    );
    await screen.findByText(/No current saved/);
    expect(read).toHaveBeenLastCalledWith({
      ...identity,
      byteSize: "2048",
      modifiedAt: record.modifiedAt,
    });
  });
});

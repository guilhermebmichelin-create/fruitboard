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
import { ANALYSIS_LABELS, type ProjectAnalysis } from "./projectAnalysis";

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
  it("clears saved patterns while refreshing and never queues analysis through Refresh details", async () => {
    const diagnostics = vi.spyOn(console, "error").mockImplementation(() => {});
    const user = userEvent.setup();
    const available = {
      ...savedDetails(),
      patternNoteCounts: {
        state: "available" as const,
        items: [
          {
            patternId: 200,
            noteCount: { status: "extracted" as const, value: 314 },
          },
        ],
      },
      patterns: {
        state: "available" as const,
        count: 1,
        items: [
          {
            patternId: 200,
            name: { status: "extracted" as const, value: "Fixture Pattern" },
          },
        ],
      },
    };
    let finish!: (details: ProjectDetails) => void;
    const read = vi
      .fn()
      .mockResolvedValueOnce(available)
      .mockImplementationOnce(
        () =>
          new Promise<ProjectDetails>((resolve) => {
            finish = resolve;
          }),
      );
    const adapter = {
      ...createFakeLibraryScanAdapter(),
      getProjectDetails: read,
      requestProjectAnalysis: vi.fn(),
    };
    render(<ProjectDetailsPanel record={record} adapter={adapter} />);
    await user.click(
      screen.getByRole("button", { name: "Project details Example.flp" }),
    );
    await screen.findByText("Fixture Pattern");
    expect(screen.getByText("314")).toBeTruthy();
    await user.click(
      screen.getByRole("button", {
        name: "Refresh project details Example.flp",
      }),
    );
    expect(screen.queryByText("Fixture Pattern")).toBeNull();
    expect(screen.queryByText("314")).toBeNull();
    await act(async () => {
      finish({ ...identity, state: "no_current" });
      await Promise.resolve();
    });
    await screen.findByText(/No current saved project details/);
    expect(screen.queryByRole("region", { name: "Saved patterns" })).toBeNull();
    expect(adapter.requestProjectAnalysis).not.toHaveBeenCalled();
    expect(diagnostics).not.toHaveBeenCalled();
    diagnostics.mockRestore();
  });
  it.each([
    "not_reported",
    "not_current",
    "queued",
    "running",
    "complete",
    "unsupported",
    "failed",
    "cancelled",
    "stale",
  ] as const)(
    "explains the recorded %s state without inventing facts",
    async (state) => {
      const user = userEvent.setup();
      const analysis: ProjectAnalysis = {
        state,
        attempts:
          state === "not_reported" || state === "not_current" ? null : 1,
        reason: null,
      };
      render(
        <ProjectDetailsPanel
          record={record}
          adapter={{
            ...createFakeLibraryScanAdapter(),
            getProjectDetails: () =>
              Promise.resolve({ ...identity, state: "no_current", analysis }),
          }}
        />,
      );
      await user.click(
        screen.getByRole("button", { name: "Project details Example.flp" }),
      );
      await screen.findByText(ANALYSIS_LABELS[state]);
      expect(screen.queryByText("120 BPM")).toBeNull();
      expect(
        screen.getByText(/does not start another analysis attempt/),
      ).toBeTruthy();
      if (analysis.attempts === null)
        expect(screen.queryByText(/Attempts started/)).toBeNull();
      else expect(screen.getByText("Attempts started: 1 of 3.")).toBeTruthy();
    },
  );
  it("refreshes waiting status into a result and keeps valid facts alongside a later problem", async () => {
    const user = userEvent.setup();
    const read = vi
      .fn()
      .mockResolvedValueOnce({
        ...identity,
        state: "no_current",
        analysis: { state: "queued", attempts: 0, reason: null },
      })
      .mockResolvedValueOnce(savedDetails())
      .mockResolvedValueOnce({
        ...savedDetails(),
        analysis: {
          state: "failed",
          attempts: 3,
          reason: "parser_unavailable",
        },
      });
    render(
      <ProjectDetailsPanel
        record={record}
        adapter={{ ...createFakeLibraryScanAdapter(), getProjectDetails: read }}
      />,
    );
    await user.click(
      screen.getByRole("button", { name: "Project details Example.flp" }),
    );
    await screen.findByText("Waiting for analysis");
    const refresh = screen.getByRole("button", {
      name: "Refresh project details Example.flp",
    });
    await user.click(refresh);
    await screen.findByText("Analysis finished");
    await screen.findByText("120 BPM");
    expect(
      screen.getByText(/Some saved events are not understood/),
    ).toBeTruthy();
    await user.click(refresh);
    await screen.findByText("Analysis failed");
    expect(screen.getByText("120 BPM")).toBeTruthy();
    expect(
      screen.getByText(/separate from this recorded attempt/),
    ).toBeTruthy();
    expect(screen.getByText(/could not complete its response/)).toBeTruthy();
    expect(read).toHaveBeenCalledTimes(3);
  });
  it("clears details on a new scan snapshot even when displayed file attributes are identical", async () => {
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
    let generation = 1;
    let finish!: (details: ProjectDetails) => void;
    const read = vi
      .fn()
      .mockResolvedValueOnce(savedDetails())
      .mockImplementationOnce(
        () =>
          new Promise<ProjectDetails>((resolve) => {
            finish = resolve;
          }),
      );
    const adapter = {
      ...createFakeLibraryScanAdapter({ roots: [root], files: [record] }),
      getLibraryPage: () =>
        Promise.resolve({
          rootId: identity.rootId,
          snapshotId: `snapshot-${generation}`,
          records: [record],
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
    generation = 2;
    act(() => {
      window.dispatchEvent(new Event("focus"));
    });
    await waitFor(() =>
      expect(
        screen
          .getByRole("button", { name: "Project details Example.flp" })
          .getAttribute("aria-expanded"),
      ).toBe("false"),
    );
    await act(async () => {
      finish(savedDetails());
      await Promise.resolve();
    });
    expect(screen.queryByText("120 BPM")).toBeNull();
    expect(screen.queryByText("Fixture Synth A")).toBeNull();
    expect(screen.queryByText("Analysis finished")).toBeNull();
  });
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
    expect(screen.queryByText("Analysis finished")).toBeNull();
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
    expect(screen.getAllByText("Inferred")).toHaveLength(5);
    expect(screen.getByText("Fixture Synth A")).toBeTruthy();
    expect(screen.getByText("3x Osc")).toBeTruthy();
    expect(screen.getByText("Analysis finished")).toBeTruthy();
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

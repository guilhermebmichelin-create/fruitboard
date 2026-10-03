import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import axe from "axe-core";
import { MemoryRouter } from "react-router";
import { describe, expect, it, vi } from "vitest";
import { ProjectSamples } from "./ProjectSamples";
import type { ProjectSampleReference } from "./sampleReferences";
import { ProjectDetailsPanel } from "./ProjectDetailsPanel";
import { LibraryPage } from "./LibraryPage";
import { createFakeLibraryScanAdapter } from "./fake";
import { identity, savedDetails } from "./projectDetails.fixture";
import type { PublishedFileLocation } from "./contracts";
import type { ProjectDetails } from "./projectDetails";

const samples: readonly ProjectSampleReference[] = [
  { position: 1, status: "extracted", value: "..\\Samples\\Kick.wav" },
  { position: 2, status: "unavailable", value: null },
  { position: 3, status: "extracted", value: "..\\Samples\\Kick.wav" },
];
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
const details = () => ({ ...savedDetails(), sampleReferences: samples });

describe("saved sample reference view", () => {
  it("preserves provenance, duplicates and channel positions without file actions", async () => {
    const view = render(<ProjectSamples samples={samples} />);
    expect(screen.getAllByText("..\\Samples\\Kick.wav")).toHaveLength(2);
    expect(
      screen.getByRole("heading", { name: "Sample reference for channel 2" }),
    ).toBeTruthy();
    expect(
      screen.getByText("No sample reference was stored for this channel."),
    ).toBeTruthy();
    expect(
      screen.getByText(/does not prove a file is present or missing/),
    ).toBeTruthy();
    expect(screen.queryByRole("link")).toBeNull();
    expect(screen.queryByRole("button")).toBeNull();
    expect(view.container.querySelector("audio,img,iframe")).toBeNull();
    expect(
      (
        await axe.run(view.container, {
          rules: { "color-contrast": { enabled: false } },
        })
      ).violations,
    ).toEqual([]);
  });
  it("renders hostile reference text inertly and escapes controls and bidi", () => {
    const value =
      "<img src=x onerror=alert(1)> https://example.invalid/音\n\u202e.wav";
    const view = render(
      <ProjectSamples
        samples={[{ position: 1, status: "extracted", value }]}
      />,
    );
    expect(
      screen.getByText(
        "<img src=x onerror=alert(1)> https://example.invalid/音\\u000a\\u202e.wav",
      ),
    ).toBeTruthy();
    expect(screen.getByText(/Control characters/)).toBeTruthy();
    expect(view.container.querySelector("img,a")).toBeNull();
  });
  it.each([
    undefined,
    [],
    [{ position: 1, status: "unavailable", value: null }] as const,
  ])("explains older, zero-channel, and missing results %#", (value) => {
    render(<ProjectSamples samples={value} />);
    expect(screen.queryByText(/Location not checked/)).toBeNull();
    expect(
      screen.getByText(
        value === undefined
          ? /did not report sample references/
          : value.length === 0
            ? /no per-channel references are shown/
            : /No sample references were stored for the reported channels/,
      ),
    ).toBeTruthy();
  });
  it("progressively renders the bounded list and preserves keyboard focus", async () => {
    const user = userEvent.setup();
    const many: readonly ProjectSampleReference[] = Array.from(
      { length: 256 },
      (_, i) => ({
        position: i + 1,
        status: "extracted",
        value: `Samples\\${i + 1}.wav`,
      }),
    );
    render(<ProjectSamples samples={many} />);
    expect(screen.getAllByRole("listitem")).toHaveLength(20);
    const button = screen.getByRole("button", {
      name: "Show next 20 channels for sample references",
    });
    button.focus();
    await user.keyboard("{Enter}");
    expect(screen.getAllByRole("listitem")).toHaveLength(40);
    expect(document.activeElement).toBe(button);
    for (let i = 0; i < 11; i++) await user.keyboard("{Enter}");
    expect(screen.getAllByRole("listitem")).toHaveLength(256);
    expect(button.textContent).toBe("Show fewer sample references");
    await user.keyboard("{Enter}");
    expect(screen.getAllByRole("listitem")).toHaveLength(20);
    expect(document.activeElement).toBe(button);
  });
  it("refreshes with read-only calls and keeps fixed errors free of saved text", async () => {
    const user = userEvent.setup();
    const read = vi
      .fn()
      .mockResolvedValueOnce(details())
      .mockRejectedValueOnce(new Error("private sample path.wav"));
    const request = vi.fn();
    render(
      <ProjectDetailsPanel
        record={record}
        adapter={{
          ...createFakeLibraryScanAdapter(),
          getProjectDetails: read,
          requestProjectAnalysis: request,
        }}
      />,
    );
    await user.click(
      screen.getByRole("button", { name: "Project details Example.flp" }),
    );
    await screen.findByRole("region", { name: "Saved sample references" });
    await user.click(
      screen.getByRole("button", {
        name: "Refresh project details Example.flp",
      }),
    );
    await screen.findByRole("alert");
    expect(screen.queryByText(/private sample path/)).toBeNull();
    expect(screen.queryByText("..\\Samples\\Kick.wav")).toBeNull();
    expect(read).toHaveBeenCalledTimes(2);
    expect(request).not.toHaveBeenCalled();
  });
  it.each(["new_scan", "disabled_root"])(
    "clears samples and ignores late reads after %s",
    async (transition) => {
      const user = userEvent.setup();
      let generation = 1;
      let enabled = true;
      let finish!: (result: ProjectDetails) => void;
      const root = {
        id: identity.rootId,
        displayName: "Projects",
        canonicalPath: record.rootCanonicalPath,
        mode: "localNtfs",
        enabled: true,
        availability: "available",
        lastErrorCode: null,
      } as const;
      const fake = createFakeLibraryScanAdapter({
        roots: [root],
        files: [record],
      });
      const read = vi
        .fn()
        .mockResolvedValueOnce(details())
        .mockImplementationOnce(
          () =>
            new Promise<ProjectDetails>((resolve) => {
              finish = resolve;
            }),
        );
      const adapter = {
        ...fake,
        getProjectDetails: read,
        getLibraryPage: () =>
          Promise.resolve({
            rootId: identity.rootId,
            snapshotId: `snapshot-${generation}`,
            records: [record],
            nextCursor: null,
          }),
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
      await screen.findByRole("region", { name: "Saved sample references" });
      await user.click(
        screen.getByRole("button", {
          name: "Refresh project details Example.flp",
        }),
      );
      await screen.findByText(/Loading saved project details/);
      if (transition === "new_scan") generation = 2;
      else enabled = false;
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
        finish(details());
        await Promise.resolve();
      });
      expect(
        screen.queryByRole("region", { name: "Saved sample references" }),
      ).toBeNull();
      expect(screen.queryByText("..\\Samples\\Kick.wav")).toBeNull();
    },
  );
});

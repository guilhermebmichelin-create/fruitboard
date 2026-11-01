import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ProjectDetailsPanel } from "./ProjectDetailsPanel";
import { sequenceDetails } from "./noteSequences.fixture";
import { createFakeLibraryScanAdapter } from "./fake";
import { identity } from "./projectDetails.fixture";
import type { ProjectDetails } from "./projectDetails";
import type { PublishedFileLocation } from "./contracts";

const record: PublishedFileLocation = {
  ...identity,
  rootDisplayName: "Projects",
  rootCanonicalPath: "C:\\Synthetic",
  fileName: "Notes.flp",
  relativePath: "Notes.flp",
  byteSize: "1024",
  modifiedAt: "2026-01-02T00:00:00Z",
  presence: "present",
};

describe("note sequence details lifecycle", () => {
  it.each(["missing", "revoked", "changed"] as const)(
    "clears notes immediately on %s authority",
    async (change) => {
      const user = userEvent.setup();
      const read = vi
        .fn()
        .mockResolvedValueOnce(sequenceDetails())
        .mockImplementation(() => new Promise(() => {}));
      const adapter = {
        ...createFakeLibraryScanAdapter(),
        getProjectDetails: read,
      };
      const view = render(
        <ProjectDetailsPanel adapter={adapter} record={record} />,
      );
      await user.click(
        screen.getByRole("button", { name: "Project details Notes.flp" }),
      );
      await user.click(
        await screen.findByText("Saved note sequence for pattern 1"),
      );
      expect(screen.getByText("Saved record 1")).toBeTruthy();
      view.rerender(
        <ProjectDetailsPanel
          adapter={adapter}
          record={
            change === "missing"
              ? { ...record, presence: "missing" }
              : change === "changed"
                ? { ...record, byteSize: "2048" }
                : record
          }
          sourceEligible={change !== "revoked"}
        />,
      );
      expect(screen.queryByText("Saved record 1")).toBeNull();
      expect(screen.queryByText("Ordinary notes")).toBeNull();
    },
  );
  it("clears on refresh, ignores late replies and resets disclosure on a new snapshot", async () => {
    const user = userEvent.setup();
    let finish!: (details: ProjectDetails) => void;
    const read = vi
      .fn()
      .mockResolvedValueOnce(sequenceDetails(130))
      .mockImplementationOnce(
        () =>
          new Promise<ProjectDetails>((resolve) => {
            finish = resolve;
          }),
      )
      .mockResolvedValue({
        ...sequenceDetails(130),
        snapshotId: "12345678-1234-1234-1234-123456789abd",
      });
    const adapter = {
      ...createFakeLibraryScanAdapter(),
      getProjectDetails: read,
      requestProjectAnalysis: vi.fn(),
    };
    const view = render(
      <ProjectDetailsPanel adapter={adapter} record={record} />,
    );
    const toggle = screen.getByRole("button", {
      name: "Project details Notes.flp",
    });
    await user.click(toggle);
    await user.click(
      await screen.findByText("Saved note sequence for pattern 1"),
    );
    await user.click(
      screen.getByRole("button", { name: "Show more notes for pattern 1" }),
    );
    expect(screen.getByText("Saved record 128")).toBeTruthy();
    await user.click(
      screen.getByRole("button", { name: "Refresh details Notes.flp" }),
    );
    expect(screen.queryByText("Saved record 1")).toBeNull();
    await waitFor(() => expect(read).toHaveBeenCalledTimes(2));
    view.rerender(
      <ProjectDetailsPanel
        adapter={adapter}
        record={record}
        sourceEligible={false}
      />,
    );
    await act(async () => {
      finish(sequenceDetails());
      await Promise.resolve();
    });
    expect(screen.queryByText("Saved record 1")).toBeNull();
    view.rerender(<ProjectDetailsPanel adapter={adapter} record={record} />);
    await user.click(
      await screen.findByText("Saved note sequence for pattern 1"),
    );
    expect(screen.getByText("Saved record 64")).toBeTruthy();
    expect(screen.queryByText("Saved record 65")).toBeNull();
    expect(adapter.requestProjectAnalysis).not.toHaveBeenCalled();
  });
});

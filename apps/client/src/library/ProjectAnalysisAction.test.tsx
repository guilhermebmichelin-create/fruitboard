import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import axe from "axe-core";
import { MemoryRouter } from "react-router";
import { describe, expect, it, vi } from "vitest";
import { LibraryAdapterError, type PublishedFileLocation } from "./contracts";
import { createFakeLibraryScanAdapter } from "./fake";
import { LibraryPage } from "./LibraryPage";
import { ProjectDetailsPanel } from "./ProjectDetailsPanel";
import { identity, savedDetails } from "./projectDetails.fixture";
import {
  ANALYSIS_REQUEST_BLOCKS,
  ANALYSIS_REQUEST_NOTES,
  type ProjectAnalysisRequestResult,
} from "./projectAnalysisRequest";
import type { ProjectDetails } from "./projectDetails";

const requestKey = "a".repeat(64);
const ready = { state: "ready", requestKey } as const;
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
const facts = () => ({ ...savedDetails(), analysisRequest: ready });
const queued = { ...identity, state: "queued", reason: null } as const;
const expand = async () => {
  await userEvent.setup().click(
    await screen.findByRole("button", {
      name: "Project details Example.flp",
    }),
  );
};

describe("explicit Library analysis action", () => {
  it("queues once on keyboard activation, retains facts, and refreshes only by reading", async () => {
    const user = userEvent.setup();
    let finish!: (result: ProjectAnalysisRequestResult) => void;
    let finishRead!: (result: ProjectDetails) => void;
    const request = vi.fn().mockImplementation(
      () =>
        new Promise<ProjectAnalysisRequestResult>((resolve) => {
          finish = resolve;
        }),
    );
    const read = vi
      .fn()
      .mockResolvedValueOnce(facts())
      .mockImplementationOnce(
        () =>
          new Promise<ProjectDetails>((resolve) => {
            finishRead = resolve;
          }),
      )
      .mockResolvedValue(facts());
    const view = render(
      <ProjectDetailsPanel
        record={record}
        adapter={{
          ...createFakeLibraryScanAdapter(),
          getProjectDetails: read,
          requestProjectAnalysis: request,
        }}
      />,
    );
    await expand();
    const button = await screen.findByRole("button", {
      name: "Analyze again Example.flp",
    });
    expect(request).not.toHaveBeenCalled();
    button.focus();
    await user.keyboard("{Enter}{Enter}");
    expect(request).toHaveBeenCalledExactlyOnceWith(
      { ...identity, byteSize: record.byteSize, modifiedAt: record.modifiedAt },
      requestKey,
    );
    expect(button.hasAttribute("disabled")).toBe(true);
    expect(screen.getByText("120 BPM")).toBeTruthy();
    expect(screen.queryByText(requestKey)).toBeNull();
    await act(async () => {
      finish(queued);
      await Promise.resolve();
    });
    await waitFor(() => expect(read).toHaveBeenCalledTimes(2));
    expect(screen.getByText("120 BPM")).toBeTruthy();
    expect(screen.queryByText(/Loading saved project details/)).toBeNull();
    await act(async () => {
      finishRead({
        ...facts(),
        analysis: { state: "queued", attempts: 1, reason: null },
        analysisRequest: { state: "pending", requestKey: null },
      });
      await Promise.resolve();
    });
    await screen.findByText("Waiting for analysis");
    expect(screen.getByText("Attempts started: 1 of 3.")).toBeTruthy();
    expect(screen.getByText("120 BPM")).toBeTruthy();
    expect(request).toHaveBeenCalledTimes(1);
    const accessibility = await axe.run(view.container, {
      rules: { "color-contrast": { enabled: false } },
    });
    expect(accessibility.violations).toEqual([]);
    await user.click(
      screen.getByRole("button", {
        name: "Refresh project details Example.flp",
      }),
    );
    await screen.findByRole("button", { name: "Analyze again Example.flp" });
    expect(read).toHaveBeenCalledTimes(3);
    expect(request).toHaveBeenCalledTimes(1);
  });

  it.each([
    ["no_current", "not_reported", "Analyze project"],
    ["no_current", "failed", "Retry analysis"],
    ["available", "cancelled", "Retry analysis"],
  ] as const)(
    "labels %s / %s without inventing an attempt",
    async (state, analysisState, label) => {
      const details: ProjectDetails = {
        ...(state === "available"
          ? facts()
          : { ...identity, state, analysisRequest: ready }),
        analysis: {
          state: analysisState,
          attempts: analysisState === "not_reported" ? null : 1,
          reason: null,
        },
      };
      const request = vi.fn().mockResolvedValue(queued);
      render(
        <ProjectDetailsPanel
          record={record}
          adapter={{
            ...createFakeLibraryScanAdapter(),
            getProjectDetails: () => Promise.resolve(details),
            requestProjectAnalysis: request,
          }}
        />,
      );
      await expand();
      expect(
        await screen.findByRole("button", { name: `${label} Example.flp` }),
      ).toBeTruthy();
      expect(request).not.toHaveBeenCalled();
    },
  );

  it.each(ANALYSIS_REQUEST_BLOCKS)(
    "explains and disables %s without a request",
    async (state) => {
      const request = vi.fn();
      render(
        <ProjectDetailsPanel
          record={record}
          adapter={{
            ...createFakeLibraryScanAdapter(),
            getProjectDetails: () =>
              Promise.resolve({
                ...facts(),
                analysisRequest: { state, requestKey: null },
              }),
            requestProjectAnalysis: request,
          }}
        />,
      );
      await expand();
      const button = await screen.findByRole("button", {
        name: "Analyze again Example.flp",
      });
      expect(button.hasAttribute("disabled")).toBe(true);
      expect(screen.getByText(ANALYSIS_REQUEST_NOTES[state])).toBeTruthy();
      await userEvent.setup().click(button);
      expect(request).not.toHaveBeenCalled();
    },
  );

  it.each([
    new LibraryAdapterError("conflict"),
    new Error("C:\\Private\\Project.flp parser-secret"),
  ])(
    "contains request failures and requires a fresh read before another click",
    async (error) => {
      const user = userEvent.setup();
      const request = vi
        .fn()
        .mockRejectedValueOnce(error)
        .mockResolvedValue(queued);
      const read = vi
        .fn()
        .mockResolvedValueOnce(facts())
        .mockResolvedValue({
          ...facts(),
          analysisRequest: { state: "ready", requestKey: "b".repeat(64) },
        });
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
      await expand();
      await user.click(
        await screen.findByRole("button", {
          name: "Analyze again Example.flp",
        }),
      );
      await screen.findByText(/Refresh details before trying again/);
      expect(screen.getByText("120 BPM")).toBeTruthy();
      expect(screen.queryByText(/parser-secret/)).toBeNull();
      expect(read).toHaveBeenCalledTimes(1);
      expect(
        screen
          .getByRole("button", { name: "Analyze again Example.flp" })
          .hasAttribute("disabled"),
      ).toBe(true);
      await user.click(
        screen.getByRole("button", {
          name: "Refresh project details Example.flp",
        }),
      );
      const next = await screen.findByRole("button", {
        name: "Analyze again Example.flp",
      });
      expect(next.hasAttribute("disabled")).toBe(false);
      expect(request).toHaveBeenCalledTimes(1);
      await user.click(next);
      await screen.findByText(/Analysis requested/);
      expect(request.mock.calls[1]?.[1]).toBe("b".repeat(64));
    },
  );

  it("hides the action for older adapters and older detail responses", async () => {
    const view = render(
      <ProjectDetailsPanel
        record={record}
        adapter={{
          ...createFakeLibraryScanAdapter(),
          getProjectDetails: () => Promise.resolve(facts()),
        }}
      />,
    );
    await expand();
    await screen.findByText("120 BPM");
    expect(
      screen.queryByRole("button", { name: "Analyze again Example.flp" }),
    ).toBeNull();
    view.unmount();
    render(
      <ProjectDetailsPanel
        record={record}
        adapter={{
          ...createFakeLibraryScanAdapter(),
          getProjectDetails: () => Promise.resolve(savedDetails()),
          requestProjectAnalysis: vi.fn(),
        }}
      />,
    );
    await expand();
    await screen.findByText("120 BPM");
    expect(
      screen.queryByRole("button", { name: "Analyze again Example.flp" }),
    ).toBeNull();
  });

  it.each(["close", "new_scan", "disabled_root"])(
    "ignores a late request result after %s",
    async (transition) => {
      let finish!: (result: ProjectAnalysisRequestResult) => void;
      let generation = 1;
      let enabled = true;
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
      const read = vi.fn().mockResolvedValue(facts());
      const request = vi.fn().mockImplementation(
        () =>
          new Promise<ProjectAnalysisRequestResult>((resolve) => {
            finish = resolve;
          }),
      );
      const adapter = {
        ...fake,
        getProjectDetails: read,
        requestProjectAnalysis: request,
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
      await expand();
      await userEvent.setup().click(
        await screen.findByRole("button", {
          name: "Analyze again Example.flp",
        }),
      );
      if (transition === "close") await expand();
      else {
        if (transition === "new_scan") generation = 2;
        else enabled = false;
        act(() => {
          window.dispatchEvent(new Event("focus"));
        });
      }
      await waitFor(() =>
        expect(
          screen
            .getByRole("button", { name: "Project details Example.flp" })
            .getAttribute("aria-expanded"),
        ).toBe("false"),
      );
      await act(async () => {
        finish(queued);
        await Promise.resolve();
      });
      expect(read).toHaveBeenCalledTimes(1);
      expect(screen.queryByText(/Analysis requested/)).toBeNull();
      expect(screen.queryByText("120 BPM")).toBeNull();
    },
  );
});

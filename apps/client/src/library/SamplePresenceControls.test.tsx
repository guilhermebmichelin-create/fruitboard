import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import axe from "axe-core";
import { describe, it, expect, vi } from "vitest";
import { ProjectSamples } from "./ProjectSamples";
import { createFakeLibraryScanAdapter } from "./fake";
import type { PublishedFileLocation } from "./contracts";
import type { ProjectSampleReference } from "./sampleReferences";
import type { SampleCheckResult, SampleCheckRequest } from "./samplePresence";

const samples: readonly ProjectSampleReference[] = [
  { position: 1, status: "extracted", value: "C:\\Private\\one.wav" },
  { position: 2, status: "extracted", value: "relative.wav" },
  { position: 3, status: "unavailable", value: null },
];
const record: PublishedFileLocation = {
  rootId: "root",
  locationId: "location",
  rootDisplayName: "Selected projects",
  rootCanonicalPath: "C:\\Private",
  fileName: "Example.flp",
  relativePath: "Example.flp",
  byteSize: "123",
  modifiedAt: "2026-01-02T00:00:00Z",
  presence: "present",
};
const complete: SampleCheckResult = {
  state: "complete",
  report: {
    checkedAt: 1791234567890,
    channels: [
      { position: 1, status: "present" },
      { position: 2, status: "not_checked", reason: "relative_reference" },
      { position: 3, status: "no_saved_reference" },
    ],
  },
};
function setup(
  check = vi
    .fn<
      (
        request: SampleCheckRequest,
        samples: readonly ProjectSampleReference[],
      ) => Promise<SampleCheckResult>
    >()
    .mockResolvedValue(complete),
) {
  const adapter = {
    ...createFakeLibraryScanAdapter(),
    checkSavedSamples: check,
    cancelSampleCheck: vi.fn().mockResolvedValue({ state: "cancelled" }),
  };
  return { adapter, view: { adapter, record, snapshotId: "snapshot" } };
}
describe("explicit sample check controls", () => {
  it("opening references performs no probes; keyboard action shows time-of-check results and fixed unchecked copy", async () => {
    const { adapter, view } = setup();
    const user = userEvent.setup();
    const rendered = render(
      <ProjectSamples samples={samples} checkView={view} />,
    );
    expect(adapter.checkSavedSamples).not.toHaveBeenCalled();
    expect(screen.getByText("Selected projects")).toBeTruthy();
    const button = screen.getByRole("button", { name: "Check saved samples" });
    button.focus();
    await user.keyboard("{Enter}");
    expect(await screen.findByText(/Present when checked/)).toBeTruthy();
    expect(
      screen.getByText(/Relative references cannot be checked yet/),
    ).toBeTruthy();
    expect(
      screen.getByText(/Results describe when this check ran/),
    ).toBeTruthy();
    expect(screen.queryByRole("link")).toBeNull();
    expect(rendered.container.querySelector("audio")).toBeNull();
    expect(
      (
        await axe.run(rendered.container, {
          rules: { "color-contrast": { enabled: false } },
        })
      ).violations,
    ).toEqual([]);
    expect(adapter.checkSavedSamples.mock.calls[0]?.[0]).not.toHaveProperty(
      "path",
    );
    await user.click(
      screen.getByRole("button", { name: "Check saved samples again" }),
    );
    expect(adapter.checkSavedSamples).toHaveBeenCalledTimes(2);
    expect(adapter.checkSavedSamples.mock.calls[0]?.[0].requestId).not.toBe(
      adapter.checkSavedSamples.mock.calls[1]?.[0].requestId,
    );
  });
  it("cancel fences late replies and clears prior results before check-again", async () => {
    let resolve!: (result: SampleCheckResult) => void;
    const check = vi
      .fn<
        (
          r: SampleCheckRequest,
          s: readonly ProjectSampleReference[],
        ) => Promise<SampleCheckResult>
      >()
      .mockImplementationOnce(
        () =>
          new Promise((r) => {
            resolve = r;
          }),
      )
      .mockResolvedValueOnce(complete);
    const { adapter, view } = setup(check);
    const user = userEvent.setup();
    const rendered = render(
      <ProjectSamples samples={samples} checkView={view} />,
    );
    await user.click(
      screen.getByRole("button", { name: "Check saved samples" }),
    );
    expect(screen.getByText(/Checking saved samples/)).toBeTruthy();
    const cancel = screen.getByRole("button", { name: "Cancel sample check" });
    cancel.focus();
    await user.keyboard("{Enter}");
    expect(adapter.cancelSampleCheck).toHaveBeenCalledTimes(1);
    expect(document.activeElement).toBe(
      screen.getByRole("button", { name: "Check saved samples" }),
    );
    expect(screen.getByText(/Sample check cancelled/)).toBeTruthy();
    await act(async () => {
      resolve(complete);
      await Promise.resolve();
    });
    expect(screen.queryByText(/Present when checked/)).toBeNull();
    expect(
      (
        await axe.run(rendered.container, {
          rules: { "color-contrast": { enabled: false } },
        })
      ).violations,
    ).toEqual([]);
    await user.click(
      screen.getByRole("button", { name: "Check saved samples" }),
    );
    expect(await screen.findByText(/Present when checked/)).toBeTruthy();
  });
  it("navigation/snapshot/fingerprint changes cancel pending work and discard its report", async () => {
    for (const change of ["root", "snapshot", "fingerprint"]) {
      let resolve!: (result: SampleCheckResult) => void;
      const check = vi
        .fn<
          (
            r: SampleCheckRequest,
            s: readonly ProjectSampleReference[],
          ) => Promise<SampleCheckResult>
        >()
        .mockImplementation(
          () =>
            new Promise((r) => {
              resolve = r;
            }),
        );
      const { adapter, view } = setup(check);
      const user = userEvent.setup();
      const rendered = render(
        <ProjectSamples samples={samples} checkView={view} />,
      );
      await user.click(
        screen.getByRole("button", { name: "Check saved samples" }),
      );
      const next = {
        ...view,
        snapshotId: change === "snapshot" ? "new-snapshot" : view.snapshotId,
        record: {
          ...record,
          rootId: change === "root" ? "other-root" : record.rootId,
          byteSize: change === "fingerprint" ? "124" : record.byteSize,
        },
      };
      rendered.rerender(<ProjectSamples samples={samples} checkView={next} />);
      await act(async () => {
        resolve(complete);
        await Promise.resolve();
      });
      expect(adapter.cancelSampleCheck).toHaveBeenCalledTimes(1);
      expect(screen.queryByText(/Present when checked/)).toBeNull();
      rendered.unmount();
    }
  });
  it("successful report also disappears when its source context changes", async () => {
    const { view } = setup();
    const user = userEvent.setup();
    const rendered = render(
      <ProjectSamples samples={samples} checkView={view} />,
    );
    await user.click(
      screen.getByRole("button", { name: "Check saved samples" }),
    );
    await screen.findByText(/Present when checked/);
    rendered.rerender(
      <ProjectSamples
        samples={samples}
        checkView={{ ...view, snapshotId: "new-snapshot" }}
      />,
    );
    expect(screen.queryByText(/Present when checked/)).toBeNull();
  });
  it.each(["busy", "stale", "deadline", "disabled"] as const)(
    "shows a fixed %s state with no absence claim",
    async (state) => {
      const check = vi
        .fn<
          (
            r: SampleCheckRequest,
            s: readonly ProjectSampleReference[],
          ) => Promise<SampleCheckResult>
        >()
        .mockResolvedValue({ state });
      const { view } = setup(check);
      const user = userEvent.setup();
      const rendered = render(
        <ProjectSamples samples={samples} checkView={view} />,
      );
      await user.click(
        screen.getByRole("button", { name: "Check saved samples" }),
      );
      await waitFor(() => expect(check).toHaveBeenCalled());
      expect(
        screen.queryByText(/Not found at saved path when checked/),
      ).toBeNull();
      expect(
        (
          await axe.run(rendered.container, {
            rules: { "color-contrast": { enabled: false } },
          })
        ).violations,
      ).toEqual([]);
    },
  );
  it("errors never display private transport diagnostics", async () => {
    const check = vi
      .fn<
        (
          r: SampleCheckRequest,
          s: readonly ProjectSampleReference[],
        ) => Promise<SampleCheckResult>
      >()
      .mockRejectedValue(new Error("private OS diagnostic"));
    const { view } = setup(check);
    const user = userEvent.setup();
    render(<ProjectSamples samples={samples} checkView={view} />);
    await user.click(
      screen.getByRole("button", { name: "Check saved samples" }),
    );
    expect(await screen.findByRole("alert")).toHaveProperty(
      "textContent",
      "Samples could not be checked safely. Try again.",
    );
    expect(screen.queryByText(/private OS diagnostic/)).toBeNull();
  });
});

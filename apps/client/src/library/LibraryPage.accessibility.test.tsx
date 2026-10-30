import { MemoryRouter } from "react-router";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import axe from "axe-core";
import { describe, expect, it, vi } from "vitest";
import type { ScanRoot } from "../platform/contracts";
import { createFakeLibraryScanAdapter } from "./fake";
import { LibraryPage } from "./LibraryPage";
import type { PublishedFileLocation } from "./contracts";

const root: ScanRoot = {
  id: "root-a11y",
  displayName: "Accessible Projects",
  canonicalPath: "C:\\Synthetic\\Accessible",
  mode: "localNtfs",
  enabled: true,
  availability: "available",
  lastErrorCode: null,
};

const sibling: ScanRoot = {
  id: "root-a11y-sibling",
  displayName: "Accessible Projects",
  canonicalPath: "D:\\Synthetic\\Accessible",
  mode: "localNtfs",
  enabled: true,
  availability: "available",
  lastErrorCode: null,
};

const record: PublishedFileLocation = {
  locationId: "location-a11y",
  rootId: root.id,
  rootDisplayName: root.displayName,
  rootCanonicalPath: root.canonicalPath,
  fileName: "Accessible.flp",
  relativePath: "Nested\\Accessible.flp",
  byteSize: "2048",
  modifiedAt: "2026-02-03T04:05:06.123456789Z",
  presence: "present",
};

function renderLibrary(
  adapter: ReturnType<typeof createFakeLibraryScanAdapter>,
) {
  return render(
    <MemoryRouter>
      <LibraryPage adapter={adapter} />
    </MemoryRouter>,
  );
}

async function auditStableDom(container: HTMLElement): Promise<axe.AxeResults> {
  // act can flush a queued refresh before it returns. Discard an audit only
  // when its DOM changed; never retry a violation from an unchanged view.
  for (let attempt = 0; attempt < 3; attempt += 1) {
    let changed = false;
    const observer = new MutationObserver(() => {
      changed = true;
    });
    observer.observe(container, {
      subtree: true,
      childList: true,
      characterData: true,
      attributes: true,
    });
    try {
      let results!: axe.AxeResults;
      await act(async () => {
        results = await axe.run(container, {
          rules: { "color-contrast": { enabled: false } },
        });
      });
      if (!changed && observer.takeRecords().length === 0) return results;
    } finally {
      observer.disconnect();
    }
  }
  throw new Error("The accessibility audit never reached an unchanged DOM.");
}

async function expectNoViolations(container: HTMLElement) {
  const results = await auditStableDom(container);
  expect(
    results.violations.map(({ id, nodes }) => ({
      id,
      targets: nodes.map((node) => node.target),
    })),
  ).toEqual([]);
  // "Needs review" (`incomplete`) results can hide real defects — for
  // example `aria-prohibited-attr` when the element has descendant text —
  // so they are triaged here instead of silently ignored.
  expect(
    results.incomplete.map(({ id, nodes }) => ({
      id,
      targets: nodes.map((node) => node.target),
    })),
  ).toEqual([]);
}

describe("LibraryPage accessibility", () => {
  it("rechecks an audit crossed by a DOM transition", async () => {
    const view = render(
      <section>
        <h1>Library audit control</h1>
        <h3>Transitioning heading</h3>
      </section>,
    );
    const run = vi.spyOn(axe, "run");
    try {
      const audit = expectNoViolations(view.container);
      queueMicrotask(() => {
        const heading = document.createElement("h2");
        heading.textContent = "Settled heading";
        view.container.querySelector("h3")!.replaceWith(heading);
      });
      await audit;
      expect(run).toHaveBeenCalledTimes(2);
    } finally {
      run.mockRestore();
    }
  });

  it("rejects a stable heading-order violation without retrying it", async () => {
    const view = render(
      <section>
        <h1>Library audit control</h1>
        <h3>Skipped heading level</h3>
      </section>,
    );
    const run = vi.spyOn(axe, "run");
    try {
      await expect(expectNoViolations(view.container)).rejects.toThrow(
        /heading-order/,
      );
      expect(run).toHaveBeenCalledTimes(1);
    } finally {
      run.mockRestore();
    }
  });

  it("exposes recovery history without taking focus from current scan controls", async () => {
    const adapter = createFakeLibraryScanAdapter({
      roots: [root],
      files: [record],
    });
    const view = renderLibrary(adapter);
    await screen.findByRole("heading", { name: "Accessible.flp" });
    await adapter.scanNow(root.id);
    adapter.advanceRun(root.id);
    adapter.interruptScan(root.id);
    await screen.findByText("Interrupted");
    await adapter.scanNow(root.id);
    await screen.findByText("Queued");
    const cancel = screen.getByRole("button", {
      name: "Cancel scan Accessible Projects",
    });
    cancel.focus();
    await expectNoViolations(view.container);
    adapter.advanceRun(root.id);
    await screen.findByText("Running");
    expect(document.activeElement).toBe(cancel);
    expect(screen.getByText("Last finished attempt")).toBeTruthy();
    await expectNoViolations(view.container);
  });

  it("has no automated violations in queued, running, cancelled, retry, and unavailable states", async () => {
    const user = userEvent.setup();
    const adapter = createFakeLibraryScanAdapter({
      roots: [root],
      files: [record],
    });
    const view = renderLibrary(adapter);
    await screen.findByRole("heading", { name: "Accessible.flp" });
    await expectNoViolations(view.container);

    await user.click(
      screen.getByRole("button", { name: "Scan now Accessible Projects" }),
    );
    await screen.findByText("Queued");
    await expectNoViolations(view.container);
    await screen.findByRole("button", {
      name: "Cancel scan Accessible Projects",
    });
    adapter.advanceRun(root.id);
    await screen.findByText("Running");
    await expectNoViolations(view.container);

    await user.click(
      screen.getByRole("button", { name: "Cancel scan Accessible Projects" }),
    );
    await screen.findByText("Cancelled");
    await expectNoViolations(view.container);
    await user.click(
      screen.getByRole("button", { name: "Scan now Accessible Projects" }),
    );
    await screen.findByText("Queued");
    await expectNoViolations(view.container);
    view.unmount();

    const unavailable = createFakeLibraryScanAdapter({
      roots: [{ ...root, availability: "unavailable" }],
    });
    const unavailableView = renderLibrary(unavailable);
    await screen.findByRole("heading", { name: "A scan root is unavailable" });
    await expectNoViolations(unavailableView.container);
    unavailableView.unmount();
  });

  it("keeps keyboard activation and focus through queue, cancel, and retry", async () => {
    const user = userEvent.setup();
    const adapter = createFakeLibraryScanAdapter({
      roots: [root],
      files: [record],
    });
    renderLibrary(adapter);
    await screen.findByRole("heading", { name: "Accessible.flp" });

    const scan = screen.getByRole("button", {
      name: "Scan now Accessible Projects",
    });
    scan.focus();
    await user.keyboard("{Enter}");
    await screen.findByText("Queued");
    const cancel = screen.getByRole("button", {
      name: "Cancel scan Accessible Projects",
    });
    await waitFor(() => expect(document.activeElement).toBe(cancel));

    await user.keyboard("{Enter}");
    await screen.findByText("Cancelled");
    const scanAgain = screen.getByRole("button", {
      name: "Scan now Accessible Projects",
    });
    await waitFor(() => expect(document.activeElement).toBe(scanAgain));

    await user.keyboard("{Enter}");
    await screen.findByText("Queued");
    await waitFor(() =>
      expect(document.activeElement).toBe(
        screen.getByRole("button", {
          name: "Cancel scan Accessible Projects",
        }),
      ),
    );
  });

  it("keeps the per-root selector keyboard operable and disambiguates duplicates", async () => {
    const user = userEvent.setup();
    const siblingRecord: PublishedFileLocation = {
      ...record,
      locationId: "location-a11y-sibling",
      rootId: sibling.id,
      rootDisplayName: sibling.displayName,
      rootCanonicalPath: sibling.canonicalPath,
      fileName: "Sibling.flp",
      relativePath: "Sibling.flp",
    };
    const adapter = createFakeLibraryScanAdapter({
      roots: [root, sibling],
      files: [record, siblingRecord],
    });
    const view = renderLibrary(adapter);
    await screen.findByRole("heading", { name: "Accessible.flp" });
    await expectNoViolations(view.container);

    const selector = screen.getByLabelText("Scan root");
    const options = within(selector).getAllByRole("option");
    expect(options.map((option) => option.textContent)).toEqual([
      "Accessible Projects (C:\\Synthetic\\Accessible)",
      "Accessible Projects (D:\\Synthetic\\Accessible)",
    ]);

    selector.focus();
    await user.selectOptions(selector, sibling.id);
    await screen.findByRole("heading", { name: "Sibling.flp" });
    expect(
      screen.queryByRole("heading", { name: "Accessible.flp" }),
    ).toBeNull();
    await expectNoViolations(view.container);
    expect(document.activeElement).toBe(selector);
  });

  it("keeps refreshed per-root pagination accessible after a snapshot restart", async () => {
    const secondRecord = {
      ...record,
      locationId: "location-a11y-second",
      fileName: "Second accessible.flp",
      relativePath: "Second accessible.flp",
    };
    const thirdRecord = {
      ...record,
      locationId: "location-a11y-third",
      fileName: "Third accessible.flp",
      relativePath: "Third accessible.flp",
    };
    const adapter = createFakeLibraryScanAdapter({
      roots: [root],
      files: [record, secondRecord, thirdRecord],
      pageLimit: 1,
    });
    const user = userEvent.setup();
    const view = renderLibrary(adapter);
    await screen.findByRole("heading", { name: "Accessible.flp" });
    await expectNoViolations(view.container);

    await user.click(screen.getByRole("button", { name: "Next library page" }));
    await screen.findByRole("heading", { name: "Second accessible.flp" });
    await expectNoViolations(view.container);

    adapter.completeScan(root.id);
    await screen.findByRole("heading", { name: "Accessible.flp" });
    // The scan refresh also restores keyboard focus asynchronously. Audit the
    // completed transition rather than a document still being replaced.
    await waitFor(() =>
      expect(document.activeElement).toBe(
        screen.getByRole("heading", { name: "File locations" }),
      ),
    );
    await expectNoViolations(view.container);
    // The audit can release a queued follow-up read. Check its final ready view too.
    await screen.findByRole("heading", { name: "Accessible.flp" });
    await expectNoViolations(view.container);
  });
});

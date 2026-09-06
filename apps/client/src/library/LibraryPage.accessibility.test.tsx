import { MemoryRouter } from "react-router";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import axe from "axe-core";
import { describe, expect, it } from "vitest";
import type { ScanRoot } from "../platform/contracts";
import { createFakeLibraryScanAdapter } from "./fake";
import { LibraryPage } from "./LibraryPage";
import type { PublishedFileLocation } from "./contracts";

const root: ScanRoot = {
  id: "root-a11y",
  displayName: "Accessible Projects",
  canonicalPath: "C:\\Synthetic\\Accessible",
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
  byteSize: 2048,
  modifiedAt: "2026-02-03T04:05:00.000Z",
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

async function expectNoViolations(container: HTMLElement) {
  const results = await axe.run(container, {
    rules: { "color-contrast": { enabled: false } },
  });
  expect(
    results.violations.map(({ id, nodes }) => ({
      id,
      targets: nodes.map((node) => node.target),
    })),
  ).toEqual([]);
}

describe("LibraryPage accessibility", () => {
  it("has no automated violations in populated, running, cancelled, and unavailable states", async () => {
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
    adapter.advanceRun(root.id);
    await screen.findByText("Running");
    await expectNoViolations(view.container);

    await user.click(
      screen.getByRole("button", { name: "Cancel scan Accessible Projects" }),
    );
    await screen.findByText("Cancelled");
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
});

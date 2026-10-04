import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import axe from "axe-core";
import { MemoryRouter } from "react-router";
import { describe, expect, it, vi } from "vitest";
import { PluginExplorerPanel } from "./PluginExplorerPanel";
import { createFakeLibraryScanAdapter } from "./fake";
import { explorerFixture } from "./pluginExplorer.fixture";
import type { PluginExplorerResult } from "./pluginExplorer";
import { LibraryPage } from "./LibraryPage";
import { savedDetails } from "./projectDetails.fixture";

const props = {
  rootId: "root-test",
  rootLabel: "Projects",
  sourceSnapshotId: "snapshot-1",
};
const adapterWith = (read: () => Promise<PluginExplorerResult>) => ({
  ...createFakeLibraryScanAdapter(),
  getPluginExplorer: read,
});
async function open() {
  await userEvent.click(
    screen.getByRole("button", { name: "Open Plugin Explorer" }),
  );
}
async function accessible(container: HTMLElement) {
  const result = await axe.run(container, {
    rules: { "color-contrast": { enabled: false } },
  });
  expect(
    result.violations.map((issue) => ({
      id: issue.id,
      nodes: issue.nodes.map((node) => node.target),
    })),
  ).toEqual([]);
  expect(
    result.incomplete.map((issue) => ({
      id: issue.id,
      nodes: issue.nodes.map((node) => node.target),
    })),
  ).toEqual([]);
}

describe("saved Plugin Explorer", () => {
  it("loads on demand, searches without merging exact groups, and reveals matching file copies with the keyboard", async () => {
    const read = vi.fn(() => Promise.resolve(explorerFixture()));
    const view = render(
      <PluginExplorerPanel {...props} adapter={adapterWith(read)} />,
    );
    expect(read).not.toHaveBeenCalled();
    await open();
    await screen.findByText(
      /5 exact saved plugin groups across 2 of 5 Library entries/,
    );
    expect(
      screen.getByText(/8 references, including 1 without a usable name/),
    ).toBeTruthy();
    expect(
      screen.getByText(/4 entries have missing or incomplete/),
    ).toBeTruthy();
    const user = userEvent.setup();
    await user.type(screen.getByRole("searchbox"), "3x osc");
    expect(screen.getByText("Showing 2 of 2 matching groups.")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "3x Osc" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "3X Osc" })).toBeTruthy();
    const reveal = screen.getByRole("button", {
      name: "Show matching entries for 3x Osc",
    });
    reveal.focus();
    await user.keyboard("{Enter}");
    const matches = screen.getByRole("list", {
      name: "Matching entries for 3x Osc",
    });
    expect(
      within(matches).getAllByRole("heading", { name: "First.flp" }),
    ).toHaveLength(2);
    expect(within(matches).getByText(/Beats\/First.flp/)).toBeTruthy();
    expect(within(matches).getByText(/Copies\/First.flp/)).toBeTruthy();
    expect(document.activeElement).toBe(reveal);
    await accessible(view.container);
    await user.clear(screen.getByRole("searchbox"));
    await user.type(screen.getByRole("searchbox"), "Other Vendor");
    expect(screen.getByText("Showing 1 of 1 matching groups.")).toBeTruthy();
    await user.clear(screen.getByRole("searchbox"));
    await user.type(screen.getByRole("searchbox"), "No such plugin");
    expect(screen.getByText(/No saved plugin groups match/)).toBeTruthy();
  });

  it("keeps hostile saved text inert and escapes controls, with no links, images or plugin actions", async () => {
    const fixture = explorerFixture();
    const text =
      "<img src=x onerror=alert(1)> https://example.invalid/音\n\u202e";
    const group = fixture.groups[0]!;
    const data = {
      ...fixture,
      referenceCount: 3,
      unnamedReferenceCount: 0,
      entries: fixture.entries
        .slice(0, 2)
        .map((entry) => ({ ...entry, unnamedReferenceCount: 0 })),
      groups: [
        {
          ...group,
          plugin: {
            ...group.plugin,
            name: { status: "extracted" as const, value: text },
            className: { status: "extracted" as const, value: text },
          },
        },
      ],
    };
    const view = render(
      <PluginExplorerPanel
        {...props}
        adapter={adapterWith(() => Promise.resolve(data))}
      />,
    );
    await open();
    await screen.findByRole("heading", { name: /\\u000a\\u202e/ });
    expect(
      view.container.querySelector("img,a,iframe,object,audio"),
    ).toBeNull();
    expect(
      screen.getByText(
        /Plugin installation and availability have not been checked/,
      ),
    ).toBeTruthy();
    await accessible(view.container);
  });

  it.each(["disabled", "unavailable", "limited"] as const)(
    "explains %s without showing partial totals",
    async (state) => {
      const view = render(
        <PluginExplorerPanel
          {...props}
          adapter={adapterWith(() =>
            Promise.resolve({ rootId: props.rootId, state }),
          )}
        />,
      );
      await open();
      await screen.findByText(
        state === "limited"
          ? /No partial totals/
          : /Plugin Explorer is unavailable/,
      );
      expect(screen.queryByRole("searchbox")).toBeNull();
      expect(screen.queryByText(/exact saved plugin groups across/)).toBeNull();
      await accessible(view.container);
    },
  );

  it("covers loading, retryable error, empty and unnamed-only saved coverage", async () => {
    let resolve!: (data: PluginExplorerResult) => void;
    const read = vi.fn(
      () =>
        new Promise<PluginExplorerResult>((done) => {
          resolve = done;
        }),
    );
    const view = render(
      <PluginExplorerPanel {...props} adapter={adapterWith(read)} />,
    );
    await open();
    await screen.findByText(/Loading saved plugin references/);
    await accessible(view.container);
    const empty = {
      ...explorerFixture(),
      entries: [],
      groups: [],
      referenceCount: 0,
      unnamedReferenceCount: 0,
    };
    await act(() => Promise.resolve(resolve(empty)));
    expect(screen.getByText(/No committed Library entries/)).toBeTruthy();
    await accessible(view.container);
    view.unmount();
    const failing = render(
      <PluginExplorerPanel
        {...props}
        adapter={adapterWith(() =>
          Promise.reject(new Error("C:\\Private\\plugin.dll")),
        )}
      />,
    );
    await open();
    await screen.findByRole("alert");
    expect(failing.container.textContent).not.toContain("Private");
    await accessible(failing.container);
    failing.unmount();
    const fixture = explorerFixture();
    render(
      <PluginExplorerPanel
        {...props}
        adapter={adapterWith(() =>
          Promise.resolve({
            ...fixture,
            entries: [fixture.entries[0]!],
            groups: [],
            referenceCount: 1,
            unnamedReferenceCount: 1,
          }),
        )}
      />,
    );
    await open();
    await screen.findByText(
      /No named plugin references in current saved results/,
    );
    expect(screen.getByText(/1 unnamed references/)).toBeTruthy();
  });

  it("discards late results after refresh, root switches and close, and clears obsolete saved details", async () => {
    const pending: ((result: PluginExplorerResult) => void)[] = [];
    const adapter = adapterWith(
      () =>
        new Promise<PluginExplorerResult>((resolve) => pending.push(resolve)),
    );
    const view = render(<PluginExplorerPanel {...props} adapter={adapter} />);
    await open();
    await screen.findByText(/Loading saved plugin references/);
    await userEvent.click(
      screen.getByRole("button", { name: "Refresh Plugin Explorer" }),
    );
    await act(() => Promise.resolve(pending[1]!(explorerFixture())));
    await screen.findByText(/5 exact saved plugin groups/);
    await act(() =>
      Promise.resolve(pending[0]!({ rootId: props.rootId, state: "limited" })),
    );
    expect(screen.queryByText(/No partial totals/)).toBeNull();
    view.rerender(
      <PluginExplorerPanel
        {...props}
        rootId="another-root"
        adapter={adapter}
      />,
    );
    await screen.findByText(/Loading saved plugin references/);
    expect(screen.queryByRole("searchbox")).toBeNull();
    await userEvent.click(
      screen.getByRole("button", { name: "Close Plugin Explorer" }),
    );
    await act(() =>
      Promise.resolve(pending[2]!(explorerFixture("another-root"))),
    );
    expect(screen.queryByRole("searchbox")).toBeNull();
  });

  it("integrates with the Library while keeping matching-entry detail IDs distinct from Library rows", async () => {
    const fixture = explorerFixture();
    const root = {
      id: props.rootId,
      displayName: "Projects",
      canonicalPath: "C:\\Synthetic\\Projects",
      mode: "localNtfs" as const,
      enabled: true,
      availability: "available" as const,
      lastErrorCode: null,
    };
    const base = createFakeLibraryScanAdapter({
      roots: [root],
      files: fixture.entries,
      projectDetails: {
        "entry-a": {
          ...savedDetails(),
          rootId: props.rootId,
          locationId: "entry-a",
        },
      },
    });
    const adapter = {
      ...base,
      getPluginExplorer: () => Promise.resolve(fixture),
    };
    const view = render(
      <MemoryRouter>
        <LibraryPage adapter={adapter} />
      </MemoryRouter>,
    );
    await screen.findByRole("heading", { name: "File locations" });
    await open();
    await screen.findByText(/5 exact saved plugin groups/);
    await userEvent.click(
      screen.getByRole("button", { name: "Show matching entries for 3x Osc" }),
    );
    const details = screen.getAllByRole("button", {
      name: "Project details First.flp",
    });
    const ids = details.map((button) => button.getAttribute("aria-controls"));
    expect(new Set(ids).size).toBe(ids.length);
    await userEvent.click(details[0]!);
    await screen.findByText(/Matched the scanned file/);
    await accessible(view.container);
  });
});

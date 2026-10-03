import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import axe from "axe-core";
import { MemoryRouter } from "react-router";
import { describe, expect, it, vi } from "vitest";
import { ProjectPlugins } from "./ProjectPlugins";
import { builtinPlugin, savedPlugins } from "./pluginReferences.fixture";
import { ProjectDetailsPanel } from "./ProjectDetailsPanel";
import { LibraryPage } from "./LibraryPage";
import { createFakeLibraryScanAdapter } from "./fake";
import { identity, savedDetails } from "./projectDetails.fixture";
import type { PublishedFileLocation } from "./contracts";
import type { ProjectDetails } from "./projectDetails";
import type { ProjectPluginReferences } from "./pluginReferences";

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
const details = () => ({ ...savedDetails(), pluginReferences: savedPlugins() });
describe("saved plugin reference view", () => {
  it("shows per-field provenance and duplicates without installation/unique-count claims", async () => {
    const view = render(<ProjectPlugins plugins={savedPlugins()} />);
    expect(screen.getAllByText("3x Osc")).toHaveLength(4);
    expect(screen.getByText("Synthetic Vendor")).toBeTruthy();
    expect(
      screen.getByText(/Default Sampler name for this verified saved build/),
    ).toBeTruthy();
    expect(
      screen.getByText(/not checked plugin installation or availability/),
    ).toBeTruthy();
    expect(screen.getByText(/not a count of unique plugins/)).toBeTruthy();
    expect(
      screen.getByText(/not channel numbers or stable plugin IDs/),
    ).toBeTruthy();
    expect(screen.queryByRole("link")).toBeNull();
    expect(screen.queryByRole("button")).toBeNull();
    expect(view.container.querySelector("audio,img,iframe,object")).toBeNull();
    expect(
      (
        await axe.run(view.container, {
          rules: { "color-contrast": { enabled: false } },
        })
      ).violations,
    ).toEqual([]);
  });
  it("keeps hostile name/class/vendor text inert and visibly escapes controls/bidi", () => {
    const value =
      "<img src=x onerror=alert(1)> https://example.invalid/音\n\u202e";
    const raw = savedPlugins();
    const view = render(
      <ProjectPlugins plugins={{ ...raw, items: [builtinPlugin(1, value)] }} />,
    );
    expect(
      screen.getAllByText(
        "<img src=x onerror=alert(1)> https://example.invalid/音\\u000a\\u202e",
      ),
    ).toHaveLength(2);
    expect(view.container.querySelector("img,a")).toBeNull();
  });
  it.each([
    undefined,
    { coverage: "top-level-saved-references", items: [] } as const,
  ])("explains older and empty results %#", (plugins) => {
    render(<ProjectPlugins plugins={plugins} />);
    expect(
      screen.getByText(
        plugins === undefined
          ? /did not report plugin references/
          : /does not prove the project uses no plugins/,
      ),
    ).toBeTruthy();
    expect(screen.queryByRole("button")).toBeNull();
  });
  it("explains constructed missing/unsupported wrapper metadata separately", () => {
    const wrapper = savedPlugins().items[2]!;
    render(
      <ProjectPlugins
        plugins={{
          coverage: "top-level-saved-references",
          items: [
            {
              ...wrapper,
              position: 1,
              name: {
                status: "unavailable",
                value: null,
                reason: "PLUGIN_NAME_NOT_STORED",
              },
            },
            {
              ...wrapper,
              position: 2,
              name: {
                status: "unsupported",
                value: null,
                reason: "MULTIPLE_VST_METADATA_RECORDS",
              },
              vendor: {
                status: "unsupported",
                value: null,
                reason: "MULTIPLE_VST_METADATA_RECORDS",
              },
            },
          ],
        }}
      />,
    );
    expect(screen.getByText("No name was stored.")).toBeTruthy();
    expect(screen.getAllByText(/Multiple saved wrapper records/)).toHaveLength(
      2,
    );
  });
  it("shows batches and the final remainder with stable keyboard focus", async () => {
    const user = userEvent.setup();
    const plugins: ProjectPluginReferences = {
      coverage: "top-level-saved-references",
      items: Array.from({ length: 43 }, (_, i) => builtinPlugin(i + 1)),
    };
    render(<ProjectPlugins plugins={plugins} />);
    expect(screen.getAllByRole("listitem")).toHaveLength(20);
    const button = screen.getByRole("button", {
      name: "Show next 20 plugin references",
    });
    button.focus();
    await user.keyboard("{Enter}");
    expect(screen.getAllByRole("listitem")).toHaveLength(40);
    expect(document.activeElement).toBe(button);
    expect(button.textContent).toBe("Show next 3 plugin references");
    await user.keyboard("{Enter}");
    expect(screen.getAllByRole("listitem")).toHaveLength(43);
    expect(document.activeElement).toBe(button);
    await user.keyboard("{Enter}");
    expect(screen.getAllByRole("listitem")).toHaveLength(20);
    expect(document.activeElement).toBe(button);
  });
  it("accepts the maximum list while limiting initial rendering", () => {
    render(
      <ProjectPlugins
        plugins={{
          coverage: "top-level-saved-references",
          items: Array.from({ length: 1024 }, (_, i) => builtinPlugin(i + 1)),
        }}
      />,
    );
    expect(screen.getAllByRole("listitem")).toHaveLength(20);
    expect(screen.getByRole("status").textContent).toBe(
      "Showing 20 of 1024 plugin references.",
    );
  });
  it("refreshes read-only, resets batching for a new snapshot and keeps errors fixed", async () => {
    const user = userEvent.setup();
    const plugins: ProjectPluginReferences = {
      coverage: "top-level-saved-references",
      items: Array.from({ length: 43 }, (_, i) => builtinPlugin(i + 1)),
    };
    const read = vi
      .fn()
      .mockResolvedValueOnce({ ...details(), pluginReferences: plugins })
      .mockResolvedValueOnce({
        ...details(),
        snapshotId: "abcdefab-1234-1234-1234-123456789abc",
        pluginReferences: plugins,
      })
      .mockRejectedValueOnce(new Error("private plugin vendor.dll"));
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
    await screen.findByRole("region", { name: "Saved plugin references" });
    await user.click(
      screen.getByRole("button", { name: "Show next 20 plugin references" }),
    );
    expect(
      screen.getByText("Showing 40 of 43 plugin references."),
    ).toBeTruthy();
    await user.click(
      screen.getByRole("button", {
        name: "Refresh project details Example.flp",
      }),
    );
    await screen.findByText("Showing 20 of 43 plugin references.");
    await user.click(
      screen.getByRole("button", {
        name: "Refresh project details Example.flp",
      }),
    );
    await screen.findByRole("alert");
    expect(screen.queryByText(/private plugin/)).toBeNull();
    expect(
      screen.queryByRole("region", { name: "Saved plugin references" }),
    ).toBeNull();
    expect(read).toHaveBeenCalledTimes(3);
    expect(request).not.toHaveBeenCalled();
  });
  it("ignores a pending reply after closure", async () => {
    const user = userEvent.setup();
    let finish!: (value: ProjectDetails) => void;
    const read = vi.fn().mockImplementation(
      () =>
        new Promise<ProjectDetails>((resolve) => {
          finish = resolve;
        }),
    );
    render(
      <ProjectDetailsPanel
        record={record}
        adapter={{ ...createFakeLibraryScanAdapter(), getProjectDetails: read }}
      />,
    );
    const toggle = screen.getByRole("button", {
      name: "Project details Example.flp",
    });
    await user.click(toggle);
    await user.click(toggle);
    await act(async () => {
      finish(details());
      await Promise.resolve();
    });
    expect(
      screen.queryByRole("region", { name: "Saved plugin references" }),
    ).toBeNull();
    expect(document.activeElement).toBe(toggle);
  });
  it.each(["new_scan", "disabled_root"])(
    "clears references and ignores late reads after %s",
    async (transition) => {
      const user = userEvent.setup();
      let generation = 1;
      let enabled = true;
      let finish!: (value: ProjectDetails) => void;
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
      await screen.findByRole("region", { name: "Saved plugin references" });
      await user.click(
        screen.getByRole("button", {
          name: "Refresh project details Example.flp",
        }),
      );
      if (transition === "new_scan") generation += 1;
      else enabled = false;
      await act(async () => {
        window.dispatchEvent(new Event("focus"));
        await Promise.resolve();
      });
      // Await reconciliation, not the loading state's already-empty view.
      await waitFor(() =>
        expect(
          screen
            .getByRole("button", { name: "Project details Example.flp" })
            .getAttribute("aria-expanded"),
        ).toBe("false"),
      );
      await waitFor(() =>
        expect(
          screen.queryByRole("region", { name: "Saved plugin references" }),
        ).toBeNull(),
      );
      await act(async () => {
        finish(details());
        await Promise.resolve();
      });
      expect(
        screen.queryByRole("region", { name: "Saved plugin references" }),
      ).toBeNull();
      expect(screen.queryByText("Synthetic Vendor")).toBeNull();
    },
  );
});

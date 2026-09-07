import { MemoryRouter } from "react-router";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import type { ScanRoot } from "../platform/contracts";
import { createFakeLibraryScanAdapter } from "./fake";
import { LibraryPage } from "./LibraryPage";
import type {
  LibraryPage as LibraryPageData,
  LibraryPageRequest,
  LibraryScanAdapter,
  PublishedFileLocation,
  ScanStatus,
} from "./contracts";

const rootA: ScanRoot = {
  id: "root-a",
  displayName: "Projects",
  canonicalPath: "C:\\Synthetic\\Music\\Projects",
  enabled: true,
  availability: "available",
  lastErrorCode: null,
};

const rootB: ScanRoot = {
  id: "root-b",
  displayName: "Projects",
  canonicalPath: "D:\\Synthetic\\Archive\\Projects",
  enabled: true,
  availability: "available",
  lastErrorCode: null,
};

const makeRecord = (
  root: ScanRoot,
  locationId: string,
  fileName: string,
  relativePath: string,
  presence: PublishedFileLocation["presence"] = "present",
  byteSize = "1024",
): PublishedFileLocation => ({
  locationId,
  rootId: root.id,
  rootDisplayName: root.displayName,
  rootCanonicalPath: root.canonicalPath,
  fileName,
  relativePath,
  byteSize,
  modifiedAt: "2026-01-02T03:04:05.123456789Z",
  presence,
});

function renderLibrary(adapter: LibraryScanAdapter) {
  return render(
    <MemoryRouter>
      <LibraryPage adapter={adapter} />
    </MemoryRouter>,
  );
}

type Deferred<T> = {
  readonly promise: Promise<T>;
  readonly resolve: (value: T) => void;
  readonly reject: (error: unknown) => void;
};

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((nextResolve, nextReject) => {
    resolve = nextResolve;
    reject = nextReject;
  });
  return { promise, resolve, reject };
}

type DeferredPageRequest = {
  readonly request: LibraryPageRequest;
  readonly deferred: Deferred<LibraryPageData>;
};

type DeferredStatusRequest = Deferred<readonly ScanStatus[]>;

function makeStatus(
  root: ScanRoot,
  state: ScanStatus["state"] = "idle",
  jobId: string | null = null,
  runId: string | null = null,
): ScanStatus {
  return {
    root,
    state,
    jobId,
    runId,
    cancellationRequested: false,
    counters: {
      filesObserved: 0,
      directoriesVisited: 0,
      totalFiles: null,
    },
    lastSuccessfulScanAt: null,
    lastOutcomeAt: null,
    errorCode: null,
  };
}

function makeDeferredAdapter(roots: readonly ScanRoot[]) {
  const primary = roots[0] ?? rootA;
  const pageRequests: DeferredPageRequest[] = [];
  const statusRequests: DeferredStatusRequest[] = [];
  const listeners = new Set<() => void>();
  const adapter: LibraryScanAdapter = {
    getLibraryPage(request) {
      const entry = deferred<LibraryPageData>();
      pageRequests.push({ request: { ...request }, deferred: entry });
      return entry.promise;
    },
    listScanStatuses() {
      const entry = deferred<readonly ScanStatus[]>();
      statusRequests.push(entry);
      return entry.promise;
    },
    scanNow: (rootId) =>
      Promise.resolve({
        rootId,
        jobId: "deferred-job",
        runId: null,
        outcome: "queued" as const,
      }),
    cancelScan: (jobId) =>
      Promise.resolve({
        rootId: primary.id,
        jobId,
        runId: null,
        outcome: "cancelled" as const,
      }),
    retryScan: (jobId) =>
      Promise.resolve({
        rootId: primary.id,
        jobId,
        runId: null,
        outcome: "queued" as const,
      }),
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
  return {
    adapter,
    pageRequests,
    statusRequests,
    emit() {
      for (const listener of listeners) listener();
    },
  };
}

async function settle<T>(entry: Deferred<T>, value: T) {
  await act(async () => {
    entry.resolve(value);
    await Promise.resolve();
  });
}

function page(
  rootId: string,
  snapshotId: string,
  records: readonly PublishedFileLocation[],
  nextCursor: string | null = null,
): LibraryPageData {
  return { rootId, snapshotId, records, nextCursor };
}

describe("LibraryPage", () => {
  it("renders the initial loading state before an empty committed dataset", async () => {
    const adapter = createFakeLibraryScanAdapter();
    const view = renderLibrary(adapter);

    expect(
      view.container.querySelector('[data-library-state="loading"]'),
    ).toBeTruthy();
    expect(
      screen.getByRole("heading", { name: "Loading your library" }),
    ).toBeTruthy();
    expect(
      await screen.findByRole("heading", { name: "No committed files yet" }),
    ).toBeTruthy();
    expect(
      view.container.querySelector('[data-library-state="empty"]'),
    ).toBeTruthy();
    expect(screen.getByText(/No scan roots are configured/)).toBeTruthy();
  });

  it("loads a bounded per-root page and never mixes roots", async () => {
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA, rootB],
      pageLimit: 2,
      files: [
        makeRecord(rootA, "location-a", "First.flp", "First.flp"),
        makeRecord(rootB, "location-b", "Second.flp", "Second.flp"),
        makeRecord(rootB, "location-c", "Third.flp", "Third.flp", "missing"),
      ],
    });
    renderLibrary(adapter);

    expect(
      await screen.findByRole("heading", { name: "Your FLP library" }),
    ).toBeTruthy();
    expect(
      await screen.findByRole("heading", { name: "First.flp" }),
    ).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "Second.flp" })).toBeNull();
    expect(screen.getByText("Present")).toBeTruthy();
    expect(screen.getByText("1,024 bytes")).toBeTruthy();
    expect(screen.getAllByText(/Jan 2, 2026/)).not.toHaveLength(0);
    expect(
      screen.getByLabelText("Root Projects (C:\\Synthetic\\Music\\Projects)"),
    ).toBeTruthy();
    expect(adapter.calls.pages.every((limit) => limit <= 200)).toBe(true);
    expect(
      adapter.calls.pageRequests.every(
        (request) => request.rootId === rootA.id && request.cursor === null,
      ),
    ).toBe(true);

    const selector = screen.getByLabelText("Scan root");
    const options = within(selector).getAllByRole("option");
    expect(options.map((option) => option.textContent)).toEqual([
      "Projects (C:\\Synthetic\\Music\\Projects)",
      "Projects (D:\\Synthetic\\Archive\\Projects)",
    ]);

    await userEvent.setup().selectOptions(selector, rootB.id);
    expect(
      await screen.findByRole("heading", { name: "Second.flp" }),
    ).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Third.flp" })).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "First.flp" })).toBeNull();
    expect(screen.getByText("Missing")).toBeTruthy();
    expect(
      adapter.calls.pageRequests[adapter.calls.pageRequests.length - 1],
    ).toEqual({ rootId: rootB.id, limit: 4, cursor: null });
  });

  it("navigates per-root pages back without losing focus", async () => {
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA],
      pageLimit: 2,
      files: [
        makeRecord(rootA, "location-a", "First.flp", "First.flp"),
        makeRecord(rootA, "location-b", "Second.flp", "Second.flp"),
        makeRecord(rootA, "location-c", "Third.flp", "Third.flp", "missing"),
      ],
    });
    renderLibrary(adapter);

    expect(
      await screen.findByRole("heading", { name: "First.flp" }),
    ).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Second.flp" })).toBeTruthy();
    expect(screen.queryByText("Missing")).toBeNull();

    const next = screen.getByRole("button", { name: "Next library page" });
    await userEvent.setup().click(next);
    expect(
      await screen.findByRole("heading", { name: "Third.flp" }),
    ).toBeTruthy();
    expect(screen.getByText("Missing")).toBeTruthy();
    await waitFor(() => {
      expect(document.activeElement).toBe(
        screen.getByRole("heading", { name: "File locations" }),
      );
    });

    await userEvent
      .setup()
      .click(screen.getByRole("button", { name: "Previous library page" }));
    expect(
      await screen.findByRole("heading", { name: "First.flp" }),
    ).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "Third.flp" })).toBeNull();
    await waitFor(() => {
      expect(document.activeElement).toBe(
        screen.getByRole("heading", { name: "File locations" }),
      );
    });
  });

  it("discards a stale response when switching roots while a request is pending", async () => {
    const harness = makeDeferredAdapter([rootA, rootB]);
    renderLibrary(harness.adapter);
    await waitFor(() => {
      expect(harness.pageRequests).toHaveLength(0);
      expect(harness.statusRequests).toHaveLength(1);
    });
    await settle(harness.statusRequests[0]!, [
      makeStatus(rootA),
      makeStatus(rootB),
    ]);
    await waitFor(() => expect(harness.pageRequests).toHaveLength(1));
    expect(harness.pageRequests[0]!.request.rootId).toBe(rootA.id);
    await settle(
      harness.pageRequests[0]!.deferred,
      page(rootA.id, "snapshot-a", [
        makeRecord(rootA, "a", "RootA.flp", "RootA.flp"),
      ]),
    );
    await screen.findByRole("heading", { name: "RootA.flp" });

    harness.emit();
    await waitFor(() => expect(harness.pageRequests).toHaveLength(2));

    await userEvent
      .setup()
      .selectOptions(screen.getByLabelText("Scan root"), rootB.id);
    await waitFor(() => expect(harness.pageRequests).toHaveLength(3));
    expect(harness.pageRequests[2]!.request).toEqual({
      rootId: rootB.id,
      limit: 4,
      cursor: null,
    });

    await settle(
      harness.pageRequests[1]!.deferred,
      page(rootA.id, "snapshot-a", [
        makeRecord(rootA, "late", "LateRootA.flp", "LateRootA.flp"),
      ]),
    );
    await settle(
      harness.pageRequests[2]!.deferred,
      page(rootB.id, "snapshot-b", [
        makeRecord(rootB, "b", "RootB.flp", "RootB.flp"),
      ]),
    );

    expect(screen.getByRole("heading", { name: "RootB.flp" })).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "LateRootA.flp" })).toBeNull();
    expect(screen.queryByRole("heading", { name: "RootA.flp" })).toBeNull();
  });

  it("keeps committed results separate while a fake scan runs and cancels", async () => {
    const user = userEvent.setup();
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA],
      files: [
        makeRecord(rootA, "location-a", "Committed.flp", "Committed.flp"),
      ],
    });
    renderLibrary(adapter);
    await screen.findByRole("heading", { name: "Committed.flp" });

    await user.click(screen.getByRole("button", { name: "Scan now Projects" }));
    expect(await screen.findByText("Queued")).toBeTruthy();
    expect(screen.getByText("Total work unknown")).toBeTruthy();
    expect(
      screen.getByText(
        "Progress is shown as counters; no percentage is estimated.",
      ),
    ).toBeTruthy();
    expect(document.body.textContent).not.toMatch(/\d+%/);
    await waitFor(() => {
      expect(document.activeElement).toBe(
        screen.getByRole("button", { name: "Cancel scan Projects" }),
      );
    });

    adapter.advanceRun(rootA.id);
    expect(await screen.findByText("Running")).toBeTruthy();
    await user.click(
      screen.getByRole("button", { name: "Cancel scan Projects" }),
    );

    expect(await screen.findByText("Cancelled")).toBeTruthy();
    expect(screen.getByText("Showing previous committed results")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Committed.flp" })).toBeTruthy();
    expect(screen.getByText("Present")).toBeTruthy();
    await waitFor(() => {
      expect(document.activeElement).toBe(
        screen.getByRole("button", { name: "Retry scan Projects" }),
      );
    });
    expect(adapter.calls.scanNow).toEqual([rootA.id]);
    expect(adapter.calls.cancelScan).toEqual(["fake-job-1"]);
  });

  it("labels failed and interrupted runs while retaining the old page", async () => {
    const user = userEvent.setup();
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA],
      files: [makeRecord(rootA, "location-a", "Kept.flp", "Kept.flp")],
    });
    renderLibrary(adapter);
    await screen.findByRole("heading", { name: "Kept.flp" });

    await user.click(screen.getByRole("button", { name: "Scan now Projects" }));
    adapter.advanceRun(rootA.id);
    adapter.failScan(rootA.id, "access_denied");
    expect(await screen.findByText("Failed")).toBeTruthy();
    expect(
      screen.getByText(
        "The folder could not be read. Previous committed results were kept.",
      ),
    ).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Kept.flp" })).toBeTruthy();
    expect(screen.getByText("Showing previous committed results")).toBeTruthy();

    await user.click(
      screen.getByRole("button", { name: "Retry scan Projects" }),
    );
    expect(await screen.findByText("Queued")).toBeTruthy();
    expect(
      screen.getByRole("button", { name: "Cancel scan Projects" }),
    ).toBeTruthy();

    adapter.advanceRun(rootA.id);
    adapter.interruptScan(rootA.id);
    expect(await screen.findByText("Interrupted")).toBeTruthy();
    expect(
      screen.getByText(/previous committed results were kept/i),
    ).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Kept.flp" })).toBeTruthy();
  });

  it("shows an unavailable-root state without manufacturing missing files", async () => {
    const user = userEvent.setup();
    const unavailableRoot = { ...rootA, availability: "unavailable" as const };
    const adapter = createFakeLibraryScanAdapter({ roots: [unavailableRoot] });
    renderLibrary(adapter);

    expect(
      await screen.findByRole("heading", {
        name: "A scan root is unavailable",
      }),
    ).toBeTruthy();
    expect(screen.getAllByText("Unavailable")).toHaveLength(2);
    expect(screen.queryByText("Missing")).toBeNull();
    const retry = screen.getByRole("button", { name: "Retry scan Projects" });
    expect(retry).toHaveProperty("disabled", false);
    await user.click(retry);
    expect(
      await screen.findByText(
        "The folder is unavailable. Previous committed results were kept.",
      ),
    ).toBeTruthy();
  });

  it("recovers a page read error through the explicit retry action", async () => {
    const user = userEvent.setup();
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA],
      files: [
        makeRecord(rootA, "location-a", "Recovered.flp", "Recovered.flp"),
      ],
    });
    adapter.setPageError("unavailable");
    renderLibrary(adapter);

    expect(
      await screen.findByRole("heading", {
        name: "Library results unavailable",
      }),
    ).toBeTruthy();
    expect(screen.getByText(/No files were marked missing/)).toBeTruthy();
    adapter.setPageError(null);
    await user.click(screen.getByRole("button", { name: "Try again" }));
    expect(
      await screen.findByRole("heading", { name: "Recovered.flp" }),
    ).toBeTruthy();
  });

  it("cancels queued work by job ID and keeps the committed page", async () => {
    const user = userEvent.setup();
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA],
      files: [makeRecord(rootA, "location-a", "Queued.flp", "Queued.flp")],
    });
    renderLibrary(adapter);
    await screen.findByRole("heading", { name: "Queued.flp" });

    await user.click(screen.getByRole("button", { name: "Scan now Projects" }));
    await screen.findByText("Queued");
    await expect(adapter.listScanStatuses()).resolves.toEqual([
      expect.objectContaining({
        jobId: "fake-job-1",
        runId: null,
        state: "queued",
      }),
    ]);
    await user.click(
      screen.getByRole("button", { name: "Cancel scan Projects" }),
    );

    await screen.findByText("Cancelled");
    expect(screen.getByRole("heading", { name: "Queued.flp" })).toBeTruthy();
    expect(adapter.calls.cancelScan).toEqual(["fake-job-1"]);
  });

  it("allows unknown availability to recover without presenting it as a live check", async () => {
    const user = userEvent.setup();
    const adapter = createFakeLibraryScanAdapter({
      roots: [{ ...rootA, availability: "unknown" }],
    });
    renderLibrary(adapter);

    await screen.findByRole("heading", { name: "No committed files yet" });
    expect(
      screen.getByText(
        "Reachability has not been checked recently; Scan now will verify access.",
      ),
    ).toBeTruthy();
    const scan = screen.getByRole("button", { name: "Scan now Projects" });
    expect(scan).toHaveProperty("disabled", false);

    adapter.setRootAvailability(rootA.id, "available");
    await user.click(screen.getByRole("button", { name: "Scan now Projects" }));
    expect(await screen.findByText("Queued")).toBeTruthy();
  });

  it("renders maximum decimal numerics without precision loss", async () => {
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA],
      files: [
        {
          ...makeRecord(rootA, "location-max", "Huge.flp", "Huge.flp"),
          byteSize: "18446744073709551615",
          modifiedAt: "2026-01-02T03:04:05.123456789Z",
        },
      ],
    });
    renderLibrary(adapter);

    expect(
      await screen.findByRole("heading", { name: "Huge.flp" }),
    ).toBeTruthy();
    expect(screen.getByText("18,446,744,073,709,551,615 bytes")).toBeTruthy();
    expect(screen.getAllByText(/Jan 2, 2026/)).not.toHaveLength(0);
  });

  it("ignores an older page subscription refresh that resolves after the newer one", async () => {
    const harness = makeDeferredAdapter([rootA]);
    renderLibrary(harness.adapter);
    await waitFor(() => {
      expect(harness.statusRequests).toHaveLength(1);
    });
    await settle(harness.statusRequests[0]!, [makeStatus(rootA)]);
    await waitFor(() => expect(harness.pageRequests).toHaveLength(1));
    expect(harness.pageRequests[0]!.request).toEqual({
      rootId: rootA.id,
      limit: 4,
      cursor: null,
    });
    await settle(
      harness.pageRequests[0]!.deferred,
      page(rootA.id, "snapshot-1", [
        makeRecord(rootA, "base", "Base.flp", "Base.flp"),
      ]),
    );
    await screen.findByRole("heading", { name: "Base.flp" });

    harness.emit();
    harness.emit();
    await waitFor(() => {
      expect(harness.pageRequests).toHaveLength(3);
      expect(harness.statusRequests).toHaveLength(3);
    });
    await settle(
      harness.pageRequests[2]!.deferred,
      page(rootA.id, "snapshot-1", [
        makeRecord(rootA, "new", "Newest.flp", "Newest.flp"),
      ]),
    );
    await screen.findByRole("heading", { name: "Newest.flp" });
    await settle(
      harness.pageRequests[1]!.deferred,
      page(rootA.id, "snapshot-1", [
        makeRecord(rootA, "old", "Older.flp", "Older.flp"),
      ]),
    );

    expect(screen.getByRole("heading", { name: "Newest.flp" })).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "Older.flp" })).toBeNull();
  });

  it("invalidates a page-one response as soon as the cursor changes", async () => {
    const harness = makeDeferredAdapter([rootA]);
    const user = userEvent.setup();
    renderLibrary(harness.adapter);
    await waitFor(() => expect(harness.statusRequests).toHaveLength(1));
    await settle(harness.statusRequests[0]!, [makeStatus(rootA)]);
    await waitFor(() => expect(harness.pageRequests).toHaveLength(1));
    await settle(
      harness.pageRequests[0]!.deferred,
      page(
        rootA.id,
        "snapshot-1",
        [makeRecord(rootA, "first", "First.flp", "First.flp")],
        "cursor-1",
      ),
    );
    await screen.findByRole("heading", { name: "First.flp" });

    harness.emit();
    await user.click(screen.getByRole("button", { name: "Next library page" }));
    await waitFor(() => expect(harness.pageRequests).toHaveLength(3));
    expect(harness.pageRequests[2]!.request.cursor).toBe("cursor-1");
    expect(harness.pageRequests[2]!.request.rootId).toBe(rootA.id);
    await settle(
      harness.pageRequests[2]!.deferred,
      page(rootA.id, "snapshot-1", [
        makeRecord(rootA, "second", "Second.flp", "Second.flp"),
      ]),
    );
    await screen.findByRole("heading", { name: "Second.flp" });
    await settle(
      harness.pageRequests[1]!.deferred,
      page(
        rootA.id,
        "snapshot-1",
        [
          makeRecord(
            rootA,
            "late",
            "Late first page.flp",
            "Late first page.flp",
          ),
        ],
        "cursor-1",
      ),
    );

    expect(screen.getByRole("heading", { name: "Second.flp" })).toBeTruthy();
    expect(
      screen.queryByRole("heading", { name: "Late first page.flp" }),
    ).toBeNull();
  });

  it("ignores an older status refresh that resolves after the newer one", async () => {
    const harness = makeDeferredAdapter([rootA]);
    renderLibrary(harness.adapter);
    await waitFor(() => {
      expect(harness.statusRequests).toHaveLength(1);
    });
    await settle(harness.statusRequests[0]!, [makeStatus(rootA)]);
    await waitFor(() => expect(harness.pageRequests).toHaveLength(1));
    await settle(
      harness.pageRequests[0]!.deferred,
      page(rootA.id, "snapshot-1", [
        makeRecord(rootA, "base", "Base.flp", "Base.flp"),
      ]),
    );
    await screen.findByRole("heading", { name: "Base.flp" });

    harness.emit();
    harness.emit();
    await waitFor(() => expect(harness.statusRequests).toHaveLength(3));
    await settle(harness.statusRequests[2]!, [
      makeStatus(rootA, "queued", "new-job", null),
    ]);
    await screen.findByText("Queued");
    await settle(harness.statusRequests[1]!, [makeStatus(rootA)]);

    expect(screen.getByText("Queued")).toBeTruthy();
    expect(screen.queryByText("Not scanned yet")).toBeNull();
  });

  it("restarts per-root pagination when publication invalidates a cursor", async () => {
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA, rootB],
      pageLimit: 1,
      files: [
        makeRecord(rootA, "location-a", "First.flp", "First.flp"),
        makeRecord(rootA, "location-b", "Second.flp", "Second.flp"),
        makeRecord(rootA, "location-d", "Third.flp", "Third.flp"),
        makeRecord(rootB, "location-c", "Other.flp", "Other.flp"),
      ],
    });
    const user = userEvent.setup();
    renderLibrary(adapter);
    await screen.findByRole("heading", { name: "First.flp" });
    await user.click(screen.getByRole("button", { name: "Next library page" }));
    await screen.findByRole("heading", { name: "Second.flp" });

    adapter.completeScan(rootA.id);

    await screen.findByRole("heading", { name: "First.flp" });
    expect(screen.getByText("Page 1")).toBeTruthy();
    const lastRequest =
      adapter.calls.pageRequests[adapter.calls.pageRequests.length - 1];
    expect(lastRequest).toEqual({
      rootId: rootA.id,
      cursor: null,
      limit: 4,
    });
  });

  it("keeps a sibling root page valid when another root publishes", async () => {
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA, rootB],
      pageLimit: 1,
      files: [
        makeRecord(rootA, "location-a", "First.flp", "First.flp"),
        makeRecord(rootB, "location-b", "BFirst.flp", "BFirst.flp"),
        makeRecord(rootB, "location-c", "BSecond.flp", "BSecond.flp"),
        makeRecord(rootB, "location-d", "BThird.flp", "BThird.flp"),
      ],
    });
    const user = userEvent.setup();
    renderLibrary(adapter);
    await screen.findByRole("heading", { name: "First.flp" });

    await userEvent
      .setup()
      .selectOptions(screen.getByLabelText("Scan root"), rootB.id);
    await screen.findByRole("heading", { name: "BFirst.flp" });
    await user.click(screen.getByRole("button", { name: "Next library page" }));
    await screen.findByRole("heading", { name: "BSecond.flp" });

    adapter.completeScan(rootA.id);

    expect(screen.getByRole("heading", { name: "BSecond.flp" })).toBeTruthy();
    expect(screen.queryByText(/snapshot changed/i)).toBeNull();
  });

  it("ignores deferred responses after the adapter changes or the page unmounts", async () => {
    const first = makeDeferredAdapter([rootA]);
    const secondRoot = { ...rootA, id: "root-b" };
    const second = makeDeferredAdapter([secondRoot]);
    const view = renderLibrary(first.adapter);
    await waitFor(() => {
      expect(first.statusRequests).toHaveLength(1);
    });
    await settle(first.statusRequests[0]!, [makeStatus(rootA)]);
    await waitFor(() => {
      expect(first.pageRequests).toHaveLength(1);
    });

    view.rerender(
      <MemoryRouter>
        <LibraryPage adapter={second.adapter} />
      </MemoryRouter>,
    );
    await waitFor(() => {
      expect(second.statusRequests).toHaveLength(1);
    });
    await settle(second.statusRequests[0]!, [makeStatus(secondRoot)]);
    await waitFor(() => {
      expect(second.pageRequests).toHaveLength(1);
    });
    await settle(
      second.pageRequests[0]!.deferred,
      page(secondRoot.id, "snapshot-second", [
        makeRecord(
          secondRoot,
          "second",
          "Second adapter.flp",
          "Second adapter.flp",
        ),
      ]),
    );
    await screen.findByRole("heading", { name: "Second adapter.flp" });

    await settle(
      first.pageRequests[0]!.deferred,
      page(rootA.id, "snapshot-first", [
        makeRecord(rootA, "first", "First adapter.flp", "First adapter.flp"),
      ]),
    );
    await settle(first.statusRequests[0]!, [makeStatus(rootA)]);
    expect(
      screen.getByRole("heading", { name: "Second adapter.flp" }),
    ).toBeTruthy();
    expect(
      screen.queryByRole("heading", { name: "First adapter.flp" }),
    ).toBeNull();

    second.emit();
    await waitFor(() => {
      expect(second.pageRequests).toHaveLength(2);
      expect(second.statusRequests).toHaveLength(2);
    });
    view.unmount();
    await settle(
      second.pageRequests[1]!.deferred,
      page(secondRoot.id, "snapshot-late", []),
    );
    await settle(second.statusRequests[1]!, [makeStatus(secondRoot)]);
  });

  it("surfaces a recoverable error when a queued status carries no job ID", async () => {
    const harness = makeDeferredAdapter([rootA]);
    renderLibrary(harness.adapter);
    await waitFor(() => {
      expect(harness.statusRequests).toHaveLength(1);
    });
    await settle(harness.statusRequests[0]!, [
      makeStatus(rootA, "queued", null, null),
    ]);
    const cancel = await screen.findByRole("button", {
      name: "Cancel scan Projects",
    });
    await userEvent.setup().click(cancel);

    expect(
      await screen.findByText(
        "The scan could not be cancelled safely. Refresh and try again.",
      ),
    ).toBeTruthy();
    expect(document.activeElement).toBe(cancel);
  });

  it("coalesces restarts during a rapid snapshot burst instead of looping", async () => {
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA],
      pageLimit: 1,
      files: [
        makeRecord(rootA, "location-a", "First.flp", "First.flp"),
        makeRecord(rootA, "location-b", "Second.flp", "Second.flp"),
        makeRecord(rootA, "location-c", "Third.flp", "Third.flp"),
      ],
    });
    renderLibrary(adapter);
    await screen.findByRole("heading", { name: "First.flp" });
    await userEvent
      .setup()
      .click(screen.getByRole("button", { name: "Next library page" }));
    await screen.findByRole("heading", { name: "Second.flp" });

    act(() => {
      adapter.completeScan(rootA.id);
      adapter.completeScan(rootA.id);
      adapter.completeScan(rootA.id);
    });

    await screen.findByRole("heading", { name: "First.flp" });
    expect(screen.getByText("Page 1")).toBeTruthy();
    expect(
      screen.getByText(
        "The committed Library snapshot changed. Pagination restarted at page 1.",
      ),
    ).toBeTruthy();

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 500));
    });
    const stableRequestCount = adapter.calls.pageRequests.length;
    expect(stableRequestCount).toBeLessThan(12);
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 500));
    });
    expect(adapter.calls.pageRequests.length).toBe(stableRequestCount);
    expect(screen.getByRole("heading", { name: "First.flp" })).toBeTruthy();
    expect(screen.getByText("Page 1")).toBeTruthy();
  });

  it("disambiguates detached-history records that share an active root name", async () => {
    const statuses = [makeStatus(rootA)];
    const recordPage = page(rootA.id, "snapshot-union", [
      makeRecord(rootA, "location-active", "Active.flp", "Active.flp"),
      {
        ...makeRecord(
          rootA,
          "location-detached",
          "Detached.flp",
          "Detached.flp",
        ),
        rootId: "root-detached-history",
        rootDisplayName: rootA.displayName,
        rootCanonicalPath: "E:\\Removed\\Projects",
      },
    ]);
    const adapter: LibraryScanAdapter = {
      getLibraryPage: () => Promise.resolve(recordPage),
      listScanStatuses: () => Promise.resolve(statuses),
      scanNow: (rootId) =>
        Promise.resolve({
          rootId,
          jobId: "static-job",
          runId: null,
          outcome: "queued" as const,
        }),
      cancelScan: (jobId) =>
        Promise.resolve({
          rootId: rootA.id,
          jobId,
          runId: null,
          outcome: "cancelled" as const,
        }),
      retryScan: (jobId) =>
        Promise.resolve({
          rootId: rootA.id,
          jobId,
          runId: null,
          outcome: "queued" as const,
        }),
      subscribe: () => () => {},
    };
    renderLibrary(adapter);

    await screen.findByRole("heading", { name: "Detached.flp" });
    expect(
      screen.getByLabelText("Root Projects (C:\\Synthetic\\Music\\Projects)"),
    ).toBeTruthy();
    expect(
      screen.getByLabelText("Root Projects (E:\\Removed\\Projects)"),
    ).toBeTruthy();
  });
});

import { MemoryRouter } from "react-router";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ScanRoot } from "../platform/contracts";
import { LibraryAdapterError } from "./contracts";
import { createFakeLibraryScanAdapter } from "./fake";
import {
  createNativeLibraryScanAdapter,
  GET_LIBRARY_PAGE_COMMAND,
  GET_SCAN_CONSOLE_STATE_COMMAND,
  LIST_SCAN_STATUSES_COMMAND,
  type NativeLibraryTransport,
} from "./native";
import { LibraryPage } from "./LibraryPage";
import type {
  LibraryPage as LibraryPageData,
  LibraryPageRequest,
  LibraryRenderContext,
  LibraryScanAdapter,
  PublishedFileLocation,
  ScanStatus,
} from "./contracts";

afterEach(() => {
  vi.useRealTimers();
});

const rootA: ScanRoot = {
  id: "root-a",
  displayName: "Projects",
  canonicalPath: "C:\\Synthetic\\Music\\Projects",
  mode: "localNtfs",
  enabled: true,
  availability: "available",
  lastErrorCode: null,
};

const rootB: ScanRoot = {
  id: "root-b",
  displayName: "Projects",
  canonicalPath: "D:\\Synthetic\\Archive\\Projects",
  mode: "localNtfs",
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

/** The record facts render as a `dt`/`dd` definition list; the root fact is
 * located by its visible definition text (the `dt` "Root" names the pair). */
const recordRootFact = (label: string) =>
  screen.getByText(
    (_content, element) =>
      element?.tagName === "DD" && element.textContent === label,
  );

function renderLibrary(
  adapter: LibraryScanAdapter,
  renderContext: LibraryRenderContext = "native",
) {
  return render(
    <MemoryRouter>
      <LibraryPage adapter={adapter} renderContext={renderContext} />
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
  retryAvailable = state === "failed" && root.enabled,
): ScanStatus {
  return {
    root,
    state,
    jobId,
    runId,
    cancellationRequested: false,
    retryAvailable,
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
    getConsoleState: () =>
      Promise.resolve({ enabled: true, runtimeAvailable: true }),
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
    subscribe(listener, onStateChange) {
      listeners.add(listener);
      onStateChange?.("attached");
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

async function drainFakeTimerTurns() {
  // A health response can commit after one act, and that commit mounts effects
  // which schedule their own zero-delay reads. Advance by a bounded positive
  // step because the fake clock may schedule a new zero-delay timer at +1 ms.
  for (let pass = 0; pass < 3; pass += 1) {
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
      await Promise.resolve();
    });
  }
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
  it("keeps harness labeling out of the native product rendering", async () => {
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA],
      files: [makeRecord(rootA, "location-a", "Native.flp", "Native.flp")],
    });
    const nativeView = renderLibrary(adapter);

    await screen.findByRole("heading", { name: "Native.flp" });
    expect(
      nativeView.container.querySelector('[data-review-adapter="native"]'),
    ).toBeTruthy();
    expect(screen.queryByText("Review harness · fake adapter")).toBeNull();
    nativeView.unmount();

    const harnessView = renderLibrary(adapter, "review-harness");
    await screen.findByRole("heading", { name: "Native.flp" });
    expect(
      harnessView.container.querySelector(
        '[data-review-adapter="fake-or-proposed"]',
      ),
    ).toBeTruthy();
    expect(screen.getByText("Review harness · fake adapter")).toBeTruthy();
  });

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

  it("shows the honest disabled panel when this build has no scanning", async () => {
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA],
      consoleEnabled: false,
    });
    const view = renderLibrary(adapter);

    expect(
      await screen.findByRole("heading", {
        name: "Library scanning is not enabled in this build",
      }),
    ).toBeTruthy();
    expect(
      view.container.querySelector('[data-library-state="disabled"]'),
    ).toBeTruthy();
    expect(
      screen.getByText(/produced without the scanning feature/),
    ).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Scan now/i })).toBeNull();
    expect(
      view.container.querySelector('[data-library-state="error"]'),
    ).toBeNull();
    expect(screen.queryByText(/Try again/)).toBeNull();
  });

  it("polls without a listener, refreshes stopped health, and reconciles on focus", async () => {
    vi.useFakeTimers();
    const idleRoot: ScanRoot = {
      ...rootB,
      id: "root-idle",
      displayName: "Archive",
      canonicalPath: "D:\\Synthetic\\Archive",
    };
    let execution: ScanStatus["state"] = "running";
    let runtimeAvailable = true;
    let records: readonly PublishedFileLocation[] = [
      makeRecord(rootA, "saved-location", "Saved.flp", "Saved.flp"),
    ];
    const calls: string[] = [];
    const transport: NativeLibraryTransport = {
      invoke: (command) => {
        calls.push(command);
        const data =
          command === GET_SCAN_CONSOLE_STATE_COMMAND
            ? { enabled: true, runtimeAvailable }
            : command === LIST_SCAN_STATUSES_COMMAND
              ? [
                  {
                    ...makeStatus(
                      rootA,
                      execution,
                      "native-job",
                      execution === "idle" ? null : "native-run",
                    ),
                    lastOutcomeAt:
                      execution === "completed" ? "2026-09-29T00:00:00Z" : null,
                    lastSuccessfulScanAt:
                      execution === "completed" ? "2026-09-29T00:00:00Z" : null,
                  },
                  makeStatus(rootB, "failed", "retry-job", null),
                  makeStatus(idleRoot),
                ]
              : {
                  rootId: rootA.id,
                  snapshotId:
                    execution === "completed" ? "snapshot-2" : "snapshot-1",
                  records,
                  nextCursor: null,
                };
        return Promise.resolve({
          status: "ok",
          schemaVersion: 1,
          correlationId: "correlation_00000000000000000000000000000001",
          data,
        });
      },
      listen: () => Promise.reject(new Error("synthetic listener failure")),
    };
    const view = renderLibrary(createNativeLibraryScanAdapter(transport));
    try {
      await act(async () => {
        await vi.advanceTimersByTimeAsync(250);
      });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0);
      });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(250);
      });
      await drainFakeTimerTurns();
      expect(
        screen.getByRole("button", { name: /Cancel scan Projects/ }),
      ).toBeTruthy();
      expect(
        screen.getByText(/Live scan updates are not connected/),
      ).toBeTruthy();

      runtimeAvailable = false;
      await act(async () => {
        await vi.advanceTimersByTimeAsync(5_000);
      });
      expect(screen.getByRole("alert").textContent).toMatch(
        /Restart Fruitboard/,
      );
      expect(
        screen.getByRole("button", { name: /Cancel scan Projects/ }),
      ).toHaveProperty("disabled", true);
      expect(
        screen.getByRole("button", { name: /Retry scan Projects/ }),
      ).toHaveProperty("disabled", true);
      expect(
        screen.getByRole("button", { name: "Scan now Archive" }),
      ).toHaveProperty("disabled", true);
      expect(screen.getByRole("heading", { name: "Saved.flp" })).toBeTruthy();

      execution = "completed";
      records = [
        makeRecord(rootA, "native-location", "Published.flp", "Published.flp"),
      ];
      fireEvent.focus(window);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });
      await drainFakeTimerTurns();

      expect(
        screen.queryByRole("button", { name: /Cancel scan Projects/ }),
      ).toBeNull();
      expect(screen.getByText("Completed")).toBeTruthy();
      expect(
        screen.getByRole("heading", { name: "Published.flp" }),
      ).toBeTruthy();
      expect(
        calls.filter((command) => command === LIST_SCAN_STATUSES_COMMAND)
          .length,
      ).toBeLessThanOrEqual(4);
      expect(
        calls.filter((command) => command === GET_LIBRARY_PAGE_COMMAND).length,
      ).toBeLessThanOrEqual(4);
    } finally {
      view.unmount();
      vi.useRealTimers();
    }
  });

  it("reconciles after listener attachment and on focus, then cleans up", async () => {
    vi.useFakeTimers();
    let execution: ScanStatus["state"] = "idle";
    let records: readonly PublishedFileLocation[] = [];
    let resolveListen!: (unlisten: () => void) => void;
    const eventHandler: { current: (() => void) | null } = { current: null };
    const unlisten = vi.fn();
    const calls: string[] = [];
    const transport: NativeLibraryTransport = {
      invoke: (command) => {
        calls.push(command);
        const data =
          command === GET_SCAN_CONSOLE_STATE_COMMAND
            ? { enabled: true, runtimeAvailable: true }
            : command === LIST_SCAN_STATUSES_COMMAND
              ? [
                  {
                    ...makeStatus(
                      rootA,
                      execution,
                      execution === "idle" ? null : "gap-job",
                      execution === "running" || execution === "completed"
                        ? "gap-run"
                        : null,
                    ),
                    lastOutcomeAt:
                      execution === "completed" ? "2026-09-29T00:00:00Z" : null,
                    lastSuccessfulScanAt:
                      execution === "completed" ? "2026-09-29T00:00:00Z" : null,
                  },
                ]
              : {
                  rootId: rootA.id,
                  snapshotId:
                    execution === "completed"
                      ? "gap-snapshot-2"
                      : "gap-snapshot-1",
                  records,
                  nextCursor: null,
                };
        return Promise.resolve({
          status: "ok",
          schemaVersion: 1,
          correlationId: "correlation_00000000000000000000000000000001",
          data,
        });
      },
      listen: (_event, handler) => {
        eventHandler.current = handler;
        return new Promise((resolve) => {
          resolveListen = resolve;
        });
      },
    };
    const view = renderLibrary(createNativeLibraryScanAdapter(transport));
    try {
      await act(async () => {
        await vi.advanceTimersByTimeAsync(150);
      });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0);
      });
      await drainFakeTimerTurns();
      expect(screen.getByText("Not scanned yet")).toBeTruthy();

      // Work changes after the initial status snapshot but before the async
      // event registration resolves, so no event can report this transition.
      execution = "running";
      await act(async () => {
        resolveListen(unlisten);
        await Promise.resolve();
        await vi.advanceTimersByTimeAsync(100);
      });
      expect(
        screen.getByRole("button", { name: /Cancel scan Projects/ }),
      ).toBeTruthy();

      // The window-focus reconciliation catches a completion that happened
      // while this page was away, then refreshes only the selected root page.
      execution = "completed";
      records = [
        makeRecord(rootA, "gap-location", "AfterFocus.flp", "AfterFocus.flp"),
      ];
      fireEvent.focus(window);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });
      await drainFakeTimerTurns();
      expect(screen.getByText("Completed")).toBeTruthy();
      expect(
        screen.getByRole("heading", { name: "AfterFocus.flp" }),
      ).toBeTruthy();

      const statusReadsBeforeUnmount = calls.filter(
        (command) => command === LIST_SCAN_STATUSES_COMMAND,
      ).length;
      view.unmount();
      expect(unlisten).toHaveBeenCalledOnce();
      eventHandler.current?.();
      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });
      expect(
        calls.filter((command) => command === LIST_SCAN_STATUSES_COMMAND),
      ).toHaveLength(statusReadsBeforeUnmount);
    } finally {
      view.unmount();
      vi.useRealTimers();
    }
  });

  it("falls back to the normal surfaces when the console probe fails", async () => {
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA],
      files: [makeRecord(rootA, "location-a", "Native.flp", "Native.flp")],
      consoleProbeFails: true,
    });
    renderLibrary(adapter);

    expect(
      await screen.findByRole("heading", { name: "Native.flp" }),
    ).toBeTruthy();
    expect(
      screen.queryByRole("heading", {
        name: "Library scanning is not enabled in this build",
      }),
    ).toBeNull();
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
      recordRootFact("Projects (C:\\Synthetic\\Music\\Projects)"),
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
        screen.getByRole("button", { name: "Scan now Projects" }),
      );
    });
    expect(
      screen.queryByRole("button", { name: "Retry scan Projects" }),
    ).toBeNull();
    expect(adapter.calls.scanNow).toEqual([rootA.id]);
    expect(adapter.calls.cancelScan).toEqual(["fake-job-1"]);

    await user.click(screen.getByRole("button", { name: "Scan now Projects" }));
    expect(await screen.findByText("Queued")).toBeTruthy();
    expect(adapter.calls.scanNow).toEqual([rootA.id, rootA.id]);
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

  it("keeps the interrupted attempt visible while its successor queues and runs", async () => {
    const user = userEvent.setup();
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA],
      files: [makeRecord(rootA, "kept-recovery", "Kept.flp", "Kept.flp")],
    });
    renderLibrary(adapter);
    await screen.findByRole("heading", { name: "Kept.flp" });
    await user.click(screen.getByRole("button", { name: "Scan now Projects" }));
    adapter.advanceRun(rootA.id);
    adapter.interruptScan(rootA.id);
    await screen.findByText("Interrupted");
    const finished = (await adapter.listScanStatuses())[0]!.lastFinishedAttempt;
    await user.click(screen.getByRole("button", { name: "Scan now Projects" }));
    await screen.findByText("Queued");
    expect(
      screen.getByText(
        /last finished attempt was interrupted.*next scan is queued/i,
      ),
    ).toBeTruthy();
    expect((await adapter.listScanStatuses())[0]!.lastFinishedAttempt).toEqual(
      finished,
    );
    adapter.advanceRun(rootA.id);
    await screen.findByText("Running");
    expect(
      screen.getByText(
        /last finished attempt was interrupted.*next scan is running/i,
      ),
    ).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Kept.flp" })).toBeTruthy();
    adapter.completeScan(rootA.id);
    await screen.findByText("Completed");
    expect(
      screen.queryByText(/last finished attempt was interrupted/i),
    ).toBeNull();
  });

  it("starts a fresh scan for an exhausted failed chain and keeps its page", async () => {
    const user = userEvent.setup();
    const exhausted = makeStatus(
      rootA,
      "failed",
      "exhausted-job",
      "run-4",
      false,
    );
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA],
      files: [
        makeRecord(rootA, "location-a", "Exhausted.flp", "Exhausted.flp"),
      ],
      initialStatuses: [exhausted],
    });
    renderLibrary(adapter);

    await screen.findByRole("heading", { name: "Exhausted.flp" });
    expect(screen.getByText("Failed")).toBeTruthy();
    expect(
      screen.queryByRole("button", { name: "Retry scan Projects" }),
    ).toBeNull();
    const scan = screen.getByRole("button", { name: "Scan now Projects" });
    expect(scan).toHaveProperty("disabled", false);

    await user.click(scan);
    expect(await screen.findByText("Queued")).toBeTruthy();
    expect(adapter.calls.retryScan).toEqual([]);
    expect(adapter.calls.scanNow).toEqual([rootA.id]);
    expect(screen.getByRole("heading", { name: "Exhausted.flp" })).toBeTruthy();
  });

  it("converges a retry that races a terminal transition with a fresh scan", async () => {
    const user = userEvent.setup();
    const base = createFakeLibraryScanAdapter({
      roots: [rootA],
      files: [makeRecord(rootA, "location-a", "Raced.flp", "Raced.flp")],
      initialStatuses: [
        makeStatus(rootA, "failed", "raced-job", "raced-run", true),
      ],
    });
    const adapter: LibraryScanAdapter = {
      ...base,
      // The durable chain moved on between the status snapshot and the click:
      // the native retry cannot revive it and reports the typed conflict.
      retryScan: () => Promise.reject(new LibraryAdapterError("conflict")),
    };
    renderLibrary(adapter);

    await screen.findByRole("heading", { name: "Raced.flp" });
    await user.click(
      screen.getByRole("button", { name: "Retry scan Projects" }),
    );

    // The Retry action still converges: the client falls back to a fresh
    // explicit scan instead of surfacing "The scan state changed".
    expect(await screen.findByText("Queued")).toBeTruthy();
    expect(base.calls.scanNow).toEqual([rootA.id]);
    expect(screen.queryByText(/state changed/i)).toBeNull();
    expect(screen.getByRole("heading", { name: "Raced.flp" })).toBeTruthy();
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
    const scan = screen.getByRole("button", { name: "Scan now Projects" });
    expect(scan).toHaveProperty("disabled", false);
    await user.click(scan);
    expect(await screen.findByText("Queued")).toBeTruthy();
    expect(adapter.calls.scanNow).toEqual([rootA.id]);
  });

  it("keeps a sibling root's unavailable status out of the selected root's banners", async () => {
    const unavailableRoot = {
      ...rootB,
      availability: "unavailable" as const,
    };
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA, unavailableRoot],
      files: [makeRecord(rootA, "location-a", "Healthy.flp", "Healthy.flp")],
    });
    const user = userEvent.setup();
    const view = renderLibrary(adapter);

    await screen.findByRole("heading", { name: "Healthy.flp" });
    expect(
      view.container.querySelector('[data-library-state="populated"]'),
    ).toBeTruthy();
    expect(
      view.container.querySelector('[data-library-state="stale-results"]'),
    ).toBeNull();
    expect(screen.queryByText("Showing previous committed results")).toBeNull();
    expect(
      screen.queryByRole("heading", { name: "A scan root is unavailable" }),
    ).toBeNull();

    await user.selectOptions(
      screen.getByLabelText("Scan root"),
      unavailableRoot.id,
    );
    expect(
      await screen.findByRole("heading", {
        name: "A scan root is unavailable",
      }),
    ).toBeTruthy();
  });

  it("keeps a healthy empty selected root's panel when a sibling is unavailable", async () => {
    const unavailableRoot = {
      ...rootB,
      availability: "unavailable" as const,
    };
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA, unavailableRoot],
    });
    const view = renderLibrary(adapter);

    expect(
      await screen.findByRole("heading", { name: "No committed files yet" }),
    ).toBeTruthy();
    expect(
      screen.queryByRole("heading", { name: "A scan root is unavailable" }),
    ).toBeNull();
    expect(screen.queryByText("Showing previous committed results")).toBeNull();
    expect(
      view.container.querySelector('[data-library-state="empty"]'),
    ).toBeTruthy();
  });

  it("does not expose a recovery action for a disabled exhausted root", async () => {
    const disabledRoot = { ...rootA, enabled: false };
    const adapter = createFakeLibraryScanAdapter({
      roots: [disabledRoot],
      files: [
        makeRecord(disabledRoot, "location-a", "Disabled.flp", "Disabled.flp"),
      ],
      initialStatuses: [
        makeStatus(
          disabledRoot,
          "failed",
          "disabled-job",
          "run-disabled",
          false,
        ),
      ],
    });
    renderLibrary(adapter);

    await screen.findByRole("heading", { name: "Disabled.flp" });
    expect(
      screen.queryByRole("button", { name: "Retry scan Projects" }),
    ).toBeNull();
    expect(
      screen.getByRole("button", { name: "Scan now Projects" }),
    ).toHaveProperty("disabled", true);
    expect(screen.getByText("Enable this root in Preferences.")).toBeTruthy();
    expect(adapter.calls.scanNow).toEqual([]);
    expect(adapter.calls.retryScan).toEqual([]);
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

  it("returns cancellation focus when the worker finishes after acknowledging the request", async () => {
    const user = userEvent.setup();
    const fake = createFakeLibraryScanAdapter({ roots: [rootA] });
    const adapter: LibraryScanAdapter = {
      ...fake,
      cancelScan: async (jobId) => ({
        rootId: rootA.id,
        jobId,
        runId: "fake-run-1",
        outcome: "cancellation_requested",
      }),
    };
    renderLibrary(adapter);
    await screen.findByRole("heading", { name: "No committed files yet" });
    await user.click(screen.getByRole("button", { name: "Scan now Projects" }));
    act(() => fake.advanceRun(rootA.id));
    await screen.findByText("Running");
    await user.click(
      screen.getByRole("button", { name: "Cancel scan Projects" }),
    );
    // The acknowledgement refresh still reports Running. Completion arrives
    // in a later status event, after the old ten zero-delay focus retries.
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Cancel scan Projects" }),
      ).toHaveProperty("disabled", false),
    );
    await new Promise((resolve) => setTimeout(resolve, 100));
    await act(async () => {
      await fake.cancelScan("fake-job-1");
    });
    await screen.findByText("Cancelled");
    await waitFor(() =>
      expect(document.activeElement).toBe(
        screen.getByRole("button", { name: "Scan now Projects" }),
      ),
    );
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

  it("coalesces a burst of subscription events into one page refresh", async () => {
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
      expect(harness.pageRequests).toHaveLength(2);
      expect(harness.statusRequests).toHaveLength(2);
    });
    await settle(
      harness.pageRequests[1]!.deferred,
      page(rootA.id, "snapshot-1", [
        makeRecord(rootA, "new", "Newest.flp", "Newest.flp"),
      ]),
    );
    await screen.findByRole("heading", { name: "Newest.flp" });
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
    await waitFor(() => {
      expect(harness.pageRequests).toHaveLength(2);
      expect(harness.statusRequests).toHaveLength(2);
    });
    // The coalesced event refresh must be in flight before pagination changes;
    // its older page-one response is intentionally settled after page two.
    expect(harness.pageRequests[1]!.request).toEqual({
      rootId: rootA.id,
      limit: 4,
      cursor: null,
    });
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
    await settle(harness.statusRequests[1]!, [makeStatus(rootA)]);

    expect(screen.getByRole("heading", { name: "Second.flp" })).toBeTruthy();
    expect(
      screen.queryByRole("heading", { name: "Late first page.flp" }),
    ).toBeNull();
  });

  it("retries an invalidated in-flight page when selection returns to its root", async () => {
    const harness = makeDeferredAdapter([rootA, rootB]);
    const user = userEvent.setup();
    renderLibrary(harness.adapter);
    await waitFor(() => expect(harness.statusRequests).toHaveLength(1));
    await settle(harness.statusRequests[0]!, [
      makeStatus(rootA),
      makeStatus(rootB),
    ]);
    await waitFor(() => expect(harness.pageRequests).toHaveLength(1));
    expect(harness.pageRequests[0]!.request.rootId).toBe(rootA.id);

    await user.selectOptions(screen.getByLabelText("Scan root"), rootB.id);
    await waitFor(() => expect(harness.pageRequests).toHaveLength(2));
    await settle(
      harness.pageRequests[1]!.deferred,
      page(rootB.id, "snapshot-b", [
        makeRecord(rootB, "b", "Other root.flp", "Other root.flp"),
      ]),
    );
    await screen.findByRole("heading", { name: "Other root.flp" });

    await user.selectOptions(screen.getByLabelText("Scan root"), rootA.id);
    expect(harness.pageRequests).toHaveLength(2);
    await settle(
      harness.pageRequests[0]!.deferred,
      page(rootA.id, "snapshot-old", [
        makeRecord(
          rootA,
          "old",
          "Stale first read.flp",
          "Stale first read.flp",
        ),
      ]),
    );
    await waitFor(() => expect(harness.pageRequests).toHaveLength(3));
    expect(harness.pageRequests[2]!.request.rootId).toBe(rootA.id);
    await settle(
      harness.pageRequests[2]!.deferred,
      page(rootA.id, "snapshot-fresh", [
        makeRecord(rootA, "fresh", "Fresh read.flp", "Fresh read.flp"),
      ]),
    );

    await screen.findByRole("heading", { name: "Fresh read.flp" });
    expect(
      screen.queryByRole("heading", { name: "Stale first read.flp" }),
    ).toBeNull();
  });

  it("fails terminally instead of looping when a page arrives for another root", async () => {
    const harness = makeDeferredAdapter([rootA]);
    const user = userEvent.setup();
    renderLibrary(harness.adapter);
    await waitFor(() => expect(harness.statusRequests).toHaveLength(1));
    await settle(harness.statusRequests[0]!, [makeStatus(rootA)]);
    await waitFor(() => expect(harness.pageRequests).toHaveLength(1));

    // A page whose rootId disagrees with the request is a contract violation.
    await settle(
      harness.pageRequests[0]!.deferred,
      page(rootB.id, "snapshot-1", [
        makeRecord(rootB, "wrong", "Wrong.flp", "Wrong.flp"),
      ]),
    );

    await screen.findByRole("heading", { name: /results unavailable/i });

    // The regression: this used to restart pagination, which re-allocated
    // `pagePosition` and re-fired the load effect forever (~4 requests/second).
    // Assert the request count stays put instead.
    await waitFor(() => expect(harness.pageRequests).toHaveLength(1));
    await new Promise((resolve) => setTimeout(resolve, 400));
    expect(harness.pageRequests).toHaveLength(1);
    expect(harness.pageRequests[0]!.request.rootId).toBe(rootA.id);
    expect(screen.queryByRole("heading", { name: "Wrong.flp" })).toBeNull();

    // Recovery still works, and only when the user asks for it.
    await user.click(screen.getByRole("button", { name: /try again/i }));
    await waitFor(() => expect(harness.pageRequests).toHaveLength(2));
    await settle(
      harness.pageRequests[1]!.deferred,
      page(rootA.id, "snapshot-2", [
        makeRecord(rootA, "recovered", "Recovered.flp", "Recovered.flp"),
      ]),
    );
    await screen.findByRole("heading", { name: "Recovered.flp" });
  });

  it("coalesces a burst of subscription events into one status refresh", async () => {
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
    await waitFor(() => expect(harness.statusRequests).toHaveLength(2));
    await settle(harness.statusRequests[1]!, [
      makeStatus(rootA, "queued", "new-job", null),
    ]);
    await screen.findByText("Queued");

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
    const requestCountAfterFirstWindow = adapter.calls.pageRequests.length;
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 500));
    });
    const stableRequestCount = adapter.calls.pageRequests.length;
    // A status refresh can request one final page reconciliation after the
    // coalesced event refresh finishes. Allow that single follow-up, then
    // require the page to remain stable through another full window.
    expect(
      stableRequestCount - requestCountAfterFirstWindow,
    ).toBeLessThanOrEqual(1);
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
      getConsoleState: () =>
        Promise.resolve({ enabled: true, runtimeAvailable: true }),
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
      subscribe: (_listener, onStateChange) => {
        onStateChange?.("attached");
        return () => {};
      },
    };
    renderLibrary(adapter);

    await screen.findByRole("heading", { name: "Detached.flp" });
    expect(
      recordRootFact("Projects (C:\\Synthetic\\Music\\Projects)"),
    ).toBeTruthy();
    expect(recordRootFact("Projects (E:\\Removed\\Projects)")).toBeTruthy();
  });
});

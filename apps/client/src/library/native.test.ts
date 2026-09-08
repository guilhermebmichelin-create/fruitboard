import { describe, expect, it, vi } from "vitest";
import {
  createNativeLibraryScanAdapter,
  clampLibraryLimit,
  parseCancelScanResult,
  parseScanStartResult,
  parseScanStatus,
  CANCEL_SCAN_COMMAND,
  GET_LIBRARY_PAGE_COMMAND,
  LIST_SCAN_STATUSES_COMMAND,
  RETRY_SCAN_COMMAND,
  SCAN_NOW_COMMAND,
  SCAN_STATUS_CHANGED_EVENT,
  type NativeLibraryTransport,
} from "./native";
import { LibraryAdapterError } from "./contracts";

const correlationId = "correlation_00000000000000000000000000000001";

const okEnvelope = (data: unknown) => ({
  status: "ok",
  schemaVersion: 1,
  correlationId,
  data,
});

const errorEnvelope = (
  code: string,
  message: string,
  retryable: boolean,
) => ({
  status: "error",
  schemaVersion: 1,
  correlationId,
  error: { code, message, retryable },
});

const scanRoot = {
  id: "root-1",
  displayName: "Projects",
  canonicalPath: "C:\\Music\\Projects",
  enabled: true,
  availability: "available",
  lastErrorCode: null,
};

const baseStatus = {
  root: scanRoot,
  state: "queued",
  jobId: "job-1",
  runId: null,
  cancellationRequested: false,
  counters: { filesObserved: 0, directoriesVisited: 0, totalFiles: null },
  lastSuccessfulScanAt: null,
  lastOutcomeAt: "2026-01-02T03:04:05.123Z",
  errorCode: null,
};

const baseRecord = {
  locationId: "location-1",
  rootId: "root-1",
  rootDisplayName: "Projects",
  rootCanonicalPath: "C:\\Music\\Projects",
  fileName: "Kick.flp",
  relativePath: "Kick.flp",
  byteSize: "49152",
  modifiedAt: "2026-08-30T13:15:00.123456789Z",
  presence: "present",
};

function transportWith(
  invokeImpl: () => unknown,
  listenImpl: NativeLibraryTransport["listen"] = () => () => {},
): {
  transport: NativeLibraryTransport;
  invoke: ReturnType<typeof vi.fn>;
  listen: ReturnType<typeof vi.fn>;
} {
  const invoke: NativeLibraryTransport["invoke"] = () => {
    try {
      return Promise.resolve(invokeImpl());
    } catch {
      return Promise.reject(new Error("native invoke failed"));
    }
  };
  const invokeMock = vi.fn(invoke);
  const listenMock = vi.fn(listenImpl);
  return {
    transport: { invoke: invokeMock, listen: listenMock },
    invoke: invokeMock,
    listen: listenMock,
  };
}

async function expectCode(promise: Promise<unknown>, code: string) {
  await expect(promise).rejects.toMatchObject({ code });
  await expect(promise).rejects.toBeInstanceOf(LibraryAdapterError);
}

describe("native scan-now seam", () => {
  it("queues with jobId set and a null runId", async () => {
    const { transport, invoke } = transportWith(() =>
      okEnvelope({
        rootId: "root-1",
        jobId: "job-1",
        runId: null,
        outcome: "queued",
      }),
    );
    const adapter = createNativeLibraryScanAdapter(transport);
    await expect(adapter.scanNow("root-1")).resolves.toEqual({
      rootId: "root-1",
      jobId: "job-1",
      runId: null,
      outcome: "queued",
    });
    expect(invoke).toHaveBeenCalledExactlyOnceWith(SCAN_NOW_COMMAND, {
      request: { schemaVersion: 1, rootId: "root-1" },
    });
  });

  it("surfaces already_queued and already_running coalescing verbatim", async () => {
    const queued = transportWith(() =>
      okEnvelope({
        rootId: "root-1",
        jobId: "job-1",
        runId: null,
        outcome: "already_queued",
      }),
    );
    await expect(
      createNativeLibraryScanAdapter(queued.transport).scanNow("root-1"),
    ).resolves.toMatchObject({ outcome: "already_queued", runId: null });

    const running = transportWith(() =>
      okEnvelope({
        rootId: "root-1",
        jobId: "job-1",
        runId: "run-7",
        outcome: "already_running",
      }),
    );
    await expect(
      createNativeLibraryScanAdapter(running.transport).scanNow("root-1"),
    ).resolves.toMatchObject({ outcome: "already_running", runId: "run-7" });
  });

  it("rejects a queued result that carries a runId", async () => {
    const { transport } = transportWith(() =>
      okEnvelope({
        rootId: "root-1",
        jobId: "job-1",
        runId: "run-1",
        outcome: "queued",
      }),
    );
    await expectCode(
      createNativeLibraryScanAdapter(transport).scanNow("root-1"),
      "internal",
    );
  });

  it("maps not_found, conflict, unavailable, and internal codes", async () => {
    for (const [code, message, retryable] of [
      ["not_found", "The requested item is no longer available.", false],
      [
        "conflict",
        "The request could not be completed because its state changed.",
        true,
      ],
      [
        "unavailable",
        "The requested service is temporarily unavailable.",
        true,
      ],
      ["internal", "Fruitboard could not complete the request.", false],
    ] as const) {
      const { transport } = transportWith(() =>
        errorEnvelope(code, message, retryable),
      );
      await expectCode(
        createNativeLibraryScanAdapter(transport).scanNow("root-1"),
        code,
      );
    }
  });

  it("surfaces the feature-off unavailable envelope as recoverable", async () => {
    const { transport } = transportWith(() =>
      errorEnvelope(
        "unavailable",
        "The requested service is temporarily unavailable.",
        true,
      ),
    );
    const error = await createNativeLibraryScanAdapter(transport)
      .scanNow("root-1")
      .catch((failure: unknown) => failure);
    expect(error).toBeInstanceOf(LibraryAdapterError);
    expect((error as LibraryAdapterError).code).toBe("unavailable");
  });

  it("rejects empty root and job IDs without invoking native code", async () => {
    const { transport, invoke } = transportWith(() =>
      okEnvelope({}),
    );
    const adapter = createNativeLibraryScanAdapter(transport);
    await expectCode(adapter.scanNow(""), "not_found");
    await expectCode(adapter.cancelScan(""), "not_found");
    await expectCode(adapter.retryScan(""), "not_found");
    await expectCode(
      adapter.getLibraryPage({ rootId: "", limit: 4, cursor: null }),
      "not_found",
    );
    expect(invoke).not.toHaveBeenCalled();
  });
});

describe("native cancel/retry seam", () => {
  it("returns terminal already_* no-ops with the recorded runId", async () => {
    for (const outcome of [
      "already_cancelled",
      "already_completed",
      "already_failed",
    ] as const) {
      const { transport, invoke } = transportWith(() =>
        okEnvelope({
          rootId: "root-1",
          jobId: "job-1",
          runId: "run-3",
          outcome,
        }),
      );
      const adapter = createNativeLibraryScanAdapter(transport);
      await expect(adapter.cancelScan("job-1")).resolves.toEqual({
        rootId: "root-1",
        jobId: "job-1",
        runId: "run-3",
        outcome,
      });
      expect(invoke).toHaveBeenCalledExactlyOnceWith(CANCEL_SCAN_COMMAND, {
        request: { schemaVersion: 1, jobId: "job-1" },
      });
    }
  });

  it("returns queued cancellation with a null runId", async () => {
    const { transport } = transportWith(() =>
      okEnvelope({
        rootId: "root-1",
        jobId: "job-1",
        runId: null,
        outcome: "cancelled",
      }),
    );
    await expect(
      createNativeLibraryScanAdapter(transport).cancelScan("job-1"),
    ).resolves.toMatchObject({ outcome: "cancelled", runId: null });
  });

  it("surfaces unknown jobs as not_found", async () => {
    const { transport } = transportWith(() =>
      errorEnvelope(
        "not_found",
        "The requested item is no longer available.",
        false,
      ),
    );
    await expectCode(
      createNativeLibraryScanAdapter(transport).cancelScan("job-missing"),
      "not_found",
    );
  });

  it("retries with exact typed arguments and coalescing", async () => {
    const { transport, invoke } = transportWith(() =>
      okEnvelope({
        rootId: "root-1",
        jobId: "job-1",
        runId: null,
        outcome: "queued",
      }),
    );
    await expect(
      createNativeLibraryScanAdapter(transport).retryScan("job-1"),
    ).resolves.toMatchObject({ outcome: "queued" });
    expect(invoke).toHaveBeenCalledExactlyOnceWith(RETRY_SCAN_COMMAND, {
      request: { schemaVersion: 1, jobId: "job-1" },
    });
  });

  it("surfaces exhausted terminal retry as conflict", async () => {
    const { transport } = transportWith(() =>
      errorEnvelope(
        "conflict",
        "The request could not be completed because its state changed.",
        true,
      ),
    );
    await expectCode(
      createNativeLibraryScanAdapter(transport).retryScan("job-1"),
      "conflict",
    );
  });
});

describe("native status list seam", () => {
  it("invokes only the typed list command and keeps native order", async () => {
    const second = { ...baseStatus, root: { ...scanRoot, id: "root-2" } };
    const { transport, invoke } = transportWith(() =>
      okEnvelope([{ ...baseStatus }, second]),
    );
    const statuses = await createNativeLibraryScanAdapter(
      transport,
    ).listScanStatuses();
    expect(statuses.map((status) => status.root.id)).toEqual([
      "root-1",
      "root-2",
    ]);
    expect(invoke).toHaveBeenCalledExactlyOnceWith(
      LIST_SCAN_STATUSES_COMMAND,
      { request: { schemaVersion: 1 } },
    );
  });

  it("keeps running counters with an honest null total", async () => {
    const { transport } = transportWith(() =>
      okEnvelope([
        {
          ...baseStatus,
          state: "running",
          runId: "run-1",
          counters: {
            filesObserved: 2,
            directoriesVisited: 0,
            totalFiles: null,
          },
        },
      ]),
    );
    const [status] = await createNativeLibraryScanAdapter(
      transport,
    ).listScanStatuses();
    expect(status?.counters).toEqual({
      filesObserved: 2,
      directoriesVisited: 0,
      totalFiles: null,
    });
  });

  it("rejects queued statuses that carry a runId or an error code", async () => {
    for (const patch of [
      { runId: "run-1" },
      { errorCode: "internal" },
      { counters: { filesObserved: 1.5, directoriesVisited: 0, totalFiles: null } },
      { counters: { filesObserved: 0, directoriesVisited: 0, totalFiles: 10.5 } },
    ]) {
      const { transport } = transportWith(() =>
        okEnvelope([{ ...baseStatus, ...patch }]),
      );
      await expectCode(
        createNativeLibraryScanAdapter(transport).listScanStatuses(),
        "internal",
      );
    }
  });

  it("rejects malformed statuses without trusting them", async () => {
    for (const status of [
      { ...baseStatus, state: "scanning" },
      { ...baseStatus, counters: { filesObserved: -1, directoriesVisited: 0, totalFiles: null } },
      { ...baseStatus, lastOutcomeAt: "1717386245123456789" },
      { ...baseStatus, lastOutcomeAt: "2026-01-02T03:04:05.1234567890Z" },
      { ...baseStatus, errorCode: "future_error" },
    ]) {
      const { transport } = transportWith(() =>
        okEnvelope([status]),
      );
      await expectCode(
        createNativeLibraryScanAdapter(transport).listScanStatuses(),
        "internal",
      );
    }
  });
});

describe("native library page seam", () => {
  it("clamps the limit to 1..200 and sends a null snapshot", async () => {
    expect(clampLibraryLimit(0)).toBe(1);
    expect(clampLibraryLimit(500)).toBe(200);
    expect(clampLibraryLimit(Number.NaN)).toBe(4);
    const { transport, invoke } = transportWith(() =>
      okEnvelope({
        rootId: "root-1",
        snapshotId: "snapshot-1",
        records: [],
        nextCursor: null,
      }),
    );
    const adapter = createNativeLibraryScanAdapter(transport);
    await adapter.getLibraryPage({ rootId: "root-1", limit: 500, cursor: null });
    expect(invoke).toHaveBeenCalledExactlyOnceWith(
      GET_LIBRARY_PAGE_COMMAND,
      {
        request: {
          schemaVersion: 1,
          rootId: "root-1",
          limit: 200,
          cursor: null,
          snapshotId: null,
        },
      },
    );
  });

  it("returns per-root records verbatim without merging or sorting", async () => {
    const second = { ...baseRecord, locationId: "location-2" };
    const { transport } = transportWith(() =>
      okEnvelope({
        rootId: "root-1",
        snapshotId: "snapshot-1",
        records: [second, baseRecord],
        nextCursor: "cursor-2",
      }),
    );
    const page = await createNativeLibraryScanAdapter(
      transport,
    ).getLibraryPage({ rootId: "root-1", limit: 4, cursor: null });
    // Native order is kept; the adapter never sorts by display spelling.
    expect(page.records.map((record) => record.locationId)).toEqual([
      "location-2",
      "location-1",
    ]);
    expect(page.nextCursor).toBe("cursor-2");
  });

  it("surfaces invalid_cursor and stale_cursor for page-one restart", async () => {
    for (const [code, message] of [
      [
        "invalid_cursor",
        "The page continuation is not valid; start from the first page.",
      ],
      ["stale_cursor", "The list changed; start again from the first page."],
    ] as const) {
      const { transport } = transportWith(() =>
        errorEnvelope(code, message, false),
      );
      await expectCode(
        createNativeLibraryScanAdapter(transport).getLibraryPage({
          rootId: "root-1",
          limit: 4,
          cursor: "cursor-stale",
        }),
        code,
      );
    }
  });

  it("rejects numeric byte sizes and malformed timestamps", async () => {
    for (const record of [
      { ...baseRecord, byteSize: 1024 },
      { ...baseRecord, byteSize: "007" },
      { ...baseRecord, byteSize: "18446744073709551616" },
      { ...baseRecord, byteSize: "9223372036854775808" },
      { ...baseRecord, modifiedAt: "not-a-date" },
      { ...baseRecord, modifiedAt: "2026-01-02T03:04:05.1234567890Z" },
      { ...baseRecord, presence: "partial" },
    ]) {
      const { transport } = transportWith(() =>
        okEnvelope({
          rootId: "root-1",
          snapshotId: "snapshot-1",
          records: [record],
          nextCursor: null,
        }),
      );
      await expectCode(
        createNativeLibraryScanAdapter(transport).getLibraryPage({
          rootId: "root-1",
          limit: 4,
          cursor: null,
        }),
        "internal",
      );
    }
  });

  it("accepts maximum decimal numerics and nanosecond fractions", async () => {
    const { transport } = transportWith(() =>
      okEnvelope({
        rootId: "root-1",
        snapshotId: "snapshot-1",
        records: [
          {
            ...baseRecord,
            byteSize: "9223372036854775807",
            modifiedAt: "2026-01-02T03:04:05.123456789Z",
          },
        ],
        nextCursor: null,
      }),
    );
    const page = await createNativeLibraryScanAdapter(
      transport,
    ).getLibraryPage({ rootId: "root-1", limit: 4, cursor: null });
    expect(page.records[0]?.byteSize).toBe("9223372036854775807");
    expect(page.records[0]?.modifiedAt).toBe(
      "2026-01-02T03:04:05.123456789Z",
    );
    expect(page.snapshotId).toBe("snapshot-1");
  });
});

describe("native envelope hardening", () => {
  it("maps invalid_request and unknown codes to internal", async () => {
    for (const envelope of [
      errorEnvelope("invalid_request", "The request was not valid.", false),
      errorEnvelope("future_error", "C:\\private.flp", true),
      { status: "ok", correlationId, data: {} },
      { status: "ok", schemaVersion: 2, correlationId, data: {} },
      {
        status: "error",
        schemaVersion: 1,
        correlationId,
        error: {
          code: "unavailable",
          message: "wrong message",
          retryable: true,
        },
      },
      {
        status: "error",
        schemaVersion: 1,
        correlationId,
        error: {
          code: "conflict",
          message:
            "The request could not be completed because its state changed.",
          retryable: false,
        },
      },
    ]) {
      const { transport } = transportWith(() => envelope);
      await expectCode(
        createNativeLibraryScanAdapter(transport).listScanStatuses(),
        "internal",
      );
    }
  });

  it("maps transport failures to recoverable unavailable", async () => {
    const { transport } = transportWith(() => {
      throw new Error("Bearer secret C:\\private.flp");
    });
    const error = await createNativeLibraryScanAdapter(transport)
      .listScanStatuses()
      .catch((failure: unknown) => failure);
    expect(error).toBeInstanceOf(LibraryAdapterError);
    expect((error as LibraryAdapterError).code).toBe("unavailable");
  });

  it("parses the closed outcome unions strictly", () => {
    expect(() =>
      parseScanStartResult({
        rootId: "root-1",
        jobId: "job-1",
        runId: null,
        outcome: "already_failed",
      }),
    ).toThrowError(LibraryAdapterError);
    expect(() =>
      parseCancelScanResult({
        rootId: "root-1",
        jobId: "job-1",
        runId: null,
        outcome: "queued",
      }),
    ).toThrowError(LibraryAdapterError);
    expect(() =>
      parseScanStatus({ ...baseStatus, state: "queued", runId: "run-1" }),
    ).toThrowError(LibraryAdapterError);
  });
});

describe("native scan-status-changed subscription", () => {
  it("subscribes to the typed event and unsubscribes safely", () => {
    const handlers = new Map<string, () => void>();
    const unlisten = vi.fn();
    const { transport, listen } = transportWith(
      () => okEnvelope([]),
      (event: string, handler: () => void) => {
        handlers.set(event, handler);
        return unlisten;
      },
    );
    const adapter = createNativeLibraryScanAdapter(transport);
    const listener = vi.fn();
    const unsubscribe = adapter.subscribe(listener);
    expect(listen).toHaveBeenCalledExactlyOnceWith(
      SCAN_STATUS_CHANGED_EVENT,
      expect.any(Function),
    );
    handlers.get(SCAN_STATUS_CHANGED_EVENT)?.();
    expect(listener).toHaveBeenCalledTimes(1);
    unsubscribe();
    expect(unlisten).toHaveBeenCalledOnce();
    handlers.get(SCAN_STATUS_CHANGED_EVENT)?.();
    expect(listener).toHaveBeenCalledTimes(1);
  });

  it("handles async listen and dispose-before-resolve", async () => {
    let resolveListen!: (unlisten: () => void) => void;
    const gate = new Promise<() => void>((resolve) => {
      resolveListen = resolve;
    });
    const unlisten = vi.fn();
    const { transport } = transportWith(
      () => okEnvelope([]),
      () => gate,
    );
    const adapter = createNativeLibraryScanAdapter(transport);
    const unsubscribe = adapter.subscribe(vi.fn());
    unsubscribe();
    resolveListen(unlisten);
    await gate;
    await Promise.resolve();
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it("survives a listen failure without throwing", () => {
    const { transport } = transportWith(
      () => okEnvelope([]),
      () => {
        throw new Error("listen failed");
      },
    );
    const adapter = createNativeLibraryScanAdapter(transport);
    expect(() => adapter.subscribe(vi.fn())).not.toThrow();
  });
});

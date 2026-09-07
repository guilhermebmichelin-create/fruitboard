import type { ScanRoot } from "../platform/contracts";
import {
  LIBRARY_PAGE_LIMIT,
  LibraryAdapterError,
  MAX_LIBRARY_PAGE_LIMIT,
  MIN_LIBRARY_PAGE_LIMIT,
  type LibraryPage,
  type LibraryPageRequest,
  type LibraryScanAdapter,
  type PublishedFileLocation,
  type LibraryErrorCode,
  type ScanProgressCounters,
  type ScanStartResult,
  type ScanStatus,
  type CancelScanResult,
} from "./contracts";

export interface FakeLibraryAdapterOptions {
  readonly roots?: readonly ScanRoot[];
  readonly files?: readonly PublishedFileLocation[];
  readonly pageLimit?: number;
  readonly initialStatuses?: readonly ScanStatus[];
  readonly progressAfterMs?: number | null;
}

export interface FakeLibraryScanAdapter extends LibraryScanAdapter {
  advanceRun(rootId: string): void;
  updateProgress(rootId: string, counters: ScanProgressCounters): void;
  completeScan(
    rootId: string,
    records?: readonly PublishedFileLocation[],
  ): void;
  failScan(
    rootId: string,
    code?: Exclude<LibraryErrorCode, "stale_cursor" | "invalid_cursor">,
  ): void;
  interruptScan(rootId: string): void;
  setRootAvailability(
    rootId: string,
    availability: ScanRoot["availability"],
  ): void;
  setPageError(code: LibraryErrorCode | null): void;
  readonly calls: {
    readonly scanNow: readonly string[];
    readonly cancelScan: readonly string[];
    readonly retryScan: readonly string[];
    readonly pages: readonly number[];
    readonly pageRequests: readonly LibraryPageRequest[];
  };
}

const now = () => new Date().toISOString();

const clampPageLimit = (limit: number) =>
  Math.min(
    MAX_LIBRARY_PAGE_LIMIT,
    Math.max(
      MIN_LIBRARY_PAGE_LIMIT,
      Number.isFinite(limit) ? Math.floor(limit) : LIBRARY_PAGE_LIMIT,
    ),
  );

/**
 * Fake limitation: native orders by `(locator_key BINARY, location_id)`;
 * the fake has no locator keys, so it uses display spelling only for a
 * deterministic test order. Its cursor remains opaque and locationId-bound
 * for test purposes; this is not native ordering or filesystem evidence.
 */
const compareBinary = (left: string, right: string): number => {
  if (left < right) return -1;
  if (left > right) return 1;
  return 0;
};

const orderRootRecords = (records: readonly PublishedFileLocation[]) =>
  [...records].sort(
    (left, right) =>
      compareBinary(left.relativePath, right.relativePath) ||
      compareBinary(left.locationId, right.locationId),
  );

const makeCounters = (): ScanProgressCounters => ({
  filesObserved: 0,
  directoriesVisited: 0,
  totalFiles: null,
});

const cloneStatus = (status: ScanStatus): ScanStatus => ({
  ...status,
  root: { ...status.root },
  counters: { ...status.counters },
});

interface FakeCursorPayload {
  readonly v: 1;
  readonly rootId: string;
  readonly snapshotId: string;
  readonly index: number;
  readonly locationId: string;
}

const toBase64Url = (raw: string): string => {
  const binary = Array.from(raw)
    .map((char) => {
      const code = char.charCodeAt(0);
      if (code > 255) throw new LibraryAdapterError("invalid_cursor");
      return String.fromCharCode(code);
    })
    .join("");
  return btoa(binary)
    .replaceAll("+", "-")
    .replaceAll("/", "_")
    .replace(/=+$/, "");
};

const fromBase64Url = (encoded: string): string => {
  const padded = encoded.replaceAll("-", "+").replaceAll("_", "/");
  const remainder = padded.length % 4;
  const normalized =
    remainder === 0 ? padded : padded + "=".repeat(4 - remainder);
  return atob(normalized);
};

const encodeCursor = (payload: FakeCursorPayload): string =>
  `fake-cursor.${toBase64Url(JSON.stringify(payload))}`;

/**
 * Decodes an opaque cursor. Malformed, structurally inconsistent, or
 * wrong-root cursors are `invalid_cursor`; a well-formed cursor whose
 * committed root snapshot has changed is `stale_cursor`.
 */
const decodeCursor = (
  rootId: string,
  currentSnapshotId: string,
  cursor: string,
): number => {
  let payload: unknown;
  try {
    if (!cursor.startsWith("fake-cursor.")) {
      throw new LibraryAdapterError("invalid_cursor");
    }
    payload = JSON.parse(fromBase64Url(cursor.slice("fake-cursor.".length)));
  } catch (error) {
    if (error instanceof LibraryAdapterError) throw error;
    throw new LibraryAdapterError("invalid_cursor");
  }
  if (
    typeof payload !== "object" ||
    payload === null ||
    (payload as Record<string, unknown>)["v"] !== 1 ||
    typeof (payload as Record<string, unknown>)["rootId"] !== "string" ||
    typeof (payload as Record<string, unknown>)["snapshotId"] !== "string" ||
    typeof (payload as Record<string, unknown>)["index"] !== "number" ||
    typeof (payload as Record<string, unknown>)["locationId"] !== "string"
  ) {
    throw new LibraryAdapterError("invalid_cursor");
  }
  const typed = payload as FakeCursorPayload;
  if (
    !Number.isSafeInteger(typed.index) ||
    typed.index < 0 ||
    typed.rootId.length === 0 ||
    typed.snapshotId.length === 0 ||
    typed.locationId.length === 0
  ) {
    throw new LibraryAdapterError("invalid_cursor");
  }
  if (typed.rootId !== rootId) throw new LibraryAdapterError("invalid_cursor");
  if (typed.snapshotId !== currentSnapshotId) {
    throw new LibraryAdapterError("stale_cursor");
  }
  return typed.index;
};

export function createFakeLibraryScanAdapter(
  options: FakeLibraryAdapterOptions = {},
): FakeLibraryScanAdapter {
  const roots = [...(options.roots ?? [])];
  const recordsByRoot = new Map<string, PublishedFileLocation[]>();
  const snapshotSequenceByRoot = new Map<string, number>();
  for (const root of roots) {
    recordsByRoot.set(root.id, []);
    snapshotSequenceByRoot.set(root.id, 1);
  }
  for (const record of options.files ?? []) {
    const bucket = recordsByRoot.get(record.rootId);
    if (bucket !== undefined) bucket.push(record);
  }
  for (const [rootId, bucket] of recordsByRoot) {
    recordsByRoot.set(rootId, orderRootRecords(bucket));
  }
  const snapshotIdFor = (rootId: string): string =>
    `fake-snapshot-${rootId}-${snapshotSequenceByRoot.get(rootId) ?? 1}`;
  const pageLimit = clampPageLimit(options.pageLimit ?? LIBRARY_PAGE_LIMIT);
  let pageErrorCode: LibraryErrorCode | null = null;
  let jobSequence = 0;
  let runSequence = 0;
  const listeners = new Set<() => void>();
  const statusByRoot = new Map<string, ScanStatus>();
  const progressTimers = new Map<string, ReturnType<typeof setTimeout>>();
  const calls = {
    scanNow: [] as string[],
    cancelScan: [] as string[],
    retryScan: [] as string[],
    pages: [] as number[],
    pageRequests: [] as LibraryPageRequest[],
  };

  for (const root of roots) {
    const supplied = options.initialStatuses?.find(
      (status) => status.root.id === root.id,
    );
    statusByRoot.set(
      root.id,
      cloneStatus(
        supplied ?? {
          root: { ...root },
          state: "idle",
          jobId: null,
          runId: null,
          cancellationRequested: false,
          counters: makeCounters(),
          lastSuccessfulScanAt: null,
          lastOutcomeAt: null,
          errorCode: null,
        },
      ),
    );
  }

  const emit = () => {
    for (const listener of listeners) listener();
  };

  const getStatus = (rootId: string) => {
    const status = statusByRoot.get(rootId);
    if (status === undefined) throw new LibraryAdapterError("not_found");
    return status;
  };

  const getRootRecords = (rootId: string) => {
    const bucket = recordsByRoot.get(rootId);
    if (bucket === undefined) throw new LibraryAdapterError("not_found");
    return bucket;
  };

  const createQueuedRun = (rootId: string): ScanStartResult => {
    const current = getStatus(rootId);
    if (current.state === "queued" && current.jobId !== null) {
      return {
        rootId,
        jobId: current.jobId,
        runId: null,
        outcome: "already_queued",
      };
    }
    if (current.state === "running" && current.jobId !== null) {
      return {
        rootId,
        jobId: current.jobId,
        runId: current.runId,
        outcome: "already_running",
      };
    }
    if (!current.root.enabled || current.root.availability === "unavailable") {
      throw new LibraryAdapterError(
        current.root.availability === "unavailable"
          ? "unavailable"
          : "conflict",
      );
    }

    jobSequence += 1;
    const jobId = `fake-job-${jobSequence}`;
    statusByRoot.set(rootId, {
      ...current,
      state: "queued",
      jobId,
      runId: null,
      cancellationRequested: false,
      counters: makeCounters(),
      lastOutcomeAt: null,
      errorCode: null,
    });
    emit();

    if (
      options.progressAfterMs !== null &&
      options.progressAfterMs !== undefined
    ) {
      const timer = setTimeout(() => {
        progressTimers.delete(rootId);
        const status = statusByRoot.get(rootId);
        if (status?.state === "queued") advanceRun(rootId);
      }, options.progressAfterMs);
      progressTimers.set(rootId, timer);
    }

    return { rootId, jobId, runId: null, outcome: "queued" };
  };

  const advanceRun = (rootId: string) => {
    const current = getStatus(rootId);
    if (current.state !== "queued" || current.jobId === null) return;
    runSequence += 1;
    const runId = `fake-run-${runSequence}`;
    statusByRoot.set(rootId, {
      ...current,
      state: "running",
      runId,
      counters: {
        filesObserved: 2,
        directoriesVisited: 1,
        totalFiles: null,
      },
    });
    emit();
  };

  const adapter: FakeLibraryScanAdapter = {
    async getLibraryPage(request): Promise<LibraryPage> {
      await Promise.resolve();
      calls.pages.push(request.limit);
      calls.pageRequests.push({ ...request });
      if (pageErrorCode !== null) throw new LibraryAdapterError(pageErrorCode);
      const records = getRootRecords(request.rootId);
      const snapshotId = snapshotIdFor(request.rootId);
      const start =
        request.cursor === null
          ? 0
          : decodeCursor(request.rootId, snapshotId, request.cursor);
      const limit = Math.min(clampPageLimit(request.limit), pageLimit);
      const pageRecords = records.slice(start, start + limit);
      const nextIndex = start + pageRecords.length;
      const last = pageRecords[pageRecords.length - 1];
      return {
        rootId: request.rootId,
        snapshotId,
        records: pageRecords,
        nextCursor:
          nextIndex < records.length && last !== undefined
            ? encodeCursor({
                v: 1,
                rootId: request.rootId,
                snapshotId,
                index: nextIndex,
                locationId: last.locationId,
              })
            : null,
      };
    },

    async listScanStatuses() {
      await Promise.resolve();
      return roots.map((root) => cloneStatus(statusByRoot.get(root.id)!));
    },

    async scanNow(rootId) {
      await Promise.resolve();
      calls.scanNow.push(rootId);
      return createQueuedRun(rootId);
    },

    async cancelScan(jobId): Promise<CancelScanResult> {
      await Promise.resolve();
      calls.cancelScan.push(jobId);
      const entry = [...statusByRoot.entries()].find(
        ([, status]) => status.jobId === jobId,
      );
      if (entry === undefined) throw new LibraryAdapterError("not_found");
      const [rootId, current] = entry;
      if (current.state === "queued" || current.state === "running") {
        const wasRunning = current.state === "running";
        const timer = progressTimers.get(rootId);
        if (timer !== undefined) clearTimeout(timer);
        progressTimers.delete(rootId);
        statusByRoot.set(rootId, {
          ...current,
          state: "cancelled",
          cancellationRequested: wasRunning,
          lastOutcomeAt: now(),
          errorCode: "cancelled",
        });
        emit();
        return {
          rootId,
          jobId,
          runId: current.runId,
          outcome: wasRunning ? "cancellation_requested" : "cancelled",
        };
      }
      if (current.state === "cancelled") {
        return {
          rootId,
          jobId,
          runId: current.runId,
          outcome: "already_cancelled",
        };
      }
      if (current.state === "completed" || current.state === "idle") {
        return {
          rootId,
          jobId,
          runId: current.runId,
          outcome: "already_completed",
        };
      }
      return { rootId, jobId, runId: current.runId, outcome: "already_failed" };
    },

    async retryScan(jobId) {
      await Promise.resolve();
      calls.retryScan.push(jobId);
      const entry = [...statusByRoot.entries()].find(
        ([, status]) => status.jobId === jobId,
      );
      if (entry === undefined) throw new LibraryAdapterError("not_found");
      const [rootId, current] = entry;
      if (current.state === "queued") {
        return { rootId, jobId, runId: null, outcome: "already_queued" };
      }
      if (current.state === "running") {
        return {
          rootId,
          jobId,
          runId: current.runId,
          outcome: "already_running",
        };
      }
      if (
        current.state !== "failed" &&
        current.state !== "cancelled" &&
        current.state !== "interrupted"
      ) {
        throw new LibraryAdapterError("conflict");
      }
      statusByRoot.set(rootId, {
        ...current,
        state: "queued",
        runId: null,
        cancellationRequested: false,
        counters: makeCounters(),
        lastOutcomeAt: null,
        errorCode: null,
      });
      emit();
      return { rootId, jobId, runId: null, outcome: "queued" };
    },

    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },

    advanceRun,

    updateProgress(rootId, counters) {
      const current = getStatus(rootId);
      if (current.state !== "running") return;
      statusByRoot.set(rootId, { ...current, counters: { ...counters } });
      emit();
    },

    completeScan(rootId, nextRecords) {
      const current = getStatus(rootId);
      const timer = progressTimers.get(rootId);
      if (timer !== undefined) clearTimeout(timer);
      progressTimers.delete(rootId);
      if (nextRecords !== undefined) {
        recordsByRoot.set(rootId, orderRootRecords(nextRecords));
      }
      snapshotSequenceByRoot.set(
        rootId,
        (snapshotSequenceByRoot.get(rootId) ?? 1) + 1,
      );
      statusByRoot.set(rootId, {
        ...current,
        state: "completed",
        lastSuccessfulScanAt: now(),
        lastOutcomeAt: now(),
        errorCode: null,
      });
      emit();
    },

    failScan(rootId, code = "internal") {
      const current = getStatus(rootId);
      const timer = progressTimers.get(rootId);
      if (timer !== undefined) clearTimeout(timer);
      progressTimers.delete(rootId);
      statusByRoot.set(rootId, {
        ...current,
        state: "failed",
        lastOutcomeAt: now(),
        errorCode: code,
      });
      emit();
    },

    interruptScan(rootId) {
      const current = getStatus(rootId);
      statusByRoot.set(rootId, {
        ...current,
        state: "interrupted",
        lastOutcomeAt: now(),
        errorCode: "internal",
      });
      emit();
    },

    setRootAvailability(rootId, availability) {
      const current = getStatus(rootId);
      statusByRoot.set(rootId, {
        ...current,
        root: { ...current.root, availability },
      });
      emit();
    },

    setPageError(code) {
      pageErrorCode = code;
    },

    calls,
  };

  return adapter;
}

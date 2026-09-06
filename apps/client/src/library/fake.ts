import type { ScanRoot } from "../platform/contracts";
import {
  LIBRARY_PAGE_LIMIT,
  LibraryAdapterError,
  MAX_LIBRARY_PAGE_LIMIT,
  type LibraryPage,
  type LibraryScanAdapter,
  type PublishedFileLocation,
  type ScanErrorCode,
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
  failScan(rootId: string, code?: ScanErrorCode): void;
  interruptScan(rootId: string): void;
  setRootAvailability(
    rootId: string,
    availability: ScanRoot["availability"],
  ): void;
  setPageError(code: ScanErrorCode | null): void;
  readonly calls: {
    readonly scanNow: readonly string[];
    readonly cancelScan: readonly string[];
    readonly retryScan: readonly string[];
    readonly pages: readonly number[];
  };
}

const now = () => new Date().toISOString();

const clampPageLimit = (limit: number) =>
  Math.min(
    MAX_LIBRARY_PAGE_LIMIT,
    Math.max(
      1,
      Number.isFinite(limit) ? Math.floor(limit) : LIBRARY_PAGE_LIMIT,
    ),
  );

const orderRecords = (records: readonly PublishedFileLocation[]) =>
  [...records].sort((left, right) => {
    const rootOrder = left.rootId.localeCompare(right.rootId);
    if (rootOrder !== 0) return rootOrder;
    const pathOrder = left.relativePath.localeCompare(right.relativePath);
    if (pathOrder !== 0) return pathOrder;
    return left.locationId.localeCompare(right.locationId);
  });

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

const readCursor = (cursor: string | null): number => {
  if (cursor === null) return 0;
  const match = /^fake-page-(\d+)$/.exec(cursor);
  if (!match) throw new LibraryAdapterError("internal");
  const index = Number(match[1]);
  if (!Number.isSafeInteger(index) || index < 0) {
    throw new LibraryAdapterError("internal");
  }
  return index;
};

export function createFakeLibraryScanAdapter(
  options: FakeLibraryAdapterOptions = {},
): FakeLibraryScanAdapter {
  const roots = [...(options.roots ?? [])];
  let records = orderRecords(options.files ?? []);
  const pageLimit = clampPageLimit(options.pageLimit ?? LIBRARY_PAGE_LIMIT);
  let pageErrorCode: ScanErrorCode | null = null;
  let runSequence = 0;
  const listeners = new Set<() => void>();
  const statusByRoot = new Map<string, ScanStatus>();
  const progressTimers = new Map<string, ReturnType<typeof setTimeout>>();
  const calls = {
    scanNow: [] as string[],
    cancelScan: [] as string[],
    retryScan: [] as string[],
    pages: [] as number[],
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
          runId: null,
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

  const createQueuedRun = (rootId: string): ScanStartResult => {
    const current = getStatus(rootId);
    if (current.state === "queued" && current.runId !== null) {
      return { rootId, runId: current.runId, outcome: "already_queued" };
    }
    if (current.state === "running" && current.runId !== null) {
      return { rootId, runId: current.runId, outcome: "already_running" };
    }
    if (!current.root.enabled || current.root.availability !== "available") {
      throw new LibraryAdapterError(
        current.root.availability === "available" ? "conflict" : "unavailable",
      );
    }

    runSequence += 1;
    const runId = `fake-run-${runSequence}`;
    statusByRoot.set(rootId, {
      ...current,
      state: "queued",
      runId,
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

    return { rootId, runId, outcome: "queued" };
  };

  const advanceRun = (rootId: string) => {
    const current = getStatus(rootId);
    if (current.state !== "queued" || current.runId === null) return;
    statusByRoot.set(rootId, {
      ...current,
      state: "running",
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
      if (pageErrorCode !== null) throw new LibraryAdapterError(pageErrorCode);
      const limit = Math.min(clampPageLimit(request.limit), pageLimit);
      const start = readCursor(request.cursor);
      const pageRecords = records.slice(start, start + limit);
      const nextIndex = start + pageRecords.length;
      return {
        records: pageRecords,
        nextCursor:
          nextIndex < records.length ? `fake-page-${nextIndex}` : null,
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

    async cancelScan(runId): Promise<CancelScanResult> {
      await Promise.resolve();
      calls.cancelScan.push(runId);
      const entry = [...statusByRoot.entries()].find(
        ([, status]) => status.runId === runId,
      );
      if (entry === undefined) throw new LibraryAdapterError("not_found");
      const [rootId, current] = entry;
      if (current.state === "queued" || current.state === "running") {
        const timer = progressTimers.get(rootId);
        if (timer !== undefined) clearTimeout(timer);
        progressTimers.delete(rootId);
        statusByRoot.set(rootId, {
          ...current,
          state: "cancelled",
          lastOutcomeAt: now(),
          errorCode: "cancelled",
        });
        emit();
        return { rootId, runId, outcome: "cancellation_requested" };
      }
      if (current.state === "cancelled") {
        return { rootId, runId, outcome: "already_cancelled" };
      }
      if (current.state === "completed" || current.state === "idle") {
        return { rootId, runId, outcome: "already_completed" };
      }
      return { rootId, runId, outcome: "already_failed" };
    },

    async retryScan(rootId) {
      await Promise.resolve();
      calls.retryScan.push(rootId);
      return createQueuedRun(rootId);
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
      if (nextRecords !== undefined) records = orderRecords(nextRecords);
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

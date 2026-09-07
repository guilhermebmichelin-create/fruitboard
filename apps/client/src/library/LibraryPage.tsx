import {
  AlertCircle,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  CircleSlash2,
  Clock3,
  FolderSearch,
  LoaderCircle,
  RefreshCw,
  ScanLine,
  XCircle,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link } from "react-router";
import {
  LIBRARY_PAGE_LIMIT,
  LibraryAdapterError,
  formatByteSizeDecimal,
  isScanAvailabilityUnavailable,
  isScanAvailabilityUnknown,
  type DecimalString,
  type LibraryPage as LibraryPageData,
  type LibraryScanAdapter,
  type PublishedFileLocation,
  type ScanErrorCode,
  type ScanStatus,
} from "./contracts";

type PageState =
  | { readonly kind: "loading" }
  | { readonly kind: "error" }
  | {
      readonly kind: "ready";
      readonly page: LibraryPageData;
      readonly refreshError: boolean;
    };

type PagePosition = {
  readonly cursor: string | null;
  readonly snapshotId: string | null;
};

type StatusState =
  | { readonly kind: "loading" }
  | { readonly kind: "error" }
  | {
      readonly kind: "ready";
      readonly statuses: readonly ScanStatus[];
      readonly refreshError: boolean;
    };

type ActionState =
  | { readonly kind: "idle" }
  | {
      readonly kind: "working";
      readonly rootId: string;
      readonly message: string;
    }
  | {
      readonly kind: "error";
      readonly rootId: string;
      readonly message: string;
    };

const terminalStates = new Set(["cancelled", "failed", "interrupted"]);

const scanErrorMessages: Readonly<Record<ScanErrorCode, string>> = {
  access_denied:
    "The folder could not be read. Previous committed results were kept.",
  unavailable:
    "The folder is unavailable. Previous committed results were kept.",
  unsupported:
    "This folder is not supported for scanning yet. Previous committed results were kept.",
  resource_limit:
    "The scan reached a safe resource limit. Previous committed results were kept.",
  conflict: "The scan state changed. Refresh the status and try again.",
  not_found:
    "That folder is no longer tracked. Refresh Preferences to confirm.",
  cancelled: "The scan was cancelled. No new results were published.",
  internal:
    "The scan could not complete safely. Previous committed results were kept.",
};

const formatDateTime = (value: string | null): string => {
  if (value === null) return "Never";
  const date = new Date(value);
  if (Number.isNaN(date.valueOf())) return "Unavailable";
  return `${new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: "UTC",
  }).format(date)} UTC`;
};

const formatBytes = (bytes: DecimalString): string => {
  try {
    return formatByteSizeDecimal(bytes);
  } catch {
    return "Unavailable";
  }
};

const formatAvailability = (
  availability: ScanStatus["root"]["availability"],
) => {
  if (availability === "available") return "Available";
  if (availability === "unavailable") return "Unavailable";
  return "Availability unknown";
};

const formatExecution = (status: ScanStatus): string => {
  if (status.state === "idle") return "Not scanned yet";
  if (status.state === "queued") return "Queued";
  if (status.state === "running") return "Running";
  if (status.state === "completed") return "Completed";
  if (status.state === "cancelled") return "Cancelled";
  if (status.state === "failed") return "Failed";
  return "Interrupted";
};

const rootControlName = (
  status: ScanStatus,
  duplicateDisplayNames: ReadonlySet<string>,
): string =>
  duplicateDisplayNames.has(status.root.displayName)
    ? `${status.root.displayName} (${status.root.canonicalPath})`
    : status.root.displayName;

const statusNeedsPreviousResults = (status: ScanStatus): boolean =>
  terminalStates.has(status.state) ||
  status.state === "queued" ||
  status.state === "running" ||
  isScanAvailabilityUnavailable(status.root.availability);

const isStaleCursorError = (error: unknown): boolean =>
  error instanceof LibraryAdapterError && error.code === "stale_cursor";

const isInvalidCursorError = (error: unknown): boolean =>
  error instanceof LibraryAdapterError && error.code === "invalid_cursor";

const adapterKeys = new WeakMap<object, number>();
let nextAdapterKey = 0;

const getAdapterKey = (adapter: LibraryScanAdapter): number => {
  const existing = adapterKeys.get(adapter);
  if (existing !== undefined) return existing;
  nextAdapterKey += 1;
  adapterKeys.set(adapter, nextAdapterKey);
  return nextAdapterKey;
};

function LibraryIntegrationDisabled() {
  return (
    <section
      aria-labelledby="library-disabled-title"
      className="state-panel state-panel--empty"
      data-library-state="disabled"
    >
      <div className="state-panel__icon">
        <FolderSearch aria-hidden="true" className="app-icon app-icon--large" />
      </div>
      <p className="eyebrow">Scanner integration pending</p>
      <h2 id="library-disabled-title">Library scanning is not enabled</h2>
      <p className="state-panel__description">
        The production desktop adapter is not connected to the Phase 2 scan
        contracts yet. No scan controls are available in this build.
      </p>
      <Link className="inline-action" to="/preferences">
        Review scan roots
      </Link>
    </section>
  );
}

export function LibraryPage({
  adapter,
}: {
  readonly adapter: LibraryScanAdapter | undefined;
}) {
  if (adapter === undefined) return <LibraryIntegrationDisabled />;
  return (
    <ConnectedLibraryPage adapter={adapter} key={getAdapterKey(adapter)} />
  );
}

function ConnectedLibraryPage({
  adapter,
}: {
  readonly adapter: LibraryScanAdapter;
}) {
  const [selectedRootId, setSelectedRootId] = useState<string | null>(null);
  const [pagePosition, setPagePosition] = useState<PagePosition>({
    cursor: null,
    snapshotId: null,
  });
  const [cursorHistory, setCursorHistory] = useState<readonly PagePosition[]>(
    [],
  );
  const [pageState, setPageState] = useState<PageState>({ kind: "loading" });
  const [statusState, setStatusState] = useState<StatusState>({
    kind: "loading",
  });
  const [pageAttempt, setPageAttempt] = useState(0);
  const [statusAttempt, setStatusAttempt] = useState(0);
  const [paginationNotice, setPaginationNotice] = useState<string | null>(null);
  const [actionState, setActionState] = useState<ActionState>({ kind: "idle" });
  const mounted = useRef(true);
  const pageRequestSequence = useRef(0);
  const statusRequestSequence = useRef(0);
  const actionSequence = useRef(0);

  const pendingPageFocus = useRef(false);
  const listHeadingReference = useRef<HTMLHeadingElement | null>(null);
  const scanButtonReferences = useRef(new Map<string, HTMLButtonElement>());
  const cancelButtonReferences = useRef(new Map<string, HTMLButtonElement>());
  const retryButtonReferences = useRef(new Map<string, HTMLButtonElement>());

  const focusLater = useCallback((target: () => HTMLElement | null) => {
    window.setTimeout(() => {
      if (mounted.current) target()?.focus();
    }, 0);
  }, []);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  const statuses = useMemo(
    () => (statusState.kind === "ready" ? statusState.statuses : []),
    [statusState],
  );

  /**
   * The page always reads exactly one root. An explicit user choice wins
   * while it is still tracked; otherwise the first tracked root is read.
   * Derived during render so no effect needs to synchronize selection.
   */
  const resolvedRootId =
    selectedRootId !== null &&
    statuses.some((status) => status.root.id === selectedRootId)
      ? selectedRootId
      : (statuses[0]?.root.id ?? null);

  const hasNoRoots = statusState.kind === "ready" && statuses.length === 0;

  const duplicateDisplayNames = useMemo(() => {
    const counts = new Map<string, number>();
    for (const status of statuses) {
      counts.set(
        status.root.displayName,
        (counts.get(status.root.displayName) ?? 0) + 1,
      );
    }
    return new Set(
      [...counts.entries()]
        .filter(([, count]) => count > 1)
        .map(([name]) => name),
    );
  }, [statuses]);

  const restartPagination = useCallback((notice?: string) => {
    pageRequestSequence.current += 1;
    pendingPageFocus.current = true;
    setPagePosition({ cursor: null, snapshotId: null });
    setCursorHistory([]);
    setPageState({ kind: "loading" });
    setPaginationNotice(
      notice ??
        "The committed Library snapshot changed. Pagination restarted at page 1.",
    );
    setPageAttempt((attempt) => attempt + 1);
  }, []);

  const selectRoot = useCallback(
    (rootId: string) => {
      if (rootId === resolvedRootId) return;
      pageRequestSequence.current += 1;
      pendingPageFocus.current = false;
      setSelectedRootId(rootId);
      setPagePosition({ cursor: null, snapshotId: null });
      setCursorHistory([]);
      setPaginationNotice(null);
      setPageState({ kind: "loading" });
      setPageAttempt((attempt) => attempt + 1);
    },
    [resolvedRootId],
  );

  const loadPage = useCallback(async () => {
    const requestId = ++pageRequestSequence.current;
    const requestPosition = pagePosition;
    const requestRootId = resolvedRootId;
    const isCurrentRequest = () =>
      mounted.current && pageRequestSequence.current === requestId;

    if (requestRootId === null) return;

    try {
      const page = await adapter.getLibraryPage({
        rootId: requestRootId,
        cursor: requestPosition.cursor,
        limit: LIBRARY_PAGE_LIMIT,
      });
      if (!isCurrentRequest()) return;
      if (page.rootId !== requestRootId) {
        restartPagination(
          "The Library returned a page for the wrong scan root. Pagination restarted at page 1.",
        );
        return;
      }
      if (
        requestPosition.cursor !== null &&
        (requestPosition.snapshotId === null ||
          page.snapshotId !== requestPosition.snapshotId)
      ) {
        restartPagination();
        return;
      }
      setPageState({ kind: "ready", page, refreshError: false });
    } catch (error) {
      if (!isCurrentRequest()) return;
      if (
        (isStaleCursorError(error) || isInvalidCursorError(error)) &&
        requestPosition.cursor !== null
      ) {
        restartPagination(
          isInvalidCursorError(error)
            ? "The saved Library position was invalid. Pagination restarted at page 1."
            : undefined,
        );
        return;
      }
      setPageState((previous) =>
        previous.kind === "ready"
          ? { ...previous, refreshError: true }
          : { kind: "error" },
      );
    }
  }, [adapter, pagePosition, resolvedRootId, restartPagination]);

  const loadStatuses = useCallback(async () => {
    const requestId = ++statusRequestSequence.current;
    const isCurrentRequest = () =>
      mounted.current && statusRequestSequence.current === requestId;

    try {
      const statuses = await adapter.listScanStatuses();
      if (!isCurrentRequest()) return;
      setStatusState({ kind: "ready", statuses, refreshError: false });
    } catch {
      if (!isCurrentRequest()) return;
      setStatusState((previous) =>
        previous.kind === "ready"
          ? { ...previous, refreshError: true }
          : { kind: "error" },
      );
    }
  }, [adapter]);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      void loadPage();
    }, 0);
    return () => window.clearTimeout(timer);
  }, [loadPage, pageAttempt]);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      void loadStatuses();
    }, 0);
    return () => window.clearTimeout(timer);
  }, [loadStatuses, statusAttempt]);

  useEffect(() => {
    let active = true;
    const unsubscribe = adapter.subscribe(() => {
      if (!active) return;
      void loadPage();
      void loadStatuses();
    });
    return () => {
      active = false;
      unsubscribe();
    };
  }, [adapter, loadPage, loadStatuses]);

  useEffect(() => {
    if (
      pageState.kind === "ready" &&
      pendingPageFocus.current &&
      listHeadingReference.current !== null
    ) {
      pendingPageFocus.current = false;
      focusLater(() => listHeadingReference.current);
    }
  }, [focusLater, pageState]);

  const refreshAfterAction = useCallback(
    async (requestId: number) => {
      const isCurrentAction = () =>
        mounted.current && actionSequence.current === requestId;
      if (!isCurrentAction()) return false;
      await loadStatuses();
      if (!isCurrentAction()) return false;
      await loadPage();
      return isCurrentAction();
    },
    [loadPage, loadStatuses],
  );

  const runScan = async (status: ScanStatus, retry: boolean) => {
    const requestId = ++actionSequence.current;
    const isCurrentAction = () =>
      mounted.current && actionSequence.current === requestId;
    const rootId = status.root.id;
    const name = rootControlName(status, duplicateDisplayNames);
    setActionState({
      kind: "working",
      rootId,
      message: retry ? `Retrying ${name}…` : `Queueing a scan for ${name}…`,
    });
    try {
      if (retry && status.jobId !== null) await adapter.retryScan(status.jobId);
      else await adapter.scanNow(rootId);
      if (!isCurrentAction()) return;
      if (!(await refreshAfterAction(requestId))) return;
      setActionState({ kind: "idle" });
      focusLater(() => cancelButtonReferences.current.get(rootId) ?? null);
    } catch (error) {
      if (!isCurrentAction()) return;
      await refreshAfterAction(requestId);
      if (!isCurrentAction()) return;
      const code =
        error instanceof LibraryAdapterError &&
        error.code !== "stale_cursor" &&
        error.code !== "invalid_cursor"
          ? error.code
          : "internal";
      setActionState({
        kind: "error",
        rootId,
        message: scanErrorMessages[code],
      });
      focusLater(
        () =>
          retryButtonReferences.current.get(rootId) ??
          scanButtonReferences.current.get(rootId) ??
          null,
      );
    }
  };

  const cancelScan = async (status: ScanStatus) => {
    if (status.jobId === null) return;
    const requestId = ++actionSequence.current;
    const isCurrentAction = () =>
      mounted.current && actionSequence.current === requestId;
    const rootId = status.root.id;
    const name = rootControlName(status, duplicateDisplayNames);
    setActionState({
      kind: "working",
      rootId,
      message: `Cancelling the scan for ${name}…`,
    });
    try {
      await adapter.cancelScan(status.jobId);
      if (!isCurrentAction()) return;
      if (!(await refreshAfterAction(requestId))) return;
      setActionState({ kind: "idle" });
      focusLater(
        () =>
          retryButtonReferences.current.get(rootId) ??
          scanButtonReferences.current.get(rootId) ??
          null,
      );
    } catch {
      if (!isCurrentAction()) return;
      await refreshAfterAction(requestId);
      if (!isCurrentAction()) return;
      setActionState({
        kind: "error",
        rootId,
        message:
          "The scan could not be cancelled safely. Refresh and try again.",
      });
      focusLater(() => cancelButtonReferences.current.get(rootId) ?? null);
    }
  };

  const goToNextPage = () => {
    if (pageState.kind !== "ready" || pageState.page.nextCursor === null)
      return;
    pageRequestSequence.current += 1;
    pendingPageFocus.current = true;
    setPaginationNotice(null);
    setPageState({ kind: "loading" });
    setCursorHistory((history) => [...history, pagePosition]);
    setPagePosition({
      cursor: pageState.page.nextCursor,
      snapshotId: pageState.page.snapshotId,
    });
  };

  const goToPreviousPage = () => {
    if (cursorHistory.length === 0) return;
    const previousPosition = cursorHistory[cursorHistory.length - 1];
    if (previousPosition === undefined) return;
    pageRequestSequence.current += 1;
    pendingPageFocus.current = true;
    setPaginationNotice(null);
    setPageState({ kind: "loading" });
    setCursorHistory((history) => history.slice(0, -1));
    setPagePosition(previousPosition);
  };

  if (pageState.kind === "loading" && statusState.kind === "loading") {
    return (
      <section
        aria-busy="true"
        aria-labelledby="library-loading-title"
        className="library-page library-page--loading"
        data-library-state="loading"
      >
        <p className="eyebrow">Committed file locations</p>
        <h2 id="library-loading-title">Loading your library</h2>
        <p aria-live="polite" className="library-loading-copy" role="status">
          Reading committed results and scan status…
        </p>
      </section>
    );
  }

  const page = pageState.kind === "ready" ? pageState.page : null;
  const pageHasRecords = page !== null && page.records.length > 0;
  const staleResults = statuses.some(statusNeedsPreviousResults);
  const rootUnavailable = statuses.some((status) =>
    isScanAvailabilityUnavailable(status.root.availability),
  );
  const showEmptyForMissingRoots = hasNoRoots && pageState.kind !== "error";
  const libraryState =
    pageState.kind === "error"
      ? "error"
      : showEmptyForMissingRoots
        ? "empty"
        : page === null
          ? "loading"
          : pageHasRecords && staleResults
            ? "stale-results"
            : !pageHasRecords && rootUnavailable
              ? "unavailable"
              : !pageHasRecords && staleResults
                ? "stale-results"
                : !pageHasRecords
                  ? "empty"
                  : "populated";

  const selectedStatus =
    resolvedRootId === null
      ? null
      : (statuses.find((status) => status.root.id === resolvedRootId) ?? null);
  const selectedRootLabel =
    selectedStatus === null
      ? null
      : rootControlName(selectedStatus, duplicateDisplayNames);

  return (
    <div
      className="library-page"
      data-library-state={libraryState}
      data-review-adapter="fake-or-proposed"
    >
      <section
        aria-labelledby="library-content-title"
        className="library-intro"
      >
        <div>
          <p className="eyebrow">Committed file locations</p>
          <h2 id="library-content-title">Your FLP library</h2>
          <p>
            These are published filesystem observations. Scan progress below
            never edits this list until a run completes authoritatively.
          </p>
        </div>
        <span className="library-review-badge">
          Review harness · fake adapter
        </span>
      </section>

      <ScanStatusPanel
        actionState={actionState}
        duplicateDisplayNames={duplicateDisplayNames}
        onRetryStatus={() => {
          statusRequestSequence.current += 1;
          setStatusState({ kind: "loading" });
          setStatusAttempt((attempt) => attempt + 1);
        }}
        onRunScan={(status, retry) => {
          void runScan(status, retry);
        }}
        onCancelScan={(status) => {
          void cancelScan(status);
        }}
        retryButtonReferences={retryButtonReferences}
        scanButtonReferences={scanButtonReferences}
        cancelButtonReferences={cancelButtonReferences}
        statusState={statusState}
      />

      {statusState.kind === "ready" && statuses.length > 0 && (
        <section
          aria-labelledby="library-root-select-title"
          className="library-root-select"
        >
          <div>
            <p className="eyebrow" id="library-root-select-title">
              Per-root pages
            </p>
            <label
              className="library-root-select__label"
              htmlFor="library-root-select"
            >
              Scan root
            </label>
            <p className="library-root-select__hint">
              Library pages read one root at a time from its own committed
              snapshot. Switching roots discards the current page position.
            </p>
          </div>
          <select
            id="library-root-select"
            onChange={(event) => selectRoot(event.target.value)}
            value={resolvedRootId ?? ""}
          >
            {statuses.map((status) => (
              <option key={status.root.id} value={status.root.id}>
                {rootControlName(status, duplicateDisplayNames)}
              </option>
            ))}
          </select>
        </section>
      )}

      {pageState.kind === "error" && (
        <section
          aria-labelledby="library-error-title"
          className="library-state-panel library-state-panel--error"
          data-library-state="error"
        >
          <AlertCircle
            aria-hidden="true"
            className="library-state-panel__icon"
          />
          <div role="alert">
            <h2 id="library-error-title">Library results unavailable</h2>
            <p>
              The committed list could not be read. No files were marked
              missing.
            </p>
          </div>
          <button
            className="library-button library-button--primary"
            onClick={() => {
              pageRequestSequence.current += 1;
              setPaginationNotice(null);
              setPageState({ kind: "loading" });
              setPageAttempt((attempt) => attempt + 1);
            }}
            type="button"
          >
            Try again
          </button>
        </section>
      )}

      {paginationNotice !== null && (
        <p aria-live="polite" className="library-inline-notice" role="status">
          {paginationNotice}
        </p>
      )}

      {pageState.kind === "ready" && pageState.refreshError && (
        <p className="library-inline-error" role="alert">
          The latest committed page could not be refreshed. Previous results
          remain unchanged.
        </p>
      )}

      {pageState.kind === "ready" && staleResults && (
        <section
          aria-labelledby="library-stale-title"
          className="library-stale-banner"
          data-library-state="stale-results"
          role="status"
        >
          <Clock3 aria-hidden="true" className="library-stale-banner__icon" />
          <div>
            <h2 id="library-stale-title">Showing previous committed results</h2>
            <p>
              The current scan is not authoritative yet. Existing locations
              remain visible with their last committed Present or Missing state;
              no new missing files are being inferred.
            </p>
          </div>
        </section>
      )}

      {(pageState.kind === "ready" || showEmptyForMissingRoots) &&
        !pageHasRecords && (
          <section
            aria-labelledby="library-empty-title"
            className="library-state-panel"
            data-library-state={libraryState}
          >
            {rootUnavailable ? (
              <CircleSlash2
                aria-hidden="true"
                className="library-state-panel__icon"
              />
            ) : (
              <FolderSearch
                aria-hidden="true"
                className="library-state-panel__icon"
              />
            )}
            <h2 id="library-empty-title">
              {rootUnavailable
                ? "A scan root is unavailable"
                : "No committed files yet"}
            </h2>
            <p>
              {rootUnavailable
                ? "The last committed results were not changed. Restore access or retry the root before expecting new locations."
                : "Add and scan a root to discover FLP-named files. A run must complete authoritatively before anything is published here."}
            </p>
            {!rootUnavailable && (
              <Link className="inline-action" to="/preferences">
                Manage scan roots
              </Link>
            )}
          </section>
        )}

      {pageState.kind === "ready" && pageHasRecords && (
        <section
          aria-labelledby="library-list-title"
          className="library-results"
        >
          <div className="library-results__heading">
            <div>
              <p className="eyebrow">Published dataset</p>
              <h2
                id="library-list-title"
                ref={listHeadingReference}
                tabIndex={-1}
              >
                File locations
              </h2>
            </div>
            <p aria-live="polite">
              {page.records.length} committed{" "}
              {page.records.length === 1 ? "location" : "locations"} on page{" "}
              {cursorHistory.length + 1}
              {selectedRootLabel === null ? "" : ` in ${selectedRootLabel}`}
            </p>
          </div>

          <ul
            aria-label="Committed library file locations"
            className="library-record-list"
          >
            {page.records.map((record) => (
              <LibraryRecord
                duplicateDisplayNames={duplicateDisplayNames}
                key={record.locationId}
                record={record}
              />
            ))}
          </ul>

          {(cursorHistory.length > 0 || page.nextCursor !== null) && (
            <nav aria-label="Library pages" className="library-pagination">
              <button
                aria-label="Previous library page"
                className="library-button library-button--secondary"
                disabled={cursorHistory.length === 0}
                onClick={goToPreviousPage}
                type="button"
              >
                <ChevronLeft aria-hidden="true" />
                Previous
              </button>
              <span aria-current="page">Page {cursorHistory.length + 1}</span>
              <button
                aria-label="Next library page"
                className="library-button library-button--secondary"
                disabled={page.nextCursor === null}
                onClick={goToNextPage}
                type="button"
              >
                Next
                <ChevronRight aria-hidden="true" />
              </button>
            </nav>
          )}
        </section>
      )}
    </div>
  );
}

function ScanStatusPanel({
  actionState,
  duplicateDisplayNames,
  onRetryStatus,
  onRunScan,
  onCancelScan,
  retryButtonReferences,
  scanButtonReferences,
  cancelButtonReferences,
  statusState,
}: {
  readonly actionState: ActionState;
  readonly duplicateDisplayNames: ReadonlySet<string>;
  readonly onRetryStatus: () => void;
  readonly onRunScan: (status: ScanStatus, retry: boolean) => void;
  readonly onCancelScan: (status: ScanStatus) => void;
  readonly retryButtonReferences: React.MutableRefObject<
    Map<string, HTMLButtonElement>
  >;
  readonly scanButtonReferences: React.MutableRefObject<
    Map<string, HTMLButtonElement>
  >;
  readonly cancelButtonReferences: React.MutableRefObject<
    Map<string, HTMLButtonElement>
  >;
  readonly statusState: StatusState;
}) {
  return (
    <section
      aria-busy={statusState.kind === "loading"}
      aria-labelledby="scan-status-title"
      className="library-scan-panel"
    >
      <div className="library-scan-panel__heading">
        <div>
          <p className="eyebrow">Execution</p>
          <h2 id="scan-status-title">Scan status</h2>
        </div>
        <p>
          Availability is the last stored adapter observation, not a live
          reachability check. Incomplete work never publishes partial results.
        </p>
      </div>

      {actionState.kind === "working" && (
        <p aria-live="polite" className="library-action-message" role="status">
          <LoaderCircle
            aria-hidden="true"
            className="library-action-message__icon"
          />
          {actionState.message}
        </p>
      )}
      {actionState.kind === "error" && (
        <p
          className="library-action-message library-action-message--error"
          role="alert"
        >
          <AlertCircle
            aria-hidden="true"
            className="library-action-message__icon"
          />
          {actionState.message}
        </p>
      )}

      {statusState.kind === "loading" && (
        <p aria-live="polite" className="library-status-loading" role="status">
          Loading scan status…
        </p>
      )}
      {statusState.kind === "error" && (
        <div className="library-status-error" role="alert">
          <p>
            Scan status is unavailable. Committed Library results are unchanged.
          </p>
          <button
            className="library-button library-button--secondary"
            onClick={onRetryStatus}
            type="button"
          >
            Retry status
          </button>
        </div>
      )}
      {statusState.kind === "ready" && (
        <>
          {statusState.refreshError && (
            <p className="library-inline-error" role="alert">
              Scan status could not be refreshed. The last known status is
              shown.
            </p>
          )}
          {statusState.statuses.length === 0 ? (
            <p className="library-status-empty">
              No scan roots are configured. Add one in Preferences before using
              the review harness controls.
            </p>
          ) : (
            <ul aria-label="Scan root status" className="library-scan-list">
              {statusState.statuses.map((status) => (
                <ScanStatusCard
                  actionState={actionState}
                  cancelButtonReferences={cancelButtonReferences}
                  duplicateDisplayNames={duplicateDisplayNames}
                  key={status.root.id}
                  onCancel={() => onCancelScan(status)}
                  onRun={(retry) => onRunScan(status, retry)}
                  retryButtonReferences={retryButtonReferences}
                  scanButtonReferences={scanButtonReferences}
                  status={status}
                />
              ))}
            </ul>
          )}
        </>
      )}
    </section>
  );
}

function ScanStatusCard({
  actionState,
  cancelButtonReferences,
  duplicateDisplayNames,
  onCancel,
  onRun,
  retryButtonReferences,
  scanButtonReferences,
  status,
}: {
  readonly actionState: ActionState;
  readonly cancelButtonReferences: React.MutableRefObject<
    Map<string, HTMLButtonElement>
  >;
  readonly duplicateDisplayNames: ReadonlySet<string>;
  readonly onCancel: () => void;
  readonly onRun: (retry: boolean) => void;
  readonly retryButtonReferences: React.MutableRefObject<
    Map<string, HTMLButtonElement>
  >;
  readonly scanButtonReferences: React.MutableRefObject<
    Map<string, HTMLButtonElement>
  >;
  readonly status: ScanStatus;
}) {
  const name = rootControlName(status, duplicateDisplayNames);
  const busy =
    actionState.kind === "working" && actionState.rootId === status.root.id;
  const canScan =
    status.root.enabled &&
    !isScanAvailabilityUnavailable(status.root.availability);
  const isActive = status.state === "queued" || status.state === "running";
  const showRetry =
    terminalStates.has(status.state) ||
    isScanAvailabilityUnavailable(status.root.availability);

  return (
    <li className="library-scan-item" data-root-id={status.root.id}>
      <div className="library-scan-item__copy">
        <div className="library-scan-item__title">
          <h3>{status.root.displayName}</h3>
          <span
            className={`library-status-chip library-status-chip--${status.root.availability}`}
          >
            {formatAvailability(status.root.availability)}
          </span>
        </div>
        <p className="library-scan-item__path">{status.root.canonicalPath}</p>
        <p className="library-scan-item__availability-note">
          {isScanAvailabilityUnknown(status.root.availability)
            ? "Reachability has not been checked recently; Scan now will verify access."
            : "This availability is the last adapter observation, not a live check."}
        </p>
        <dl className="library-scan-facts">
          <div>
            <dt>Root availability</dt>
            <dd>{formatAvailability(status.root.availability)}</dd>
          </div>
          <div>
            <dt>Execution</dt>
            <dd>{formatExecution(status)}</dd>
          </div>
          <div>
            <dt>Last successful results</dt>
            <dd>{formatDateTime(status.lastSuccessfulScanAt)}</dd>
          </div>
        </dl>
      </div>

      {(isActive || status.state === "completed") && (
        <div
          aria-live="polite"
          className="library-progress"
          data-scan-progress={status.state}
        >
          <div className="library-progress__heading">
            <strong>
              {status.state === "completed" ? "Committed" : "Progress counters"}
            </strong>
            {status.state !== "completed" && <span>Total work unknown</span>}
          </div>
          <dl className="library-progress__facts">
            <div>
              <dt>Files observed</dt>
              <dd>{status.counters.filesObserved}</dd>
            </div>
            <div>
              <dt>Directories visited</dt>
              <dd>{status.counters.directoriesVisited}</dd>
            </div>
            <div>
              <dt>Total files</dt>
              <dd>
                {status.counters.totalFiles === null
                  ? "Unknown"
                  : status.counters.totalFiles}
              </dd>
            </div>
          </dl>
          {status.state !== "completed" && (
            <p className="library-progress__note">
              Progress is shown as counters; no percentage is estimated.
            </p>
          )}
        </div>
      )}

      {terminalStates.has(status.state) && (
        <p className="library-scan-outcome">
          {status.state === "cancelled"
            ? scanErrorMessages.cancelled
            : status.state === "interrupted"
              ? "The previous run was interrupted. Previous committed results were kept."
              : scanErrorMessages[status.errorCode ?? "internal"]}
        </p>
      )}

      <div className="library-scan-item__actions">
        {isActive ? (
          <button
            aria-label={`Cancel scan ${name}`}
            className="library-button library-button--secondary"
            disabled={busy}
            onClick={onCancel}
            ref={(button) => {
              if (button === null)
                cancelButtonReferences.current.delete(status.root.id);
              else cancelButtonReferences.current.set(status.root.id, button);
            }}
            type="button"
          >
            <XCircle aria-hidden="true" />
            Cancel
          </button>
        ) : showRetry ? (
          <button
            aria-label={`Retry scan ${name}`}
            className="library-button library-button--primary"
            disabled={busy || !status.root.enabled}
            onClick={() => onRun(true)}
            ref={(button) => {
              if (button === null)
                retryButtonReferences.current.delete(status.root.id);
              else retryButtonReferences.current.set(status.root.id, button);
            }}
            type="button"
          >
            <RefreshCw aria-hidden="true" />
            Retry
          </button>
        ) : (
          <button
            aria-label={`Scan now ${name}`}
            className="library-button library-button--primary"
            disabled={busy || !canScan}
            onClick={() => onRun(false)}
            ref={(button) => {
              if (button === null)
                scanButtonReferences.current.delete(status.root.id);
              else scanButtonReferences.current.set(status.root.id, button);
            }}
            type="button"
          >
            <ScanLine aria-hidden="true" />
            Scan now
          </button>
        )}
        {!status.root.enabled && (
          <span className="library-scan-item__hint">
            Enable this root in Preferences.
          </span>
        )}
      </div>
    </li>
  );
}

function LibraryRecord({
  duplicateDisplayNames,
  record,
}: {
  readonly duplicateDisplayNames: ReadonlySet<string>;
  readonly record: PublishedFileLocation;
}) {
  const rootLabel = duplicateDisplayNames.has(record.rootDisplayName)
    ? `${record.rootDisplayName} (${record.rootCanonicalPath})`
    : record.rootDisplayName;
  return (
    <li className="library-record-item" data-presence={record.presence}>
      <article aria-labelledby={`library-record-${record.locationId}`}>
        <div className="library-record-item__heading">
          <h3 id={`library-record-${record.locationId}`}>{record.fileName}</h3>
          <span
            className={`library-presence library-presence--${record.presence}`}
          >
            {record.presence === "present" ? (
              <CheckCircle2 aria-hidden="true" />
            ) : (
              <XCircle aria-hidden="true" />
            )}
            {record.presence === "present" ? "Present" : "Missing"}
          </span>
        </div>
        <dl className="library-record-facts">
          <div>
            <dt>Root</dt>
            <dd aria-label={`Root ${rootLabel}`}>{rootLabel}</dd>
          </div>
          <div>
            <dt>Relative path</dt>
            <dd>{record.relativePath}</dd>
          </div>
          <div>
            <dt>Byte size</dt>
            <dd>{formatBytes(record.byteSize)}</dd>
          </div>
          <div>
            <dt>Modified</dt>
            <dd>{formatDateTime(record.modifiedAt)}</dd>
          </div>
        </dl>
      </article>
    </li>
  );
}

import { NATIVE_COMMAND_SCHEMA_VERSION } from "../platform/contracts";
import {
  LibraryAdapterError,
  MAX_LIBRARY_PAGE_LIMIT,
  MIN_LIBRARY_PAGE_LIMIT,
  LIBRARY_PAGE_LIMIT,
  extractModifiedAtFraction,
  validateByteSizeDecimal,
  type CancelScanResult,
  type LibraryErrorCode,
  type LibraryPage,
  type LibraryPageRequest,
  type LibraryScanAdapter,
  type PublishedFileLocation,
  type ScanErrorCode,
  type ScanExecutionState,
  type ScanStartResult,
  type ScanStatus,
} from "./contracts";
import { parseScanRoot } from "../platform/contracts";

export const SCAN_NOW_COMMAND = "scan_now";
export const CANCEL_SCAN_COMMAND = "cancel_scan";
export const RETRY_SCAN_COMMAND = "retry_scan";
export const LIST_SCAN_STATUSES_COMMAND = "list_scan_statuses";
export const GET_LIBRARY_PAGE_COMMAND = "get_library_page";
export const GET_SCAN_CONSOLE_STATE_COMMAND = "get_scan_console_state";
export const SCAN_STATUS_CHANGED_EVENT = "scan-status-changed";

export type NativeInvoke = (
  command: string,
  args: Record<string, unknown>,
) => Promise<unknown>;

export type NativeListen = (
  event: string,
  handler: () => void,
) => Promise<() => void> | (() => void);

export interface NativeLibraryTransport {
  readonly invoke: NativeInvoke;
  readonly listen: NativeListen;
}

const LIBRARY_ERROR_MESSAGES: Readonly<Record<LibraryErrorCode, string>> = {
  access_denied: "The requested location could not be accessed.",
  unavailable: "The requested service is temporarily unavailable.",
  unsupported: "The requested operation is not supported for this location.",
  resource_limit: "The operation exceeded a resource limit.",
  conflict: "The request could not be completed because its state changed.",
  not_found: "The requested item is no longer available.",
  cancelled: "The operation was cancelled.",
  internal: "Fruitboard could not complete the request.",
  invalid_cursor:
    "The page continuation is not valid; start from the first page.",
  stale_cursor: "The list changed; start again from the first page.",
};

const LIBRARY_RETRYABLE = new Set<LibraryErrorCode>([
  "conflict",
  "unavailable",
]);

const LIBRARY_ERROR_CODES = new Set<string>([
  "access_denied",
  "unavailable",
  "unsupported",
  "resource_limit",
  "conflict",
  "not_found",
  "cancelled",
  "internal",
  "invalid_cursor",
  "stale_cursor",
]);

function isLibraryErrorCode(code: string): code is LibraryErrorCode {
  return LIBRARY_ERROR_CODES.has(code);
}

function isScanExecutionState(state: string): state is ScanExecutionState {
  return SCAN_STATES.has(state as ScanExecutionState);
}

function isScanErrorCodeValue(code: string): code is ScanErrorCode {
  return SCAN_ERROR_CODES.has(code as ScanErrorCode);
}

const NATIVE_INVALID_REQUEST_MESSAGE = "The request was not valid.";

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null;

const isCorrelationId = (value: unknown): value is string =>
  typeof value === "string" && /^correlation_[0-9a-f]{32}$/.test(value);

const isNonEmptyString = (value: unknown): value is string =>
  typeof value === "string" && value.length > 0;

function unwrapLibraryEnvelope<T>(
  value: unknown,
  parseData: (data: unknown) => T,
): T {
  if (
    !isRecord(value) ||
    value["schemaVersion"] !== NATIVE_COMMAND_SCHEMA_VERSION ||
    !isCorrelationId(value["correlationId"])
  ) {
    throw new LibraryAdapterError("internal");
  }
  if (value["status"] === "error") {
    const error = value["error"];
    if (!isRecord(error)) {
      throw new LibraryAdapterError("internal");
    }
    const codeValue: unknown = error["code"];
    if (typeof codeValue !== "string") {
      throw new LibraryAdapterError("internal");
    }
    const code = codeValue;
    // `invalid_request` is not part of the library seam; a malformed
    // request can never be retried from the UI, so it surfaces as the
    // safe internal envelope instead of leaking a new code.
    if (code === "invalid_request") {
      if (
        error["message"] !== NATIVE_INVALID_REQUEST_MESSAGE ||
        error["retryable"] !== false
      ) {
        throw new LibraryAdapterError("internal");
      }
      throw new LibraryAdapterError("internal");
    }
    if (!isLibraryErrorCode(code)) {
      throw new LibraryAdapterError("internal");
    }
    const typed = code;
    if (
      error["message"] !== LIBRARY_ERROR_MESSAGES[typed] ||
      error["retryable"] !== LIBRARY_RETRYABLE.has(typed)
    ) {
      throw new LibraryAdapterError("internal");
    }
    throw new LibraryAdapterError(typed);
  }
  if (value["status"] !== "ok" || !("data" in value)) {
    throw new LibraryAdapterError("internal");
  }
  try {
    return parseData(value["data"]);
  } catch (error) {
    if (error instanceof LibraryAdapterError) throw error;
    throw new LibraryAdapterError("internal");
  }
}

export const clampLibraryLimit = (limit: number): number => {
  const floored = Number.isFinite(limit)
    ? Math.floor(limit)
    : LIBRARY_PAGE_LIMIT;
  return Math.min(
    MAX_LIBRARY_PAGE_LIMIT,
    Math.max(MIN_LIBRARY_PAGE_LIMIT, floored),
  );
};

const SCAN_STATES = new Set<ScanExecutionState>([
  "idle",
  "queued",
  "running",
  "completed",
  "cancelled",
  "failed",
  "interrupted",
]);

const SCAN_ERROR_CODES = new Set<ScanErrorCode>([
  "access_denied",
  "unavailable",
  "unsupported",
  "resource_limit",
  "conflict",
  "not_found",
  "cancelled",
  "internal",
]);

const SCAN_START_OUTCOMES = new Set([
  "queued",
  "already_queued",
  "already_running",
]);

const CANCEL_OUTCOMES = new Set([
  "cancelled",
  "cancellation_requested",
  "already_cancelled",
  "already_completed",
  "already_failed",
  "not_found",
]);

function parseOptionalId(value: unknown): string | null {
  if (value === null) return null;
  if (isNonEmptyString(value)) return value;
  throw new LibraryAdapterError("internal");
}

function parseCounters(value: unknown): ScanStatus["counters"] {
  if (!isRecord(value)) throw new LibraryAdapterError("internal");
  const filesObservedValue: unknown = value["filesObserved"];
  const directoriesVisitedValue: unknown = value["directoriesVisited"];
  const totalFilesValue: unknown = value["totalFiles"];
  const isSafeCount = (entry: unknown): entry is number =>
    typeof entry === "number" && Number.isSafeInteger(entry) && entry >= 0;
  if (
    !isSafeCount(filesObservedValue) ||
    !isSafeCount(directoriesVisitedValue)
  ) {
    throw new LibraryAdapterError("internal");
  }
  let totalFiles: number | null;
  if (totalFilesValue === null) {
    totalFiles = null;
  } else if (isSafeCount(totalFilesValue)) {
    totalFiles = totalFilesValue;
  } else {
    throw new LibraryAdapterError("internal");
  }
  return {
    filesObserved: filesObservedValue,
    directoriesVisited: directoriesVisitedValue,
    totalFiles,
  };
}

function parseOptionalTimestamp(value: unknown): string | null {
  if (value === null) return null;
  if (typeof value !== "string" || value.length === 0) {
    throw new LibraryAdapterError("internal");
  }
  // RFC 3339 with up to nanosecond precision; retained verbatim for
  // display. Rejects integer timestamps and over-precise fractions.
  if (extractModifiedAtFraction(value) === null) {
    // The native millisecond formatter trims trailing zeroes and always
    // emits `Z`; accept exactly that shape. Anything else is untrusted.
    throw new LibraryAdapterError("internal");
  }
  return value;
}

function parseScanErrorCode(value: unknown): ScanErrorCode | null {
  if (value === null) return null;
  if (typeof value === "string" && isScanErrorCodeValue(value)) {
    return value;
  }
  throw new LibraryAdapterError("internal");
}

export function parseScanStatus(value: unknown): ScanStatus {
  if (!isRecord(value)) throw new LibraryAdapterError("internal");
  const root = parseScanRootSafe(value["root"]);
  const stateValue: unknown = value["state"];
  if (typeof stateValue !== "string" || !isScanExecutionState(stateValue)) {
    throw new LibraryAdapterError("internal");
  }
  const state = stateValue;
  const jobId = parseOptionalId(value["jobId"]);
  const runId = parseOptionalId(value["runId"]);
  // Queued work has a job but no run yet; running work carries both.
  if ((state === "queued" || state === "idle") && runId !== null) {
    throw new LibraryAdapterError("internal");
  }
  const cancellationValue: unknown = value["cancellationRequested"];
  if (cancellationValue !== true && cancellationValue !== false) {
    throw new LibraryAdapterError("internal");
  }
  const retryAvailableValue: unknown = value["retryAvailable"];
  if (retryAvailableValue !== true && retryAvailableValue !== false) {
    throw new LibraryAdapterError("internal");
  }
  if (
    retryAvailableValue === true &&
    (state !== "failed" || jobId === null || !root.enabled)
  ) {
    throw new LibraryAdapterError("internal");
  }
  const counters = parseCounters(value["counters"]);
  const lastSuccessfulScanAt = parseOptionalTimestamp(
    value["lastSuccessfulScanAt"],
  );
  const lastOutcomeAt = parseOptionalTimestamp(value["lastOutcomeAt"]);
  const errorCode = parseScanErrorCode(value["errorCode"]);
  // Queued/running/idle attempts have no outcome yet.
  if (
    (state === "queued" || state === "running" || state === "idle") &&
    errorCode !== null
  ) {
    throw new LibraryAdapterError("internal");
  }
  return {
    root,
    state,
    jobId,
    runId,
    cancellationRequested: cancellationValue,
    retryAvailable: retryAvailableValue,
    counters,
    lastSuccessfulScanAt,
    lastOutcomeAt,
    errorCode,
  };
}

function parseScanRootSafe(value: unknown): ScanStatus["root"] {
  try {
    return parseScanRoot(value);
  } catch {
    throw new LibraryAdapterError("internal");
  }
}

export function parseScanStatusList(value: unknown): readonly ScanStatus[] {
  if (!Array.isArray(value)) throw new LibraryAdapterError("internal");
  // Per-root only: the adapter never merges or sorts; native order is kept.
  return value.map(parseScanStatus);
}

type ScanStartOutcomeValue = ScanStartResult["outcome"];
type CancelOutcomeValue = CancelScanResult["outcome"];

function isScanStartOutcome(outcome: string): outcome is ScanStartOutcomeValue {
  return SCAN_START_OUTCOMES.has(outcome);
}

function isCancelOutcome(outcome: string): outcome is CancelOutcomeValue {
  return CANCEL_OUTCOMES.has(outcome);
}

export function parseScanStartResult(value: unknown): ScanStartResult {
  if (!isRecord(value)) throw new LibraryAdapterError("internal");
  const rootId: unknown = value["rootId"];
  const jobId: unknown = value["jobId"];
  const runId: unknown = value["runId"];
  const outcomeValue: unknown = value["outcome"];
  if (!isNonEmptyString(rootId) || !isNonEmptyString(jobId)) {
    throw new LibraryAdapterError("internal");
  }
  if (typeof outcomeValue !== "string" || !isScanStartOutcome(outcomeValue)) {
    throw new LibraryAdapterError("internal");
  }
  const outcome = outcomeValue;
  let parsedRunId: string | null;
  if (outcome === "already_running") {
    if (!isNonEmptyString(runId)) throw new LibraryAdapterError("internal");
    parsedRunId = runId;
  } else if (runId === null) {
    parsedRunId = null;
  } else {
    // `queued` and `already_queued` never carry a run; null while queued.
    throw new LibraryAdapterError("internal");
  }
  return {
    rootId,
    jobId,
    runId: parsedRunId,
    outcome,
  };
}

export function parseCancelScanResult(value: unknown): CancelScanResult {
  if (!isRecord(value)) throw new LibraryAdapterError("internal");
  const rootId: unknown = value["rootId"];
  const jobId: unknown = value["jobId"];
  const runId: unknown = value["runId"];
  const outcomeValue: unknown = value["outcome"];
  if (!isNonEmptyString(rootId) || !isNonEmptyString(jobId)) {
    throw new LibraryAdapterError("internal");
  }
  if (typeof outcomeValue !== "string" || !isCancelOutcome(outcomeValue)) {
    throw new LibraryAdapterError("internal");
  }
  const outcome = outcomeValue;
  let parsedRunId: string | null;
  if (runId === null) {
    parsedRunId = null;
  } else if (isNonEmptyString(runId)) {
    parsedRunId = runId;
  } else {
    throw new LibraryAdapterError("internal");
  }
  return {
    rootId,
    jobId,
    runId: parsedRunId,
    outcome,
  };
}

function parseRecord(value: unknown): PublishedFileLocation {
  if (!isRecord(value)) throw new LibraryAdapterError("internal");
  const locationId: unknown = value["locationId"];
  const rootId: unknown = value["rootId"];
  const rootDisplayName: unknown = value["rootDisplayName"];
  const rootCanonicalPath: unknown = value["rootCanonicalPath"];
  const fileName: unknown = value["fileName"];
  const relativePath: unknown = value["relativePath"];
  const byteSize: unknown = value["byteSize"];
  const modifiedAt: unknown = value["modifiedAt"];
  const presence: unknown = value["presence"];
  if (
    !isNonEmptyString(locationId) ||
    !isNonEmptyString(rootId) ||
    !isNonEmptyString(rootDisplayName) ||
    !isNonEmptyString(rootCanonicalPath) ||
    !isNonEmptyString(fileName) ||
    !isNonEmptyString(relativePath)
  ) {
    throw new LibraryAdapterError("internal");
  }
  // Decimal-string byte sizes round-trip exactly; never a JSON number.
  if (
    typeof byteSize !== "string" ||
    validateByteSizeDecimal(byteSize).ok !== true
  ) {
    throw new LibraryAdapterError("internal");
  }
  // RFC 3339 with nanosecond precision retained verbatim.
  if (
    typeof modifiedAt !== "string" ||
    extractModifiedAtFraction(modifiedAt) === null
  ) {
    throw new LibraryAdapterError("internal");
  }
  if (presence !== "present" && presence !== "missing") {
    throw new LibraryAdapterError("internal");
  }
  return {
    locationId,
    rootId,
    rootDisplayName,
    rootCanonicalPath,
    fileName,
    relativePath,
    byteSize,
    modifiedAt,
    presence,
  };
}

export function parseLibraryPage(value: unknown): LibraryPage {
  if (!isRecord(value)) throw new LibraryAdapterError("internal");
  const rootId: unknown = value["rootId"];
  const snapshotId: unknown = value["snapshotId"];
  const records: unknown = value["records"];
  const nextCursor: unknown = value["nextCursor"];
  if (!isNonEmptyString(rootId) || !isNonEmptyString(snapshotId)) {
    throw new LibraryAdapterError("internal");
  }
  if (!Array.isArray(records)) throw new LibraryAdapterError("internal");
  // Per-root only: records are returned verbatim, never merged or sorted.
  const parsed = records.map(parseRecord);
  let parsedCursor: string | null;
  if (nextCursor === null) {
    parsedCursor = null;
  } else if (isNonEmptyString(nextCursor)) {
    parsedCursor = nextCursor;
  } else {
    throw new LibraryAdapterError("internal");
  }
  return {
    rootId,
    snapshotId,
    records: parsed,
    nextCursor: parsedCursor,
  };
}

export function createScanNowArguments(rootId: string) {
  return Object.freeze({
    request: Object.freeze({
      schemaVersion: NATIVE_COMMAND_SCHEMA_VERSION,
      rootId,
    }),
  });
}

export function createCancelScanArguments(jobId: string) {
  return Object.freeze({
    request: Object.freeze({
      schemaVersion: NATIVE_COMMAND_SCHEMA_VERSION,
      jobId,
    }),
  });
}

export function createRetryScanArguments(jobId: string) {
  return Object.freeze({
    request: Object.freeze({
      schemaVersion: NATIVE_COMMAND_SCHEMA_VERSION,
      jobId,
    }),
  });
}

export const LIST_SCAN_STATUSES_ARGUMENTS = Object.freeze({
  request: Object.freeze({ schemaVersion: NATIVE_COMMAND_SCHEMA_VERSION }),
});

export function createGetLibraryPageArguments(
  rootId: string,
  limit: number,
  cursor: string | null,
) {
  return Object.freeze({
    request: Object.freeze({
      schemaVersion: NATIVE_COMMAND_SCHEMA_VERSION,
      rootId,
      limit: clampLibraryLimit(limit),
      cursor,
      snapshotId: null,
    }),
  });
}

export const GET_SCAN_CONSOLE_STATE_ARGUMENTS = Object.freeze({
  request: Object.freeze({ schemaVersion: NATIVE_COMMAND_SCHEMA_VERSION }),
});

/**
 * Native `LibraryScanAdapter` behind the scan-console IPC surface.
 *
 * Maps 1:1 to `docs/review/phase-2-ipc/README.md` §2-§4: queued work keeps
 * `jobId` with a null `runId`, `already_queued`/`already_running` coalesce,
 * terminal cancel `already_*` outcomes are no-ops carrying the recorded
 * `runId`, and `invalid_cursor`/`stale_cursor` surface for the renderer's
 * page-one restart (`cursor: null`, same `rootId`). The adapter clamps
 * `limit` to 1..200, validates decimal-string `byteSize` and RFC 3339-ns
 * `modifiedAt` without numeric conversion, and never merges or sorts
 * across roots.
 *
 * Transport is injected so unit tests can drive typed envelopes without a
 * Tauri runtime; production wires the real `invoke`/`listen` in
 * `apps/client/src/platform/tauri.ts`.
 */
export function createNativeLibraryScanAdapter(
  transport: NativeLibraryTransport,
): LibraryScanAdapter {
  const execute = async <Result>(
    command: string,
    args: Record<string, unknown>,
    parseData: (data: unknown) => Result,
  ): Promise<Result> => {
    let envelope: unknown;
    try {
      envelope = await transport.invoke(command, args);
    } catch {
      // Never surface invoke diagnostics (paths, tokens); the UI treats a
      // transport failure as a recoverable unavailable state.
      throw new LibraryAdapterError("unavailable");
    }
    return unwrapLibraryEnvelope(envelope, parseData);
  };

  return {
    async getLibraryPage(request: LibraryPageRequest): Promise<LibraryPage> {
      if (request.rootId === "") throw new LibraryAdapterError("not_found");
      return execute(
        GET_LIBRARY_PAGE_COMMAND,
        createGetLibraryPageArguments(
          request.rootId,
          request.limit,
          request.cursor,
        ) as unknown as Record<string, unknown>,
        parseLibraryPage,
      );
    },

    async listScanStatuses(): Promise<readonly ScanStatus[]> {
      return execute(
        LIST_SCAN_STATUSES_COMMAND,
        LIST_SCAN_STATUSES_ARGUMENTS as unknown as Record<string, unknown>,
        parseScanStatusList,
      );
    },

    async scanNow(rootId: string) {
      if (rootId === "") throw new LibraryAdapterError("not_found");
      return execute(
        SCAN_NOW_COMMAND,
        createScanNowArguments(rootId) as unknown as Record<string, unknown>,
        parseScanStartResult,
      );
    },

    async cancelScan(jobId: string): Promise<CancelScanResult> {
      if (jobId === "") throw new LibraryAdapterError("not_found");
      return execute(
        CANCEL_SCAN_COMMAND,
        createCancelScanArguments(jobId) as unknown as Record<string, unknown>,
        parseCancelScanResult,
      );
    },

    async retryScan(jobId: string) {
      if (jobId === "") throw new LibraryAdapterError("not_found");
      return execute(
        RETRY_SCAN_COMMAND,
        createRetryScanArguments(jobId) as unknown as Record<string, unknown>,
        parseScanStartResult,
      );
    },

    subscribe(listener: () => void): () => void {
      let disposed = false;
      let unlisten: (() => void) | null = null;
      try {
        const pending = transport.listen(SCAN_STATUS_CHANGED_EVENT, () => {
          if (!disposed) listener();
        });
        if (typeof pending === "function") {
          unlisten = pending;
        } else {
          void pending.then(
            (resolved) => {
              if (disposed) {
                resolved();
              } else {
                unlisten = resolved;
              }
            },
            () => undefined,
          );
        }
      } catch {
        return () => {
          disposed = true;
        };
      }
      return () => {
        disposed = true;
        // When the async listen has not resolved yet, the pending
        // continuation above calls the resolver exactly once; no second
        // attach here, so dispose-before-resolve unlistens exactly once.
        if (unlisten !== null) unlisten();
      };
    },
  };
}

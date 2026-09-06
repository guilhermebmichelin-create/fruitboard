import type { ScanRoot, ScanRootAvailability } from "../platform/contracts";

/**
 * The UI-only seam used by the fake-adapter review harness. The native
 * PlatformPort must not grow these methods in this slice; the matching shared
 * IPC proposal lives in docs/review/phase-2-library/CONTRACT_PROPOSAL.md.
 */
export const LIBRARY_PAGE_LIMIT = 4;
export const MAX_LIBRARY_PAGE_LIMIT = 200;

export type LibraryFilePresence = "present" | "missing";

export interface PublishedFileLocation {
  readonly locationId: string;
  readonly rootId: string;
  readonly rootDisplayName: string;
  readonly rootCanonicalPath: string;
  readonly fileName: string;
  readonly relativePath: string;
  readonly byteSize: number;
  readonly modifiedAt: string;
  readonly presence: LibraryFilePresence;
}

export interface LibraryPageRequest {
  readonly cursor: string | null;
  readonly limit: number;
}

export interface LibraryPage {
  readonly records: readonly PublishedFileLocation[];
  readonly nextCursor: string | null;
}

export type ScanExecutionState =
  | "idle"
  | "queued"
  | "running"
  | "completed"
  | "cancelled"
  | "failed"
  | "interrupted";

export type ScanErrorCode =
  | "access_denied"
  | "unavailable"
  | "unsupported"
  | "resource_limit"
  | "conflict"
  | "not_found"
  | "cancelled"
  | "internal";

export interface ScanProgressCounters {
  readonly filesObserved: number;
  readonly directoriesVisited: number;
  readonly totalFiles: number | null;
}

export interface ScanStatus {
  readonly root: ScanRoot;
  readonly state: ScanExecutionState;
  readonly runId: string | null;
  readonly counters: ScanProgressCounters;
  readonly lastSuccessfulScanAt: string | null;
  readonly lastOutcomeAt: string | null;
  readonly errorCode: ScanErrorCode | null;
}

export type ScanStartOutcome = "queued" | "already_queued" | "already_running";

export interface ScanStartResult {
  readonly rootId: string;
  readonly runId: string;
  readonly outcome: ScanStartOutcome;
}

export type CancelScanOutcome =
  | "cancellation_requested"
  | "already_cancelled"
  | "already_completed"
  | "already_failed"
  | "not_found";

export interface CancelScanResult {
  readonly rootId: string;
  readonly runId: string;
  readonly outcome: CancelScanOutcome;
}

export interface LibraryScanAdapter {
  getLibraryPage(request: LibraryPageRequest): Promise<LibraryPage>;
  listScanStatuses(): Promise<readonly ScanStatus[]>;
  scanNow(rootId: string): Promise<ScanStartResult>;
  cancelScan(runId: string): Promise<CancelScanResult>;
  retryScan(rootId: string): Promise<ScanStartResult>;
  subscribe(listener: () => void): () => void;
}

export class LibraryAdapterError extends Error {
  readonly code: ScanErrorCode;

  constructor(code: ScanErrorCode) {
    super(code);
    this.name = "LibraryAdapterError";
    this.code = code;
  }
}

export function isScanAvailabilityUnavailable(
  availability: ScanRootAvailability,
): boolean {
  return availability !== "available";
}

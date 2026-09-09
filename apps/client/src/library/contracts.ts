import type { ScanRoot, ScanRootAvailability } from "../platform/contracts";

/**
 * Per-root Library/scan seam used by the fake-adapter review harness.
 *
 * This mirrors §5 of the authoritative `PHASE_2_INTEGRATION_CONTRACT.md`
 * (storage owner, Agent 1): one root per query, opaque snapshot-bound
 * cursors, decimal-string numerics, and job/run scan identity. The native
 * PlatformPort must not grow these methods in this slice; the UI-only
 * proposal in docs/review/phase-2-library/CONTRACT_PROPOSAL.md is
 * superseded for pagination scope and numeric encoding.
 */
export const LIBRARY_PAGE_LIMIT = 4;
export const MIN_LIBRARY_PAGE_LIMIT = 1;
export const MAX_LIBRARY_PAGE_LIMIT = 200;

export type LibraryFilePresence = "present" | "missing";

/**
 * Identifies the surface that owns the Library rendering. The review harness
 * opts in explicitly; the native product surface is the default.
 */
export type LibraryRenderContext = "native" | "review-harness";

/**
 * Canonical unsigned decimal encoding of a Rust integer (u64/u128 bound).
 * No leading zeroes except `"0"` itself; never a JSON number, so values
 * beyond JavaScript's exact-integer range round-trip exactly.
 */
export type DecimalString = string;

/** Maximum Rust `u64` value, exact decimal. */
export const U64_MAX_DECIMAL: DecimalString = "18446744073709551615";
/** Maximum SQLite-backed byte size (`i64::MAX`), exact decimal. */
export const MAX_SQLITE_BYTE_SIZE_DECIMAL: DecimalString =
  "9223372036854775807";

const U64_MAX = BigInt(U64_MAX_DECIMAL);
const MAX_SQLITE_BYTE_SIZE = BigInt(MAX_SQLITE_BYTE_SIZE_DECIMAL);

const DECIMAL_PATTERN = /^\d+$/;
const CANONICAL_DECIMAL_PATTERN = /^(0|[1-9][0-9]*)$/;

export type ByteSizeValidationReason =
  "format" | "u64_overflow" | "sqlite_bound";

export type ByteSizeValidation =
  | { readonly ok: true; readonly value: bigint }
  | { readonly ok: false; readonly reason: ByteSizeValidationReason };

/**
 * Validates a canonical decimal byte size: `/^\d+$/` shape, no leading
 * zeroes (except `"0"`), Rust `u64` range, and the `<= i64::MAX` SQLite
 * boundary. Never converts through `Number`, so `u64::MAX` is exact.
 */
export function validateByteSizeDecimal(value: unknown): ByteSizeValidation {
  if (typeof value !== "string") return { ok: false, reason: "format" };
  if (!DECIMAL_PATTERN.test(value)) return { ok: false, reason: "format" };
  if (!CANONICAL_DECIMAL_PATTERN.test(value)) {
    return { ok: false, reason: "format" };
  }
  const parsed = BigInt(value);
  if (parsed > U64_MAX) return { ok: false, reason: "u64_overflow" };
  if (parsed > MAX_SQLITE_BYTE_SIZE) {
    return { ok: false, reason: "sqlite_bound" };
  }
  return { ok: true, value: parsed };
}

export function byteSizeValidationMessage(
  reason: ByteSizeValidationReason,
): string {
  if (reason === "u64_overflow") {
    return `The stored byte size exceeds the maximum u64 value (${groupDecimalDigits(U64_MAX_DECIMAL)} bytes).`;
  }
  if (reason === "sqlite_bound") {
    return `The stored byte size exceeds the maximum supported SQLite value (${groupDecimalDigits(MAX_SQLITE_BYTE_SIZE_DECIMAL)} bytes).`;
  }
  return "The stored byte size is not a canonical unsigned decimal string.";
}

/**
 * Groups ASCII decimal digits with `,` separators without passing through
 * `Number`, so values up to `u64::MAX` format exactly.
 */
export function groupDecimalDigits(digits: string): string {
  const canonical = digits.replace(/^0+(?=\d)/, "");
  const body = canonical === "" ? "0" : canonical;
  let grouped = "";
  let count = 0;
  for (let index = body.length - 1; index >= 0; index -= 1) {
    const char = body[index];
    if (char === undefined || char < "0" || char > "9") {
      throw new LibraryAdapterError("internal");
    }
    if (count > 0 && count % 3 === 0) grouped = `,${grouped}`;
    grouped = `${char}${grouped}`;
    count += 1;
  }
  return grouped;
}

/**
 * Formats a canonical decimal byte size with `Intl`-style grouping and no
 * `Number()` precision loss. Accepts the full `u64` range (display only);
 * storage ingestion additionally enforces the SQLite bound.
 */
export function formatByteSizeDecimal(value: DecimalString): string {
  if (typeof value !== "string" || !CANONICAL_DECIMAL_PATTERN.test(value)) {
    throw new LibraryAdapterError("internal");
  }
  if (BigInt(value) > U64_MAX) throw new LibraryAdapterError("internal");
  return `${groupDecimalDigits(value)} bytes`;
}

/**
 * Extracts the retained nanosecond fraction (0-9 digits) from a UTC
 * RFC 3339 `modifiedAt` string without converting through `Date` or any
 * integer timestamp. Returns `null` when the value is malformed.
 */
export function extractModifiedAtFraction(value: string): string | null {
  const match =
    /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.(\d{1,9}))?(?:Z|[+-]\d{2}:\d{2})$/.exec(
      value,
    );
  if (match === null) return null;
  return match[1] ?? "";
}

export interface PublishedFileLocation {
  readonly locationId: string;
  readonly rootId: string;
  readonly rootDisplayName: string;
  /** Management display only; never diagnostics or error text. */
  readonly rootCanonicalPath: string;
  readonly fileName: string;
  /** Display spelling only; never a key, ordering, or cursor input. */
  readonly relativePath: string;
  /** Canonical decimal string; exact up to `u64::MAX`. */
  readonly byteSize: DecimalString;
  /** UTC RFC 3339 with nanosecond precision retained. */
  readonly modifiedAt: string;
  readonly presence: LibraryFilePresence;
}

export interface LibraryPageRequest {
  /** The single root this page reads; pages never mix roots. */
  readonly rootId: string;
  /** Requested page size; the native boundary clamps to 1..200. */
  readonly limit: number;
  /**
   * Opaque cursor binding root + committed snapshot + locator + location.
   * Null starts the current committed root snapshot.
   */
  readonly cursor: string | null;
}

export interface LibraryPage {
  /** The single root this page was read from. */
  readonly rootId: string;
  /** Opaque identity of the committed root snapshot used for this page. */
  readonly snapshotId: string;
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

export type LibraryErrorCode =
  ScanErrorCode | "invalid_cursor" | "stale_cursor";

export interface ScanProgressCounters {
  readonly filesObserved: number;
  readonly directoriesVisited: number;
  readonly totalFiles: number | null;
}

export interface ScanStatus {
  readonly root: ScanRoot;
  readonly state: ScanExecutionState;
  /** A queued job exists before a worker leases a run. */
  readonly jobId: string | null;
  /** A run is allocated when the queued job is leased; null while queued. */
  readonly runId: string | null;
  /** Durably set once a leased cancellation is requested. */
  readonly cancellationRequested: boolean;
  /** True only for an enabled failed job below its durable retry budget. */
  readonly retryAvailable: boolean;
  readonly counters: ScanProgressCounters;
  readonly lastSuccessfulScanAt: string | null;
  readonly lastOutcomeAt: string | null;
  readonly errorCode: ScanErrorCode | null;
}

export type ScanStartOutcome = "queued" | "already_queued" | "already_running";

export interface ScanStartResult {
  readonly rootId: string;
  readonly jobId: string;
  /** Null while queued; set only after a worker leases the attempt. */
  readonly runId: string | null;
  readonly outcome: ScanStartOutcome;
}

export type CancelScanOutcome =
  | "cancelled"
  | "cancellation_requested"
  | "already_cancelled"
  | "already_completed"
  | "already_failed"
  | "not_found";

export interface CancelScanResult {
  readonly rootId: string;
  readonly jobId: string;
  readonly runId: string | null;
  readonly outcome: CancelScanOutcome;
}

export interface LibraryScanAdapter {
  getLibraryPage(request: LibraryPageRequest): Promise<LibraryPage>;
  listScanStatuses(): Promise<readonly ScanStatus[]>;
  scanNow(rootId: string): Promise<ScanStartResult>;
  cancelScan(jobId: string): Promise<CancelScanResult>;
  retryScan(jobId: string): Promise<ScanStartResult>;
  subscribe(listener: () => void): () => void;
}

export class LibraryAdapterError extends Error {
  readonly code: LibraryErrorCode;

  constructor(code: LibraryErrorCode) {
    super(code);
    this.name = "LibraryAdapterError";
    this.code = code;
  }
}

export function isScanAvailabilityUnavailable(
  availability: ScanRootAvailability,
): boolean {
  return availability === "unavailable";
}

export function isScanAvailabilityUnknown(
  availability: ScanRootAvailability,
): boolean {
  return availability === "unknown";
}

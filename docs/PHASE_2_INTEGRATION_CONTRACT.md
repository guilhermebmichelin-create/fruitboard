# Phase 2 storage integration contract

Status: **authoritative for the #62 integration round; stop for owner review
before merge**.

- Contract-Revision: **2** (2026-09-07, storage owner, Agent 1).
- This document **supersedes the combined global-page section of the client
  `CONTRACT_PROPOSAL`** (`docs/review/phase-2-library/CONTRACT_PROPOSAL.md`,
  "Combined bounded Library query") **for this round**. The locked pagination
  shape is **one root per query** (§5). A future global page must introduce an
  explicit cross-root snapshot contract; silently concatenating per-root
  snapshots is prohibited.
- Enumerator docs and code (`crates/filesystem-enumeration`, Agent 2) and
  Library/scan UI docs and code (`apps/client/src/library/*`, Agent 3) are
  **proposals until the storage owner signs** their boundary changes in the
  handoff table (§6).
- This contract does not activate production scanning. Filesystem traversal,
  Windows boundary probing, watchers, parser access, renderer implementation,
  and native scan activation remain outside this PR.

## 1. Locator equality, display spelling, and ordering

Agent 2 must publish a versioned, root-relative `LocatorKeyV1` with every
observation. The key is an opaque, ASCII, durable comparison/order encoding;
storage does not lowercase, canonicalize, fold, or apply a SQLite `NOCASE`
collation. The key must encode the case-sensitivity mode of every traversed
directory component.

```rust
struct StorageObservation {
    locator_key: String,       // LocatorKeyV1, opaque to storage
    relative_path: String,     // display-preserving root-relative spelling
    byte_size: u64,
    modified_at_ns: i128,      // source precision; checked to i64 at staging
    identity: Option<EncodedIdentity>,
}
```

### 1.1 LocatorKeyV1 syntax (ABNF, RFC 5234)

```abnf
locator-key-v1 = "v1:" segment *("/" segment)
segment        = mode ":" 1*comp-char
mode           = "i" / "s"   ; i = case-insensitive component, s = case-sensitive
comp-char      = unreserved / pct-encoded
unreserved     = ALPHA / DIGIT / "-" / "_" / "." / "~"
pct-encoded    = "%" HEXDIG HEXDIG   ; emit uppercase hex; storage accepts either case; UTF-8 of NFC text
```

Envelope rules, all enforced by storage at the staging boundary:

- The literal prefix is `v1:`. Any key without it (including the v4
  `normalized_path` spelling and the private in-memory ordering struct's debug
  form) is rejected with `staging_rejected`.
- ASCII-only, no NUL (`%x00`), total length 1..=32768 bytes.
- Every `/`-separated segment carries its own `i:`/`s:` mode tag. No empty
  segments, no `\` separators, no `:` inside the encoded component.
- Storage rejects a display `relative_path` containing NUL or `:`; this ban is
  independent of the opaque locator-key encoding.
- `pct-encoded` bytes decode to UTF-8 of Unicode NFC text. The enumerator
  NFC-normalizes every component before encoding.
- A case-insensitive (`i:`) component is case-folded by the enumerator before
  encoding (Unicode simple case folding of the NFC form; ASCII subset folds to
  lowercase). A case-sensitive (`s:`) component preserves spelling. Storage
  never folds; byte equality is the equality rule.

The storage equality rule is exact equality of `locator_key` bytes within one
root. The durable order is:

```text
(scan_root_id, locator_key COLLATE BINARY, location_id)
```

Library pages are root-scoped, so their effective order is
`(locator_key COLLATE BINARY, location_id)`. `relative_path` is presentation
only. It never participates in uniqueness, ordering, cursor comparison,
identity lookup, or rename inference.

Required boundary behavior:

- A case-only rename in a case-insensitive Windows directory has the same
  locator key. Publication updates the display spelling and retains the
  location and physical association.
- `Foo.flp` and `foo.flp` in a Windows case-sensitive directory have distinct
  keys and remain distinct locations, even if their display spellings differ
  only by case.
- A change in directory case-mode semantics invalidates the enumeration/run;
  it must not reinterpret existing keys silently (the mode tag changes, so the
  key changes, so exact-path continuity misses by construction).
- Duplicate keys in one run reject the whole run. Enumeration order never
  chooses a winner.

### 1.2 Worked examples

1. Case-insensitive rename, same key. Directory `Projects` (`i:`) holds
   `Foo.flp`, later respelled `foo.flp`. The enumerator folds both to:
   `v1:i:projects/i:foo.flp`. One location; publication updates
   `relative_path` from `Projects\Foo.flp` to `Projects\foo.flp` and retains
   `locationId`, `project_file_id`, and identity.
2. Case-sensitive pair, distinct keys. Directory `Projects` (`s:`) holds both
   spellings: `v1:s:Projects/s:Foo.flp` vs `v1:s:Projects/s:foo.flp`. Two
   locations. `COLLATE BINARY` orders `Foo` (`0x46…`) before `foo`
   (`0x66…`); the tie-break `location_id` only orders the impossible
   full-key collision.
3. Mixed modes. Insensitive root `Shared`, sensitive directory `BuildOutput`,
   insensitive leaf: `v1:i:shared/s:BuildOutput/i:artifact.flp`. Flipping any
   component's mode changes its tag and therefore the key.
4. Unicode NFC. `café.flp` written with precomposed `é` (U+00E9, NFC) encodes
   to `v1:i:caf%C3%A9.flp`. The decomposed spelling `cafe` + U+0301 (NFD) must
   first be NFC-normalized by the enumerator to the identical key. Percent
   encoding keeps the key ASCII, satisfying the staging guard, while
   `relative_path` retains the display spelling verbatim.
5. Maximum length. A 32768-byte key is accepted; a 32769-byte key rejects the
   batch with `staging_rejected`. The bound counts key bytes only
   (`MAX_OBSERVATION_PATH_BYTES`); `relative_path` has its own 32768-byte
   bound and the pair is additionally capped by the run path-bytes budget.

### 1.3 Migration 005 legacy-key policy (quarantine, not copy)

Migration 005 does **not** copy `normalized_path` into `locator_key`. The v4
path column lacks per-component case-mode evidence, so no v4 value can be
trusted as a `LocatorKeyV1`: promoting one would risk aliasing two distinct
files (or splitting one file) under case-mode ambiguity.

- (a) Mapping: every legacy row is quarantined into a disjoint namespace,
  `locator_key = 'v0:legacy:' || normalized_path`, in both `file_location`
  and `scan_stage_observation`. The `v0:legacy:` prefix can never equal a
  `v1:` key, so a first new-format scan cannot alias a legacy row by path.
  `normalized_path` is retained untouched as migration-history compatibility
  data. New code reads and writes `locator_key`.
- (b) Unicode/non-ASCII: quarantine is a byte-preserving copy inside SQLite
  `TEXT`; it never fails on non-ASCII paths. The ASCII guard applies only to
  new `v1:` observations at the staging boundary. Non-ASCII display spellings
  continue to round-trip through `relative_path`.
- (c) First new-format scan: exact-path continuity misses by construction
  (disjoint namespaces), so every V1 observation creates a **new**
  `locationId`. The `project_file_id` association is retained through the
  qualified-identity evidence rules (§3/publication planner): an unambiguous
  identity match reuses the prior physical record; ambiguous or absent
  identity mints a fresh one. Legacy rows unobserved by the V1 scan become
  `missing` through the normal unseen-present rule. Detached-root history is
  retained. In short: **locationIds are intentionally not retained across the
  v4→V1 key-space break; project associations are retained via identity.**

The timestamp guards, identity backfill filter, and index replacements for
this migration are specified with the migration (§3, migration file header).

Agent 2 handoff: expose `LocatorKeyV1` in the public `Observation`; do not
pass the case-preserving `NormalizedLocator` string as the storage key. Make
the Windows `SystemTime` to Unix-nanosecond conversion checked rather than
saturating. Define the byte order used when converting `FILE_ID_128` to
`u128` and use that same encoding everywhere. Exact lines in §6.

## 2. Timestamp precision and checked conversions

Filesystem change-detection timestamps are signed nanoseconds. The Rust
enumeration boundary supplies `i128`; storage performs `i64::try_from` and
rejects an out-of-range observation as terminal `staging_rejected`. SQLite
stores the checked value in a signed `INTEGER` `modified_at_ns` column. The
valid storage range is `i64::MIN..=i64::MAX` nanoseconds.

If the native observation DTO is serialized, `byte_size`, the source
`modified_at_ns`, `volumeSerial`, and `fileId` are canonical decimal strings;
no JSON number is used for a value that could exceed JavaScript's
exact-integer range.

### 2.1 Canonical decimal strings (ABNF)

```abnf
decimal-string        = "0" / (%x31-39 *%x30-39)
                      ; unsigned: no leading zeros except "0" itself
signed-decimal-string = ["-"] decimal-string
```

- `byte_size`: `decimal-string`, range 0..2⁶⁴-1 (Rust `u64`).
- Source `modified_at_ns`: `signed-decimal-string`, range
  -(2¹²⁷)..2¹²⁷-1 (Rust `i128`).
- `volumeSerial`: `decimal-string`, range 0..2⁶⁴-1 (Rust `u64`).
- `fileId`: `decimal-string`, range 0..2¹²⁸-1 (Rust `u128`).
- Rejected: leading zeros (`"01"`), explicit signs on unsigned values
  (`"+1"`, `"-1"` for `byte_size`/`volumeSerial`/`fileId`), `+` on any value
  (`"+1"`), whitespace, non-decimal digits, empty strings, JSON numbers,
  floats, and out-of-range magnitudes. Deserialization enforces-canonical or
  fails; storage never coerces.

The v4 millisecond columns are migration compatibility only. Migration 005
multiplies them by 1,000,000 under an aborting bounds guard; an out-of-range
legacy value fails the migration instead of being clamped or silently
truncated. Publication and change detection read `modified_at_ns` only.

`byte_size` is Rust `u64`, but the SQLite boundary accepts only
`0..=i64::MAX`. Values above that range reject the batch with
`staging_rejected`. The storage DTO serializes exact byte sizes and
nanosecond values as decimal strings when they cross JSON; the TypeScript
contract must not use JavaScript `number` for either field. The UI-facing
`modifiedAt` is UTC RFC 3339 (`YYYY-MM-DDTHH:MM:SS[.fffffffff]Z`) with zero
to nine fractional digits, produced by the native adapter from the stored
`i64` nanoseconds without dropping precision.

## 3. Identity encoding and bounds

The source identity is qualified only by Agent 2 for the agreed local NTFS
scope. It is a non-unique lookup signal; hardlink aliases remain separate
locations.

```rust
struct EncodedIdentity {
    volume_serial: String, // canonical decimal encoding of Rust u64
    file_id: String,       // canonical decimal encoding of Rust u128
}
```

The storage pair is all-or-none, canonical unsigned decimal (§2.1), with no
leading zeroes except `"0"`. SQLite stores it as nullable `TEXT` columns
`identity_volume_serial` and `identity_file_id`; it does not use a signed
`INTEGER` for either value. Rust checks `u64`/`u128` bounds before writing and
decodes the pair with checked parsing. TypeScript receives the same fields as
decimal strings, never numbers.

```ts
type DecimalString = string; // ABNF decimal-string, §2.1

interface EncodedIdentity {
  volumeSerial: DecimalString; // 0..2^64-1
  fileId: DecimalString;       // 0..2^128-1
}
```

Negative, signed (`+1`), nondecimal, leading-zero, overflow, and partial pairs
are invalid. Staging validation rejects them with `staging_rejected` before
any write. Invalid historical v4 identity text is treated as unavailable
evidence during migration; it is never guessed into a new identity. The
migration backfill copies only rows with a non-negative legacy `volume_id`
and a canonical decimal `filesystem_file_id` within `u128` range; every other
row keeps `NULL` identity columns.

## 4. Jobs, leased attempts, and cancellation

A queued request has a durable `jobId`. Leasing creates a distinct `runId` for
that attempt. Retries keep the job/retry-chain identity and allocate a fresh
run ID. A queued request therefore has no run ID yet.

The storage-side shapes are already separated:

```rust
EnqueueResult { job_id, coalesced, follow_up_requested }
LeasedScan { job: ScanJob { id: job_id, ... }, run: ScanRun { id: run_id, scan_job_id: job_id, ... } }
```

The UI/native adapter must use the following external meaning:

```ts
interface ScanStartResult {
  rootId: string;
  jobId: string;
  runId: string | null; // null while queued; set only after leasing
  outcome: "queued" | "already_queued" | "already_running";
}

interface ScanStatus {
  root: ScanRoot;
  jobId: string | null;
  runId: string | null;
  state: "idle" | "queued" | "running" | "completed" |
    "cancelled" | "failed" | "interrupted";
  cancellationRequested: boolean;
  retryAvailable: boolean;
}
```

Queued cancellation addresses `jobId`: in one transaction it marks the job
cancelled and no run or stage is created. After leasing, cancellation addresses
the job/run attempt, durably sets both cancellation flags, discards open
staging, and returns a cancellation-requested/running outcome until the worker
records terminal `cancelled`. Worker staging, lease renewal, publication, and
run-level fencing use `runId + leaseToken`.

SQLite commit order defines the race: cancellation committed before final
publication prevents publication; publication committed first remains
completed and a late cancellation reports that outcome without rollback.
User cancellation is not an automatic retry. `retryAvailable` is true only for
an enabled failed job whose durable attempt count is below `max_attempts`;
otherwise the client uses `scanNow(rootId)`. That explicit action creates a
fresh job and retry chain for cancelled or exhausted work. It never revives the
terminal job or silently resets its automatic retry budget. Disabled or
removed roots remain rejected by the native enqueue contract, including when
the root changes between rendering and the action.

## 5. Smallest coherent Library query

This round deliberately chooses **one root per query**. A global page would
need a single cross-root committed snapshot; silently concatenating per-root
snapshots would permit duplicates/skips and is prohibited. A future global
query must introduce an explicit global snapshot contract.

Native storage types:

```rust
LibraryQuery {
    scan_root_id: String,
    page_size: usize,                 // 1..=200
    cursor: Option<LibraryCursor>,
    snapshot: Option<LibrarySnapshot>,
}

LibraryCursor {
    scan_root_id: String,
    snapshot: LibrarySnapshot,
    locator_key: String,              // LocatorKeyV1
    location_id: String,
}

LibraryPage {
    scan_root_id: String,
    locations: Vec<PublishedLocation>,
    next_cursor: Option<LibraryCursor>,
    snapshot: LibrarySnapshot,
    has_more: bool,
}
```

The native IPC adapter serializes `LibraryCursor` opaquely. Agent 3 should use:

```ts
interface LibraryPageRequest {
  rootId: string;
  limit: number;          // native rejects outside 1..=200 with storage_invalid_schema
  cursor: string | null;  // null starts the current committed root snapshot
}

interface PublishedFileLocation {
  locationId: string;
  rootId: string;
  rootDisplayName: string;
  rootCanonicalPath: string; // management display only
  fileName: string;
  relativePath: string;      // display spelling, not a key
  byteSize: DecimalString;   // §2.1, never number
  modifiedAt: string;        // UTC RFC 3339, nanosecond precision retained
  presence: "present" | "missing";
}

interface LibraryPage {
  rootId: string;
  snapshotId: string;         // opaque committed root snapshot identity
  records: readonly PublishedFileLocation[];
  nextCursor: string | null;
}
```

The cursor binds root, committed snapshot, locator key, and location ID. Its
expiry is snapshot-bound, not wall-clock-bound: queued/running/failed/cancelled
work does not expire it, and publication in another root does not expire it.
Malformed, structurally inconsistent, or wrong-root cursors return the fixed
`invalid_cursor` error. A valid cursor whose root snapshot has changed returns
`stale_cursor`; the client discards it and requests `cursor: null`. A first
page with a stale explicit snapshot also returns `stale_cursor`. No page may
combine rows from two committed snapshots.

## 6. Handoff: exact lines Agent 2 and Agent 3 must change

The storage owner does not edit other agents' branches. The table below is the
complete review checklist; Agent 2/3 revisions must cite it.

| # | Owner | File (their branch) | Exact location | Required change |
| - | ----- | ------------------- | -------------- | --------------- |
| H1 | Agent 2 | `crates/filesystem-enumeration/src/lib.rs` (branch `feat/36-filesystem-enumeration`) | line 40 `pub struct NormalizedLocator(String)` + line 467 `pub locator: NormalizedLocator` in `Observation` | Publish a versioned `LocatorKeyV1` string (§1.1, `v1:` + per-component `i:`/`s:` tags, NFC + case-fold, percent-encoded) as the observation key. Do not pass the case-preserving `NormalizedLocator` string as the storage key; storage rejects keys without the `v1:` envelope. |
| H2 | Agent 2 | same file | line 148 `struct LocatorKey(Vec<LocatorComponent>)` (private ordering struct) | Keep private for traversal ordering, but serialize only the §1.1 `LocatorKeyV1` string across the boundary. The private struct must never cross into storage DTOs or logs. |
| H3 | Agent 2 | same file | line 272 `pub const fn windows_filetime_100ns_to_unix_ns` | Make the `SystemTime`/FILETIME→Unix-ns conversion checked (return `Option`/`Result`, reject on overflow) instead of saturating/wrapping const arithmetic; document that storage performs the final `i128→i64` check and rejects with `staging_rejected`. |
| H4 | Agent 2 | same file | lines 277–278 `windows_file_id_to_u128` / `u128::from_le_bytes` | Define the `FILE_ID_128`→`u128` byte order once (currently little-endian) and use that same encoding in every producer and test; cite this contract revision. |
| H5 | Agent 3 | `docs/review/phase-2-library/CONTRACT_PROPOSAL.md` (branch `feat/phase-2-library-scan-ui`) | "Combined bounded Library query" section (global `rootId`+`relativePath`+`locationId` ordering) | Replace with the §5 per-root query. This contract revision supersedes that section for this round; no global page ships without an explicit cross-root snapshot contract. |
| H6 | Agent 3 | `apps/client/src/library/contracts.ts` | line 20 `readonly byteSize: number` | Change to `readonly byteSize: DecimalString` (§2.1); never use `number` for byte sizes, nanoseconds, `volumeSerial`, or `fileId`. Add `DecimalString` + `EncodedIdentity` types. |
| H7 | Agent 3 | same file | lines 25–30 `LibraryPageRequest` (`snapshotId`, `cursor`, `limit`, no root) | Change to per-root `{ rootId, limit, cursor }` (§5); cursor is opaque and snapshot-bound per root. |
| H8 | Agent 3 | `apps/client/src/library/fake.ts` | lines 62–68 `orderRecords` (`localeCompare` global root/path/location ordering) | Change the fake to per-root `BINARY`-equivalent ordering (`locatorKey` bytes, then `locationId`) and per-root snapshots; the global `localeCompare` order must not ship as native behavior. |
| H9 | Agent 3 | same file | lines 83–85 `readCursor` + 212–219 page slicing | Bind fake cursors to (root, snapshot, key, location) with `invalid_cursor`/`stale_cursor` semantics per §5 instead of bare numeric offsets. |

## Acceptance tests

The storage boundary must retain these regressions:

1. Case-only rename preserves `locationId`, project association, and identity
   while changing only the display path.
2. A case-sensitive pair has two locations and deterministic binary-key order.
3. Duplicate locator keys reject a run without partial publication.
4. Adjacent nanosecond mtimes remain distinguishable; minimum/maximum checked
   values round-trip; out-of-range timestamps reject without changing rows.
5. `u64::MAX`, `u128::MAX`, and the maximum supported SQLite byte size round
   trip exactly as decimal strings.
6. Invalid identity encodings and partial pairs reject before publication.
7. Non-canonical JSON numerics (numbers, leading zeros, `+` signs) reject;
   canonical decimal strings round-trip.
8. Keys without the `v1:` envelope, non-ASCII keys, NUL keys, and over-length
   keys reject without changing rows.
9. A cursor from root A cannot query root B; per-root pages never mix roots.
10. Publication during pagination returns `stale_cursor`; restart from null
    returns a complete new-snapshot page.
11. Publication in another root leaves the original root cursor valid.
12. Queued cancellation creates no run; leased cancellation prevents final
    publication; publication-before-cancellation remains completed.
13. Pages never expose open staging or partial publication.
14. Populated v4 database migrates (legacy keys quarantined, timestamps and
    identities backfilled under guards), then a first V1 scan with a
    case-only rename plus Unicode retains project association via identity
    while minting new locationIds and marking unobserved legacy rows missing.
15. No test enables traversal, parser/content reads, source mutation, watcher
    activation, or WAL.

Implemented in #62's storage suite:
`integration_locator_keys_preserve_case_renames_and_case_sensitive_aliases`,
`integration_identity_timestamp_and_numeric_bounds_are_checked`,
`integration_canonical_decimal_and_locator_key_shape_are_enforced`,
`integration_library_pages_are_root_scoped_and_snapshot_bound`,
`integration_cancellation_uses_job_before_lease_and_run_after_lease`,
`integration_staging_is_never_visible_to_library_reads`,
`migration_quarantines_legacy_keys_and_first_v1_scan_retains_projects`,
plus the existing atomic publication/recovery tests. The next agents must add
their own boundary/adapter tests against these exact semantics before
production scan controls are enabled.

# Durable staging and atomic publication (#40)

This storage slice adds migration 004 for run-scoped staging and the first
device-local Library read model. Migration 005 records the shared integration
contract for boundary-owned locator keys, nanosecond metadata, bounded decimal
identities, and root-scoped snapshot pagination. It remains below the scanner
boundary: callers provide already validated metadata observations, while
filesystem traversal, watcher events, FLP parsing, and renderer controls stay
in their owning issues. WAL remains disabled and this crate performs no
filesystem I/O. See
[`PHASE_2_INTEGRATION_CONTRACT.md`](PHASE_2_INTEGRATION_CONTRACT.md) for the
cross-agent handoff and exact IPC shape.

## Observation and identity contract

`ScanObservation` carries a boundary-owned `locator_key`, a separate
display-preserving relative path, byte size, nanosecond modified time, and an
optional qualified physical identity. The identity is an all-or-none canonical
decimal `u64`/`u128` pair stored as SQLite `TEXT`; a missing or partial identity
is continuity uncertainty and never proves a rename or logical project match.
Storage never derives a comparison key by lowercasing the display path. The
key must already encode Windows case-sensitive-directory semantics.
The native enumerator owns Windows normalization and must reject paths outside
the root before calling storage.

Publication first loads the complete staged observation set in locator-key
order and plans every association before changing committed rows. A qualified
identity can reuse only current evidence: a present prior location supported by
the complete current set, or an unobserved present location that is an
unambiguous rename source. Historical `missing` locations are never identity
lookup candidates. Exact-path continuity is handled separately so restoring a
missing path remains reversible without making a different new path inherit its
historical `project_file_id`.

This preserves independent hardlink locations, surviving aliases, same-scan
rename evidence, and conservative replacement behavior. Conflicting prior
associations produce a fresh physical record rather than choosing by traversal
or SQL row order. Stage rows are locator-key ordered, so caller batch/input
order cannot affect the association plan.

## Durable completion and failure protocol

A worker calls `begin_scan_staging`, appends bounded batches with
`stage_scan_observations`, and calls `publish_scan_run` only after an
authoritative enumeration has completed. `begin_scan_staging` followed by
`publish_scan_run` with no observations is the explicit successful empty scan.
Publication validates the active session, lease token and deadline, running
job, enabled root, captured generation, configuration revision, cancellation
flags, and follow-up invalidation inside one immediate SQLite transaction.

Terminal staging input errors include invalid observations, duplicate paths in
one batch, a changed duplicate path across batches, and record/path quota
exhaustion. They return `StorageError::StagingRejected`, durably mark the run
and job failed with the fixed `staging_rejected` code, mark the stage discarded,
and delete staged observations in the same transaction. A later publish or
stage call for that run is fenced, so earlier successful batches cannot be
published as an incomplete run. An explicit retry uses a fresh run/stage.

`StorageError::Busy` and unexpected SQLite `Database` failures are retryable at
the staging boundary: the transaction rolls back and the open stage remains
unchanged, allowing the same batch to be retried. Lease, cancellation,
follow-up, restart, root-disable, and root-removal conflicts are execution
fences, not successful completion; the execution owner discards their stage or
requeues a fresh attempt according to the durable queue contract.

On successful publication, every observed location is applied, only eligible
unseen present locations become `missing`, the root success marker advances,
the run/job complete, and the stage is retired atomically. Any validation or
SQL failure rolls back the entire publication, preserving the prior committed
dataset and marker. Backup recovery and restart discard open stages before
resuming eligible work.

## Bounded Library query for the client agent

The typed storage boundary exposes `Database::query_library(LibraryQuery)`;
the existing unbounded fixture/maintenance readers are not IPC surfaces.

```text
LibraryQuery {
  scanRootId: string,
  pageSize: 1..=200,
  cursor: { scanRootId, snapshot, locatorKey, locationId } | null,
  snapshot: { lastSuccessfulRunId, lastSuccessfulGeneration,
               lastSuccessfulAtMs } | null
}

LibraryPage {
  scanRootId,
  locations: PublishedLocation[],  // at most 200
  nextCursor: { scanRootId, snapshot, locatorKey, locationId } | null,
  snapshot,
  hasMore
}
```

This is deliberately one root per query; there is no implicit global page over
multiple root snapshots. Rows have stable `(locator_key COLLATE BINARY, id)`
ordering, and display paths are not keys. Locator keys are `LocatorKeyV1`
values (`v1:` envelope with per-component case-mode tags); the staging
boundary rejects unversioned, non-ASCII, NUL, and over-length keys. The first
request omits `snapshot`;
subsequent requests echo the returned snapshot and cursor. A publication in
the same root returns `stale_cursor`; malformed or wrong-root cursors return
`invalid_cursor`. Work that has not published does not expire a cursor, and a
publication in another root leaves it valid. The client restarts from page one
with `cursor: null` after expiry. The IPC adapter should map only these typed
fields and fixed safe errors; it must not expose SQL, absolute paths, or an
unbounded list.

Migration 005 quarantines v4 paths instead of promoting them: every legacy
row keeps `normalized_path` untouched and receives
`locator_key = 'v0:legacy:' || normalized_path`, a namespace that can never
equal a `v1:` key. The first new-format scan therefore mints new locationIds
while retaining `project_file_id` associations through qualified-identity
evidence; unobserved legacy rows become `missing` through the normal rule.
See contract §1.3 for the full policy.

## Evidence

Storage tests cover staging invisibility, explicit empty publication,
historical identity reuse, same-scan rename/replacement, surviving hardlink
aliases, input-order independence, conflicting duplicates across batches,
resource exhaustion after earlier batches, terminal publication rejection,
cancellation, follow-up invalidation, stale leases, restart and backup
recovery, and atomic rollback preserving the prior dataset. Integration tests
also cover case-only renames, case-sensitive locator keys, nanosecond
precision, decimal identity bounds, canonical JSON numerics, key-shape
rejection, duplicate keys, queued/leased cancellation, malformed
cursors, root scope, staging invisibility, publication during pagination, and
snapshot invalidation. The v4-to-V1 quarantine migration is covered by a
populated-fixture regression (legacy keys quarantined, timestamps and
identities backfilled under guards, first V1 scan retains projects).

The storage crate still does no traversal, watcher activation, parser/content
read, renderer scan activation, or source-file mutation. DriveFS, cross-volume,
and FAT32 identity support remain unverified under #47/#48.

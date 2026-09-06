# Durable staging and atomic publication (#40)

This storage slice adds migration 004 for run-scoped staging and the first
device-local Library read model. It is deliberately below the scanner
boundary: callers provide already normalized metadata observations, while
filesystem traversal, watcher events, FLP parsing, and renderer controls stay
in their owning issues. WAL remains disabled and this crate performs no
filesystem I/O.

## Observation and identity contract

`ScanObservation` carries a normalized root-relative locator, relative display
locator, byte size, modified time, and an optional qualified physical identity.
The identity is qualified only when both `volume_id` and
`filesystem_file_id` are present and valid. A missing or partial identity is
continuity uncertainty; it never proves a rename or a logical project match.
The native enumerator owns Windows normalization and must reject paths outside
the root before calling storage.

Publication first loads the complete staged observation set in normalized-path
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
or SQL row order. Stage rows are normalized-path ordered, so caller batch/input
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
  cursor: { normalizedPath, locationId } | null,
  snapshot: { lastSuccessfulRunId, lastSuccessfulGeneration,
               lastSuccessfulAtMs } | null
}

LibraryPage {
  scanRootId,
  locations: PublishedLocation[],  // at most 200
  nextCursor: { normalizedPath, locationId } | null,
  snapshot,
  hasMore
}
```

Rows have stable `(normalized_path, id)` ordering. The first request omits
`snapshot`; subsequent requests echo the returned snapshot and cursor. A
publication between pages returns a safe conflict, and the client restarts
from page one. The IPC adapter should map only these typed fields and fixed
safe errors; it must not expose SQL, absolute paths, or an unbounded list.

## Evidence

Storage tests cover staging invisibility, explicit empty publication,
historical identity reuse, same-scan rename/replacement, surviving hardlink
aliases, input-order independence, conflicting duplicates across batches,
resource exhaustion after earlier batches, terminal publication rejection,
cancellation, follow-up invalidation, stale leases, restart and backup
recovery, and atomic rollback preserving the prior dataset. Pagination tests
cover the 200-row bound, cursor order, and snapshot invalidation.

The storage crate still does no traversal, watcher activation, parser/content
read, renderer scan activation, or source-file mutation. DriveFS, cross-volume,
and FAT32 identity support remain unverified under #47/#48.

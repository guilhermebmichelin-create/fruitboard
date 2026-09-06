# Durable staging and atomic publication (#40)

This storage slice adds migration 004 for run-scoped staging and the first
device-local Library read model. It is deliberately below the scanner
boundary: callers provide already normalized metadata observations, while
filesystem traversal, watcher events, FLP parsing, and renderer controls stay
in their owning issues.

A worker calls `begin_scan_staging`, appends bounded batches with
`stage_scan_observations`, and calls `publish_scan_run` only after an
authoritative enumeration has completed. Staged rows are keyed by run ID and
remain outside `file_location` until publication. An empty stage is explicit
and represents a successfully enumerated empty root. A bare
`finish_scan_run(...Completed)` cannot write a completed ledger row; when an
open stage exists it delegates to the same publication transaction.

Publication validates the active session, lease token and deadline, running
job, enabled root, captured generation, configuration revision, cancellation
flags, and follow-up invalidation inside one immediate SQLite transaction. It
then applies every observed location, marks only eligible unseen active paths
missing, updates the per-root last-success marker, completes the run and job,
and retires the stage. Any validation or SQL failure rolls back all of those
changes together, leaving the previous committed dataset and marker intact.
The traversal boundary remains responsible for withholding publication when an
enumeration is offline, denied, partial, or resource-limited; those outcomes
use the non-authoritative execution path and discard their stage.

Locations are per-path rows. Qualified `(volume_id, filesystem_file_id)` values
are an indexed, non-unique lookup so hardlink aliases can share one
`project_file` while retaining independent presence. A missing path remains
visible and is restored by a later observation. Identity changes on a stable
path create a new physical file record; unavailable identity is treated as
continuity uncertainty and never invents a rename. Removing a root first
discards its open staging and detaches its locations, retaining history without
letting a newly added root reuse the old identity.

The starting budgets are 512 observations per batch, 10,000 staged records per
run, and four MiB of aggregate normalized and relative path bytes. Exceeding a
budget rejects the whole batch before it becomes visible. These are safety
limits, not the provisional performance targets in the execution plan; the
streaming enumerator and benchmark evidence remain open.

Tests cover staging invisibility, empty authoritative publication, missing and
restored aliases, cancellation and disable fencing, stale workers, rollback
from an injected apply failure, backup recovery discarding open staging, root
removal detachment, and both record and path budgets. The storage crate still
does no filesystem I/O and does not expose production scanning.

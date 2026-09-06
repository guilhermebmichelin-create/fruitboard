# Durable scan execution foundation (#38)

This slice adds the device-local execution ledger behind the accepted Phase 2
contracts. Migration 003 preserves the existing startup preference and scan
roots, then adds root configuration revisions/generations, process sessions,
deduplicated jobs, and leased runs. The storage crate remains the only owner of
the SQLite connection; no renderer command or filesystem traversal is added.

## State and fencing

Each enabled root has a monotonically increasing configuration revision and
generation. Disabling a root cancels queued/running work in the same
transaction and advances both values. Re-enabling advances them again, so an
old run cannot publish into the new generation. Removal cancels work before
deleting the configuration row; job/run history retains the former root ID.
Re-adding the same canonical path receives a new root ID and starts at revision
and generation zero.

Jobs have a persisted retry chain, attempt count, retry budget, not-before time,
and one active slot per root. This slice leases one run globally at a time;
parallel worker policy can be widened only with a later measured review.
Repeated triggers coalesce. A trigger received
while a run is active records one follow-up request. The current attempt then
becomes interrupted and non-authoritative when it finishes, and one fresh
queued job is created for the follow-up. Lease acquisition creates a new run
ID and opaque token, increments the root generation in the same transaction,
captures the resulting generation/revision, and records the process session.
Every retry and follow-up therefore has a distinct generation. Renewal and
completion require the exact session/token, an unexpired lease, an enabled
root, matching revisions, and a running job. Expired leases become interrupted
and are requeued with bounded 1/2/4-second backoff until the persisted budget
is exhausted.

Cancellation is durable. A running cancellation request remains observable by
the worker; completion after that request is recorded as cancelled. A worker's
explicit `cancelled` outcome has the same precedence, including when a
follow-up trigger is already pending, so it cannot schedule implicit work. A
completion committed first remains completed when a later cancellation arrives.
Disable/removal invalidation is terminal and fences a stale worker immediately.

Starting a new process session ends the prior session and fences all prior
running attempts. An interrupted attempt keeps its job ID, retry-chain ID,
attempt count, remaining budget, and backoff; the job is requeued only when it
still has budget. A root without eligible interrupted or queued work may
receive one deduplicated recovery job, while cancelled or exhausted work is not
replaced implicitly. Exhaustion is determined from the durable terminal state
and attempt budget; diagnostic error codes do not affect retry eligibility. A
previous-session token cannot renew or finish, even if its wall-clock deadline
has not elapsed.
Recovery from a backup applies the same fence and retry-chain preservation to
in-flight runs before the recovered database is published.

## Evidence and boundary

The storage tests use fixed timestamps and synthetic paths. They cover forward
migration preservation and rollback, duplicate enqueueing, follow-up
invalidation, fresh generations for successive and retried attempts, lease
expiry and retry persistence across reopen, active-session restart and backup
recovery without retry-budget reset, replaced tokens and sessions,
cancellation precedence and commit ordering, disable/removal invalidation,
terminal-attempt immutability, fresh identity after re-adding a path, and
table-driven finish/restart/backup transition matrices. The existing
backup/recovery tests continue to run against migration 003 (48 storage tests).

This is an execution and recovery foundation, not a scanner. It does not walk
the filesystem, read FLP contents, stage observations, publish a Library
dataset, maintain missing/restored location history, or expose scan controls.
Run-scoped staging and atomic publication are the next #40 slice. Bounded
enumeration remains #36; watcher hints remain #37. Keep WAL disabled, PyFLP
absent, and DriveFS/non-NTFS support unverified under #47/#48.

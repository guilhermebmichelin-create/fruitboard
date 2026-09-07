# Integrated durable scan worker (#36/#38/#40 composition)

`crates/scan-execution` composes the three landed Phase 2 scanner foundations
into one bounded manual-scan journey: durable queueing, leasing and recovery
from `fruitboard-storage`, bounded metadata traversal from
`fruitboard-filesystem-enumeration`, and the deterministic per-path decision
core from `fruitboard-reconciliation`. The worker is still completely hidden
from the renderer: there is no Tauri dependency, no IPC surface, no renderer
command, and no production scan entry point. The future desktop host owns the
single worker instance, the started process session and the poll loop.

## Watcher follow-up wiring (#37, P2-09 storage half)

`WatcherFollowUpAdapter` is the compile-time seam that connects the watcher's
coalesced hints and overflow signals to the durable follow-up queue:

- Hints are scheduling observations, never authority: a hint only requests a
  reconciliation, and the durable queue decides how that request coalesces.
  A burst inside one coalescing window collapses to at most one hint per
  root, and repeated hints never grow the queue — storage's `enqueue_scan`
  dedup is the authority (`ScanKind::Periodic` requests).
- A coverage-loss (overflow) signal produces the same single full-reconciliation
  follow-up with the overflow cause recorded in the request's safe metadata
  (root id, job id, cause). Overflow is never absence evidence.
- Generation fencing drops hints whose `(root, generation)` is older than the
  current watch generation; `watch_ended` makes replayed hints from a stopped
  watch inert until a strictly newer generation starts a fresh watch (#69).
- Suppression lives in storage: disabled roots refuse requests (counted as
  suppressed), removed roots drop hints (mapping or `NotFound`), and roots
  with active running work coalesce the hint onto the running attempt's
  follow-up flag. Cancelled chains are never revived.
- Privacy: only opaque root ids, job ids and counters cross this boundary;
  no path data exists in the adapter's public types.

The adapter is not wired into any host: the desktop host owns watcher
lifecycle, the watcher→storage root mapping (`RootIdMapping`), the poll loop
and activation behind its console flag. `tests.rs` provides a compiling fake
consumer proving the exact trait shape the host binds.

## Worker loop contract

- One worker globally. Storage refuses to lease while any run is running and
  coalesces repeated triggers into at most one queued follow-up per root
  (`enqueue_scan` dedup APIs; `request_manual_scan` is a thin wrapper).
- A 30 s lease renewed every 5 s at the batch fence inside the staging
  adapter; a failed renewal, a lost lease, or a durable invalidation aborts
  the traversal immediately and the run ends non-authoritative.
- The durable cancellation/invalidation flags (run, job, queued follow-up)
  are checked between batches and again before publication; publication
  itself revalidates generation, revision, lease token, session and
  cancellation inside its transaction.
- Disable/remove invalidates work in one configuration transaction; the
  worker observes it at the next fence check and stops without publishing.
  Re-enabling runs a fresh generation.
- Automatic retries: three per chain at 1/2/4 s backoff plus deterministic
  jitter of at most 20% (`retry_eligible_at` is derived from the immutable
  job ID). Attempts persist on the durable job row, so restarts never reset
  the budget; cancelled and exhausted chains are never revived.
- Startup recovery (`start_session`): prior-session running leases become
  interrupted, their staging is discarded, and enabled roots without eligible
  interrupted/queued work receive exactly one deduplicated recovery job.

## Authoritative-completion invariant

Only a run whose enumeration reported `Complete` while every fence still
holds reaches `publish_scan_run`. Every other outcome — denied, offline,
partial, cancelled, resource-limited, sink failure, stale lease, invalidation
— discards the staging through `finish_scan_run` and leaves committed Library
rows untouched. After a `Complete` run the worker also classifies the
observed changes against the committed rows with `reconcile` and reports the
advisory `ChangeSummary`; classification never gates or bypasses the durable
publication.

## Tests and evidence

Run the crate gates:

```text
cargo fmt --all -- --check
cargo clippy -p fruitboard-scan-execution --all-targets --locked -- -D warnings
cargo test -p fruitboard-scan-execution --locked
```

Thirty-six deterministic tests (injected clock, scripted fake port, no
wall-clock sleeps) cover: idempotent clean deltas; add/modify; rename,
missing and restore; denial mid-traversal; offline roots with the persisted
retry chain; durable cancellation between batches and before apply; local
cooperative cancellation; follow-up invalidation during staging; disable and
remove while queued/running; injected SQL failure rolling back publication;
lease expiry and lease replacement fencing stale workers; restart recovery
with persisted attempt budgets; deduplicated recovery scans; trigger
coalescing; quota-limited traversal; empty roots marking previous rows
missing; both cancel/commit orderings; and the watcher follow-up wiring
(burst coalescing per window, overflow scheduling with recorded cause, stale
generation drops, watch-ended replay discard, disabled/removed/running
suppression, cancelled-chain semantics, idempotent replay, watcher restart
with a fresh generation, the fake-consumer host-loop shape, and the
no-path-data privacy regression). Each fault case asserts no publication,
discarded staging, and unchanged committed rows.

Two real-NTFS tempdir fixture tests (authoritative success and a denied
subtree via `icacls`) run on Windows; the denied-subtree fixture is
environment-dependent and fails loudly if the ACL cannot be applied. One
`#[ignore]`d test records the scale/perf budget measurement plan (10,000
records, 512-record batches, 256 MiB staging, <=128 MiB working memory)
against the #41 synthetic-tree benchmark harness.

## Explicit non-goals

No content reads, hashing, hydration, or FLP parsing; no renderer/IPC
exposure; no host wiring of the watcher adapter (watcher lifecycle, root
mapping and activation stay in the desktop host behind its console flag, per
#37); no DriveFS, FAT32, or non-NTFS qualification (#47/#48 remain open); no
benchmark claims (#41). Production scanning stays hidden until P2-03 through
P2-08 have integrated evidence.

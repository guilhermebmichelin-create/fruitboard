# Handle-bound filesystem watcher foundation (#37)

This isolated Rust crate watches one configured root directory with a
handle-bound `ReadDirectoryChangesW` watch on Windows and turns raw
notifications into bounded, coalesced, non-authoritative hints. It has no
parser, no SQLite connection, no content reads, no hydration, no logging, no
storage follow-up wiring, and no production scan entry point. The only
consumer seam is the `WatcherPort` trait; connecting it to durable
reconciliation follow-ups is the next slice. It implements the
[accepted scanner contracts](https://github.com/guilhermebmichelin-create/fruitboard/blob/main/docs/PHASE_2_EXECUTION_PLAN.md)
watcher slice, not the integrated scanner.

## Hints are never authority

`WatcherPort::poll_hints` produces at most one hint per root per poll, in
deterministic order:

- `ReconciliationRequested`: activity was observed under the root. This is a
  scheduling hint only; the durable reconciliation that follows remains the
  sole authority for presence, absence, and changes.
- `CoverageLost`: events were missed (OS notification-buffer overflow, raw
  queue drop with a counter, truncated notification batch) or the watch
  ended. Consumers must turn this into a full, NON-AUTHORITATIVE
  reconciliation follow-up. Missed events never imply absence (P2-09
  invariant), and a `RootLost` outcome never proves any file is missing.

`WatcherPort::outcome` reports the typed lifecycle result (`Stopped`,
`RootLost`, `WatchFailed` with an opaque OS status code). `stop()` is
idempotent: it signals the starter-owned stop event (which stays valid and
is closed only after the worker is joined, so stopping an already-exited
worker is safe), cancels its own pending read, and joins the
thread; a restart opens a fresh handle and must use a fresh caller-supplied
generation that is strictly greater than the previous generation for the
same root, which replaces all pending state for that root (debug builds
assert this monotonicity). Downstream consumers key hints by
`(root, generation)` and must discard stale-generation signals. The platform
worker signals "armed" before `start()` returns, so every change made after
`start()` is observed. If the worker exits before arming, `start()` reports
`RootUnavailable` with the worker-captured OS code preserved (never
`ResourceUnavailable{0}` for a lost root).

## Bounds and drop policy

- Coalescing uses a fixed, non-extending window (default 1 s in caller
  timestamp units): a burst collapses into at most one reconciliation hint
  per root per window. The coalescer is pure and fake-clock injectable.
- The coalescer tracks at most 8 roots per watcher (one root plus restart
  bookkeeping); signals beyond the bound are rejected and counted, never
  queued.
- The raw event queue is bounded (default 256, clamped at start). A full
  queue drops the event, counts it, and raises the sticky coverage-loss
  signal. The same signal covers OS buffer overflows (`ERROR_MORE_DATA` /
  `ERROR_NOTIFY_ENUM_DIR`, partial batches dropped and counted) and
  truncated notification chains.
- The OS notification buffer is clamped to [4096, 1 MiB]; the root presence
  re-check interval is clamped to [50 ms, 10 s]. Smaller values only bound
  detection latency, never correctness.

## Reparse points: policy exclusions, distinct from I/O failures

A reparse-point root (junction or symlink) observed at start is refused with
`StartError::ReparseRootExcluded` — a policy exclusion consistent with the
enumeration contract, deliberately separate from the I/O failure
classifications (`RootUnavailable`, `NotADirectory`). Limitation tied to
#47/#48: the pre-check races with the handle open
(`GetFileAttributesW`-to-`CreateFileW` window, see `platform.rs`
TODO(#47)), parent-directory junctions are followed by the OS open, and the
OS may still report activity beneath reparse directories inside the watched
subtree. No traversal safety is claimed: the watcher performs no per-event
I/O, such reports remain hints that cannot override the enumeration
boundary's reparse exclusions, and enforcement stays with the authoritative
enumeration. A future hardening is to open with
`FILE_FLAG_OPEN_REPARSE_POINT` plus a post-open reparse verify to close the
final-component window.

## Privacy

Notification names are reduced to validated root-relative path bytes plus an
action code. Absolute-shaped, traversal-shaped, or otherwise invalid bytes
are never surfaced: they become counted `Obscured` activity. No public type
can carry an absolute path, `Debug` output redacts path bytes, errors never
echo the root path, and the crate performs no logging. Watcher events are
hints only and never decide file state.

## Explicitly unverified

DriveFS (#47), non-NTFS volumes (#48), network-share roots, ACL revocation
mid-watch, and real OS buffer-overflow timing are not qualified by this
crate. The corresponding `#[ignore]`d test fixtures are labeled unverified
and must never be counted as support; `RootLost` treats an unreachability
observation honestly without claiming recovery behavior for those
environments.

## Incremental #41 evidence

Run `cargo test -p fruitboard-filesystem-watcher --locked` and
`cargo clippy -p fruitboard-filesystem-watcher --all-targets --locked -- -D
warnings`. The `filesystem-watcher-windows` CI job runs both plus
`cargo fmt --all -- --check`, and the workspace Windows gate includes the
crate. Thirty-one deterministic tests cover coalescing determinism under a
fake clock, burst collapse and at-most-one-per-window invariants, immediate
coverage-loss delivery with stale-generation rules, tracking-bound
rejection, relative-path validation (absolute, traversal, ADS, UTF-8, and
length shapes), defensive notification-chain parsing with truncation,
privacy redaction, the drop-with-counter queue policy, live delivery and
coalescing against a real NTFS temp tree, typed rename/deletion lifecycle,
policy exclusions, idempotent stop with join, stop-after-worker-exit safety,
pre-arm root-loss classification with preserved OS codes, terminal-reason
mapping (including the sharing-violation audit), and fresh handle/generation
restarts. Four fixtures are `#[ignore]`d as unverified (network, DriveFS,
ACL, real overflow timing). This is foundation evidence only: no storage
wiring, no durable follow-up integration, no DriveFS or non-NTFS support
claims. #37 and #41 remain open until their broader acceptance is
satisfied.

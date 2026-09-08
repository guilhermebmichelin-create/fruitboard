# Phase 2 scan console IPC (native half of P2-08)

Status: **draft for owner review** — native typed command surface for the
hidden scan worker (#38/#40), citing P2-08 (native half). Owner acceptance is
pending; production scanning controls stay hidden until the client seam from
draft PR #63 is accepted and the renderer adapter is wired.

- Branch: `feat/scan-ipc-console` (worktree `.tools/worktrees/scan-ipc`).
- Scope: `apps/desktop/src-tauri/**` only, plus this document.
- Cross-references: `docs/PHASE_2_INTEGRATION_CONTRACT.md` (authoritative
  storage contract, §4 jobs/cancellation, §5 Library query) and the client
  seam `apps/client/src/library/contracts.ts` on
  `feat/phase-2-library-scan-ui` (draft PR #63) — the command shapes below
  map 1:1 to that seam.

## 1. Feature flag: `scan-console`

- Cargo feature on `fruitboard-desktop`, default **OFF**
  (`scan-console = ["dep:fruitboard-scan-execution", "dep:fruitboard-filesystem-enumeration"]`).
  With the feature off the scanner crates are not compiled into the desktop
  build at all.
- The six commands are **always registered** so the client contract is
  stable. With the feature off they return the typed envelope
  `{status:"error", error:{code:"unavailable", retryable:true,
  message:"The requested service is temporarily unavailable."}}` with
  diagnostic `scan_console_disabled`, and `get_scan_console_state` returns
  `{enabled: false}`.
- Default builds (`pnpm check`, packaging smoke, feature-off CI) are
  unaffected: `cargo test -p fruitboard-desktop --locked` and
  `cargo clippy -p fruitboard-desktop --all-targets --locked -- -D warnings`
  pass in both configurations; the feature-on integration suite is run
  locally with `--features scan-console` (like the `packaging-smoke`
  precedent, CI covers the default configuration).
- The commands are not added to any Tauri capability permission, and
  `removeUnusedCommands` is true, so the renderer cannot invoke them yet: the
  capability flip is part of the client-adapter slice, not this wave.

## 2. Command surface

All commands use the foundation envelope
(`status`, `schemaVersion: 1`, `correlationId`, `data`/`error`) and the
foundation request pattern (`schemaVersion` + `deny_unknown_fields` +
camelCase). Request schemas match the client seam exactly.

| Command | Request (camelCase) | Data on success |
| --- | --- | --- |
| `scan_now` | `{rootId}` | `ScanStartResult` |
| `cancel_scan` | `{jobId}` | `CancelScanResult` |
| `retry_scan` | `{jobId}` | `ScanStartResult` |
| `list_scan_statuses` | `{}` | `ScanStatus[]` (one per configured root) |
| `get_library_page` | `{rootId, limit, cursor, snapshotId}` | `LibraryPage` |
| `get_scan_console_state` | `{}` | `{enabled: boolean}` |

`ScanStartResult` `{rootId, jobId, runId, outcome}` with
`outcome: "queued" | "already_queued" | "already_running"`; `runId` is null
while queued.

`CancelScanResult` `{rootId, jobId, runId, outcome}` with
`outcome: "cancelled" | "cancellation_requested" | "already_cancelled" |
"already_completed" | "already_failed" | "not_found"` (the outcome union
keeps `not_found` for parity; unknown jobs currently surface the typed
`not_found` *error*, matching the draft PR's fake adapter).

`ScanStatus` `{root, state, jobId, runId, cancellationRequested, counters,
lastSuccessfulScanAt, lastOutcomeAt, errorCode}` — the client contract's
exact shape. `root` is the platform `ScanRoot`; `root.canonicalPath` appears
only in that management field, never in diagnostics or error payloads.

`LibraryPage` `{rootId, snapshotId, records, nextCursor}`; records are the
client's `PublishedFileLocation` shape with `byteSize` as a canonical decimal
string and `modifiedAt` as UTC RFC 3339 with nanosecond precision.

### `get_library_page` details

- `limit` is clamped to `1..=200` (the seam's `LibraryPageRequest.limit`
  documents "the native boundary clamps to 1..200"); out-of-range values
  never reach storage.
- `cursor` and `snapshotId` are opaque versioned tokens
  (`{"v":1, ...}`, `i64` values encoded as canonical decimal strings).
  Malformed, structurally inconsistent, or wrong-root tokens return
  `invalid_cursor`; a token bound to a replaced committed snapshot returns
  `stale_cursor`. A plain cursor implicitly echoes the snapshot it was
  minted against (the seam's request has no snapshot field).
- `nextCursor` is null exactly when no more rows are pending (the storage
  page always carries a cursor when rows exist; the adapter only exposes it
  while `has_more`).

## 3. Event surface

After every durable state transition and on poll-loop run completion the
host emits the typed event `scan-status-changed` carrying **only** root ids,
states and counters:

```json
{ "roots": [{ "rootId": "...", "state": "running", "counters": {
  "filesObserved": 2, "directoriesVisited": 0, "totalFiles": null } }] }
```

Emitter: `scan_now` (queued), `cancel_scan` (queued cancel → cancelled),
`retry_scan` (queued), the poll loop (running on claim, terminal on
completion). Client subscription wiring is deliberately not built.

## 4. Mapping tables (worker/storage type → IPC shape)

### State

| Durable `ScanJobState` | `ScanStatus.state` |
| --- | --- |
| (no job) | `idle` |
| `queued` | `queued` |
| `running` | `running` |
| `completed` | `completed` |
| `failed` | `failed` |
| `cancelled` | `cancelled` |
| `interrupted` | `interrupted` |

`runId` is the latest run of the latest job (null while queued). The status
for the latest job only: an interrupted run whose job was resumed on restart
surfaces as `queued` with the interrupted run retained in the durable ledger.

### Outcomes

| Storage result | `scan_now.outcome` | `cancel_scan.outcome` | `retry_scan.outcome` |
| --- | --- | --- | --- |
| new queued job | `queued` | — | — |
| coalesced queued job | `already_queued` | — | `already_queued` |
| job running | `already_running` (with runId) | `cancellation_requested` (with runId) | `already_running` (with runId) |
| queued job cancelled | — | `cancelled` (runId null) | — |
| running job, worker committed its outcome first | `already_running` (with runId) | `already_completed` / `already_cancelled` / `already_failed` | `already_running` (with runId) |
| terminal cancelled | — | `already_cancelled` | `conflict` error |
| terminal completed | — | `already_completed` | `conflict` error |
| terminal failed | — | `already_failed` | requeue → `queued`; exhausted budget → `conflict` error |
| terminal interrupted | — | `already_failed` | `conflict` error |
| unknown job | `not_found` error | `not_found` error | `not_found` error |
| unknown/disabled root | `not_found` / `conflict` error | — | — |

Notes: `retry_failed_scan_job` returns false for exhausted chains; the closed
`ScanStartOutcome` union has no "already_failed", so the safe `conflict`
error is used (documented divergence from the UI-only fake, which can always
requeue). Cancelled chains are never revived (durable contract, not a fake
behavior). Cancel/commit race: SQLite serializes the cancel write against the
worker's terminal write, so whichever commits first decides the outcome. A
first-time cancel of a running scan can therefore surface `already_cancelled`
(or `already_completed`/`already_failed`) when the worker committed its
outcome first; every `already_*` outcome is a terminal no-op for the UI and
carries the recorded `runId` for reference.

### Error codes

| Source | IPC error code |
| --- | --- |
| Feature off | `unavailable` |
| Unknown root | `not_found` |
| Unknown job | `not_found` |
| Disabled root enqueue / terminal retry / terminal cancel race | `conflict` |
| Storage busy | `unavailable` |
| Malformed/wrong-root cursor or snapshot token | `invalid_cursor` |
| Replaced committed snapshot | `stale_cursor` |
| All other storage failures | `internal` |

`ScanStatus.errorCode` (closed client set): the host captures the last
enumeration outcome in memory (`Denied` → `access_denied`,
`UnsupportedFilesystem` → `unsupported`, `ResourceLimit` → `resource_limit`,
`RootUnavailable` → `unavailable`, `Cancelled` → `cancelled`, `Interrupted` →
`conflict`, otherwise `internal`); after a restart the persisted durable job
diagnostics map to the same closed set. Queued/running attempts report a null
error code (no outcome yet). Raw diagnostics, SQL, lease tokens, correlation
tokens and paths never cross the boundary.

### Counters

`filesObserved`: while running, the live staged-record count from
`scan_staging`; on completion, the published `location_count` (or 0 for
non-published terminal runs). `directoriesVisited`: 0 — the worker API does
not currently surface the enumeration report to the host, so no value is
fabricated. `totalFiles`: always null (honest unknown work; no percentage is
implied). Counters are in-memory and reset on restart; the durable statuses
survive, the counters do not (documented limitation of this slice).

## 5. Host wiring and concurrency notes

- One `ScanWorker` + one process session, shared through the same
  `Arc<Mutex<Database>>` as the command layer. One poll-loop thread iterates
  `service_retries -> claim -> execute` with the real Windows port
  (`WindowsFilesystemPort`) and idles on a 1 s `recv_timeout`; action events
  (`scan_now`, `cancel_scan`, `retry_scan`) wake it immediately through an
  unbounded channel. No busy spin, no wall-clock sleeps anywhere in the host.
- The single SQLite connection is non-reentrant, but the worker never holds
  it across filesystem I/O. Traversal runs lock-free; the shared-mutex
  staging adapter (`SharedStagingAdapter` in `crates/scan-execution`) acquires
  short per-batch transactions (`<=512` records, re-checked by storage's
  `MAX_STAGED_BATCH_RECORDS` fence) for lease renewal, fence re-read, and
  staging write, plus short transactions for the pre-publication fence,
  change-plan page reads, and the final atomic `publish_scan_run`. Every
  staging and publication call still revalidates generation, revision, lease
  token, and durable cancellation flags inside storage's transaction, so the
  cancel/commit race keeps SQLite serialization as the winner. `claim_due`
  and `execute_pending` are short-lock only, and the worker mutex is cloned
  before executing, so `list_scan_statuses`/`get_library_page` stay
  responsive mid-scan while the durability and shape contracts are unchanged.
- Cancellation composition: the `cancel_scan` command flips a scoped
  in-memory mirror first (it never blocks and never needs the database, so
  the enumerator stops at its next cooperative poll between entries and
  batches, non-blocking per the `Cancellation` trait), and then durably
  commits the cancellation — the durable write is the acknowledged
  authority. Between batches the scan-execution staging adapter
  independently re-reads the durable flags from storage, so a cancelled run
  can never publish. The mirror is keyed by job ID and replaced per claim,
  so no stale flag can abort a later chain.
- `get_scan_console_state` returns `{enabled: true}` only when the feature is
  on and the host installed (setup failure aborts startup).

## 6. Integration tests (feature-gated, deterministic)

`apps/desktop/src-tauri/src/foundation/scan_console/tests.rs` drives the real
command handlers and the real host over a tempdir database with a scripted
fake `FilesystemPort` and an injected fake `ScanClock` — no wall-clock sleeps.
The loop is split into `claim_due`/`execute_pending` so tests interleave
commands while a job is durably running:

- queued → running → completed with contract-shaped statuses/events;
- queued cancellation (immediate, no run) and running cancellation
  (`cancellation_requested`, then durable `cancelled`, nothing published);
- manual retry after failure plus automatic retry through the persisted
  backoff;
- restart recovery: prior running lease becomes interrupted, the job resumes
  with its attempt budget, and a fresh run completes;
- snapshot fencing: a cursor or explicit snapshot across a publication
  returns `stale_cursor`; fresh pagination reads the new snapshot stably;
- `invalid_cursor` for malformed/wrong-root tokens and `not_found` pages;
- coalescing (`already_queued`/`already_running`), unknown/disabled roots,
  terminal retry/cancel outcomes, limit clamping, token round-trips, RFC 3339
  formatting, and event privacy (no paths/tokens).

The feature-off suite (`disabled_tests.rs`) verifies every command returns
the typed unavailable envelope and `{enabled: false}`.

## 7. Remaining P2-08 checklist (owner acceptance pending)

- [x] Concurrency follow-up (#38/#40, P2-08 native, P2-04/P2-05): worker/host
      release the DB connection between batches (short per-batch staging
      transactions `<=512` records + final atomic publication), preserving
      generation/revision/lease/cancellation validation, cancel/commit race
      semantics, mirror-first cancel order, and per-claim job-ID-keyed mirror.
      Proven by deterministic `claim_due`/`execute_pending` tests with a fake
      port + fake clock (no sleeps): statuses/pages succeed while durably
      running and mid-traversal, cancel mid-batches leaves prior snapshots
      untouched.
- [x] Flagged test gaps closed: counters honesty (restart drops in-memory
      counters, durable statuses/library survive), retry-exhausted → `conflict`
      divergence from the fake, `Busy` → `unavailable`, `invalid_cursor` vs
      `stale_cursor` mismatch, event/error privacy (no paths/tokens/SQL).
- [ ] Owner accepts the native command/event surface against the client seam
      (`apps/client/src/library/contracts.ts` on the draft PR branch).
- [ ] Client-adapter flip: the draft PR's `PlatformPort`/`LibraryScanAdapter`
      native implementation wires `scanNow/cancelScan/retryScan/
      listScanStatuses/getLibraryPage` to these commands and parses the
      typed errors (incl. `stale_cursor`/`invalid_cursor` restart). Safe only
      after this wave's responsiveness proof; #63 flip is still gated on owner
      acceptance.
- [ ] Tauri capability/permission flip for the six commands once the adapter
      lands (commands are registered but not permitted today).
- [ ] Client-side typed IPC integration tests over the native seam.
- [ ] axe/keyboard/narrow-layout evidence via the draft PR's review harness
      against real native statuses/pages.
- [ ] Counters note: a future slice may surface the enumeration report
      (`directoriesVisited`, final totals) through the worker API; nothing is
      fabricated meanwhile (`directoriesVisited=0`, `totalFiles=null`,
      in-memory reset on restart remain documented).

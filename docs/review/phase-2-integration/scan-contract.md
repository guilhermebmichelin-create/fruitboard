# Scan contract: reconciler <-> ledger/staging <-> enumerator (#59 / #60-#62 / #64)

Status: **contract map + hidden worker composition; no production activation.**
Production scan stays hidden (no renderer controls, no watcher wiring).
`crates/scan-worker` composes the three landed foundations behind a
typed API for tests only. This document prevents contract drift between the
three landed isolated foundations and scopes the scan-worker
handoff. NTFS-local only. DriveFS (#47) and FAT32/cross-volume (#48)
are explicitly out. No renderer Scan/Cancel controls, no WAL change,
no PyFLP, privacy redaction preserved (fixed `storage_*` codes only,
never paths, SQL, or values).

Sources (branch base `origin/main` = `4219e0c`):

* Reconciler #59: `crates/reconciliation/src/lib.rs`
* Ledger #60/#61: `crates/storage-sqlite/src/execution.rs`,
  `crates/storage-sqlite/migrations/003_scan_execution.sql`
* Staging/publication #62: `crates/storage-sqlite/src/publication.rs`,
  `crates/storage-sqlite/migrations/004_scan_publication.sql`,
  `crates/storage-sqlite/migrations/005_integration_contract.sql`,
  plus fix `f691a32` (NotFound-vs-Conflict split, pre-write budget check)
* Enumerator #64: `crates/filesystem-enumeration/src/lib.rs`
* Budgets: `docs/PHASE_2_EXECUTION_PLAN.md` (provisional targets),
  `docs/PHASE_2_INTEGRATION_CONTRACT.md` (Contract-Revision 2),
  `scripts/benchmark-scan.md` (#67 scaffold)

## 1. Type map: change-sets <-> staged rows <-> sink output

### 1.1 Paths and locator keys

| Aspect | #59 reconciler | #62 staging/publication | #64 enumerator sink | Mismatch |
| --- | --- | --- | --- | --- |
| Type | `Location.path: String`, `Observation.path: String` (`crates/reconciliation/src/lib.rs:24`, `:31`) | `ScanObservation.locator_key: String` (opaque key) + `relative_path: String` (display only) (`crates/storage-sqlite/src/publication.rs:173`) | `Observation.locator: NormalizedLocator` + `locator_key_v1: LocatorKeyV1` + `display_path: DisplayRelativePath` (`crates/filesystem-enumeration/src/lib.rs:723`) | #59 has one string; #62 has two; #64 has three. Only `locator_key_v1 -> locator_key` and `display_path -> relative_path` is valid. `NormalizedLocator` must never cross into storage (display/diagnostic only). |
| Key syntax | No envelope, no mode tags, no NFC/fold, no ASCII guard. `validate_path` (`crates/reconciliation/src/lib.rs:88`) rejects empty, leading `/` or `\`, any `:` or NUL, any `""` / `.` / `..` segment on split `['/', '\\']`. | `v1:` envelope required, ASCII-only, no NUL, `1..=32768` bytes, per-segment `i:` / `s:` mode, charset `alnum / -_.~ / %`, valid `%XX` (either hex case accepted), body non-empty, bare `v1:` rejected (`crates/storage-sqlite/src/publication.rs:424`). `relative_path` validated separately (non-empty, `<=32768` B, no NUL/`:`, no leading `/` or `\`, no `""` / `.` / `..`). | `LocatorKeyV1` (`crates/filesystem-enumeration/src/lib.rs:180`): `v1:` + per-component `i:` / `s:` + NFC + insensitive-only simple case fold (re-NFC) + uppercase `%XX`. `validate_scan_observation` (`crates/filesystem-enumeration/src/lib.rs:863`) mirrors #62 plus bare-`v1:` rejection and the `32 KiB` key bound. | A #59 `path` is neither a #62 `locator_key` nor a `relative_path`. Strict-subset gap: #64 `decode` rejects lowercase hex as `NonCanonical`; #62 staging accepts either hex case. Every key accepted by #64 stages cleanly; the reverse is not guaranteed. |
| Ordering | `BTreeMap` / `BTreeSet` on `path` bytes; deterministic normalized-path order including retained missing locations. | `(scan_root_id, locator_key COLLATE BINARY, location_id)`; Library pages per root ordered by `(locator_key COLLATE BINARY, id)`. `relative_path` never participates in uniqueness, ordering, cursors, or rename inference. | Derived byte-wise `String` order on `LocatorKeyV1`, defined to equal storage `COLLATE BINARY`. `DisplayRelativePath` never used for equality, order, cursors, or lookups. | Aligned by construction, but only if the worker passes `locator_key_v1` bytes verbatim. Lowercasing or re-deriving a key from `relative_path` breaks the contract. |
| Duplicate rule | `DuplicatePath` rejects the whole run; enumeration order never picks a winner. | Intra-batch duplicate `locator_key` and cross-batch conflicting duplicate both terminal `StagingRejected`; idempotent same-content re-stage of one key is allowed; staged-set duplicate at publish is `Conflict`. | Duplicate durable keys reject the whole run (`CoverageFailureKind::DuplicateLocator` -> `Outcome::Invalid`); enumeration order never picks a winner. | Same policy, three spellings (`Rejected::DuplicatePath`, `StagingRejected`, `Invalid`). The worker must not dedupe by display spelling. |

### 1.2 Identities

| Aspect | #59 | #62 | #64 | Mismatch |
| --- | --- | --- | --- | --- |
| Type | `Identity { volume: u64, file: u128 }`, `Metadata.identity: Option<Identity>` (`crates/reconciliation/src/lib.rs:11`, `:17`) | `EncodedIdentity { volume_serial: String, file_id: String }` (`crates/storage-sqlite/src/publication.rs:138`): canonical unsigned decimal `u64` / `u128` pair, stored as nullable `TEXT` columns, all-or-none. | `QualifiedIdentity { volume_serial: u64, file_id: u128, qualification: LocalNtfs }` (`crates/filesystem-enumeration/src/lib.rs:428`) + `FilesystemQualification { LocalNtfs, Unqualified }`. Wire DTO `EncodedIdentity` is a field-for-field mirror (`crates/filesystem-enumeration/src/lib.rs:769`) with canonical decimal strings. | Numeric tuple vs decimal-string pair vs qualified tuple. #59 cannot serialize to #62 without `to_string` plus canonical validation. |
| Conversion | None (in-memory only). | Canonical checks at serde deserialize and at `observation_path_bytes`; `u64` / `u128` range enforced; partial pairs invalid; legacy `volume_id INTEGER` / `filesystem_file_id TEXT` backfilled only if non-negative plus canonical `u128` decimal, else `NULL` (migration 005). | `windows_file_id_to_u128 = from_le_bytes` fixed by contract rev 2 H4, used identically in every DTO and test; `encode_identity` returns `Some` only for `LocalNtfs` (today the only constructible qualification); `validate_identity_pair` mirrors the canonical all-or-none rule. | `encode_identity -> None` with a present input means out-of-scope qualification and must convert to `ScanConversionError::Identity`, never to silent `None`. Byte order is consistency, not OS order; the worker must not reinterpret it. |
| Absent identity | `IdentityUncertain { path, was_available, is_available }` plus optional `Modified` / `Replaced` on the same path; never proves a move. | All-or-none `NULL` pair = continuity uncertainty; never proves a rename or logical-project match; `missing` rows excluded from identity lookup; exact-path continuity handled separately. | File `None` = `identity_unavailable` counter, run can stay `Complete` (weakened evidence only). Directory `None` = `DirectoryIdentityUnavailable`, non-authoritative (traversal state depends on it). | Same "uncertainty, not proof" rule, but different consequences: file-level absence is survivable, directory/root absence is not. |

### 1.3 Generations, revisions, and run identity

| Aspect | #59 | #62 | #64 | Mismatch |
| --- | --- | --- | --- | --- |
| Run/job ids | None. | `run_id`, `job_id` / `scan_job_id`, `scan_root_id`, `retry_chain_id`, `attempt`, `session_id`, `lease_token`, `lease_expires_at_ms`, `ScanStageState { Open, Published, Discarded }`, `record_count`, `path_bytes` (`crates/storage-sqlite/src/publication.rs:191`, `crates/storage-sqlite/src/execution.rs:158`, `:177`). | Opaque `run_id: String` only, plus transport `ObservationBatch { sequence: u64, records, estimated_bytes }` and `RunScopedSinkAdapter { run_id, armed }` (`crates/filesystem-enumeration/src/lib.rs:619`, `:928`). | The enumerator owns no generation, revision, lease, session, or job. Batch `sequence` is transport-only, not a ledger generation. The worker must supply generation/revision/lease from the ledger `LeasedScan`, never from the enumerator. |
| Generation / revision | None; doc comment states storage must fence generation, revision, lease, and cancellation atomically. `reconcile` fences only `outcome == Complete`, else `Rejected::Incomplete`. | Generation allocated in `lease_next_scan` (`crates/storage-sqlite/src/execution.rs:1022`: `root.generation + 1` plus job claim in one transaction); captured per run; `configuration_revision` from `scan_root`; both re-checked on every staging and publication call (`validate_owner`, `crates/storage-sqlite/src/publication.rs:583`). | None durably; in-memory root-identity chain plus final root re-check only. Relies on the storage fence after `Complete` and on invalidation before return for everything else. | #59 and #64 fence nothing durably. Only #62 enforces generation/revision/lease/cancellation. A `Complete` enumeration is necessary but never sufficient for apply. |

### 1.4 Numeric bounds and path-byte accounting

* #59 (`crates/reconciliation/src/lib.rs:84`): `MAX_RECORDS = 10_000` (`previous.len() + observed.len()`), `MAX_PATH_BYTES = 32_768` per `path.len()`, `MAX_TOTAL_PATH_BYTES = 4 MiB` cumulative over both inputs. Whole-run rejection, no partial output.
* #62 (`crates/storage-sqlite/src/publication.rs:94`): `MAX_STAGED_BATCH_RECORDS = 512` per call, `MAX_STAGED_RECORDS: i64 = 10_000` total, `MAX_STAGED_PATH_BYTES: i64 = 4 MiB` total (`locator_key.len() + relative_path.len()` per observation), `MAX_OBSERVATION_PATH_BYTES = 32 KiB` per field, `MAX_LIBRARY_PAGE_SIZE = 200`. `byte_size: u64` staged only if `<= i64::MAX`; `modified_at_ns: i128` staged only via checked `i64::try_from`. Enforced pre-write per batch and re-checked at publish (count equality, recomputed bytes, staged-set duplicate check).
* #64 (`crates/filesystem-enumeration/src/lib.rs:21`, `:763`): `HARD_MAX_BATCH_RECORDS = 512`, `HARD_MAX_BATCH_BYTES = 256 MiB`, `MAX_OBSERVATION_KEY_BYTES = 32 KiB` (mirrors staging bound without depending on storage). `EnumerationLimits::default` additionally caps directories (`100_000`), pending directories (`4_096`), pending path bytes (`4 MiB`), entries (`1_000_000`), observations (`100_000`), per-path bytes (`32 KiB`), total path bytes (`64 MiB`), diagnostics/exclusions (`256` each), batches (`200_000`). `estimated_bytes = key + display + 8 + 16 + optional identity bytes`. Timestamps/byte sizes pre-filtered at traversal (`TimestampOutOfRange` / `ByteSizeOutOfRange` skip plus `Partial`, never staged).

Drift risk: the three byte counters measure different things (one path string vs key-plus-display vs key-plus-display-plus-struct-overhead), and #64 defaults (`100_000` observations, `64 MiB` total path bytes) exceed the #59 / #62 `10_000` / `4 MiB` quotas. The worker must configure #64 at or below the staging quota (see section 4), or every large `Complete` becomes a terminal `StagingRejected`.

### 1.5 Outcome and change vocabularies

* #59 `Outcome { Complete, Partial, Cancelled, Offline, Denied, ResourceLimit }` (`crates/reconciliation/src/lib.rs:38`); only `Complete` yields a `Plan`. `Change { Added, Modified, Replaced, IdentityUncertain, Missing, Restored, RenameEvidence }` is advisory per-path evidence; rename only for an unambiguous one-old/one-new identity match with live-alias counting; stale historical identity never proves a rename.
* #62 has no `Change` list. `plan_observations` mints or reuses `project_file_id` per staged set under the same conservative rules (current-evidence identity reuse only, `missing` excluded, exact-path continuity separate, conflicts mint a fresh record, stage order cannot affect the plan).
* #64 `Outcome { Complete, Partial, Denied, RootUnavailable, Cancelled, ResourceLimit, SinkFailed, Invalid, UnsupportedFilesystem }` (`crates/filesystem-enumeration/src/lib.rs:984`); `is_authoritative == Complete` only. Per-file skips, `PolicyExclusion` (leaf reparse, unsupported entry type), and `CoverageFailure` (20 kinds) keep siblings observable but the report non-authoritative. A directory junction at metadata/open time is `DirectoryChanged` (non-authoritative), never a policy exclusion; a leaf reparse file is an exclusion (`Complete` allowed). `SinkError { ResourceLimit, Unavailable, Rejected }` (`crates/filesystem-enumeration/src/lib.rs:663`); `ScanConversionError { LocatorKey, RelativePath, ByteSizeOutOfRange, TimestampOutOfRange, Identity }`.

`Offline` exists only in #59; `RootUnavailable` / `SinkFailed` / `Invalid` / `UnsupportedFilesystem` exist only in #64. Exclusions must never become #62 `missing`.

## 2. Unified fence matrix: ledger (#60/#61) vs publish txn (#62, incl. `f691a32`)

`validate_owner` (`crates/storage-sqlite/src/publication.rs:583`) requires, in one transaction: exact `run_id` / `session_id` / `lease_token`, `run.state == running`, `job.state == running`, `run.cancel == 0`, `job.cancel == 0`, `follow_up == 0`, session not ended, root enabled, `job.root == run.root`, `lease_expires_at_ms > now`, `root.generation == run.generation`, `root.revision == run.revision`. Missing run row maps to `NotFound` (`map_not_found`); any predicate failure maps to `Conflict`. `ensure_open_staging` (`crates/storage-sqlite/src/publication.rs:676`) additionally requires `stage.state == Open` plus matching root, generation, revision, session, and lease. Error codes (`crates/storage-sqlite/src/error.rs:5`): `Conflict` (`storage_conflict`, stale fence), `NotFound` (`storage_not_found`, no row/stage), `StagingRejected` (`storage_staging_rejected`, terminal input, stage already discarded and run/job failed), `InvalidSchema` (empty ids, bad page/cursor), `Busy` / `Database` (retryable, open stage untouched).

`f691a32` split: `finish_scan_run(Completed)` delegation preserves the delegate code. No stage row at all is `NotFound` ("no open stage"); a stale lease, revision, or cancellation fence stays `Conflict`. Either way no completed row precedes publication.

| Caller | Expected error | Discard? | Retry? |
| --- | --- | --- | --- |
| `begin_scan_staging` (`crates/storage-sqlite/src/publication.rs:1387`): fence fail / empty ids | `Conflict` / `InvalidSchema` | No (nothing created; existing open stage untouched on `Conflict`) | Fresh `lease_next_scan` run; `InvalidSchema` is a caller bug, no retry |
| `stage_scan_observations` (`crates/storage-sqlite/src/publication.rs:1408`): fence fail (stale lease/revision, cancel, follow-up, disabled root, expired lease, ended session) | `Conflict`, transaction rolled back | No auto-discard by this call; the execution owner discards via section 3 paths | Do not retry the same run; finish as failed/interrupted, use a fresh run |
| `stage_scan_observations`: terminal input (bad `locator_key` / `relative_path` / identity, `byte_size` / mtime overflow, intra-batch duplicate, cross-batch conflicting duplicate, `> 512` records or record/path budget breach, both checked before the first row write per `f691a32`) | `StagingRejected` plus durable `invalidate_terminal_staging_tx` (stage `discarded`, observations deleted, run/job `failed` with `staging_rejected`) | Yes, terminal, same transaction | Never retry the same run; explicit retry means a fresh run/stage |
| `stage_scan_observations`: `Busy` / `Database` | `Busy` / `Database`, rollback | No, open stage unchanged | Retry the same batch against the same open stage |
| `publish_scan_run` / `finish_scan_run(Completed)` delegation: no stage row | `NotFound` ("no open stage") | No | Caller bug if `begin` was skipped; do not invent a stage |
| Same: stale lease/revision/cancel/follow-up/disabled/expired, stage mismatch, `record_count` / `path_bytes` mismatch, recomputed bytes mismatch, staged-set duplicate, marker/run/job/stage `changed != 1` | `Conflict`, full rollback, committed rows and marker untouched | No auto-discard here; the run stays `running` until reaped, cancelled, or finished (section 3) | Do not republish the same run after fence loss; use a fresh run |
| `finish_scan_run(Failed / Cancelled / Interrupted)` (`crates/storage-sqlite/src/execution.rs:1251`): fence fail, wrong session/token, non-running run | `Conflict` | On the success path: `discard_staging_for_run_tx` then the terminal run/job write in the same transaction | Same-run retry prohibited; a follow-up is enqueued only when `follow_up && !cancelled` |
| `renew_scan_lease` (`crates/storage-sqlite/src/execution.rs:1144`): expired, replaced session/token, ended session, non-running job, disabled or revision-changed root | `Conflict` | No | Fresh lease/run, never the same token |
| `lease_next_scan` (`crates/storage-sqlite/src/execution.rs:1022`): no due job or an active global run | `Ok(None)` | No | Poll later |
| `lease_next_scan`: generation race / `attempt > max` | `Conflict` | No | Retry the lease |
| `request_scan_cancellation` (`crates/storage-sqlite/src/execution.rs:1192`) / `cancel_scan_job` (`crates/storage-sqlite/src/execution.rs:1214`) / disable-remove / reaper: late arrival after `Completed` | Returns the terminal state unchanged, no rollback | Already published/retired; a late cancel never rolls back a commit | No retry |
| `query_library`: bad page size, malformed/wrong-root cursor, snapshot mismatch, unknown root | `InvalidSchema` / `InvalidCursor` / `StaleCursor` / `NotFound`; read-only, never discards | Never | Client restarts from `cursor: null` on `StaleCursor` |

SQLite commit order defines the cancel/complete race: cancellation committed before apply prevents apply; completion committed first stays completed and a late cancellation reports that outcome without rollback. User cancellation never auto-retries. Retry eligibility is durable state plus attempt budget (`DEFAULT_SCAN_MAX_ATTEMPTS = 4`, `crates/storage-sqlite/src/execution.rs:9`); diagnostic codes cannot reset an exhausted chain.

## 3. Adapter outcomes -> discard triggers (7 paths)

### 3.1 The 7 #62 discard paths

Helpers: `discard_staging_for_run_tx` (`crates/storage-sqlite/src/publication.rs:1260`), `discard_staging_for_job_tx` (`:1318`), `discard_staging_for_root_tx` (`:1337`), `discard_all_open_staging_tx` (`:1356`), `invalidate_terminal_staging_tx` (`:1281`, wraps run-discard plus run/job fail). All are idempotent (`UPDATE ... WHERE state = 'open'` plus `DELETE` observations).

| # | Trigger | Call site | Scope |
| --- | --- | --- | --- |
| D1 | Queued/running invalidation on restart and recovery | `recover_interrupted_tx` (`crates/storage-sqlite/src/execution.rs:549`), `invalidate_inflight_after_recovery` -> `discard_all_open_staging_tx` (`crates/storage-sqlite/src/execution.rs:467`) | Run (plus all-open on recovery) |
| D2 | Disable/remove invalidation | `cancel_root_work` (`crates/storage-sqlite/src/execution.rs:809`) | Root |
| D3 | Lease-expiry reaping | `reap_expired_tx` (`crates/storage-sqlite/src/execution.rs:892`) | Run |
| D4 | Run-level cancellation request | `request_scan_cancellation` (`crates/storage-sqlite/src/execution.rs:1207`) | Run |
| D5 | Job-level cancellation of a running attempt | `cancel_scan_job` (`crates/storage-sqlite/src/execution.rs:1239`) | Job |
| D6 | Terminal resolution of a non-`Completed` run | `finish_scan_run` non-`Completed` branch (`crates/storage-sqlite/src/execution.rs:1333`) | Run |
| D7 | Terminal staging rejection | `stage_scan_observations` -> `invalidate_terminal_staging_tx` (`crates/storage-sqlite/src/publication.rs:1537`) | Run plus run/job fail with `staging_rejected` |

The follow-up coalescing write in `enqueue_scan_tx` discards the running run's open stage inline and belongs to the D2/D6 family; it does not add an eighth semantic path.

### 3.2 #64 adapter behavior

`enumerate` (`crates/filesystem-enumeration/src/lib.rs:1096`) calls `sink.discard()` exactly once for every non-authoritative outcome. `enumerate_into_run` (`:1149`) maps `accept -> stage_batch(run_id, batch)` and non-authoritative `discard -> invalidate_run(run_id)`. `RunScopedSinkAdapter` (`:619`) is panic-safe (`Drop` invalidates while `armed`); only an authoritative `Complete` may call `release()` before drop. The adapter has no commit method; the storage publication transaction owns the final fence. `SinkError::ResourceLimit` maps to `ResourceLimit`; `Unavailable` / `Rejected` map to `SinkFailed`; both then discard via the outer call.

### 3.3 Mapping and gaps

| #64 report | Adapter action | Required #62 worker action | Gap |
| --- | --- | --- | --- |
| `Complete` (including policy exclusions, including zero-batch empty root) | `release()`, staged rows stay eligible | `publish_scan_run` (or `finish_scan_run(Completed)` delegation); an empty root still needs `begin` plus `publish` with zero rows | Silent-drop if the worker skips `begin` for the zero-batch case (publishes as `NotFound`). Exclusions must not be staged as `missing`. |
| `Partial` (per-file skip such as `TimestampOutOfRange` / `ByteSizeOutOfRange` / `LocatorKeyTooLong`, disappeared or denied child, `DirectoryChanged`, final-root mismatch) | `discard()` -> `invalidate_run`, disarmed | D6 `finish_scan_run(Failed)` (or `Interrupted` when a follow-up is pending). Committed rows and marker unchanged. | Double-discard is safe (adapter invalidate plus D6 discard are idempotent). Gap if the worker omits D6: the run stays `running` until D3 lease expiry and looks hung. |
| `Denied` (root, directory, or entry denial) | Same discard | D6 `finish_scan_run(Failed)`; D4/D5 only when the user actually requested cancellation | Must not map to `Cancelled`; no auto-retry. |
| `RootUnavailable` (root not found, changed, reparse, final-root check) | Same discard | D6 `finish_scan_run(Failed)`; D2 when the root was disabled or removed | Must not mark unseen rows `missing`. |
| `Cancelled` (cooperative flag at root, directory, entry, batch, and finalization checkpoints) | Same discard | D6 `finish_scan_run(Cancelled)` (wins over follow-up); D4/D5 when the request came from the ledger | Double-discard is expected (ledger D4/D5 plus adapter invalidate). A late cancel after publish must keep `Completed`. |
| `ResourceLimit` (directory, entry, observation, path-byte, batch, diagnostic caps, `SinkError::ResourceLimit`) | Same discard | D6 `finish_scan_run(Failed)`; D7 only when the limit fires inside `stage_batch` validation (also terminal) | Quota layering gap (section 4): a #64 `Complete` produced above the staging quota becomes D7 `StagingRejected`. Configure the worker so this cannot happen. |
| `SinkFailed` (`Unavailable` / `Rejected` from `stage_batch`), `Invalid` (bad limits, bad root, duplicate locator), `UnsupportedFilesystem` (non-NTFS), panic unwind (`Drop` invalidate) | Same discard (`Drop` covers the panic) | D6 `finish_scan_run(Failed)`; the `StorageError` <-> `SinkError` mapping must be fixed once: `StagingRejected -> Rejected` (terminal, never retry the same run), fence `Conflict` -> `Unavailable` (finish failed, fresh run), `Busy` / `Database` -> propagate as retryable and never convert to `SinkFailed`-discard-only | Largest gap: no `RunScopedStorage` implementation exists yet, so this mapping is currently unwritten. A `Busy` misclassified as `SinkFailed` would terminally discard retryable work. The panic path leaves the ledger run `running` until D1/D3, which is acceptable only if the worker always follows with D6 or lets the reaper decide. |

Double-discard (adapter `invalidate_run` plus a ledger D-path discard of the same run) is by design and idempotent. Silent-drop (adapter discarded but the ledger run is never finished, or `Complete` is never published) leaks a `running` run until lease expiry and must be closed by worker protocol, not by the adapter alone.

## 4. Three budgets, separated

* #59 in-memory guard: `MAX_RECORDS = 10_000`, `MAX_PATH_BYTES = 32_768`, `MAX_TOTAL_PATH_BYTES = 4 MiB`. Whole-run rejection, no partial output. Not a staging quota and not a performance claim.
* #62 staging bounds (enforced pre-write per batch and re-checked at publish): `MAX_STAGED_BATCH_RECORDS = 512` per call, `MAX_STAGED_RECORDS = 10_000` total, `MAX_STAGED_PATH_BYTES = 4 MiB` total, `MAX_OBSERVATION_PATH_BYTES = 32 KiB` per field, `MAX_LIBRARY_PAGE_SIZE = 200`. Breach is terminal `StagingRejected` (D7), never a same-run retry.
* #67 provisional targets (`docs/PHASE_2_EXECUTION_PLAN.md`, `scripts/benchmark-scan.md`): baseline fixture `10_000` FLP files across `1_000` directories, qualification set `100_000` entries; first-discovery latency `<= 30 s`, warm unchanged reconciliation `p95 <= 10 s`; incremental working memory `<= 128 MiB` on the `100_000`-entry set; batch/staging `<= 512` records / `<= 256 MiB`; queue/leases one worker plus one follow-up per root, `30 s` lease renewed every `5 s`; retries `1 / 2 / 4 s` plus `<= 20%` jitter then explicit retry; progress `<= 4 / s`, pages `<= 200`, query `p95 <= 200 ms`; initially `<= 100` roots. Method: pinned release build, recorded seed and manifest hash, one warm-up plus 10 measured iterations, median/max/nearest-rank `p95`. These are owner-accepted starting points, not measured promises and not enforcement. The scaffold validates fixtures and captures the environment report only; it emits no numbers while `crates/scan-worker/Cargo.toml` is absent.

Worker configuration rule: set `EnumerationLimits.max_observations <= MAX_STAGED_RECORDS`, `max_total_path_bytes <= MAX_STAGED_PATH_BYTES`, and `max_batch_records <= MAX_STAGED_BATCH_RECORDS` (512). The #64 defaults (`100_000` observations, `64 MiB` total path bytes) are deliberately larger than the staging quota and must be tightened by the worker; otherwise a legitimate `Complete` traversal is guaranteed to die as D7.

## 5. Minimal shared types and one integration test plan

### 5.1 Proposed shared-type diff (trivial aliases, no behavior change)

The duplicate DTOs (`publication::ScanObservation` / `EncodedIdentity` vs `enumeration::ScanObservation` / `EncodedIdentity`, each documented only as a "field-for-field mirror") are the highest drift risk, followed by the three byte-bound constants and the fence/outcome mappings. The minimal fix is one contract module with type aliases adopted by both crates. No logic moves in this step.

```rust
// Proposed: crates/scan-contracts/src/lib.rs (new, NTFS-local only).
// Alternatively a `contracts` module inside fruitboard-storage re-exported
// by filesystem-enumeration; the key point is a single definition.
pub const MAX_OBSERVATION_KEY_BYTES: usize = 32 * 1024;
pub const MAX_OBSERVATION_DISPLAY_BYTES: usize = 32 * 1024;
pub const MAX_STAGED_BATCH_RECORDS: usize = 512;
pub const MAX_STAGED_RECORDS: i64 = 10_000;
pub const MAX_STAGED_PATH_BYTES: i64 = 4 * 1024 * 1024;
pub const LOCATOR_KEY_V1_PREFIX: &str = "v1:";

/// Opaque durable key. Equality and order are always exact byte equality
/// within one root (`COLLATE BINARY`); never lowercase, fold, or NOCASE.
pub struct LocatorKeyV1(String);
impl LocatorKeyV1 {
    pub fn as_str(&self) -> &str;
    pub fn root() -> Self;
    pub fn decode(value: &str) -> Result<Self, LocatorKeyError>;
}

/// Canonical unsigned decimal u64/u128 pair, all-or-none. Single validator
/// shared by staging and by the enumeration handoff.
pub struct EncodedIdentity {
    pub volume_serial: String,
    pub file_id: String,
}

pub struct ScanObservation {
    pub locator_key: String,
    pub relative_path: String,
    pub byte_size: u64,
    pub modified_at_ns: i128,
    pub identity: Option<EncodedIdentity>,
}
// Single serde (canonical decimal strings for byte_size, modified_at_ns,
// volume_serial, file_id), single observation_path_bytes(), single
// validate_identity_pair().

/// Fence-result mapping that records the f691a32 split once:
/// `Conflict` = stale lease/revision/cancellation fence,
/// `NoOpenStage` = missing stage row, `StagingRejected` = terminal input.
pub enum WorkerFence {
    Conflict,
    NoOpenStage,
    StagingRejected,
}
// Maps StorageError::{Conflict, NotFound, StagingRejected} to
// SinkError::{Unavailable, Rejected, ResourceLimit} plus Busy-retryable,
// so section 3.3 can never be re-decided per call site.

/// Complete-only authority shared by the reconciler Rejected::Incomplete
/// rule and the enumerator is_authoritative rule.
pub fn enumeration_authoritative_complete() -> bool;
```

Adoption in this step is aliases only:

```rust
// publication.rs
pub use scan_contracts::{EncodedIdentity, ScanObservation, MAX_STAGED_BATCH_RECORDS};
// lib.rs (enumeration)
pub use scan_contracts::{EncodedIdentity, ScanObservation, MAX_OBSERVATION_KEY_BYTES};
```

`Observation::to_scan_observation()` stays as the thin converter
(`locator_key_v1` bytes plus `display_path` plus `encode_identity` plus
`unix_ns_to_i64_checked`). `reconciliation` keeps its `String`-path
in-memory model; the worker converts committed rows plus staged
observations to `reconciler::Observation` for advisory decision diffing
only after the publish fence, never as a storage key.

### 5.2 One integration test plan for scan-worker (NTFS-local only)

Test: `scan_worker_ntfs_local_fenced_roundtrip`. Runs on Windows NTFS
only; skipped elsewhere. DriveFS (#47) and FAT32/cross-volume (#48)
explicitly out. No renderer, no watcher, no parser, no WAL toggle,
no PyFLP, fixed redacted diagnostics only.

1. Setup: temporary NTFS directory (assert
   `FilesystemQualification::LocalNtfs`), synthetic tree with Unicode
   names, one near-`MAX_PATH` long path, one hardlink alias pair,
   non-FLP noise, one empty directory, and one junction directory for
   the refusal case. Open `Database`, `add_scan_root`, `start_scan_session`,
   `enqueue_scan(Manual)`, `lease_next_scan` (capture generation,
   revision, session, token). Worker `EnumerationLimits` tightened per
   section 4.
2. Happy path: test-only `RunScopedStorage` over `begin_scan_staging`
   plus `stage_scan_observations` (batch `Observation` ->
   `to_scan_observation`; `StagingRejected -> SinkError::Rejected`,
   fence `Conflict -> SinkError::Unavailable`, `Busy`/`Database`
   propagated retryable) plus `invalidate_run` as run-discard.
   `enumerate_into_run` with the real Windows port must report
   `Complete` and authoritative; assert staged `record_count` /
   `path_bytes` match the converted observations. `publish_scan_run`
   must succeed; assert `location_count` counts present plus missing,
   Library pages hold `<= 200` rows in `BINARY` key order under a
   stable snapshot, and the reconciler plan over converted rows shows
   `Added` only. A second identical run must show zero changes.
3. Non-authoritative matrix (same committed baseline, one fresh lease
   each): cooperative `Cancelled` mid-run, injected `Denied`, tiny
   `max_observations` `ResourceLimit`, disappearing-entry `Partial`.
   Each must leave adapter staging invalidated, resolve through D6
   `finish_scan_run(Failed / Cancelled)`, keep committed rows and the
   success marker unchanged, and never expire unrelated Library cursors.
4. Fence matrix: publish after `request_scan_cancellation` is
   `Conflict` with no apply; `finish_scan_run(Completed)` with no
   stage row is `NotFound` (the `f691a32` split); staging and
   publication at or past `lease_expires_at_ms` are `Conflict`; a
   follow-up `enqueue_scan` during traversal discards the running
   run's open stage and resolves the attempt as `Interrupted` with
   exactly one queued follow-up; one non-`v1:` key and one
   out-of-range mtime each resolve as terminal `StagingRejected`
   with later publish fenced.
5. Privacy and durability: assert every surfaced error is a fixed
   `storage_*` code with no path, SQL, or value; assert
   `journal_mode == delete`; assert source-file bytes and the
  ened fixture are unchanged (marker hashes before/after).

Passing this one test plus `cargo test -p fruitboard-storage --locked`
is the entry gate for any real `scan-worker` implementation. It does
not qualify DriveFS, FAT32, cross-volume identity, network shares,
watcher delivery, parsing, benchmarks, or production activation.

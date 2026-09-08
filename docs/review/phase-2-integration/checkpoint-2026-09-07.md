# P2-12 checkpoint — 2026-09-07 (#41 evidence for epic #33)

Status: **checkpoint recorded, owner acceptance pending**. This is a
READ-ONLY aggregation plus this new document. No implementation changed in
this checkpoint. Production scanning stays hidden until P2-03 through P2-08
have integrated evidence, per the accepted execution plan.

- Base: `origin/main` at `1b6f65e` (rebased; journey below executed at
  `74203a2`, whose tree is identical for every scanner path — `1b6f65e`
  (#74) adds only branch protection, dependabot scope, and
  packaging/governance docs, no `apps/**` or `crates/**` behavior change).
- Branch: `docs/41-p2-12-checkpoint` (from `74203a2`, rebased onto `1b6f65e`).
- Scope (only): `docs/review/phase-2-integration/README.md` (evidence table
  only), this new `checkpoint-2026-09-07.md`,
  `scripts/generate-synthetic-tree.mjs` usage evidence only (script not
  modified). No changes to `apps/**`, `crates/**`, `.github/**`,
  `docs/review/phase-2-library/**`, or `docs/review/phase-2-ipc/**`.
- Labeling precedent (issue-35): fake-adapter rendering evidence is labeled
  separately from installed-app evidence. The Library/IPC journey below runs
  against the hidden worker/driver only. It is not installed-app scan
  evidence.

## 1. PR to P2-ID map (merged PRs listed below; branch rebased onto `1b6f65e`)

| Merged PR (commit) | P2 IDs touched | CI jobs (foundation.yml at base) | Key tests / evidence |
| --- | --- | --- | --- |
| #59 reconciliation core (`c5264a2`) | P2-02 Partial | `rust-portable` | `cargo test -p fruitboard-reconciliation --locked`: 17 tests (`unchanged_tree_is_idempotent_and_order_independent`, `every_incomplete_outcome_rejects_positive_and_absence_changes`, `hardlink_removal_preserves_surviving_alias_identity`, `rename_retains_old_path_history_and_physical_evidence`, `duplicate_and_invalid_paths_reject_the_whole_run_without_path_diagnostics`); `crates/reconciliation/README.md` |
| #60 durable ledger (`7870377`) + #61 contract preservation (`82bc651`) | P2-04 Partial, P2-05 Partial | `migration`, `rust-portable` | `cargo test -p fruitboard-storage --locked` subset: `durable_scan_jobs_coalesce_and_schedule_one_follow_up`, `execution_recovery_transition_matrix_covers_restart_and_backup`, `restart_requeues_the_interrupted_attempt_without_resetting_its_chain`, `disabling_and_removing_roots_invalidate_work_and_readding_gets_new_identity`; `docs/PHASE_2_DURABLE_EXECUTION.md` |
| #62 staging + atomic publication (`857de7d`, incl. `f691a32` split + pre-write budget check) | P2-04 Partial, P2-05 Partial, P2-06 Pending, P2-07 Pending, P2-03 Pending | `migration`, `rust-portable` | `cargo test -p fruitboard-storage --locked`: 76 passed. Key names: `staging_is_invisible_until_atomic_publication_and_tracks_aliases`, `publication_marks_missing_and_restores_each_alias_without_collapsing_it`, `publication_rejects_stale_or_cancelled_runs_without_touching_committed_rows`, `publication_rolls_back_visible_rows_and_ledger_on_apply_failure`, `recovery_discards_open_staging_and_root_removal_detaches_history`, `staging_batches_enforce_record_and_path_budgets_before_writing`, `integration_library_pages_are_root_scoped_and_snapshot_bound`, `integration_staging_is_never_visible_to_library_reads`, `integration_cancellation_uses_job_before_lease_and_run_after_lease`, `migration_quarantines_legacy_keys_and_first_v1_scan_retains_projects`, `killed_migration_recovers_the_original_committed_database`; `docs/PHASE_2_DURABLE_PUBLICATION.md`, `docs/PHASE_2_INTEGRATION_CONTRACT.md` rev 2 |
| #64 enumeration hardening (`57587b5`) | P2-02 Partial, P2-03 Pending, P2-10 Pending, P2-11 method | `enumeration-windows` | `cargo test -p fruitboard-filesystem-enumeration --locked`: 43 passed, 2 ignored (ACL-denied fixture needs disposable security-policy setup; DriveFS/streamed placeholders unavailable, #47 open). Covers truncated `STATUS_BUFFER_OVERFLOW` as non-authoritative, junction-at-metadata as `DirectoryChanged`, `RunScopedSinkAdapter` panic-safe `Drop` invalidate; `crates/filesystem-enumeration/README.md` |
| #68 watcher foundation (`4219e0c`) + #69 ownership/classification gaps (`01bd2dd`) | P2-09 Pending | `filesystem-watcher-windows` | `cargo test -p fruitboard-filesystem-watcher --locked`: portable coalescer/path/parse unit tests (`burst_inside_one_window_collapses_into_exactly_one_hint`, `privacy_regression_no_public_type_can_carry_absolute_paths`) + Windows live tests (`live_watch_delivers_coalesced_hints_and_stops_cleanly`, `overflow_surfaces_as_an_immediate_coverage_lost_hint`, `stop_after_worker_exit_is_safe_and_sticky`, `restart_uses_a_fresh_handle_and_generation`); unverified fixtures stay explicit (`acl_revocation_fixture_is_unverified`, `drivefs_root_watch_fixture_is_unverified`, `real_buffer_overflow_fixture_is_unverified`); `crates/filesystem-watcher/README.md` |
| #70 hidden worker + `scan-contract.md` (`b78e715`) | P2-02 Partial, P2-03 Pending, P2-04 Partial, P2-06 Pending, P2-09 Pending (composition, no production activation) | `scan-execution-windows` | `cargo test -p fruitboard-scan-execution --locked`: 38 passed, 1 ignored (`large_tree_budget_measurement_requires_the_benchmark_harness`). Covers idempotent deltas, add/modify, rename/missing/restore, denial mid-traversal, offline retry chain, durable + cooperative cancellation, follow-up invalidation, disable/remove fencing, SQL-failure rollback, lease expiry/replacement, restart recovery, coalescing, quota limits, empty-root missing, both cancel/commit orderings; NTFS tempdir tests `ntfs_worker_publishes_authoritative_rows_and_preserves_sources` and `ntfs_worker_denied_subtree_preserves_committed_rows` (ACL-ineffective tokens skip loudly, portable fake stays the P2-03 gate); `docs/review/phase-2-integration/scan-contract.md` (§1 type map, §2 fence matrix incl. `f691a32`, §3 seven discard paths, §4 budget separation) |
| #71 watcher follow-ups (`b148777`, #37 P2-09 storage half) | P2-09 Pending | `scan-execution-windows` | `followups.rs` + `tests.rs` additions: burst coalescing per window, overflow scheduling with recorded cause, stale-generation drops, `watch_ended` replay discard, disabled/removed/running suppression, cancelled-chain semantics, idempotent replay, restart with fresh generation, `follow_up_boundary_never_carries_path_data`, `fake_consumer_binds_the_host_loop_shape_end_to_end`; host wiring (watcher lifecycle, root mapping, activation) stays in the desktop host, not in the crate |
| #72 benchmark driver + harness + first measured report (`0935271`) | P2-11 Measured (first report) | `scan-execution-windows` + linked run below | `crates/scan-execution/examples/benchmark.rs` (production `WindowsFilesystemPort` + `ScanWorker` + durable `Database`, typed JSON lines, no paths/identities) + `scripts/run-benchmark.mjs` (pinned release build timed separately, temp fixtures with seeds/hashes, 1 warm-up + 10 measured iterations, working-set sampling, mid-run cancel latency, budget table). CI job for PR #72: <https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34163769541> (all checks green). Measured evidence: `benchmark-2026-09-07.md` + `benchmark-2026-09-07-{baseline,quota}-raw.json`. Findings F1-F3 apply (see §6) |
| #73 scan console IPC behind flag (`177cecf`) | P2-08 Pending (native half only) | `windows-foundation` (default feature-off), feature-on suite run locally | `apps/desktop/src-tauri/src/foundation/scan_console.rs` + `scan_console_host.rs` + `scan_console/tests.rs` (queued/running/completed, queued + running cancellation, manual + automatic retry, restart recovery, snapshot fencing `stale_cursor`/`invalid_cursor`, coalescing, limit clamping, event privacy) + `disabled_tests.rs` (3 tests: typed `unavailable` envelope + `{enabled:false}` with feature off). Six commands always registered, renderer cannot invoke yet (no Tauri capability, `removeUnusedCommands` true). Documented limitation: single SQLite connection serializes console commands behind a running scan. Full surface in `docs/review/phase-2-ipc/README.md` |
| #63 client seam (`74203a2`, fake-adapter Library scan UI) | P2-08 Pending (client half only; pending native flip Agent B / concurrency fix Agent A, do not block) | `client`, `windows-foundation` | `apps/client/src/library/` (`contracts.ts`, `fake.ts`, `LibraryPage.tsx`): `contracts.test.ts` (canonical decimal `byteSize`, RFC 3339 retention), `fake.test.ts` (per-root pages, `invalid_cursor`/`stale_cursor`, sibling isolation, queued `jobId` with null `runId`), `LibraryPage.test.tsx` (loading/empty/paginated per-root snapshots, stale-response discard, restart cooldown, detached-history disambiguation, queue/cancel/retry, failed/interrupted/unavailable), `LibraryPage.accessibility.test.tsx` (axe + keyboard + live-region page changes). Reported run: 12 files / 102 tests passed, lint + typecheck + build passed. Rendered fake-adapter evidence only: `docs/review/phase-2-library/README.md` + 8 PNGs (desktop/narrow populated/queued/cancelled-stale/retried) + `keyboard-trace.json` (Tab/Enter only, no `%` while total unknown). Native checklist items 1-5 in that README stay open |
| #67 scaffolding (P2-11 prep, in base) | P2-11 method only | `docs-policy` | `scripts/generate-synthetic-tree.mjs` (deterministic fixture generator, seed + manifest SHA-256, hardlink `created` on NTFS), `scripts/benchmark-scan.md` (accepted methodology), `scripts/run-benchmark.mjs` scaffold. No numbers until #72 |

Statuses held per instruction: P2-02/P2-04/P2-05 stay Partial;
P2-03/P2-06/P2-07/P2-09/P2-10 stay Pending unless merged evidence exists
(none promoted by this checkpoint). P2-01 stays Merged; P2-11 stays
Measured (first report); P2-12 stays Pending until owner acceptance (§8).
P2-08 stays Pending with the Agent A/B note above.

## 2. CI results at this checkpoint

Foundation jobs at base (`74203a2`, `.github/workflows/foundation.yml`):
`docs-policy`, `client`, `rust-portable`, `migration`,
`windows-foundation`, `security`, `filesystem-watcher-windows`,
`enumeration-windows`, `scan-execution-windows`.

- PR #72 linked CI run (all checks green):
  <https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34163769541>.
- This checkpoint re-ran the gates locally on Windows (same commit
  `74203a2`, no code changes):
  - `cargo test -p fruitboard-scan-execution --locked`: 38 passed,
    0 failed, 1 ignored.
  - `cargo test -p fruitboard-storage --locked`: 76 passed, 0 failed.
  - `cargo test -p fruitboard-filesystem-enumeration --locked`: 43 passed,
    0 failed, 2 ignored.
  - `node --test tests/*.test.mjs`: 71 passed, 0 failed.
  - `node scripts/verify-repository-privacy.mjs`: passed (224 files).
  - `pnpm lint:docs` (`markdownlint-cli2`, 44 files): 0 issues.
- `cargo build --release -p fruitboard-scan-execution --example benchmark --locked`
  (pinned toolchain Rust 1.98.1 via `rust-toolchain.toml`): success in
  ~35 s on this host; incremental harness rebuilds are timed separately and
  never inside measured scan windows.

No per-PR CI URLs other than the #72 link above are claimed. Job names are
the contract; test names in §1 are the executable evidence.

## 3. Visible completion journey — Windows transcript (hidden worker/driver)

Execution-plan reference: `docs/PHASE_2_EXECUTION_PLAN.md`, §Visible
completion scenario, steps 1-6 (ownership: issue #38 controls/status/retry,
issue #40 read-only Library query/typed IPC/list, issue #36 reconciliation,
issue #37 watcher triggers, issue #41 verifies the complete journey).

Method honesty: production scanning has no renderer entry point at this
base. Steps below drive the hidden production path end to end
(`Database` + `ScanWorker::new(WorkerConfig::default())` with the system
clock + `start_session` + `request_manual_scan` + `poll` claim/execute
through the real `WindowsFilesystemPort` up to fenced atomic publication)
via the benchmark example driver. Rendered Scan/Cancel/Retry + Library list
states are covered separately by fake-adapter evidence
(`docs/review/phase-2-library/`, labeled fake-adapter per the #35
precedent). There is no installed-app scan evidence in this checkpoint.

Recorded context (every run below):

- Commit: `74203a2` (branch `docs/41-p2-12-checkpoint`).
- Build: `cargo build --release -p fruitboard-scan-execution --example benchmark --locked`.
- Fixture tool: `scripts/generate-synthetic-tree.mjs` used without
  modification: `node scripts/generate-synthetic-tree.mjs --help` prints the
  accepted usage; the journey fixture below was built through its exported
  `buildPlan`/`writePlan`/`computeManifestHash` with
  `{sizeLabel:"p2-12-journey", seed:"p2-12-journey-0", fileCount:40, leafDirectoryCount:4}`.
- Fixture identity: seed `p2-12-journey-0`, manifest SHA-256
  `54b9bd749aa5d3ce20b5063929c5abac4a7ce004e61b53ebe662cf0e4579ec61`,
  `hardlinkSupport: created` (NTFS), counts `{flpFiles:40, otherFiles:1, aliasLocations:1, leafDirectories:4, directories:21, emptyDirectories:2, hardlinkGroups:1}`.
  Expected authoritative locations: 41 (40 files + 1 alias; the 1 `other`
  file is noise and never staged).
- Roots/DB handling: disposable fixture root plus disposable app-data DB,
  both under a temp directory outside the repository (no private data, no
  hostname/username/absolute path recorded; identity by seed + manifest hash
  only). Each driver invocation opens the same DB
  directory, calls `start_session` (new session per process = restart
  survival), and reuses the tracked root by canonical path
  (`benchmark-fixture` name on first creation; stable `root_id`
  `01a07e4d-b9fa-78e1-936b-fa6449d1cf46` across restarts). Temp fixture/DB
  are removed by the operator after the checkpoint; nothing under test reads
  file contents (see §4).
- Counters honesty: the driver reports `filesObserved` (staged-record /
  published `location_count`) and `directoriesVisited: 0` is not fabricated
  by the IPC host; `totalFiles` is always null (no percentage when total
  work is unknown). The JSON below carries no `totalFiles`/percentage.

### Step 1 — Select a disposable root, recognizable name, enabled; no scan yet

The driver creates `benchmark-fixture` on first open and emits `ready` with
opaque ids only (no paths, no usernames). Before any `scan_started` there is
no committed list for the root. Display-name edits do not invalidate
traversal; disable/remove would invalidate (covered by deterministic tests,
not toggled here to keep the fixture converging).

### Step 2 — Scan now: queued/running status, counts and Cancel, no fabricated percent

Run 1 (first discovery, empty committed state):

```text
{"phase":"ready","elapsed_ms":24,"root_id":"01a07e4d-b9fa-78e1-936b-fa6449d1cf46","session_id":"01a07e4d-b9f7-7ab0-a639-ef9020688ec4","worker_config":"default"}
{"phase":"scan_started","elapsed_ms":74}
{"phase":"scan_finished","elapsed_ms":207,"scan_ms":133,"status":"Published","authoritative":true,"outcome":"Complete","run_id":"01a07e4d-ba36-7580-90b3-5be56647a4c0","job_id":"01a07e4d-ba31-7062-80aa-80acd7db925c","error_code":null,"location_count":41,"generation":1,"changes_computed":true,"changes_added":41,"changes_modified":0,"changes_replaced":0,"changes_identity_uncertain":0,"changes_missing":0,"changes_restored":0,"changes_renames":0}
```

41 locations published (`location_count: 41`), advisory `changes_added: 41`.
No percentage, no `totalFiles`. The durable job/lease/run ids prove
queued/running happened through the ledger; the terminal `Published /
Complete` proves the staging fence and atomic apply both passed.

Run 2 (unchanged warm rescan, new process/session = restart survival):

```text
{"phase":"ready","elapsed_ms":11,"root_id":"01a07e4d-b9fa-78e1-936b-fa6449d1cf46","session_id":"01a07e4d-cffa-7070-99b2-40df021791bf","worker_config":"default"}
{"phase":"scan_started","elapsed_ms":61}
{"phase":"scan_finished","elapsed_ms":187,"scan_ms":126,"status":"Published","authoritative":true,"outcome":"Complete","run_id":"01a07e4d-d034-75e2-859d-bf8fea4b73e4","job_id":"01a07e4d-cffd-7d61-9072-0b405d77bcc4","error_code":null,"location_count":41,"generation":2,"changes_computed":true,"changes_added":0,"changes_modified":0,"changes_replaced":0,"changes_identity_uncertain":0,"changes_missing":0,"changes_restored":0,"changes_renames":0}
```

Same `root_id`, new `session_id`, `generation: 2`, zero changes: unchanged
tree causes no spurious changes (P2-02) and the last committed list
survived the restart.

### Step 3 — Library per-root page: filename / owning root / relpath / size / mtime / present-missing

The committed read model is per-root, bounded (`<= 200` rows/page), stable
`BINARY` key order, snapshot-bound cursors. Storage shape
(`crates/storage-sqlite/src/publication.rs`): `PublishedLocation`
`{id, project_file_id, scan_root_id, locator_key, relative_path, byte_size
(u64 canonical decimal), modified_at_ns (i64), identity?, presence
(present/missing), last_seen_scan_run_id?, last_seen_at_ms?}`; filename and
extension derive from `relative_path`; `LibraryPage`
`{scan_root_id, locations, next_cursor?, snapshot, has_more}`. IPC shape
(`docs/review/phase-2-ipc/README.md` §2): `LibraryPage`
`{rootId, snapshotId, records, nextCursor}` with `PublishedFileLocation`
records and `byteSize` decimal strings + RFC 3339 `modifiedAt`.

Deterministic gates (all passing at base): `integration_library_pages_are_root_scoped_and_snapshot_bound`, `integration_staging_is_never_visible_to_library_reads`, `publication_marks_missing_and_restores_each_alias_without_collapsing_it`, `staging_is_invisible_until_atomic_publication_and_tracks_aliases`.
This checkpoint asserts the row count through `location_count` (present +
missing by type contract) and leaves rendered pagination to the
fake-adapter captures (`docs/review/phase-2-library/` desktop/narrow
populated pages read one root at a time from that root's own committed
snapshot). No grouped-project or inferred-FLP-metadata labels appear
anywhere in the worker/storage/IPC path.

### Step 4 — Close/reopen: last committed list + settings survive; interrupted work requeues safely

Runs 1-2 already show close/reopen survival (same `root_id`, same 41 rows,
new session). Interrupted-to-requeue is covered deterministically and was
not forced live on the disposable DB (forcing it would require killing the
driver mid-traversal; the durable path is identical):

- `restart_requeues_interrupted_work_without_resetting_the_chain`,
- `recovery_scan_is_deduplicated_and_cancelled_work_is_not_revived`,
- `pending_follow_up_survives_restart_and_invalidates_the_resumed_attempt`,
- `worker_cancelled_outcome_suppresses_followup_on_restart_and_backup_recovery`,
- `repeated_active_restarts_exhaust_one_persisted_retry_chain`.

Claim: restart keeps the last committed list and requeues interrupted work
with its persisted attempt budget; it never resumes an abandoned cursor as
authoritative and never revives cancelled/exhausted chains.

### Step 5 — External add/rename/modify/remove/restore converge

Mutations applied to the live fixture (outside the worker):

- add `roots/shard-0000-sunset-beat/journey-added-9999.flp`,
- modify `sunset-beat-0000-00.flp` (appended bytes: size/mtime change, same identity),
- rename `night-loop-0000-01.flp` to `night-loop-0000-01-renamed.flp` (same identity, new path),
- remove `pad-layer-0000-02.flp` (expect missing, still visible),
- remove `groove-idea-0000-04.flp` (staged for the restore half; backup kept outside the root).

Run 3 (after add/rename/modify/remove):

```text
{"phase":"ready","elapsed_ms":16,"root_id":"01a07e4d-b9fa-78e1-936b-fa6449d1cf46","session_id":"01a07e4e-0acb-7123-8416-06f0856e06e6","worker_config":"default"}
{"phase":"scan_started","elapsed_ms":80}
{"phase":"scan_finished","elapsed_ms":354,"scan_ms":274,"status":"Published","authoritative":true,"outcome":"Complete","run_id":"01a07e4e-0b13-7e42-b8fc-c9bb529bb3bd","job_id":"01a07e4e-0acc-77d1-9fc7-2280ced51314","error_code":null,"location_count":43,"generation":3,"changes_computed":true,"changes_added":2,"changes_modified":1,"changes_replaced":0,"changes_identity_uncertain":0,"changes_missing":3,"changes_restored":0,"changes_renames":1}
```

Reads: added 2 (new file + renamed-new path), modified 1, missing 3
(old rename path + 2 removals), renames 1, `location_count: 43` (41 + 2
added; missing rows retained, never collapsed). Manual scan and the later
watcher reconciliation converge on the same durable result by construction
(hints only schedule a full reconciliation; see #71).

Restore half (copied `groove-idea-0000-04.flp` back from outside-root
backup; a copy mints a fresh filesystem identity, so replacement alongside
restore is the honest expectation):

```text
{"phase":"ready","elapsed_ms":14,"root_id":"01a07e4d-b9fa-78e1-936b-fa6449d1cf46","session_id":"01a07e4e-2ca6-7d93-8bc8-ab37fa72146d","worker_config":"default"}
{"phase":"scan_started","elapsed_ms":75}
{"phase":"scan_finished","elapsed_ms":332,"scan_ms":257,"status":"Published","authoritative":true,"outcome":"Complete","run_id":"01a07e4e-2cea-7ab1-a593-2aa9d555b200","job_id":"01a07e4e-2ca7-7162-8e38-a8ccc449b1d2","error_code":null,"location_count":43,"generation":4,"changes_computed":true,"changes_added":0,"changes_modified":0,"changes_replaced":1,"changes_identity_uncertain":0,"changes_missing":0,"changes_restored":1,"changes_renames":0}
```

`restored: 1` with `replaced: 1`: the location is present again under a new
physical identity. Missing entries stayed visible and reversible throughout.

### Step 6 — Cancel or unavailable: retain last committed list, label previous results, offer retry; never synthesize missing rows

Cancel mid-run (`--cancel-after-ms 5`, cooperative token between entries/batches):

```text
{"phase":"ready","elapsed_ms":14,"root_id":"01a07e4d-b9fa-78e1-936b-fa6449d1cf46","session_id":"01a07e4e-4bf6-7631-8589-9c5215bcd979","worker_config":"default"}
{"phase":"scan_started","elapsed_ms":92}
{"phase":"cancellation_requested","elapsed_ms":95}
{"phase":"scan_finished","elapsed_ms":110,"scan_ms":18,"status":"Cancelled","authoritative":false,"outcome":"Cancelled","run_id":"01a07e4e-4c4d-7283-9e8e-04ec674525c0","job_id":"01a07e4e-4bf7-73b2-aad0-06b34bab2032","error_code":null,"location_count":null,"changes_computed":false}
```

`authoritative: false`, `location_count: null`, `changes_computed: false`:
staging discarded, committed rows untouched. Retry (same DB, no cancel
flag) immediately republishes the retained 43 rows with zero changes
(`generation: 6`). User cancellation is never auto-retried by the worker.

Unavailable root (fixture directory renamed away, then restored):

```text
{"phase":"ready","elapsed_ms":14,"root_id":"01a07e4d-b9fa-78e1-936b-fa6449d1cf46","session_id":"01a07e4e-6193-73e2-8de8-1495fe0e1e61","worker_config":"default"}
{"phase":"scan_started","elapsed_ms":66}
{"phase":"scan_finished","elapsed_ms":84,"scan_ms":18,"status":"Failed","authoritative":false,"outcome":"RootUnavailable","run_id":"01a07e4e-61cd-7a13-b28a-bedd02998f61","job_id":"01a07e4e-6194-78b1-801e-032bc71809bf","error_code":null,"location_count":null,"changes_computed":false}
```

Same non-publication shape (`location_count: null`). Retry after restoring
the directory republishes 43 rows with zero changes (`generation: 8`). The
previous-results list is the only list shown; the failure is a safe
`RootUnavailable`, never a mass-missing conversion. Deterministic twins:
`denied_subtree_discards_staging_and_preserves_committed_rows` (portable
fake is the gate; the NTFS ACL twin skips loudly under elevated tokens),
`offline_root_fails_then_persisted_retry_converges`,
`durable_cancellation_between_batches_stops_without_publishing`,
`late_cancellation_after_commit_reports_completed_without_rollback`.

Rendered cancel/unavailable states (fake-adapter only) are in
`docs/review/phase-2-library/` (`desktop/narrow-cancelled-stale.png`,
queued/retried captures, `keyboard-trace.json` Cancel-lands-on-Retry with
the old list kept visible). They are not native evidence.

## 4. P2-10 verification (no parsing, hydration, or source mutation)

- Dependency check: `cargo tree -p fruitboard-scan-execution --prefix none`
  shows only `fruitboard-storage` (`rusqlite`, `serde`, `uuid`),
  `fruitboard-filesystem-enumeration` (`unicode-normalization`),
  `fruitboard-filesystem-watcher` (`windows-sys`), and
  `fruitboard-reconciliation`. No parser, archive, audio, or FLP crate
  appears. `crates/filesystem-enumeration/Cargo.toml` depends only on
  `unicode-normalization = "=0.1.25"`; `crates/scan-execution/Cargo.toml`
  adds only a test-only `rusqlite` dev-dependency for the SQL-failure
  fixture. `node --test` includes `research environment contains no PyFLP
  dependency` (passing).
- Content-read spies: `crates/filesystem-enumeration/src/lib.rs` imports
  only `CreateFileW`, `NtCreateFile`, `NtQueryDirectoryFile` (handle-bound
  metadata + directory listing). A source search for `ReadFile`,
  `NtReadFile`, `parse_flp`, `pyflp`/`PyFLP`, and `hydrat` finds zero
  matches in `crates/filesystem-enumeration/src` and
  `crates/scan-execution/src`.
- Source-byte preservation: `ntfs_worker_publishes_authoritative_rows_and_preserves_sources`
  reads the marker bytes before and after the authoritative scan and asserts
  byte equality (`source bytes are never read or written`); byte size on the
  committed row equals the pre-scan marker length. The P2-12 journey fixture
  files carry only the synthetic marker
  (`FRUITBOARD SYNTHETIC FIXTURE. NOT AN FL STUDIO PROJECT. NO PRIVATE
  DATA.`); no contents were hashed, hydrated, or parsed.
- Privacy: every surfaced error is a fixed `storage_*` code; no paths, SQL,
  or values cross the worker/IPC boundary
  (`follow_up_boundary_never_carries_path_data`, IPC event-privacy tests,
  `privacy_regression_no_public_type_can_carry_absolute_paths`).
  `journal_mode == delete` is asserted in storage tests; WAL stays disabled.

P2-10 stays Pending per instruction (dependency/privacy checks +
content-read spies + source-byte preservation exist but are not promoted by
this checkpoint alone).

## 5. Explicitly unverified (no claim in this checkpoint)

- 100k qualification set: unmeasurable under `MAX_STAGED_RECORDS = 10,000`
  (finding F3). No 100k claim.
- DriveFS modes (#47) and FAT32/cross-volume identity (#48): excluded from
  every claim. Watcher/enumeration fixtures record them as unverified
  (`drivefs_root_watch_fixture_is_unverified`,
  `real_buffer_overflow_fixture_is_unverified`,
  `acl_revocation_fixture_is_unverified`); benchmark limitations restate the
  exclusion.
- Baseline `10,005`-observation fixture vs `10,000`-record staging quota
  (finding F1): the accepted baseline preset cannot complete authoritatively
  as implemented.
- Warm-reconciliation p95 budget on laptop-class hardware (finding F2): miss
  reported honestly, hypotheses recorded, owner decision required.
- Native P2-08 flip: client `PlatformPort`/`LibraryScanAdapter` native
  implementation, Tauri capability/permission flip, typed IPC integration
  tests, and axe/keyboard/narrow evidence against real native
  statuses/pages (Agent B). Single-connection read-concurrency limitation
  (console commands serialize behind a running scan) is documented in
  `docs/review/phase-2-ipc/README.md` §5; the concurrency revisit is Agent A.
- Installed-app scan evidence: none. The only installed-app evidence in
  Phase 2 remains the #34 picker (`docs/review/issue-34/`) and #35 settings
  (`docs/review/issue-35/README.md`, fake adapter labeled separately).
- Network shares, non-NTFS filesystems, parsing/grouping/Kanban/playback/
  sync/PWA: out of scope for Phase 2 filesystem-only discovery.

## 6. Budget table (from `benchmark-2026-09-07.md`; provisional targets, not promises)

| Budget | Target | Measured | Verdict |
| --- | --- | --- | --- |
| Baseline first discovery | `<= 30 s` | 11,886 ms (Run B warm-up on the quota-fitting 10,000-observation fixture; Run A on the accepted baseline fixture not evaluable) | PASS |
| Unchanged warm reconciliation p95 | `<= 10 s` | 32,876 ms (median 12,875.5 ms, n = 10, nearest-rank p95 = max) | FAIL (F2) |
| Cooperative worker stop p95 | `<= 1 s` | 8 ms max over 6 mid-run samples (6-8 ms each) | PASS |
| Incremental private memory | `<= 128 MiB` | ~53 MiB incremental peak on the 10,000-location set | PASS with set caveat (F3) |
| Batch/staging `<= 512` records / `<= 256 MiB` | Enforced | 512-record enumerator hard bound; quota enforcement demonstrated by F1 (`ResourceLimit`, staging discarded, committed rows untouched) | Enforced by design; not directly observable through the worker public API; no violation observed |

Findings (honest, with hypotheses; owner decisions required):

- F1: accepted `baseline` preset yields 10,005 observations (10,000 files +
  5 hardlink aliases) against `MAX_STAGED_RECORDS = 10,000`. Every baseline
  run ends non-authoritative `Failed / ResourceLimit` at the quota boundary
  (warm-up 10,838 ms; measured 10,533-21,845 ms to quota failure — latency
  to failure, not scan latency). Decide: raise the staging quota above
  10,005 or define the baseline as exactly 10,000 observations including
  aliases. The budget row naming the baseline as 10,000 FLP-named files is
  invalidated as literally implemented.
- F2: warm p95 32.9 s (median 12.9 s) exceeds the 10 s provisional budget on
  the measured laptop host (i7-10750H, 2019, NVMe, active developer load:
  DriveFS sync, Defender real-time, agent tooling). Three of the first four
  measured iterations were slow (23.5-32.9 s); the rest were stable
  (~11.7-12.9 s). Suspects in order: background I/O/AV contention; per-file
  handle-bound metadata opens plus ancestor validation dominating 10k-file
  traversal; `synchronous = FULL` per-batch commit cost.
- F3: the 100,000-entry set cannot scan under the current quota, so the
  128 MiB-on-100k budget is unmeasurable today (passes on the 10k set with
  that caveat).

Machine profile, methodology deviations, and limitations are in
`benchmark-2026-09-07.md` (§Machine profile, §Methodology deviation,
§Limitations) and are not repeated here except to stress: laptop-class
hardware, non-idle host, no warm-cache control beyond one warm-up, ~100 ms
`tasklist` sampling, cooperative-token (not UI-acknowledgement)
cancellation, advisory `ChangeSummary` omitted by design at 10,000
locations, DriveFS/FAT32 excluded.

## 7. Reproducibility (this checkpoint)

```text
git checkout 74203a2
git checkout -b docs/41-p2-12-checkpoint
node scripts/generate-synthetic-tree.mjs --help
node -e "import('./scripts/generate-synthetic-tree.mjs').then(async m => { const plan = m.buildPlan({sizeLabel:'p2-12-journey', seed:'p2-12-journey-0', fileCount:40, leafDirectoryCount:4}); await m.writePlan(plan, '<empty temp dir outside repo>'); })"
cargo build --release -p fruitboard-scan-execution --example benchmark --locked
<benchmark.exe> --root <fixture> --db <temp db dir> --settle-ms 50
<benchmark.exe> --root <fixture> --db <temp db dir> --settle-ms 50 --cancel-after-ms 5
cargo test -p fruitboard-scan-execution --locked
cargo test -p fruitboard-storage --locked
cargo test -p fruitboard-filesystem-enumeration --locked
node --test tests/*.test.mjs
node scripts/verify-repository-privacy.mjs
pnpm lint:docs
```

Fixture temp dirs are removed on operator cleanup; the DB directory holds
one committed database across runs in §3 (restart survival by reuse).

## 8. Owner acceptance checklist for epic #33 close (blocking items explicit)

Epic #33 closes only on explicit owner acceptance of each line. Checked
boxes are evidence-complete; unchecked boxes block close.

- [x] P2-01 accepted (picker/settings installed + rendered evidence,
  #34/#35 precedent, fake adapter labeled).
- [x] P2-11 first measured report accepted as evidence with F1-F3 open
  (methodology + raw JSON + linked green CI run). Budgets remain
  provisional targets.
- [x] P2-12 checkpoint journey executed and recorded (§3) with honest
  commit/build/seed/hash/DB handling and fake-vs-installed labeling.
- [ ] Decide F1: raise `MAX_STAGED_RECORDS` above 10,005 or redefine the
  baseline as exactly 10,000 observations including aliases; amend the
  execution-plan budget row that is invalidated as implemented.
- [ ] Decide F2: accept investigation hypotheses or revise the 10 s warm
  p95 target for laptop-class hosts before claiming the budget met.
- [ ] Decide F3: scope the 128 MiB working-memory budget to the 10k set or
  schedule quota work that makes the 100k set measurable. No 100k claim
  until then.
- [ ] Exclude or evidence #47 (DriveFS modes) and #48 (FAT32/cross-volume
  identity). No blanket platform qualification until then.
- [ ] Accept P2-02/P2-04/P2-05 Partial as sufficient for close, or require
  the remaining fake-clock/synthetic-fixture coverage first.
- [ ] Accept P2-03/P2-06/P2-07/P2-09/P2-10 as Pending-at-close with named
  follow-ups, or require their integrated fault-injection/restart/alias/
  watcher/no-parser gates first.
- [ ] Accept P2-08 as Pending-at-close: native flip (Agent B: adapter +
  capability flip + typed IPC tests + axe/keyboard/narrow against native)
  and concurrency revisit (Agent A: single-connection serialization) land
  after #33, with production scanning staying hidden until P2-03 through
  P2-08 have integrated evidence.
- [ ] Confirm this checkpoint's scope discipline (README evidence table
  only + this new file; generator used, not modified; no
  `apps/**`/`crates/**`/`.github/**`/library/IPC review changes) and merge
  `docs/41-p2-12-checkpoint` into `main`.

Closing #41 is recommended once this checkpoint merges; closing epic #33
requires every unchecked box above to be explicitly accepted or scoped out
by the owner. No performance, platform, or production-activation claim
beyond what §§1-6 record is made here.

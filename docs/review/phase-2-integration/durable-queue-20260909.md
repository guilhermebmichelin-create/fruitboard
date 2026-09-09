# Phase 2 durable-queue and watcher-burst verification — 2026-09-09

Status: **verification record only.** This file records focused desktop
supervisor/scan-console integration evidence. It does not merge, close the
epic, change acceptance budgets, enable production scanning, or claim DriveFS,
real OS overflow timing, or blanket Phase 2 acceptance.

## Evidence boundary

Keep these classes separate:

- Merged base: `main` at `0b7612d` (merged).
- Unmerged dependency: PR #95 fix `bf0aeac`
  (`fix: align Library scan actions with durable state`, draft PR #95,
  unmerged until GitHub shows otherwise). This slice was verified against
  that dependency, not against `main` alone.
- Unmerged changes in this slice: test-only additions under
  `apps/desktop/src-tauri/src/foundation/` plus this report. No production
  application code, Library UI, enumeration code, migration, contract, or
  budget is changed.
- Automated integration evidence: the feature-gated desktop tests below, which
  drive the real supervisor, durable ledger, worker, and native status API
  over tempdir databases with scripted filesystem ports and fake clocks. No
  wall-clock sleep is used.
- Installed observations: preserved in
  `docs/review/phase-2-integration/installed-journey/run-20260909.md`
  (PR #95 repair replay). That file is not rewritten by this slice.
- Acceptance decisions: remain with the owner. F1/F2/F3, #47/#48 scope, and
  P2-03 through P2-12 promotion are untouched.

## Baseline

- PR #95 behavior is the baseline. Its native-authoritative `retryAvailable`,
  native render context, and cancelled/exhausted `Scan now` semantics are
  unchanged and are not re-tested here.
- Worktree: isolated `fruitboard-queue-verify-20260909` from
  `origin/fix/92-installed-scan-console` (`bf0aeac`), branch
  `test/95-durable-queue-watcher-20260909`.
- Build output: isolated `target/isolated-queue-verify` in that worktree. The
  shared `.tools` toolchains were used read-only. No other agent checkout was
  reset or switched. No cargo/tauri process was stopped. No antivirus,
  DriveFS, power, or system setting was changed.
- Packaging coordination: no new NSIS package was built in this slice, to
  avoid concurrent packaging with Agent 1 and to finish before Agent 3's
  benchmark window. The existing Phase 1 smoke bundle in the main checkout was
  left untouched. Installed-only behavior therefore inherits PR #95; the new
  durable behavior below is labeled automated integration evidence.

## Existing coverage inspected first

These native tests already exist on the baseline and were not duplicated:

- Queued `jobId`/null `runId` on `scan_now`:
  `apps/desktop/src-tauri/src/foundation/scan_console/tests.rs:408`
  (`scan_now_runs_to_completion_and_statuses_report_contract_fields`).
- Queued cancellation creates no run:
  `apps/desktop/src-tauri/src/foundation/scan_console/tests.rs:526`
  (`cancelling_a_queued_job_is_immediate_and_creates_no_run`).
- Scan coalescing (`already_queued`/`already_running`, follow-up queued):
  `apps/desktop/src-tauri/src/foundation/scan_console/tests.rs:922`
  (`scan_now_coalesces_and_rejects_unknown_and_disabled_roots`).
- Running-scan watcher burst sets one durable follow-up:
  `apps/desktop/src-tauri/src/foundation/watcher_supervisor_tests.rs:465`
  (`burst_activity_produces_one_durable_follow_up`).
- Coverage-loss precedence over activity:
  `apps/desktop/src-tauri/src/foundation/watcher_supervisor_tests.rs:499`
  (`coverage_loss_precedes_activity_in_the_real_supervisor_poll`).

Lower-layer halves already owned by their crates were also left alone:
storage durable queue/cancellation/follow-up matrices, scan-execution
burst/overflow/stale/restart/convergence tests, and watcher
coalescer/overflow/precedence tests.

## What was missing

| Layer | Gap closed here |
| --- | --- |
| Real supervisor | Idle-window watcher burst had no bounded-queue proof; only the running-scan follow-up-flag path was covered. Stale-generation replay after a disable/re-enable or a removal/re-add had no explicit no-work proof distinct from the pending-hint suppression and future-generation tests. |
| Durable ledger | The ledger already fences queued cancellation, coalescing, and stale/removed roots. What was missing was the desktop-driven proof that those fences surface through the supervisor without growing the queue. |
| Native status API | `scan_now` return-shape coverage existed, but no test proved that a mid-scan trigger converges through the status and Library API: interrupted first attempt publishes nothing, the follow-up is queued, and the later authoritative publication carries both the added and the removed locations. |
| Installed UI | Queued-state visibility, watcher burst-coalescing timing, and coverage-loss timing remain installed-only gaps (see below). No production delay was added to make the transient queued state visible. |

Library UI, enumeration performance code, and status summary documents were
intentionally not touched.

## New deterministic tests

All use injected fake clocks and the `claim_due`/`execute_pending` barrier or
explicit `now_ns` window control. No timing sleep is used.

### Scan console: mid-scan change converges authoritatively

`apps/desktop/src-tauri/src/foundation/scan_console/tests.rs:985`
(`changes_during_a_scan_converge_on_the_authoritative_follow_up`).

- Seeds one committed baseline (`a.flp` present) through the real
  `scan_now`/`tick` path.
- Holds the next scan durably running (`claim_due`), retriggers while running
  (`already_running`), then executes the first attempt with the unchanged
  tree. The attempt becomes `Interrupted`, schedules exactly one deduplicated
  follow-up, and publishes nothing: the Library still shows `a.flp` present
  (3 durable jobs: seed `Completed`, running `Interrupted`, follow-up
  `Queued`; status reports the follow-up as `queued` with null `runId`).
- Ticks the follow-up with the later state (`a.flp` removed, `b.flp` added).
  The authoritative full-root publication converges: status reports the
  follow-up as `completed` with a non-empty `runId`, and the Library page
  holds 2 rows with `a.flp` as `missing` and `b.flp` as `present`.
- Provenance: tempdir database per test (`fruitboard-scan-console-*`);
  in-memory scripted trees (`a.flp` id 101, `b.flp` id 202); fake clock from
  `T0_MS` with explicit `advance(1)` between chains for deterministic job
  ordering.

### Supervisor: idle burst stays bounded

`apps/desktop/src-tauri/src/foundation/watcher_supervisor_tests.rs:554`
(`idle_activity_burst_coalesces_to_the_single_queued_slot`).

- Records 5,000 raw activity signals in one coalescing window at `now=0`,
  polls at the window close (`1_000_000_000`): exactly 1 durable queued job
  for the root (startup gap and burst subsume to the single slot).
- Records a second 5,000-signal burst in the next window while that work is
  still queued, polls at `2_000_000_000`: still exactly 1 job.
- Provenance: tempdir database (`fruitboard-watcher-supervisor-tests-*`);
  fake watcher handles with `Coalescer` window `1_000_000_000`; controlled
  `Clock` at fixed `NOW_MS`; no thread, no sleep.

### Supervisor: stale generations cannot revive disabled or removed roots

`apps/desktop/src-tauri/src/foundation/watcher_supervisor_tests.rs:591`
(`stale_generations_cannot_revive_disabled_or_removed_roots`).

- Disables with no prior job, replays old-generation activity on the stopped
  handle, polls: 0 jobs.
- Re-enables to a strictly newer generation (gen 2): one fresh startup-gap
  job. Replays old generation 1 on the fresh handle, polls at the window
  close: still 1 job.
- Removes the root, re-adds the same canonical path as a replacement with a
  fresh durable id and fresh generation: one gap job for the replacement, one
  retained job for the old id. Replays the retired generation 1 on the new
  handle, polls: still 1 job for the replacement.
- Provenance: same tempdir/fake-handle/controlled-clock harness; durable
  `set_scan_root_enabled_at` / `remove_scan_root_at` / `add_scan_root`
  transitions; per-root `jobs_for` counts asserted at each step.

## Demonstration map

- A persisted queued job exists before a run is claimed: existing
  `scan_now` return-shape test plus the new convergence test's intermediate
  `queued`/null-`runId` status and the supervisor idle-burst test's single
  queued slot (1 job, 0 extra runs implied by the queued state).
- Queued cancellation creates no run and cannot publish: existing
  `cancelling_a_queued_job_is_immediate_and_creates_no_run` (no lease after
  cancel; status `cancelled` with null `runId`) plus the storage
  `integration_cancellation_uses_job_before_lease_and_run_after_lease` fence.
  No duplicate queued-cancel test was added.
- A watcher burst produces the contractually bounded follow-up work: existing
  running-burst test (one `follow_up_requested` flag, 1 job total) plus the
  new idle-burst test (5,000 + 5,000 signals collapse to 1 queued job).
- Changes arriving during a scan eventually converge: new convergence test
  (trigger while running becomes `Interrupted` with one queued follow-up;
  the follow-up publication carries the later added location).
- Stale-generation hints cannot revive disabled or removed roots: new stale
  test (0 jobs while disabled; 1 gap after re-enable with stale replay still
  1; replacement keeps 1 with retired-generation replay still 1).
- Coverage-loss hints request authoritative reconciliation: existing
  precedence/overflow tests (`overflow == 1`, coverage subsumes activity)
  plus the new convergence test's authoritative publication (unobserved
  `a.flp` becomes `missing`, added `b.flp` becomes `present`; triggers alone
  never mark absence). No real OS overflow timing is claimed.

## Synthetic fixture provenance (installed-equivalent leaf)

A disposable baseline was generated for record without building a package:

- Command:
  `node scripts/generate-synthetic-tree.mjs --size baseline --seed 20260909`.
- Manifest SHA-256:
  `bba313903d3ce4a873bca028649341c3ead6d13638d3f490171ca6aaf0b756d0`.
- Full baseline: 1,025 directories, 10,000 FLP-named files plus 4 others,
  5 hardlink groups.
- Installed-equivalent leaf `roots/shard-0000-sunset-beat`: 10 FLP-named
  files plus `readme-notes-0000.txt`; the committed Library page size of 4
  is exercised by this leaf shape.
- The isolated run directory under `%TEMP%` was kept out of the normal
  `com.fruitboard.desktop` data directory. No FLP bytes were modified by the
  generator consumer in this slice; the automated tests above use separate
  in-memory scripted trees (1-2 files) so fixture counts and test counts are
  not conflated.

## Validation

Isolated `CARGO_TARGET_DIR=target/isolated-queue-verify`, pinned toolchains
(Node 24.20.0, cargo 1.98.1):

- `cargo test -p fruitboard-desktop --features scan-console --locked`: 91
  passed (baseline 88 plus the 3 new tests).
- `cargo test -p fruitboard-storage --locked`: 76 passed.
- `cargo test -p fruitboard-scan-execution --locked`: 56 passed, 1 ignored
  (scale/perf harness gate).
- `cargo test -p fruitboard-filesystem-watcher --locked`: 32 passed,
  4 ignored (ACL/DriveFS/network/real-overflow manual fixtures).
- `cargo fmt --all -- --check`: passed.
- `cargo clippy -p fruitboard-desktop --features scan-console --all-targets
  --locked -- -D warnings`: passed.
- `git diff --check`: passed (verified before commit).

Full `pnpm check` was not rerun in this slice because it includes the shared
desktop build owned in parallel; the focused feature-gated suite plus the
three storage-adjacent suites above are the relevant validation for these
test-only changes. The draft PR records this scope explicitly.

## Remaining installed-only gap

- The installed queued snapshot remains unobserved: the synthetic scan
  completes too quickly in the packaged app and no production delay was added
  to make it visible. The durable queued state is therefore automated
  integration evidence in this slice, not an installed observation.
- Independent watcher burst-coalescing measurement in the installed app
  remains unverified, as does coverage-loss timing.
- Restart-during-scan re-verification on the repaired build and
  unavailable-root retention beyond the repairs below remain installed-only
  gaps. The earlier successful installed observations are preserved, not
  redefined: PR #92 (`run-20260908.md`) observed abrupt-termination
  recovery (installed process terminated mid-`Running`; the run was
  retained as `interrupted` with `error_code=restart` and the same durable
  job advanced to attempt 2 and completed) and unavailable-root retention
  (`Execution Failed` with `The folder is unavailable. Previous committed
  results were kept.`, prior rows kept, no auto-retry); PR #95
  (`installed-journey/run-20260909.md`) re-observed the D1-D3 repair paths
  plus the disabled-root replay on the rebuilt installer. What this slice
  does not add is a repaired-build restart kill or a repaired-build repeat
  of the unavailable-root retention beyond that replay.
- DriveFS/mirrored/streamed roots (#47), cross-volume/FAT32 identity (#48),
  network shares, ACL revocation during a watch, and real OS
  notification-buffer overflow timing are explicitly not claimed.
- Benchmark F1/F2/F3 and owner acceptance remain with Agent 3's window and
  the owner respectively. This slice changes no quota, budget, fixture
  preset, or acceptance criterion.

## Files changed

- `apps/desktop/src-tauri/src/foundation/scan_console/tests.rs`: one new
  convergence test (no production change).
- `apps/desktop/src-tauri/src/foundation/watcher_supervisor_tests.rs`: two
  new supervisor tests (no production change).
- `docs/review/phase-2-integration/durable-queue-20260909.md`: this report.

Original installed reports are preserved and unmodified.

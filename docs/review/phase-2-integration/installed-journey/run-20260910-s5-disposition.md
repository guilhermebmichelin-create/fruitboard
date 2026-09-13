# S5 restart-semantics disposition — 2026-09-10

Status: **Additive analysis only; no merge, no production scanning, no Phase 2
acceptance.** Agent 1 owns merges. This file dispositions issue #107 (S5
PARTIAL in PR #106) from the preserved artifacts and the binding contracts.
It adds one pinning regression and no product fix.

## Verdict

S5 is **expected behavior**, not a defect and not unresolved evidence. The
observed run became terminally interrupted **before** the process was killed,
via the harness's own coalesced trigger, and the successor job is the
contract-correct continuation. Successor recovery must not be labeled a
defect merely because the IDs differ, and no binding requirement is weakened.

## What the preserved evidence proves

Artifacts (read-only, under
`%TEMP%/fruitboard-fixed-20260909-20260909-154848`):

- Transcript `fixed-clean.jsonl` (135 lines), `s5-job.txt`
  (`01a087a4-9cf5-76d1-847e-d658cc8be729`), `s5-db-copy.db` (copied after
  kill, before relaunch), `phase5.mjs` (harness).
- Live run at unmerged candidate `ded02ac` (PR #106, patch-identical to PR
  #104 head `41ac09b`); lock helper from merged PR #103 only.

Sequence (transcript kinds + database wall-clock times, UTC):

1. `S5-scan-now` (19:28:30.965Z) on the cancel root returned
   `already_running` with job `...9cf5...` / run `...a116...`. Per
   `crates/storage-sqlite/src/execution.rs:751` (`enqueue_scan_tx`,
   active-`running` branch), this call set `follow_up_requested = 1` on the
   already-running startup-recovery job and discarded its open staging. The
   run under test was therefore invalidated by the harness's own probe.
2. `S5-running-observed` (19:28:30.977Z) saw `running`, 4608/8000 files.
   The flag from step 1 was already set; the run was doomed to finish as
   non-authoritative.
3. The worker finished at 19:28:31.155Z (`finished_at_ms 1788982111155`):
   run `...a116...` → `interrupted`, outcome `interrupted`, error
   `follow_up_requested`, staging `discarded`; job `...9cf5...` →
   `interrupted` 1/4, error `follow_up_requested`, `follow_up_requested = 1`;
   fresh successor `...abb4...` (`recovery`, `queued` 0/4) created in the
   same finish transaction (identical `created_at_ms 1788982111155`). This
   is the documented follow-up path in
   `crates/storage-sqlite/src/execution.rs:1376`
   (`invalidated_by_follow_up` → `Interrupted` + `enqueue_scan_tx`) and
   `docs/PHASE_2_DURABLE_EXECUTION.md:22` ("one fresh queued job is created
   for the follow-up").
4. `S5-terminated` (19:28:32.906Z, `taskkill /F`, verified gone) happened
   ~1.75 s **after** step 3. `s5-db-copy.db` (copied 19:28:32.914Z, before
   relaunch) holds **0 `running` runs**, the target run already
   `interrupted`/`follow_up_requested`, and two queued successors
   (`...9d63...` primary periodic, `...abb4...` cancel recovery). A crash
   fence would have written error `restart`/`recovery` via
   `recover_interrupted_tx` (`execution.rs:484`); the stored
   `follow_up_requested` proves the terminal finish preceded the kill.
5. After relaunch the old job stayed `interrupted` 1/4 while successor
   `...abb4...` reached `completed` 1/4 with the committed page intact
   (cancel root 8000 rows, UI `native`). No wedge, no loss.

Generation/watcher check: run generation 8 equals root generation 8; no
mismatch. The invalidation trigger here was the manual `scan_now`
coalescing, not a filesystem watcher event — both use the same
`follow_up_requested` flag, so no separate watcher explanation is needed
and none is claimed. The pre-existing primary queued job `...9d63...`
(periodoid, created 19:28:27.491Z, 111 ms after session start, before
`S5-scan-now`) does not touch the cancel-root lineage.

## Same-job versus successor: the binding rule

- **Same-job requeue is required** when restart finds a run still in
  `running` from the prior session (genuine crash mid-lease). Then
  `recover_interrupted_tx` moves run → `interrupted` (`restart`) with
  staging discarded, and the **same job ID / retry chain** → `queued` with
  persisted attempt budget and backoff. Pinned by
  `crates/storage-sqlite/src/tests.rs:1855`
  (`restart_requeues_the_interrupted_attempt_without_resetting_its_chain`)
  and `apps/desktop/src-tauri/src/foundation/scan_console/tests.rs:690`
  (`restart_marks_interrupted_work_and_resumes_it_through_recovery`, which
  asserts a fresh **run** for the resumed **same job**).
- **A new recovery job is valid** when (a) the running attempt was
  invalidated during traversal (`follow_up_requested` via `already_running`
  coalescing or a watcher hint) and finished as terminally interrupted
  before the crash — the successor queued at finish time is the
  continuation and the old job stays interrupted (active-slot guard in
  `resume_interrupted_jobs_tx`, `execution.rs:596`, plus recovery
  suppression in `enqueue_recovery_jobs_tx`, `execution.rs:665`); or (b) an
  enabled root has no eligible interrupted/queued work and gets one
  deduplicated `recovery` scan. S5 matches (a).

## Contract source for "same job must complete"

The strict expectation ("terminate while `running`, then require the **same
job** to reach `completed` within 120 s") comes from the agent-created test
plan in PR #102 (`run-20260909-queue-stall-validation.md` §8 item 5), not
from a binding contract. The binding contracts require same-job **requeue**
only for genuinely crashed running leases
(`docs/PHASE_2_EXECUTION_PLAN.md:120`,
`docs/PHASE_2_DURABLE_EXECUTION.md:41`), and separately require a **fresh
follow-up job** when a trigger arrives during traversal
(`docs/PHASE_2_DURABLE_EXECUTION.md:22`,
`apps/desktop/src-tauri/src/foundation/scan_console/tests.rs:949`). The
§8 item-5 wording is invalid for executions where the harness itself
invalidates the run before the kill, as S5's own `already_running` outcome
proves happened here.

## Why no new installed experiment was run

The task's experiment clause applies only when existing evidence cannot
distinguish the sequence. It can: the post-termination/pre-relaunch copy
already contains the exact missing observation — 0 running runs, target
`interrupted`/`follow_up_requested` with `finished_at_ms` 1.75 s before the
kill, successor queued at the same finish millisecond. No quiescent-root
re-run under the exclusive smoke lock (`scripts/foundation-smoke-lock.ps1`)
was needed, and none was performed (no `fruitboard-desktop` process was
launched or killed; no lock was acquired; the preserved `%TEMP%` evidence
and live smoke data were opened read-only or left untouched). A clean
same-job crash demonstration (quiescent root, no intervening `scan_now`
during the running window) remains a Phase 2 acceptance item, not a merge
gate — see below.

## Regression test (no product fix)

No defect exists, so no product code was changed. The smallest justified
change is a deterministic pin against future mislabeling:

- `crates/storage-sqlite/src/tests.rs`:
  `kill_after_follow_up_invalidation_keeps_successor_and_leaves_old_interrupted`
  replays the S5 shape (lease → coalesced `already_running` trigger →
  finish as `interrupted`/`follow_up_requested` + successor → close →
  `begin_scan_session`) and asserts the old job/run stay terminally
  interrupted, no duplicate recovery is created, and `lease_next_scan`
  returns the successor on a fresh chain.
- Results (pinned Windows toolchain): `cargo test -p fruitboard-storage
  --locked --lib` **77 passed** (76 existing + 1 new);
  `cargo clippy -p fruitboard-storage --all-targets --locked -- -D warnings`
  clean; `cargo fmt -p fruitboard-storage -- --check` clean.

## Statement for Agent 1 on PR #104

**PR #104 (`fix/41-queue-stall-retry-sweep`, head `00bb3e8`) is unaffected
by S5 and needs no change for it.** The #104 fix touches only
`retry_failed_scan_job` (failed → queued skip when the active slot is
owned); S5 exercises only the running → interrupted → follow-up path
(`finish_scan_run`) and crash fencing (`recover_interrupted_tx`), with no
failed job involved. S5's worker converged through the successor with rows
intact, confirming the #104 stall shape is independent. **Merge #104 on its
own checks; track S5 only as Phase 2 acceptance** (P2-06 still needs one
clean same-job crash-recovery demonstration with a quiescent root and no
intervening trigger; issue #107 is the correct tracker, not a #104
blocker).

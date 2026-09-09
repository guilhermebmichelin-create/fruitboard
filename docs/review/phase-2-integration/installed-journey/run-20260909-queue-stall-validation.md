# Independent queue-stall validation — 2026-09-09 (PR #101 baseline, frozen 9c211ad)

Status: **Evidence only; no merge, no production scanning, no Phase 2 acceptance,
no recovery claim.**

This record independently validates the worker queue-stall finding reported in
PR #101 (`run-20260909-final-regression.md`) against the preserved clean-run
transcript and the preserved live-database copy. It adds no production-code
change, preserves the failed baseline report unchanged, and makes no fix
verification claim: no Agent 1 fixed SHA was available when this was written,
so the six fix-replay scenarios are defined as a bounded protocol and marked
**PENDING**.

## 1. Provenance and freeze

- Baseline report: PR #101, branch `docs/41-final-regression-20260909`,
  commit `c3d3292`, file
  `docs/review/phase-2-integration/installed-journey/run-20260909-final-regression.md`.
  That file is preserved unchanged by this record.
- Frozen integration SHA: `9c211ad` (PR #99 head). The preserved clean-run
  transcript opens with `clean-start` recording commit
  `9c211adeb555aeacd6a12cb074cca5d359b9f0d7`; independent parsing confirms
  the recorded commit equals the frozen SHA.
- Preserved artifacts inspected read-only on 2026-09-09 (UTC):
  - clean transcript: run-specific journey root under `%TEMP%`
    (`fruitboard-final-20260909-20260909-135637`), file
    `regression-clean.jsonl`, 376 lines, span `2026-09-09T17:38:38.558Z` to
    `2026-09-09T17:42:48.247Z` (last line is a CDP `fatal` timeout, see §5).
  - polluted transcript: same journey root, `regression-full.jsonl`,
    1037 lines (four-root shared-database run, see §3).
  - live-database copy: dedicated Foundation Smoke identity under
    `%LOCALAPPDATA%\com.fruitboard.desktop.foundation-smoke`,
    `storage\fruitboard.db`, 14,954,496 bytes, SHA-256
    `52125A09…7F1` (case-insensitive match to the §6 value in the baseline
    report; computed locally read-only). `owner.lock` present. No write was
    performed against this database during validation (SQLite `mode=ro`).
  - command log: same Smoke identity, `logs\fruitboard.log`, 361 records
    (360 `info`, 1 `error`).
- Tooling for this validation is independent of the baseline run scripts
  (`regression-clean.mjs`, `regression-full.mjs`, `q2.py`, `q3.py`, `q4.py`
  were read but not executed or modified). New read-only inspectors were
  written for this task and retained outside the repository under the
  operator `%TEMP%` workspace: transcript parser, read-only SQLite checks
  (pinned interpreter, SQLite 3.53.1), and log counter. They perform no
  writes, open no database read-write handle, and launch no application.
- Repository state for this record: new additive branch from `origin/main`
  containing only this file. No scheduling, storage, worker, watcher, or
  command code was modified.

## 2. Method classes (kept separate)

- **UI actions:** none in the preserved clean run and none in this
  validation. No DOM clicks, picker interactions, or Library navigation were
  driven; the baseline observations come from programmatic probes, not from
  the renderer UI.
- **Direct native commands:** all preserved clean-run observations were made
  through the installed app's Tauri bridge via loopback WebView2 remote
  debugging (`Runtime.evaluate` calling
  `window.__TAURI_INTERNALS__.invoke` for `get_scan_console_state`,
  `add_scan_root`, `scan_now`, `retry_scan`, `list_scan_statuses`,
  `get_library_page`). One transcript line uses the older
  `window.__TAURI__.core.invoke` shape and fails; the run uses the
  `__TAURI_INTERNALS__` shape throughout. This validation re-parses those
  recorded command results; it issues no new commands to any live app.
- **Database inspection:** read-only SQLite queries against the preserved
  copy (job/run/session/root/stage/location tables) plus read-only log
  counting. Separately listed in §4 with full job/run provenance.
- **Authoritative completion rule:** a responsive status API alone never
  counts as worker progress. Only a durable `running` lease followed by a
  terminal `completed` run with published Library rows counts as
  convergence. Every scenario below applies that rule.

## 3. Clean run versus shared-database contamination

- The polluted transcript (`regression-full.jsonl`) records the
  shared-directory collision described in the baseline report §3: its
  `roots-after-add` entry lists foreign roots `Regression Primary` and
  `Regression Cancel` (fixture paths under a different `%TEMP%` regression
  root `fruitboard-installed-regression-20260909-102110`) alongside the run's
  own `Final Primary`. Four roots were present in that first live database
  before it was reversibly archived.
- The clean transcript (`regression-clean.jsonl`) records only two roots:
  `Clean Primary` (ten-file leaf) and `Clean Cancel` (large subtree). Its
  `add-primary` / `add-cancel` entries, all later status polls, and the
  preserved database copy (exactly two `scan_root` rows) agree on that
  two-root shape.
- All stall observations in §4–§5 come from the clean transcript and the
  two-root database copy. No polluted-run state is used as stall evidence.
  The polluted database archive was not opened for this validation beyond
  confirming it remains archived; its contents are not claimed here.

## 4. Independent database verification (preserved copy, read-only)

Job states, due times, sessions, leases, and absence of running work were
verified directly; transcript values are cross-checked, not trusted alone.

- **Roots (2):** `Clean Primary` enabled/available, generation 10,
  configuration revision 0; `Clean Cancel` enabled/available, generation 1,
  revision 0.
- **Jobs (9 total):** 5 terminal `completed`, 2 terminal `failed`
  (one exhausted `...2001...` at attempt 4/4 with `worker_failed`, one
  single-attempt `...80fa...` with `worker_failed`), 2 active `queued`:
  - `...8908...` (primary, kind `periodic`, attempt 0/4,
    `not_before_ms` `17:39:08.680Z`, `follow_up_requested` 0,
    `cancellation_requested` 0, no `last_error_code`). Created at
    `17:39:08.680Z`, never updated afterward.
  - `...63bc...` (cancel root, kind `manual`, attempt 0/4,
    `not_before_ms` `17:41:10.204Z`, flags 0/0, no error). Created at
    `17:41:10.204Z`, never updated afterward.
- **Due-time check:** both active jobs carry `attempt < max_attempts`,
  `cancellation_requested = 0`, enabled roots, and `not_before_ms` already
  past at every later observation (transcript polls continue to
  `17:42:27Z`; the copy was closed at `17:41:44Z` file time with the same
  rows). Under the documented `lease_next_scan` eligibility (queued,
  uncancelled, under budget, due, enabled root, no running run), both jobs
  were eligible for the entire stall window. No `running` job blocked the
  single-worker slot (see next).
- **Runs (11 total):** 5 `completed`, 6 `failed`; **0 `running`**. The four
  attempts of the exhausted chain, the two auto-recovery attempts
  (`...8089...` failed attempt 1, `...88a0...` completed attempt 2 at
  `17:39:08.599Z`), and the early cancel-root recovery (`...200a...`
  completed at `17:38:46.243Z`, 8000 rows) are all terminal with
  `finished_at_ms` set. The two stalled jobs have **no run row at all**
  (attempt 0, never leased), so there is no lease token or deadline to
  inspect for them — the absence of any lease is the finding.
- **Sessions (2):** first session `...17be...` started `17:38:39.678Z`,
  ended `17:41:44.122Z` (forced-kill time); second session `...e83a...`
  started `17:41:44.122Z`, `ended_at_ms` NULL. The NULL end is the expected
  shape for a process that has not yet started a successor session (session
  ends are written at the next `begin_scan_session`, not at exit); it is not
  evidence of running work.
- **Stages (11):** 5 `published`, 6 `discarded`, 0 `open`. Every run has a
  settled stage; no provisional staging is left publishable.
- **Committed Library rows (8010 total):** 10 `present` for the primary
  root, 8000 `present` for the cancel root, 0 `missing`. The primary page
  stayed at 10 rows through the stall (burst rows missing); the cancel-root
  8000 rows are the retained committed snapshot from the early automatic
  recovery scan.
- **Cross-check to transcript:** the stalled job IDs, states (`queued`,
  null `runId`), counters, and timestamps in the transcript polls equal the
  database rows above. The transcript's `lastSuccessfulScanAt`
  `17:39:08.599Z` / `lastOutcomeAt` `17:39:08.68Z` for the primary matches
  the auto-recovery completion and the `...8908...` creation time.

## 5. Independent transcript verification (bounded windows)

- **Unavailable → automatic recovery (observed, authoritative):** primary
  moved away, `scan_now` job `...807f...`, failed attempt 1, automatic retry
  completed attempt 2 at `17:39:08.599Z` with 10 rows. This half converges
  and is confirmed.
- **Watcher follow-up after that recovery (stalled, not converging):** the
  next explicit `scan_now` coalesced to `already_queued` on `...8908...`.
  117 consecutive `poll-UF` records from `17:39:09.591Z` to `17:40:09.083Z`
  show the same job ID, state `queued`, null `runId` every time, counters
  frozen at files 10 / directories 0 / total null. No `running` was ever
  observed and no new run row exists. Window: ~60 s of explicit polling
  inside a >3 min total stall (job due `17:39:08.680Z`, still queued at
  close). **No explicit-retry convergence is claimed.**
- **Repeated manual scans after that recovery (stalled):** every later
  `scan_now` during the window coalesced to the same `...8908...`
  (`already_queued`); none created a new job or run. Distinguished from
  recovery: the durable automatic retry is the only completion in this
  window.
- **Burst changes (not converged, installed):** five synthetic `.flp` files
  created at `17:40:09.590Z`; 30 `poll-burst` records to `17:41:08.177Z`
  plus 30 interleaved page checks, all `queued` on `...8908...` with page
  total 10 and burst-marker absent every time (`BURST-ROWS-MISSING`).
  No follow-up ran while the app was running. The at-most-one
  queued-follow-up bound is therefore not independently counted here;
  deterministic suites remain automated-only evidence.
- **Two roots making progress without one starving (not demonstrated):**
  the large-root `scan-I` job `...63bc...` stayed `queued` (59 polls,
  `17:41:10.223Z`–`17:41:40.005Z`, counters showing the retained 8000
  committed rows, never `running`) while the primary stall was already in
  effect. Neither root progressed after `17:39:08Z`; starvation-freedom
  cannot be assessed from a fully stalled worker.
- **Termination while actually Running, then restart and convergence (not
  demonstrated):** the kill at `17:41:40.509Z` happened while `...63bc...`
  was still `queued` with null `runId` (59/59 polls queued; `I-running?`
  state `queued`). After relaunch (`17:41:43.694Z`) the same job was
  retained as `queued` with counters reset to 0 and the committed page
  intact, then stayed `queued` for the full 80-poll / 120 s recovery window
  (`17:41:47.245Z`–`17:42:27.731Z`, zero terminal states) until the final
  CDP evaluation timed out at `17:42:48.247Z` (`fatal: timeout
  Runtime.evaluate`). Recovery-identity (same job, no loss) is preserved;
  restart convergence is not.
- **Cancellation and exhausted-chain fresh scans (observed, authoritative):**
  cancel → `cancelled` with retained snapshot, fresh `scan_now` completing
  on a different job; exhausted `...2001...` (`failed` 4/4,
  `retryAvailable false`) preserved while fresh `...7df1...` completed with
  a different `retry_chain_id`; `retry_scan` on the exhausted job returning
  typed `conflict`; disabled/removed-root `conflict`/`not_found` paths.
  These match the baseline report §§4.2–4.4 and are confirmed in both
  transcript and database.

## 6. Status-API responsiveness versus worker progress

The command log proves the app stayed responsive while the worker made no
progress: 313 `list_scan_statuses` completions, 34 `get_library_page`
completions, 6 `scan_now` completions, and 1 typed `retry_scan` conflict
(`scan_job_conflict`) across the run, with exactly one `error`-level record
(the expected conflict). Every stall-window poll returned promptly with a
well-formed `queued` status while job state, run existence, counters, and
Library rows never advanced. Responsiveness is therefore recorded as a
command-layer observation only; it is not worker-progress evidence.

## 7. Coordination and live-run boundary

- Before inspecting, the operator checked for running
  `fruitboard-desktop` processes: none found. No process was stopped,
  closed, or signalled to obtain a test window.
- The preserved live database and log were opened strictly read-only; no
  archive, move, restore, install, or uninstall was performed for this
  validation. The final database, both smoke archives, fixtures, and both
  transcripts remain as the baseline run left them.
- No new live installed run is included in this record. That is deliberate:
  the preserved clean run is already a bounded live run at the frozen SHA
  with full job/run provenance, and a fresh install/rebuild now would risk
  disturbing the preserved evidence and any concurrent agent's exclusive
  window. A fresh isolated-identity live run (own journey root, own fixture
  seeds, reversible smoke-directory archive, no interference with another
  agent's process) is deferred to the fix-verification run in §8.

## 8. Fix-verification protocol (PENDING — no fixed SHA available)

A search of open PRs at validation time (92, 93, 94, 95, 96, 97, 98, 99,
100, 101) found no Agent 1 queue-scheduling fix and no fixed SHA to test.
The following bounded replay is therefore specified but not executed. Each
item requires authoritative completion (a terminal `completed` run plus the
stated committed Library rows); status responsiveness alone never passes.

1. **Unavailable → automatic recovery → watcher follow-up completion:**
   move the primary leaf away, `scan_now`, restore within seconds, await
   automatic-retry `completed` without operator action (bound 60 s), then
   issue one explicit `scan_now` and require it to reach `completed` (not
   `already_queued`-forever) with 10 committed rows (bound 120 s).
2. **Repeated manual scans after that recovery:** issue three sequential
   `scan_now` calls after convergence, each required to reach its own
   terminal `completed` (or coalesce-then-complete within the bound) with
   unchanged 10-row Library content (bound 60 s each).
3. **Burst convergence to committed rows:** create five synthetic `.flp`
   files in burst succession, require exactly one follow-up to run to
   `completed` and the Library page to show 15 rows including the burst
   markers (bound 120 s), then remove the burst files and require
   reconvergence to 10 rows.
4. **Two roots without starvation:** start overlapping manual scans on both
   roots, require both to reach `completed` within the bound (120 s) with
   10 and 8000 committed rows respectively; record per-root job/run IDs to
   rule out one root starving.
5. **Termination while actually Running, then restart and convergence:**
   start the large-root scan, await observed `running` with non-null
   `runId`, terminate the exact process, verify the database copy shows no
   `running` run, relaunch, and require the same job to reach `completed`
   within 120 s with the committed page intact. Killing while `queued`
   does not satisfy this item.
6. **Cancellation and exhausted-chain fresh scans staying correct:**
   cancel a `running` scan to `cancelled` with snapshot retained, exhaust a
   moved-away chain to `failed` 4/4, verify `retry_scan` on the exhausted
   job stays `conflict`, and require a fresh `scan_now` on a new chain to
   reach `completed` with correct rows.

Timestamps and full job/run/session/lease provenance (job ID, run ID,
attempt, `not_before_ms`, lease token/deadline, session ID, outcome, error
code, counters, Library totals) must be retained for each item, with UI
actions, direct native commands, and database inspections labelled as in
§2. Recovery is successful only when work reaches authoritative completion
as defined above.

## 9. Uncertainties (not facts)

- The stall mechanism is **not determined** by this validation. The evidence
  shows eligible-but-never-leased jobs with no running work while the
  command layer stays responsive; it does not instrument whether the
  poll-loop thread died, blocked, lost its session, or stopped claiming.
  The baseline report's §5 wording is treated as a blocker description, not
  as a diagnosed root cause, and no causal claim is repeated here.
- Package hashes, fixture manifest hashes, and toolchain versions in the
  baseline report §§1–3 are quoted, not re-verified; no rebuild was run for
  this record.
- The polluted-database archive was not deep-inspected; contamination
  separation rests on transcript root listings and the clean two-root
  database shape.
- Wall-clock versus database-clock skew was not independently calibrated;
  due-past reasoning uses transcript ISO times against stored
  `not_before_ms` converted to UTC, with minute-scale margins that exceed
  any plausible skew.
- The second session's NULL `ended_at_ms` and the post-relaunch counter
  reset (8000 → 0 on the still-`queued` job) are recorded as observed
  durable shapes; their spec-conformance is not adjudicated here.

## 10. Cleanup and boundaries

- This validation performed no install, uninstall, process termination,
  fixture mutation, or database write. Only the run-specific operator
  `%TEMP%` inspector scripts and their JSON output were created, outside
  the repository.
- Production scanning was not enabled and no default-build behavior was
  exercised: all evidence concerns the feature-enabled Foundation Smoke
  identity. No merge was performed, no budget or quota was amended, and
  Phase 2 is not declared accepted.
- DriveFS (#47), FAT32/cross-volume identity (#48), network shares, ACL
  revocation during a watch, real OS buffer-overflow timing, and the
  100,000-entry qualification were not run and are not claimed.
- Evidence classes stay separate: frozen commit plus remote CI (PR #99),
  deterministic suites (automated only), preserved installed observations
  (PR #101, validated here), this independent validation record, and owner
  acceptance remain distinct.

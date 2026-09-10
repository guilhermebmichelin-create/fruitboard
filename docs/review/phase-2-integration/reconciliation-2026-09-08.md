# Phase 2 status reconciliation - 2026-09-08 (refreshed 2026-09-09, third wave)

Status: **evidence reconciliation only; Phase 2 is not accepted.** This report
audits the current status documents against the fetched `origin/main`,
live GitHub PR/issue/CI records, merged source and tests, the benchmark re-run,
and the owner decisions that are actually recorded. It does not close an issue,
change a feature gate, amend a budget, or supply an owner decision.

This reconciliation does not edit the installed-app checklist, benchmark
evidence and raw JSON files, platform research, application code, or production
gates. The checklist observations are recorded in unmerged PR #92; the platform
reports are in unmerged PR #90; the diagnostic profile is in unmerged PR #93;
the stacked optimization is in unmerged PR #94; the D1–D3 repairs are in unmerged
PR #95; the F1/F3 scale design is proposed in unmerged draft PR #96; the
durable-queue and watcher-burst integration tests are in unmerged draft PR #97;
and the PR #94 validation with diagnostic cleanup is in unmerged draft PR #98.
Historical checkpoint, continuation, acceptance-preparation, benchmark,
post-merge, and platform-research records remain historical records and are not
rewritten by this refresh.

## 2026-09-09 refresh addendum

`origin/main` re-verified 2026-09-09 local time remains
`0b7612db3570e6235d4d2c86a678dd9004264f30` (PR #89); no new merges. Open
drafts are now #90, #91, #92, #93, #94, and #95 (plus new proposal draft #96);
open issues remain #33, #36–#41, #47, #48. Deltas since 2026-09-08:

- D1–D3 repairs are verified in still-unmerged draft PR #95 head `bf0aeac`
  (Foundation 34309511415 + packaging 34309511404 passed; `pnpm check`, 134
  client tests, 88 feature-on desktop tests, additive `run-20260909.md`
  installed replay). The replay retained no durable queued snapshot; ACL,
  DriveFS, cross-volume/FAT32, 100,000-entry, and burst-timing cases remain
  outside its scope. PR #95 remains open and unmerged.
- PR #92 packaging rerun attempt 2 passed
  ([job 102314720612](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34299680193/job/102314720612));
  the original timeout ([attempt
  1](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34299680193/job/102303731791))
  remains recorded. No product fix or acceptance is inferred.
- PR #94 head `588867b` (three commits on base
  `docs/41-performance-followup-20260908`) is stacked on #93 and has no
  exact-head CI yet. Its contended whole-scan median moved 11,735.5 ms to
  11,517 ms while p95/max worsened 12,188 ms to 17,682 ms on an outlier; it
  needs combined validation with #93 plus a quiet-host A/B rerun and is not a
  qualification.
- Durable queued and watcher-burst evidence is being addressed independently
  (including the `test/95-durable-queue-watcher-20260909` worktree) and is not
  claimed here; P2-04/P2-09 remain Partial.
- Platform prerequisites (#47 Mirror/Stream consent, #48 genuine FAT32
  volume) and F1/F3 scale decisions remain unresolved. The concrete
  bounded-scan proposal (F1 exact-10,000 fixture vs quota alternative; F3
  chunked snapshot with coverage ledger, fencing, bounds, atomic publication,
  cancel/crash/expiry/cleanup/backup, slices, adversarial tests, and
  quota/deferral comparison) is in draft
  [PR #96](https://github.com/guilhermebmichelin-create/fruitboard/pull/96)
  as a proposal only. No acceptance is promoted by this refresh.

## 2026-09-09 second refresh addendum (independent wave)

`origin/main` re-verified 2026-09-09 local time remains
`0b7612db3570e6235d4d2c86a678dd9004264f30` (PR #89); no new merges. Open
drafts are now #90 through #98 (this reconciliation is #91); open issues
remain #33, #36–#41, #47, #48. Deltas since the first 2026-09-09 refresh:

- PR #97 head `aba1812` (`test/95-durable-queue-watcher-20260909`, base PR #95
  `bf0aeac` on `main` `0b7612d`) adds automated integration evidence only: one
  mid-scan convergence test through the real supervisor plus native
  status/Library API (`Interrupted` first attempt publishes nothing, one queued
  follow-up, authoritative follow-up carries the added location and the removed
  location as `missing`), one idle-burst test (5,000 + 5,000 signals collapse
  to exactly one queued job), and one stale-generation test (0 jobs while
  disabled; 1 gap after re-enable with stale replay still 1; replacement keeps
  1 with retired-generation replay still 1). No production, Library UI,
  enumeration, migration, contract, budget, or acceptance change. Foundation
  run `34339792661` and packaging run `34339792759` are green. Full `pnpm check`
  was not rerun there (shared desktop build owned in parallel); the slice
  records its focused-suite scope explicitly. Installed queued visibility,
  watcher burst timing, restart-during-scan handling, and unavailable-root
  retention beyond PR #95 remain unverified installed-only gaps. P2-04/P2-09
  ledger entries below record this as automated evidence, not installed
  observation.
- PR #98 head `15b7f17` (`perf/94-validation-20260909`) independently validates
  PR #94 without another speculative optimization: committed before/candidate
  stats recomputed and matched, identical fixtures/toolchains/profiles/flags
  confirmed, no nested-timer double counting, instrumented traversal separated
  from uninstrumented full scans, and the single-query saving explained as tens
  of milliseconds (attributes already in basic info; tag value never consumed).
  New contended-host 1+10+3 A/B on identical `custom-9995` fails the 10 s p95
  on both sides; no qualification is claimed. The branch removes only dead
  diagnostic scaffolding (8 deletions; diagnostics remain `cfg`-gated) and
  requests Agent 1 review without touching the PR #94 branch. CI for the
  validation head was separately dispatched at this refresh and is not cited
  as green until its runs complete. Recommendation recorded there is
  revise-then-keep as cleanup, not a performance win.
- Agent 1 integration evidence (PR #95 owner): no new head beyond `bf0aeac`
  at this refresh. The D1–D3 repairs stay verified-but-unmerged; durable queued
  observation and independent burst counts are carried by the independent PR
  #97 work, not by a new Agent 1 commit. Rebuilding PR #95 from the then-merged
  `main` where the owner requires it remains an open step.
- PR #96 proposal correction head `9aa97ab` (on base `3be0140`): independent
  review owning six unresolved mechanisms (bounded global identity planning,
  coverage denominator, cross-page hardlink/rename/conflict handling,
  lease/cancellation fencing inside the final transaction, publication lock
  duration under the single-connection `DELETE`-journal architecture, and
  disk/cleanup/backup bounds) with concrete options and validation gates in
  the new §2.9, plus paging caveats in §2.3/§2.4 and decision items 9–11 in
  §4. Foundation run `34339001103` and packaging run `34339001084` are green
  for the pre-correction head; the correction re-runs docs/policy checks only.
  Still a proposal: no implementation, migration, quota, budget, acceptance,
  gate, or platform-scope change.
- Merged/unmerged labels corrected throughout: PRs #82, #85–#89 are merged;
  PRs #90 and #92–#98 are unmerged drafts (this PR #91 included). No unmerged
  observation, proposal, validation, or check run is relabeled as merged
  evidence.

## 2026-09-09 third refresh addendum (Agents 1–3 finish plus #98 correction)

`origin/main` re-verified 2026-09-09 remains
`0b7612db3570e6235d4d2c86a678dd9004264f30` (PR #89, Foundation run
`34293250231` green); no new merges. Open drafts remain #90–#98 (this
reconciliation is #91); open issues remain #33, #36–#41, #47, #48. Deltas
since the second refresh:

- PR #98 head `a3e738b` (`perf/94-validation-20260909`, fast-forward from
  `15b7f17`, no force-push; scope 3 docs files only, privacy 262 files and
  docs lint 0 issues/59 files clean). Timed binaries remain the pre-cleanup
  `1d7c298`/`b35c01a` pair and exclude the later `15b7f17` pending_entries
  removal; `a3e738b` corrects the all-statistics-match claim (ancestor
  candidate `7,038 ms` to recomputed `7,043.2 ms`, `-2.5%`), expands
  provenance to the full `b35c01a` SHA, and adds the unexecuted §8
  quiet-host plan. No new benchmark was run. Exact-head CI: Foundation run
  `34352820062` on `15b7f17` succeeded; packaging run `34352822867` on
  `15b7f17` failed on the recurring hosted `installed sidecar smoke timed
  out` (`windows-foundation-smoke.ps1:147`), same signature as this PR's
  `986ca61` packaging failure and PR #92 attempt-1; build plus package
  succeeded so this is not a product regression — disposition is retry plus
  owner decision. New head `a3e738b` has no exact-head CI yet and is not
  cited as green. Cleanup stays revise-then-keep pending Agent 1 review;
  Agent 1 was notified of `a3e738b` on PR #98.
- PR #97 head `d699ecd` (`test/95-durable-queue-watcher-20260909`, base PR
  #95 `bf0aeac`): `c4754a2` pins renderer-visible terminal
  `retryAvailable=false` via the status API and corrects the wording to
  preserve PR #92 restart-recovery plus unavailable-root retention and PR
  #95 D1–D3 replay instead of listing them as never verified; `d699ecd`
  adds the additive `review-95-97-stack-20260909.md` record. Green runs
  `34339792661`/`34339792759` cover the `aba1812` slice; the two follow-up
  commits carry focused-test evidence in the PR body with no new exact-head
  CI dispatched. Still test-only; installed queued snapshot and burst
  timing remain unverified.
- PR #96 head `9aa97ab` now has exact-head green: Foundation run
  `34353066695` and packaging run `34353066715` both succeeded. Still a
  proposal only with the §2.9 six unresolved mechanisms plus validation
  gates; no implementation, migration, quota, budget, acceptance, gate, or
  platform-scope change.
- PR #94 head `588867b` now has exact-head green via dispatch: Foundation
  run `34352810308` and packaging run `34352813397` both succeeded. Still
  needs the quiet-host A/B rerun; the contended median/p95 numbers are
  unchanged and are not a qualification.
- This PR head `986ca61`: Foundation run `34353724222` succeeded (9 jobs)
  and packaging run `34353724295` failed on the same recurring hosted
  sidecar timeout; docs-only branch so not a product regression —
  disposition is retry plus owner decision, consistent with PR #92
  attempt-1/attempt-2 precedent. Completed installed observations stay:
  PR #92 persistence, paging, watcher convergence, cancellation retention,
  interrupted-work recovery; PR #95 D1–D3 plus disabled-root replay.
  Pending installed gaps stay: durable queued snapshot, burst timing,
  restart-during-scan on the repaired build, and beyond-replay retention;
  ACL, DriveFS, cross-volume/FAT32, and 100k remain outside scope. PR #97
   automated fences do not fill the installed column.

## 2026-09-09 fourth refresh addendum (integration #99 frozen, #100 in, #91-green; combined replay pending — preserved as history, superseded by fifth addendum below)

Short final status; history above is preserved and not rewritten. The pending-replay line in this fourth addendum is historical; the actual combined-build result is recorded in the fifth addendum and current summaries.

- Integration PR #99 frozen head
  `9c211adeb555aeacd6a12cb074cca5d359b9f0d7` (code-combined
  `79866c57eeefafe7eb09eda3cd6d86369c9d29ed` plus docs-only report;
  identical for packaging) includes Agent 2 packaging correction PR #100
  `983fc648e8fbb48d7eca47e017e712ffb68d63d6`. Validation-only, not a merge
  vehicle; no acceptance, budget, gate, or platform-scope change.
- Exact-head CI on `9c211ad` is green on all ten required checks:
  Foundation run `34360568998` succeeded (9 jobs) and packaging run
  `34360569032` succeeded (`windows-packaging-smoke` 4m7s with the #100
  outer-30s fix). This replaces earlier pending-CI language for the
  integration; it does not promote any constituent PR or criterion.
- Local validation on code-combined `79866c5` passed: `pnpm check` PASS
  (78 node tests including the new packaging-budget policy test), feature-on
  desktop 91 PASS plus warnings-denied Clippy PASS, diagnostics-enabled
  enumeration 44 passed plus 2 ignored (ACL, DriveFS) PASS, and packaging
  build PASS (NSIS bundle produced). Hosted
  `smoke:windows:foundation:hosted` was BLOCKED locally by the shared-host
  synthetic data directory collision (existing `LOCALAPPDATA` smoke
  directory from other agents; no deletion performed); that is a separate
  host-state issue, so install/launch/uninstall evidence for the combined
  build rests on the hosted CI above, not on a local full-smoke claim.
- Agent 2 readiness (PR #100 at `983fc64`, green exact-head CI all ten:
  Foundation `34357665979`, packaging `34357665859`): the outer harness
  deadline moves 10s to 30s against the bounded 9.25s inner sidecar budget
  plus storage/startup margin, with stage diagnostics and a policy
  regression test; focused evidence is 78 node, 39 desktop
  packaging-smoke, and 3 sidecar tests plus Clippy/fmt clean. The fix
  tolerates slow hosted runners but does not prove the absence of a rare
  deadlock; the new stage diagnostics are designed to discriminate the
  next failure. If code changes after the `9c211ad` freeze, repeat
  packaging build plus hosted
  install/launch/sidecar/uninstall/reinstall/verify, plus full `pnpm check`
  when Rust/client code changed.
- PR #98 correction `a3e738b2202740c2979e0ab804483c085636e441` is published
  (fast-forward from `15b7f17`, 3 docs files only, no new benchmark). It
  still has no exact-head CI and is not cited as green. Historical
  provenance is preserved: Foundation `34352820062` green on `15b7f17`,
  packaging `34352822867` failed on the recurring hosted sidecar timeout
  (build plus package succeeded).
- This PR head `322471f` now has exact-head green: Foundation run
  `34356977837` and packaging run `34356976441` both succeeded. This
  replaces the prior "CI not cited as green until runs complete" line.
  Historical provenance is preserved: head `986ca61` had Foundation
  `34353724222` green and packaging `34353724295` failed on the same
  recurring hosted sidecar timeout (retry disposition, not a product
  regression), consistent with PR #92 attempt-1/attempt-2 precedent.
- Final combined-build installed replay on frozen `9c211ad` (Fruitboard
  Foundation Smoke 0.1.0 NSIS package window) remains pending until Agent 1
  supplies the exact evidence commit and observations. No queued snapshot,
  burst timing, retention, platform, performance, or acceptance outcome is
  inferred from integration greens. Evidence classes stay separate.
  (Historical fourth-addendum line; superseded by the fifth addendum below.)

## 2026-09-09 fifth refresh addendum (final regression c3d3292 complete; #101 CI fail; worker stall blocks convergence)

History above is preserved and not rewritten. This addendum replaces
"final replay pending" with the actual combined-build result. No
implementation, budget, gate, platform-scope, or acceptance change is made
here. No benchmark, scale implementation, or platform survey is started.

- Freeze correction: `9c211ad` is an unmerged integration commit on branch
  `docs/41-pr-stack-integration-20260909-v2` (PR #99, validation-only,
  never merges; code-combined `79866c57eeefafe7eb09eda3cd6d86369c9d29ed`
  plus docs-only report, includes #100). It is not merged `main`.
  Merged `main` remains `0b7612db3570e6235d4d2c86a678dd9004264f30`
  (PR #89, Foundation run `34293250231`). Any reading of `9c211ad` as
  merged `main` is incorrect.
- Final regression source: PR #101 head
  `c3d3292aa8f789de1c73d2c70a942539cc1ab778` (branch
  `docs/41-final-regression-20260909`), record
  `docs/review/phase-2-integration/installed-journey/run-20260909-final-regression.md`
  (275 lines, preserves `run-20260908`/`run-20260909` history). Evidence
  wording: all installed observations are direct native-command
  observations through the installed app's Tauri bridge
  (`window.__TAURI_INTERNALS__.invoke`) plus status/page/DOM-state reads
  with JSONL transcripts (`regression-full.jsonl`, `regression-clean.jsonl`);
  they are not end-user UI clicks, keyboard interaction, or screenshots.
  Package: explicitly feature-enabled
  (`packaging-smoke,scan-console`) NSIS `Fruitboard Foundation Smoke 0.1.0`
  built at the frozen SHA in an isolated checkout (installer
  `5B455375D61B8C64DCF0AFF4F1FDD0F2835D683DB658371A0771286D6784C820`).
- #101 final CI result (exact head `c3d3292`): Foundation run
  `34384925512` SUCCESS (9 jobs including `windows-foundation` 6m56s);
  Windows Packaging Smoke run `34384925678` FAILURE
  (`windows-packaging-smoke` fail 6m40s at
  `scripts/windows-foundation-smoke.ps1:147` with
  `The installed sidecar smoke timed out`; build plus package succeeded).
  Overall 9/10, not green. Same recurring hosted installed-sidecar-timeout
  signature as PR #92 attempt-1, PR #98 `15b7f17` (`34352822867`), and
  prior #91 `986ca61` (`34353724295`); build plus package succeeded in each
  case. Disposition for the docs-only #101 head is retry plus owner
  decision, not a product regression; `c3d3292` is preserved as history and
  is not cited as green. This PR #91 head `225ce5b` remains green
  (`34380337434`/`34380337398` all ten); the new head from this fifth
  refresh carries no green claim until runs complete.
- Actual combined-build result (replaces pending):
  - Several repaired cases passed on the frozen package: native Library
    labeling PASS (4.1, D1 fixed), cancel with retention plus fresh
    `scan_now` PASS (4.2), exhausted failure with new chain and old
    preserved PASS on the clean run (4.3), disabled/removed-root stale
    actions PASS (4.4), and unavailable-root retention with automatic
    retry recovery PASS (4.5 automatic path).
  - Queued visibility was captured: ten-file leaf showed `queued` with
    `jobId` and null `runId` for about 7 s before `completed`
    (job `...9b11...`, run `...b4bc...`, `filesObserved 10`); no
    percentage displayed or inferred.
  - A reported clean-run worker queue stall blocks follow-up and restart
    convergence (5.1, product failure with isolated reproduction): after
    automatic recovery (`...807f...` attempt 2), one watcher follow-up
    (`...8908...`, `queued`, null `runId`, due time already past) never
    ran for over three minutes while the app stayed responsive; every
    later `scan_now` coalesced to `already_queued` on the same stalled
    job with no `running` observed. This blocks installed burst
    convergence (4.7 NOT OBSERVED), explicit unavailable-retry convergence
    (4.5), and running-interruption convergence (4.6) on this build.
  - Shared test-directory contention occurred and needs isolation (section
    3 plus 5.2, external): the host
    `%LOCALAPPDATA%/com.fruitboard.desktop.foundation-smoke` cannot be
    isolated per worktree; a concurrent run under a different `%TEMP%`
    root launched competing processes against the same database, polluting
    the first live database (four roots); exclusive use was coordinated by
    graceful close plus two reversible archives
    (`archived-foundation-smoke-*`, `archived-polluted-4roots-*`) with no
    deletion of prior evidence; the clean two-root run is the converged
    record. Future combined/installed runs need exclusive-host
    coordination or per-run isolation before any local full-smoke retry.
  - Running-interruption recovery on this combined build remains
    unverified (4.6 PARTIAL): the kill happened while the large-root scan
    was still `queued` (job `...63bc...`, null `runId`), not `running`;
    after relaunch the same job was retained as `queued` with counters
    reset and prior page intact, then stayed `queued` for the full 120 s
    window with no `running`/`completed`. Identity half preserved, restart
    convergence not demonstrated. The earlier `run-20260908.md`
    interruption (killed while `running`, resumed to `completed`) remains
    the only installed running-recovery evidence and is not superseded.
- Merge readiness in light of the new failure (separate tracks; #99 green
  implies no constituent merge):
  - Independent documentation/packaging track (reviewable on own merits):
    #90 platform blocker report (green), #93 contended diagnostic
    (green), #96 bounded-scan design proposal (green `9aa97ab`, six
    mechanisms unresolved, no implementation), #100 packaging correction
    (green all ten `983fc64`). Their greens do not depend on scanner
    convergence and do not transfer to scanner PRs.
  - Scanner readiness track (blocked): #94 exact-head green via dispatch
    but still needs quiet-host A/B; #95 verified D1-D3 but remains open
    draft needing review/rebuild from merged `main`; #97 automated fences
    (green slice only, follow-ups without new CI) do not fill installed
    columns; #98 `a3e738b` published with no exact-head CI pending Agent 1
    review; #99 `9c211ad` green all ten is validation-only combined
    build/test evidence (never merges) and does not qualify installed
    behavior, platform, performance, or acceptance; #101 stall above keeps
    P2-08/P2-09/P2-12 convergence open. Do not imply any scanner PR is
    safe to merge merely because #99 is green.
  - #101 head `c3d3292` itself is not merge-ready as green (9/10 fail
    above; retry disposition). This PR #91 stays documentation-only and
    merges last after constituents land; merging nothing here.
- Exact input-SHA list for the next single combined validation (verified
  2026-09-09; no newer heads at this refresh; do not repeatedly refresh
  the aggregate while code is still changing):
  base `origin/main 0b7612db3570e6235d4d2c86a678dd9004264f30`; #90
  `17e571742634eceb16f09b166edd3d77b57fcf61`; #92
  `49a5e649c688ae767c9189801dd033f2288b61f5`; #93
  `59faefc2806a725368a59e7b6fc9be7f863f4fec`; #94
  `588867bca154e798f189f6c99de8a2668005d141`; #98
  `a3e738b2202740c2979e0ab804483c085636e441`; #95
  `bf0aeac38ac46dbb90990611b940429e232f67ef`; #97
  `d699ecdb49a77bbadac08e049ca056f50ce32aad`; #96
  `9aa97abab57f90589bdce8b55f3820d6bfe9cd7a`; #100
  `983fc648e8fbb48d7eca47e017e712ffb68d63d6`; prior frozen #99
  `9c211adeb555aeacd6a12cb074cca5d359b9f0d7`
  (code `79866c5`); installed evidence #101
  `c3d3292aa8f789de1c73d2c70a942539cc1ab778` (docs-only, 9/10 fail as
  above); this PR `225ce5b` (plus the new fifth-refresh head once
  pushed, CI not cited until runs complete). When Agents 1-3 publish new
  heads, record the one new exact list once, then run the necessary
  combined gates once: isolated `pnpm check`, feature-on desktop 91 plus
  warnings-denied Clippy, diagnostics enumeration 44 plus 2 ignored,
  packaging build, exact-head hosted Foundation plus packaging CI on the
  new frozen commit, and a coordinated exclusive-host installed replay
  (queued snapshot, burst timing, running-interruption, retention) before
  any handoff. Do not refresh the aggregate on every intermediate push.
- F1/F2/F3 and platform decisions stay visible and unchanged (no new
  benchmark, scale implementation, or platform survey here): F1 exact
  10,000-observation fixture versus coordinated quota; F2 quiet-host A/B
  per section-8 plan after Agent 1 reviews `a3e738b`; F3 chunked-snapshot
  proposal at `9aa97ab` with six unresolved mechanisms versus dated
  deferral; #47 Mirror/Stream authorizations and #48 genuine FAT32 volume
  still required, with #90 blockers and #88 runbooks unchanged.

## 2026-09-09 final refresh addendum (merged 90/92/93/96/100/101/102/103 at 55668da; #104 41ac09b narrow; fixed validation S1–S4 PASS / S5 PARTIAL / S6 PASS; security failure 34397163352)

History above is preserved and not rewritten, including failed replays
(`986ca61` packaging fail, `15b7f17` packaging fail, `c3d3292` packaging fail,
`90c988d` Prettier fail). Unpublished `cccd6da` sixth-refresh history is
preserved in worktree `wt-91-final`; its stale claims are superseded here:
PR #103 is merged (not 9/10 pending), PR #104 is narrowly scoped at `41ac09b` (not
`ded02ac`-stacked), and merged `main` is `55668da` (not `0b7612d`) with a new
security failure. No benchmark, scale implementation, or platform survey is
started. Agent 1 owns merges per explicit user authorization; owner retains
acceptance/decision authority; branch protection is unchanged.

- Already-merged evidence (with push CI): PR #90 `f487aa1` (push `34392062754`
  success), PR #100 `7980b75` (push `34391314350` success; outer 30s vs 9.25s
  inner budget plus stage diagnostics and policy test), PR #92 `300c2a4` (push
  `34392921562` success), PR #93 `106335a` (push `34394018328` success), PR
  #102 `892920d` (push `34394817891` success; historical stall confirmation),
  PR #96 `326fb0a` (push `34395615910` success; proposal only, not an approved
  scale design), PR #101 `67bdc76` (push `34396496197` success; historical stall
  baseline), PR #103 `55668da` (isolation: exclusive host lock
  `scripts/foundation-smoke-lock.ps1` with `CreateNew` ownership,
  runId/pid/startedUtc/purpose, reversible archival with hash checks,
  owner-only release; shared by automated smoke and manual journey).
- Security remediation status: main `55668da` Foundation
  [34397163352](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34397163352)
  is 8/9 green; `security` fails on `pnpm audit --audit-level high`
  (`smol-toml <=1.7.0` via `markdownlint-cli2`, GHSA-7w5x-hrqm-74c2, patched
  `>=1.7.1`; privacy 259 files passed). Remediation is in the Agent 1 lane
  (`smol-toml: 1.7.1` override, local `fix/security-smol-toml-1.7.1`, not yet
  merged). This PR cites no green claim until its own runs complete and does
  not treat the failure as a product regression.
- Scanner fix review (PR #104 at
  [`41ac09b`](https://github.com/guilhermebmichelin-create/fruitboard/commit/41ac09bbf69e6db22378031625fe1b029c3bc101)):
  narrowly scoped (2 files: `crates/storage-sqlite/src/execution.rs` +17,
  `scan_console_host_recovery_tests.rs` +244), code-identical to `ded02ac`
  (verified zero diff on those paths; base differs: `41ac09b` on `f487aa1`,
  `ded02ac` stacked on unmerged `9c211ad`). Mechanism: `retry_failed_scan_job`
  returns `Ok(false)` when the root owns the active slot, so the sweep
  continues to the claim; genuine storage errors still abort by design.
  Automated evidence: regression
  `failed_retry_behind_queued_follow_up_does_not_block_the_worker` fails
  pre-fix and passes post-fix; exact-head green Foundation `34392610749` plus
  packaging `34392610789` (all ten). Full `pnpm check` still required before
  merge. Merges nothing here.
- Agent 3's actual installed results (this PR adds
  `installed-journey/run-20260909-fixed-validation.md`, built from `ded02ac`,
  applicable to `41ac09b` content): S1 PASS (unavailable → auto-retry →
  watcher follow-up converges; `already_queued`-forever wedge gone), S2 PASS
  (repeated scans converge), S3 PASS (5-file burst converges with correct
  missing transition), S4 PASS (poisoned-root fairness: other root's due work
  completes while failed history present), S5 PARTIAL (termination while
  running → successor recovery job completes with rows intact; same-job
  `interrupted → completed` not demonstrated; reproducible `s5-job.txt`,
  `s5-db-copy.db`, transcript IDs preserved; prior `run-20260908.md` remains
  the only same-job evidence), S6 PASS (real UI Cancel click plus
  exhausted-chain 4/4 freshness with old history preserved). Installed evidence
  stays distinct from automated #104 evidence; no acceptance is claimed.
- Remaining PR dependencies: open drafts are #94 (quiet-host A/B still needed),
  #95 (D1–D3 review/rebuild), #97 (follows #95), #98 (`a3e738b` pending Agent 1
  review), #99 (validation-only, never merges), #104 (`41ac09b`, needs `pnpm
  check` plus S5 disposition), and #91 (this PR, merges last). #101/#102 stay
  historical failure evidence. #96 merged as proposal only.

### Concise criterion table (implemented | automated | installed | decision)

Agent 3's replay is recorded above, so the installed column reports history
plus the fixed run; no cell promotes acceptance.

| Criterion | Implemented (merged `main`) | Automated evidence | Installed evidence | Remaining decision |
| --- | --- | --- | --- | --- |
| P2-04 durable queue / retry | #60/#61/#70/#82/#85 merged; #103 isolation merged | Migration matrices green; #97 fences green slice; #104 regression green `34392610749`/`34392610789` | #92 observed; #95 D1–D3 replay kept no queued snapshot; #101/#102 historical stall; fixed S1/S4/S6 PASS plus S5 PARTIAL in this PR | Merge #104 after `pnpm check` plus S5 disposition, then acceptance |
| P2-08 scan / cancel / retry | #73/#77/#78/#81/#82 merged behind `scan-console` | IPC/adapter/lifecycle/host-recovery green; #104 regression as above | #92 journey; #95 D1–D3 verified-but-unmerged; #101 repaired passes + queued visibility; fixed S1/S2/S6 PASS, S5 PARTIAL | Same, then explicit full-criterion acceptance (Partial, not Complete) |
| P2-09 watcher / burst | #68/#69/#71/#81/#82 merged | Coalescing/overflow/stale tests green; #97 counts automated only | #92 follow-ups; #101 burst NOT OBSERVED (stall); fixed S3 PASS under isolation | Keep #47 DriveFS open; then acceptance |
| P2-12 integrated journey | #80–#82/#85/#86/#88–#90/#92/#93/#96/#100–#103 mapped | Exact-commit CI green except new security failure `34397163352` | #92 + #95 + #101/#102 historical + fixed S1–S6 as above | Close S5 plus F1/F2/F3 + platform + P2-10, then acceptance; Pending |
| Others, F1/F2/F3, #47/#48, P2-10, acceptance | Unchanged merged set | Unchanged except #96 proposal merged | Unchanged except fixed run above | Smallest list below; all open |

## Executive result

- Native watcher supervision and shutdown recovery are implemented on `main`
  through PR #82. The old statement that watcher supervision was still open is
  stale.
- PR #85 adds merged worker-level fault, atomicity, restart/backup, alias, and
  stale-publication coverage. Its passing CI and tests strengthen P2-03,
  P2-05, P2-06, and P2-07, but they do not provide installed-app observations
  or owner acceptance.
- The P2-08 `Complete` statement introduced by PR #85 is not supportable
  as an acceptance claim. The reconciled status is Partial: native,
  automated-seam, and partial installed evidence exist, while D1/D2/D3 repairs
  (now verified in still-unmerged PR #95), durable queued observation,
  independent watcher-burst counting, and explicit full-criterion owner
  acceptance remain open as acceptance gates.
- PR #86 is the current performance evidence source. It reproduces F1, leaves
  F2 failing at a 14,267 ms warm p95 against the provisional 10 s target, and
  leaves F3 unmeasured. No budget or fixture decision was made. Stacked PR #94
  needs exact-head CI/combined validation and a quiet-host rerun; it is not a
  qualification.
- The installed journey was observed from a feature-enabled package built from
  `main` `0b7612d`: persistence, paging, watcher follow-up convergence,
  cancellation retention, and interrupted-work recovery were observed. The
  evidence record remains in unmerged PR #92; durable queued observation and
  independent watcher-burst counting remain unverified there and are addressed
  independently. D1–D3 repairs are verified in still-unmerged PR #95 (no
  durable queued snapshot retained). #47 and #48 remain open; PR #90 records
  their blockers, which are not qualifications.
- Unmerged PR #93 adds contended filesystem-port profiling. It identifies a
  measured cost center but is not an idle-host performance pass. PR #92's
  Foundation CI and packaging rerun attempt 2 are green; its first packaging
  attempt recorded `The installed sidecar smoke timed out`, which remains a
  provenance incident rather than a product-fix or acceptance result.
- No explicit full Phase 2 or P2-08 acceptance decision is recorded. Epic #33,
  child issues #36-#41, and platform follow-ups #47/#48 remain open.
  Production scanning stays hidden under the accepted P2-03 through P2-08
  gate. F1/F3 scale options are proposed in draft PR #96 without authorization.

## Audit basis

### Repository and merge state

On 2026-09-08 local time, `git fetch --prune origin main` completed and
`origin/main` resolved to (re-verified unchanged 2026-09-09 local time):

`0b7612db3570e6235d4d2c86a678dd9004264f30`

The latest first-parent sequence is:

| Commit | PR | Meaning |
| --- | --- | --- |
| `0b7612d` | [#89](https://github.com/guilhermebmichelin-create/fruitboard/pull/89) | #48 survey: no qualifying FAT32 target |
| `c8d8255` | [#88](https://github.com/guilhermebmichelin-create/fruitboard/pull/88) | #47/#48 executable manual plans |
| `e9464be` | [#87](https://github.com/guilhermebmichelin-create/fruitboard/pull/87) | Post-merge status documentation |
| `4218e41` | [#86](https://github.com/guilhermebmichelin-create/fruitboard/pull/86) | Benchmark triage re-run |
| `c05f4b6` | [#85](https://github.com/guilhermebmichelin-create/fruitboard/pull/85) | Durability close-out tests and storage fixes |
| `51f45af` | [#84](https://github.com/guilhermebmichelin-create/fruitboard/pull/84) | Acceptance-preparation matrix |
| `05d39ff` | [#82](https://github.com/guilhermebmichelin-create/fruitboard/pull/82) | Native watcher supervision and shutdown recovery |
| `e5e777d` | [#81](https://github.com/guilhermebmichelin-create/fruitboard/pull/81) | Native subscriptions and continuation lifecycle |

Live GitHub at the 2026-09-08 audit showed open issues (#33 plus #36 through #41,
plus #47 and #48). It also showed open draft PRs (#90 through #93); no reviews or
comments were recorded on those four PRs at that audit point. At the 2026-09-09
fourth refresh, open issues are unchanged and open drafts are PRs #90–#100
(this reconciliation is #91; #99 is validation-only integration, #100 is the
packaging correction); all remain unmerged drafts. The exact issue
and PR links are in the [integration index](README.md) and the parent [Phase 2
epic](https://github.com/guilhermebmichelin-create/fruitboard/issues/33).

### CI provenance

The following records were checked through GitHub for the exact commit or PR
head named. `Foundation CI` has nine jobs; `windows-packaging-smoke`
is triggered on pull requests, schedules, and manual dispatch, not on a push
to `main`.

| Evidence target | Exact commit or head | Live result |
| --- | --- | --- |
| Current `origin/main` application/build baseline | [`0b7612db3570e6235d4d2c86a678dd9004264f30`](https://github.com/guilhermebmichelin-create/fruitboard/commit/0b7612db3570e6235d4d2c86a678dd9004264f30) | [Foundation run 34293250231](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34293250231): success, all nine jobs; this is the commit used for the installed application in PR #92 |
| PR #82 merge | `05d39ff074ab6c568d998175744551561928de89` | [Foundation run 34248128449](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34248128449): success, all nine jobs; Windows feature-on tests and warnings-denied Clippy passed |
| PR #85 merge | `c05f4b6a82c5b3469900a408b19b8c979bc4974b` | [Foundation run 34281613081](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34281613081): success, all nine jobs |
| PR #86 merge | `4218e413be7dc27422eab9a45252bcdd1ca74359` | [Foundation run 34290753717](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34290753717): success, all nine jobs |
| PR #87 merge | `e9464be404744a91071343c0e7d32f79f5485fa2` | [Foundation run 34291338301](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34291338301): success, all nine jobs |
| PR #88 head | `ddeac9025dcdd9f1f65c9236a3120ef4057420f1` | [Foundation run 34291358030](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34291358030) and [packaging run 34291358010](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34291358010): both success |
| PR #89 head | `289609511e1cb3359a2a30418a1fde56e8463dc9` | [Foundation run 34292846030](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34292846030) and [packaging run 34292846032](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34292846032): both success |
| PR #90 head (unmerged platform reports) | [`17e571742634eceb16f09b166edd3d77b57fcf61`](https://github.com/guilhermebmichelin-create/fruitboard/commit/17e571742634eceb16f09b166edd3d77b57fcf61) | [Foundation run 34296963183](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34296963183) and [packaging run 34296963219](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34296963219): both success |
| PR #92 head (unmerged installed record) | [`49a5e649c688ae767c9189801dd033f2288b61f5`](https://github.com/guilhermebmichelin-create/fruitboard/commit/49a5e649c688ae767c9189801dd033f2288b61f5) | [Foundation run 34299680195](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34299680195): success; [packaging attempt 1](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34299680193/job/102303731791) failed with `The installed sidecar smoke timed out`, and [rerun attempt 2](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34299680193/job/102314720612) passed; preserve the incident without inferring a product fix or acceptance |
| PR #93 head (unmerged diagnostic profile) | [`59faefc2806a725368a59e7b6fc9be7f863f4fec`](https://github.com/guilhermebmichelin-create/fruitboard/commit/59faefc2806a725368a59e7b6fc9be7f863f4fec) | [Foundation run 34301751666](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34301751666) and [packaging run 34301751625](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34301751625): both success; profile is contended shared-host diagnostics, not an idle-host pass |
| PR #94 head (unmerged, stacked on #93) | [`588867bca154e798f189f6c99de8a2668005d141`](https://github.com/guilhermebmichelin-create/fruitboard/commit/588867bca154e798f189f6c99de8a2668005d141) (base `docs/41-performance-followup-20260908`) | [Foundation run 34352810308](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34352810308) and [packaging run 34352813397](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34352813397): both success via exact-head dispatch. Still needs a quiet-host A/B rerun. Contended whole-scan median 11,735.5 ms to 11,517 ms with p95/max 12,188 ms to 17,682 ms on an outlier; not a p95 win or 10 s qualification |
| PR #95 head (unmerged D1–D3 repairs) | [`bf0aeac38ac46dbb90990611b940429e232f67ef`](https://github.com/guilhermebmichelin-create/fruitboard/commit/bf0aeac38ac46dbb90990611b940429e232f67ef) | [Foundation run 34309511415](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34309511415) and [packaging run 34309511404](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34309511404): both success. Verified D1–D3 repairs with automated and installed-replay evidence; replay retained no durable queued snapshot; ACL/DriveFS/cross-volume/FAT32/100k/burst cases remain outside its scope; draft remains open and unmerged; no new Agent 1 head |
| PR #96 head (unmerged scale-design proposal) | [`9aa97abab57f90589bdce8b55f3820d6bfe9cd7a`](https://github.com/guilhermebmichelin-create/fruitboard/commit/9aa97abab57f90589bdce8b55f3820d6bfe9cd7a) (correction on base `3be0140`) | [Foundation run 34353066695](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34353066695) and [packaging run 34353066715](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34353066715): both success on the correction head (pre-correction `34339001103`/`34339001084` also green). Proposal plus independent-review correction (§2.9 six unresolved mechanisms with options and validation gates); no implementation, quota, budget, or acceptance change |
| PR #97 head (unmerged durable-queue/watcher tests) | [`d699ecdb49a77bbadac08e049ca056f50ce32aad`](https://github.com/guilhermebmichelin-create/fruitboard/commit/d699ecdb49a77bbadac08e049ca056f50ce32aad) (base PR #95 `bf0aeac` on `main` `0b7612d`; slice `aba1812` plus `c4754a2` plus `d699ecd`) | [Foundation run 34339792661](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34339792661) and [packaging run 34339792759](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34339792759): both success on the `aba1812` slice; follow-ups carry focused-test evidence with no new exact-head CI. Test-only plus dated report and stack review record; no production, Library UI, enumeration, migration, contract, budget, or acceptance change. Automated integration evidence; installed queued visibility and burst timing remain unverified |
| PR #98 head (unmerged PR #94 validation) | [`a3e738b2202740c2979e0ab804483c085636e441`](https://github.com/guilhermebmichelin-create/fruitboard/commit/a3e738b2202740c2979e0ab804483c085636e441) (fast-forward from `15b7f17`; timed binaries `1d7c298`/`b35c01a` precede cleanup) | [Foundation run 34352820062](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34352820062): success on `15b7f17`; [packaging run 34352822867](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34352822867): failed on recurring hosted sidecar timeout (`windows-foundation-smoke.ps1:147`), same as #91 `986ca61` and #92 attempt-1; build plus package succeeded. New head `a3e738b` (docs-only correction, no new benchmark) has no exact-head CI yet. Contended 1+10+3 A/B fails 10 s p95 on both sides; revise-then-keep as cleanup pending Agent 1 review; does not touch the PR #94 branch |
| This PR #91 prior head (history) | [`986ca61a1a840f2a4418c2007fc6838c192c311c`](https://github.com/guilhermebmichelin-create/fruitboard/commit/986ca61a1a840f2a4418c2007fc6838c192c311c) | [Foundation run 34353724222](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34353724222): success (9 jobs); [packaging run 34353724295](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34353724295): failed on the same recurring hosted sidecar timeout; docs-only branch so not a product regression — disposition is retry plus owner decision |
| PR #100 head (Agent 2 packaging correction) | [`983fc648e8fbb48d7eca47e017e712ffb68d63d6`](https://github.com/guilhermebmichelin-create/fruitboard/commit/983fc648e8fbb48d7eca47e017e712ffb68d63d6) | [Foundation run 34357665979](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34357665979) and [packaging run 34357665859](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34357665859): both success, all ten checks on the exact head; outer 30s budget plus stage diagnostics with policy regression test |
| Integration PR #99 frozen head (validation-only, unmerged integration commit) | [`9c211adeb555aeacd6a12cb074cca5d359b9f0d7`](https://github.com/guilhermebmichelin-create/fruitboard/commit/9c211adeb555aeacd6a12cb074cca5d359b9f0d7) (branch `docs/41-pr-stack-integration-20260909-v2`, code-combined `79866c5` plus docs-only report; not merged `main`) | [Foundation run 34360568998](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34360568998): success, all nine jobs; [packaging run 34360569032](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34360569032): success. Local `pnpm check`, feature-on 91, diagnostics 44 plus 2 ignored, and packaging build passed on `79866c5`; local hosted smoke blocked by the shared-host data-directory collision (separate issue). Validation-only combined build/test evidence; never merges and never implies constituent merge safety |
| PR #101 head (final installed regression, docs-only evidence) | [`c3d3292aa8f789de1c73d2c70a942539cc1ab778`](https://github.com/guilhermebmichelin-create/fruitboard/commit/c3d3292aa8f789de1c73d2c70a942539cc1ab778) | [Foundation run 34384925512](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34384925512): success (9 jobs including `windows-foundation` 6m56s); [packaging run 34384925678](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34384925678): failed (`windows-packaging-smoke` 6m40s at `windows-foundation-smoke.ps1:147`, same recurring hosted sidecar timeout; build plus package succeeded; retry disposition). Overall 9/10, not green. Record `run-20260909-final-regression.md` reports direct native-command observations (not end-user UI): repaired cases passed, queued visibility captured, clean-run worker stall blocking convergence, shared-directory contention needing isolation, running-interruption unverified |
| This PR #91 head | [`225ce5bd9544fd1635df31f2741262e3e47e1fbe`](https://github.com/guilhermebmichelin-create/fruitboard/commit/225ce5bd9544fd1635df31f2741262e3e47e1fbe) | [Foundation run 34380337434](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34380337434): success; [packaging run 34380337398](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34380337398): success. History preserved: `322471f` (`34356977837`/`34356976441` green) and prior `986ca61` (Foundation green, packaging sidecar-timeout fail with retry disposition) |

The push run for merged commit `c8d8255` was cancelled when the next
`main` push arrived. Therefore the #88 PR-head checks are cited for that
docs-only change, while the successful exact-current-commit run above covers
the final tree. A green CI result establishes build and test evidence only; it
does not establish installed behavior, broad platform support, or owner
acceptance.

### Owner decisions and authority

The audit found these explicit decisions:

| Decision | Recorded basis | Scope |
| --- | --- | --- |
| Shared scanner contracts and provisional starting budgets accepted on 2026-09-06 | [PR #57](https://github.com/guilhermebmichelin-create/fruitboard/pull/57) body | Design/implementation baseline; not measured performance, implementation completion, or Phase 2 acceptance |
| UI-only Library seam accepted on 2026-09-07 | Owner comment on [PR #63](https://github.com/guilhermebmichelin-create/fruitboard/pull/63) | `LibraryScanAdapter`, page fencing, and fake-adapter harness only; native P2-08 journey not accepted |
| Branch protection enabled on 2026-09-07 | Merged [PR #74](https://github.com/guilhermebmichelin-create/fruitboard/pull/74) and live branch settings | Repository governance only |

No explicit owner post or review accepting the full P2-01 through P2-12
checkpoint was found. PR #82 explicitly says no acceptance issue is closed.
PR #85 explicitly says no acceptance ID status is promoted. PRs #88 and #89
explicitly preserve #47/#48 as unverified. An owner merge is therefore recorded
as a merge, not inferred to be acceptance.

The historical checkpoint has checked boxes for P2-01, the first P2-11 report,
and the hidden P2-12 journey. Its own legend defines checked boxes as
evidence-complete, says unchecked boxes block close, and states that epic close
still requires explicit owner acceptance. Those checkboxes are therefore
retained as historical evidence markers, not treated as owner posts or as
current acceptance of the criteria.

## Reconciled status summary

| ID | Status after reconciliation |
| --- | --- |
| P2-01 | Evidence present; not promoted |
| P2-02 | Partial |
| P2-03 | Partial |
| P2-04 | Partial |
| P2-05 | Partial |
| P2-06 | Partial |
| P2-07 | Partial |
| P2-08 | Partial; prior `Complete` claim disputed and not accepted |
| P2-09 | Partial |
| P2-10 | Partial |
| P2-11 | Measured; not qualified; F1-F3 decisions open |
| P2-12 | Pending |

No criterion is promoted to complete by this report.

## Criterion-by-criterion evidence ledger

The categories below are deliberately independent. In particular, automated
tests do not fill the installed-app, performance, platform, or owner-acceptance
columns.

### P2-01 - Existing root settings/picker remain safe and accessible

- **Implementation:** The #34/#35 picker, root settings, onboarding, failure,
  focus, keyboard, and narrow-layout work is merged through PRs #20-#32, #44,
  #52-#58. No newer merged change invalidates that implementation evidence.
- **Automated tests:** Client, documentation, and Windows foundation checks
  cover the merged settings surface. The rendered #35 evidence is explicitly
  fake-adapter evidence.
- **Installed-app observations:** Historical #34 installed-picker evidence is
  recorded under `docs/review/issue-34/`. The unmerged [PR #92 run record](https://github.com/guilhermebmichelin-create/fruitboard/blob/49a5e649c688ae767c9189801dd033f2288b61f5/docs/review/phase-2-integration/installed-journey/run-20260908.md)
  additionally records current-main installed picker cancellation, root
  selection, settings persistence, and clean restart survival. It does not
  itself establish criterion acceptance.
- **Performance qualification:** Not applicable to this criterion; no scanner
  benchmark qualifies it.
- **Platform qualification:** The picker record is Windows evidence for the
  picker slice only. It is not a DriveFS, FAT32, or scanner platform claim.
- **Owner acceptance:** Phase 1 is accepted, but no separate current Phase 2
  P2-01 acceptance line was found.
- **Reconciled status/gate:** Evidence present; not promoted. Review the
  current-main run record and obtain explicit P2-01 owner acceptance.

### P2-02 - Unchanged tree and add/modify/rename convergence

- **Implementation:** PR #59 (`c5264a2`) provides the deterministic
  reconciliation core; PR #64 (`57587b5`) provides bounded Windows
  enumeration; PR #70 (`b78e715`) composes the hidden worker.
- **Automated tests:** Deterministic tests include
  `unchanged_tree_is_idempotent_and_order_independent` and
  `every_incomplete_outcome_rejects_positive_and_absence_changes`.
  Windows enumeration and hidden-worker fixture tests cover add, modify,
  rename, missing, and restore cases. The exact current Foundation run is
  green.
- **Installed-app observations:** The unmerged #92 record observes native
  add, rename, metadata modification, remove, and restore follow-ups. The new
  path was Present while the old renamed path stayed Missing, and the restored
  path returned to Present. This was captured from the package built at
  `main` `0b7612d`; it is not a broader platform qualification.
- **Performance qualification:** No P2-02 performance qualification is
  claimed. The benchmark driver exercises a hidden worker and is P2-11
  evidence, not installed-app evidence.
- **Platform qualification:** Windows/NTFS fixture coverage exists in CI;
  #47/#48 remain outside that qualification.
- **Owner acceptance:** No explicit P2-02 owner acceptance was found.
- **Reconciled status/gate:** Partial. Review the installed convergence record,
  retain the broader platform limitation, and obtain owner acceptance.

### P2-03 - Non-authoritative traversal never marks files missing

- **Implementation:** PRs #62 and #70 establish staged, non-authoritative
  worker behavior. PR #85 (`c05f4b6`) adds the close-out coverage and
  storage support.
- **Automated tests:** The merged `p2_03_*` cases cover offline, root
  denial, partial disappearance, unsupported filesystem, cooperative and
  durable cancellation, resource limit, and staging/sink failure. Each seeds
  committed rows, compares rows plus the root success marker byte-for-byte,
  asserts no publication, and checks discarded staging. The exact merged
  evidence is [Foundation run 34281613081](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34281613081), with the current tree rechecked by [run 34293250231](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34293250231).
- **Installed-app observations:** The unmerged #92 record observes native
  cancellation and unavailable-root retention: the previous committed rows
  remained visible and no incomplete work published a replacement list. Denied
  and resource-limit installed cases were not run.
- **Performance qualification:** The cooperative stop samples in the
  benchmark are P2-11 worker measurements only; they do not qualify the
  installed cancellation or UI acknowledgement requirement here.
- **Platform qualification:** Fake-port fault injection and Windows CI are
  automated evidence. The ACL twin may be unavailable under elevated tokens,
  and DriveFS/non-NTFS behavior remains unverified.
- **Owner acceptance:** PR #85 says the close-out does not promote an
  acceptance ID. No explicit P2-03 acceptance was found.
- **Reconciled status/gate:** Partial. Review the installed retention record,
  keep denied/resource-limit coverage open, and obtain owner acceptance.

### P2-04 - Durable generations, deduplication, leases, backoff, and cancellation

- **Implementation:** PRs #60 and #61 establish the durable ledger and
  contract preservation; PR #70 composes the worker; PR #82 connects host
  lifecycle and shutdown recovery; PR #85 adds worker close-out cases.
- **Automated tests:** The `migration` lane covers state-machine and
  restart/backup transition matrices. The feature-enabled desktop host tests
  cover lifecycle recovery and joined loops. The exact-current and PR #85
  Foundation runs are green.
- **Installed-app observations:** The unmerged #92 record observes persisted
  root/settings state, native running/cancellation behavior, and restart
  recovery. A durable queued state was not captured there. Retry after
  cancellation was non-actionable (D2), and Retry after an exhausted
  unavailable-root chain was non-actionable (D3). Both repairs are verified in
  still-unmerged PR #95 (native `retryAvailable` fencing with `Scan now` for
  terminal chains, plus the additive `run-20260909.md` replay); PR #95 remains
  an open draft and its replay retained no durable queued snapshot. No new
  Agent 1 head beyond `bf0aeac` exists at this refresh.
- **Automated integration evidence (new, PR #97, not installed):** Unmerged
  draft PR #97 head `d699ecd` (slice `aba1812` plus `c4754a2` status-flag pins
  plus `d699ecd` review record) pins three deterministic fences at the real
  supervisor plus native status/Library API with fake clocks and no sleeps:
  mid-scan trigger converges (`Interrupted` first attempt publishes nothing,
  one queued follow-up, authoritative follow-up carries the added location and
  the removed location as `missing`); 5,000 + 5,000 idle signals collapse to
  exactly one queued job; stale generations cannot revive disabled or removed
  roots (0 jobs while disabled; replacement keeps 1 with retired-generation
  replay still 1); terminal cancelled/exhausted statuses report
  `retryAvailable=false` via the status API. Green runs `34339792661` /
  `34339792759` cover the `aba1812` slice. This is automated evidence for the
  durable queued slot and stale-fencing behavior; it does not supply the
  installed queued snapshot, burst timing, restart-during-scan handling, or
  unavailable-root retention beyond PR #95, which remain installed-only gaps.
- **Performance qualification:** The re-run reports cooperative worker stop
  p95 of 22 ms over six samples, but has no renderer and does not exercise the
  250 ms UI acknowledgement budget. It is not a complete P2-04 qualification.
- **Platform qualification:** Windows CI and injected lifecycle tests qualify
  only those tested seams; they do not qualify all filesystem modes.
- **Owner acceptance:** No explicit P2-04 owner acceptance was found.
- **Reconciled status/gate:** Partial. Review verified D1–D3 repairs in
  still-unmerged PR #95 (rebuilding from merged `main` where required),
  review the automated PR #97 fences without counting them as installed
  observation, capture the installed queued snapshot and burst timing through
  installed runs, then complete the end-to-end review.

### P2-05 - Disable/remove while queued or running prevents stale publication

- **Implementation:** PRs #60 and #62 provide configuration invalidation and
  publication fencing. PR #85 adds disable, remove, queued-work, and fresh
  re-add coverage; PR #82 preserves notification ordering after durable root
  transactions.
- **Automated tests:** `p2_05_disable_invalidates_lease_and_staging_*`,
  `p2_05_remove_then_readd_*`, and
  `p2_05_queued_work_invalidated_*` cover lease/staging invalidation,
  no stale publication, fresh identity, and unchanged source markers. Green
  exact-commit CI verifies the merged tests.
- **Installed-app observations:** The unmerged #92 record observes disabling a
  running root cancelling the work while retaining prior rows, and removing an
  idle root without touching fixture files before re-adding a fresh tracked
  root. A specifically queued disable/remove observation is not recorded.
- **Performance qualification:** Not applicable; no timing result proves this
  concurrency property.
- **Platform qualification:** The source-marker assertions and fake durable
  harness are not a cross-filesystem qualification.
- **Owner acceptance:** No explicit P2-05 owner acceptance was found.
- **Reconciled status/gate:** Partial. Review the installed operation, add
  queued-operation coverage if required by the owner, and obtain acceptance.

### P2-06 - Atomic publication and restart/backup recovery preserve valid data

- **Implementation:** PR #62 supplies staging/publication and rollback
  boundaries. PR #85 adds close-out tests for crash and recovery boundaries.
- **Automated tests:** The merged cases
  `p2_06_crash_before_staging_*`,
  `p2_06_crash_after_staging_before_apply_*`,
  `p2_06_crash_during_apply_*`, and
  `p2_06_backup_recovery_*` assert no partial Library rows, ledger
  consistency, requeued convergence, and byte-identical committed data and
  marker after recovery. Existing migration fixtures remain covered. PR #85
  exact-merge CI is [run 34281613081](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34281613081).
- **Installed-app observations:** The unmerged #92 record observes committed
  rows and settings surviving clean restart, then an abruptly terminated
  installed process being recorded as `interrupted` and recovering to a
  completed attempt after relaunch. This is installed evidence from the
  current-main package, not a claim that all crash boundaries are covered.
- **Performance qualification:** Not applicable; a passing rollback test is
  not a performance result.
- **Platform qualification:** Storage and Windows CI evidence does not qualify
  DriveFS, FAT32, network, or other untested storage modes.
- **Owner acceptance:** No explicit P2-06 owner acceptance was found.
- **Reconciled status/gate:** Partial. Review the interrupted-work record and
  obtain owner acceptance for the full crash/recovery criterion.

### P2-07 - Hardlink aliases and uncertain identity preserve per-path presence

- **Implementation:** PR #59 establishes conservative identity decisions; PR
  #62 publishes alias-aware locations; PR #85 adds worker-level alias and
  replacement/rename close-out cases.
- **Automated tests:** `p2_07_hardlink_delete_one_*` verifies that
  deleting one alias leaves the other present and restores the missing path.
  The `p2_07_rename_replacement_*` cases preserve per-path history,
  mint fresh replacement records, and avoid Phase 4 grouping. Windows/NTFS
  fixture CI is green.
- **Installed-app observations:** The unmerged #92 record observes rename
  convergence with separate Present/Missing paths, but it does not establish an
  installed hardlink-alias run. The hardlink evidence remains automated and
  local-NTFS-oriented.
- **Performance qualification:** Not applicable.
- **Platform qualification:** The evidence is local-NTFS-oriented. PR #88
  supplies a manual #48 plan and PR #89 records no qualifying FAT32 target;
  neither qualifies FAT32, cross-volume, exFAT, ReFS, or network identity.
- **Owner acceptance:** No explicit P2-07 acceptance or non-NTFS scope
  exclusion was found.
- **Reconciled status/gate:** Partial. Add installed alias evidence if required
  by the owner and resolve #48 with evidence or an explicit owner decision;
  neither is supplied by this reconciliation.

### P2-08 - Scan/Cancel/Retry and Library list are usable, honest, and persistent

- **Implementation:** PR #73 adds feature-gated scan-console IPC; #77 fixes
  staged-batch responsiveness; #78 wires the native Library adapter; #81
  corrects event permissions and adds lifecycle coverage; #82 integrates the
  native watcher/scan host and shutdown recovery. These are merged behind
  `scan-console`.
- **Automated tests:** Native typed IPC, client adapter, pagination,
  cancellation, lifecycle, host recovery, and feature-enabled desktop tests
  are present. PR #82 and the exact merged Foundation run
  [34248128449](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34248128449)
  verify the feature-on desktop tests and warnings-denied Clippy; the current
  tree is green in [run 34293250231](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34293250231).
  Rendered captures remain fake-adapter evidence.
- **Installed-app observations:** The unmerged #92 record observes native
  `Scan now` running counters with no percentage, four-record Library paging,
  persisted settings and committed rows, cancellation/unavailable retention,
  watcher follow-up convergence, and interrupted-work recovery. Durable queued
  observation was not captured there. D1 (native Library mislabeled as fake),
  D2 (cancelled Retry rejected), and D3 (exhausted Retry not actionable)
  repairs are verified in still-unmerged PR #95 (native render context,
  `retryAvailable` fencing, `Scan now` for terminal chains, additive
  `run-20260909.md` replay with installer/executable SHAs and isolated data
  location). PR #95 remains an open draft; its replay retained no durable
  queued snapshot. The #92 record was captured from a package built at merged
  `main` `0b7612d`, not from an unmerged documentation head.
- **Performance qualification:** The benchmark driver uses the hidden worker,
  not the installed renderer path. It cannot qualify Library/UI latency or
  the installed journey.
- **Platform qualification:** Feature-enabled Windows CI verifies the native
  seam and test host only. It does not qualify DriveFS or non-NTFS behavior.
- **Owner acceptance:** There is a material authority conflict. The second
  commit carried by PR #85 says `P2-08 Pending -> Complete` using
  merge-train evidence and a claimed owner seam acceptance in PR #78. However,
  PR #85's body says no acceptance ID is promoted and owner review is required;
  PR #82 says no acceptance issue is closed; PR #63's explicit owner comment
  accepts only the UI-only Library seam; and the accepted plan requires
  integrated evidence for cross-issue criteria. No explicit owner post or
  review accepting the full P2-08 criterion was found.
- **Reconciled status/gate:** Partial, not Complete. Review verified D1/D2/D3
  repairs in still-unmerged PR #95 (rebuilding from merged `main` where
  required), capture durable queued state through the independently addressed
  work, retain the packaging-timeout incident, and obtain explicit
  full-criterion owner acceptance before promoting it.

### P2-09 - Watcher bursts, overflow, and event loss converge durably

- **Implementation:** PRs #68 and #69 provide bounded handle-bound watcher
  behavior; #71 connects watcher hints to durable follow-ups; #81 adds a
  deterministic lifecycle harness; #82 adds the native supervisor, root
  mapping, generation fencing, reconnect backoff, retained coverage hints,
  and joined host loops.
- **Automated tests:** Watcher coalescing, overflow/coverage-loss precedence,
  stale-generation dropping, delivery retention, shutdown recovery, empty-host
  joins, and disable/re-enable races are covered in the watcher, host, and
  recovery test files. Exact feature-on and watcher CI is green.
- **Installed-app observations:** The unmerged #92 record observes native
  watcher follow-ups for add/rename/metadata/remove/restore changes and
  convergence to committed Library results. It does not independently count a
  burst to prove the at-most-one queued-follow-up bound, and it does not record
  installed coverage-loss, overflow, or stale-generation cases.
- **Automated integration evidence (new, PR #97, not installed):** Unmerged
  draft PR #97 head `d699ecd` supplies the missing deterministic counts at the
  supervisor seam (green runs `34339792661`/`34339792759` on the `aba1812`
  slice): idle 5,000 + 5,000 bursts collapse to one queued job,
  running-burst follow-up behavior is extended by the mid-scan convergence
  test, stale-generation replay after disable/re-enable and remove/re-add is
  fenced to zero extra jobs, and terminal statuses pin
  `retryAvailable=false`. Existing running-burst, precedence, and overflow
  tests are cited, not duplicated. Real OS overflow timing, installed
  burst-coalescing measurement, coverage-loss timing, and
  DriveFS/network/ACL cases remain unverified and are explicitly not claimed
  there.
- **Performance qualification:** No real OS overflow timing or installed
  watcher latency qualification is recorded.
- **Platform qualification:** Injected handles and Windows CI qualify
  deterministic seams. #47 DriveFS behavior, network shares, ACL revocation
  during a watch, and real overflow timing remain unverified.
- **Owner acceptance:** No explicit P2-09 owner acceptance or #47 scope
  exclusion was found.
- **Reconciled status/gate:** Partial. The automated burst/stale counts now
  exist in unmerged PR #97; still required are the installed
  burst-coalescing count, loss/overflow cases as scoped, and the
  DriveFS/platform gate.

### P2-10 - No parsing, hydration, or source mutation in discovery

- **Implementation:** The filesystem-only worker and enumeration path are
  merged; PR #81 adds the repository no-parser/no-content-I/O policy guards
  and source-preservation assertions.
- **Automated tests:** `tests/no-parser.test.mjs` statically checks the
  manifests, forbidden content-read APIs, allowed-API pin, and preservation
  fixture pin. Separate enumeration and worker tests compare synthetic source
  bytes before and after. These are static source checks plus fixture equality,
  not a runtime content-read spy.
- **Installed-app observations:** No installed-app dependency or runtime
  content-read observation is recorded.
- **Performance qualification:** Not applicable; absence of parsing is not a
  speed result.
- **Platform qualification:** The checks cover the merged Windows-oriented
  discovery source policy; they do not broaden filesystem platform support.
- **Owner acceptance:** PR #57 records the filesystem-only direction and
  parser deferral as an accepted planning baseline. That does not by itself
  close P2-10, and the static-versus-runtime boundary remains an owner review
  question.
- **Reconciled status/gate:** Partial. Keep the limitation explicit and obtain
  the owner decision on whether the static guards plus fixture equality satisfy
  the criterion.

### P2-11 - Performance and resource limits are measured and safely enforced

- **Implementation:** PR #67 adds the fixture/methodology scaffold; PR #72
  adds the benchmark driver and first measured report; PR #86 adds the
  2026-09-08 re-run and profiling notes. These are evidence artifacts, not a
  budget amendment.
- **Automated tests:** Benchmark scaffold and repository policy tests pass in
  CI. The measured runs themselves are the documented protocol against the
  hidden worker, not a CI pass/fail qualification of an installed app.
- **Installed-app observations:** None. The driver uses a pinned release
  worker binary and disposable synthetic fixtures, not the packaged desktop
  UI.
- **Performance qualification:** The historical 2026-09-07 quota-fitting
  10,000-observation run measured median 12,875.5 ms and warm p95 32,876 ms.
  The current 2026-09-08 re-run was executed at the recorded `51f45af`
  benchmark baseline (before the later #85 merge), and uses the same fixture hashes: the accepted
  baseline still produces 10,005 observations and fails `ResourceLimit`;
  the quota-fitting run publishes 10,000 locations with first discovery 10,846
  ms, median 10,978 ms, and warm p95/max 14,267 ms. All ten warm iterations
  exceed the provisional 10 s target. Cooperative stop p95 is 22 ms, but
  working-set samples of about 53 MiB are from a 10,000-entry set and are not
  private-memory qualification for the 100,000-entry target. Because the
  recorded benchmark commit predates #85, this is the newest available
  measurement, not an exact-current-`origin/main` performance qualification.
  Unmerged [PR #93](https://github.com/guilhermebmichelin-create/fruitboard/blob/59faefc2806a725368a59e7b6fc9be7f863f4fec/docs/review/phase-2-integration/performance-followup/README.md)
  adds a filesystem-port diagnostic: 10.754 s of a 12.157 s warm scan (88.46%)
  on a contended shared host, with `next_entry` and `read_metadata` dominant.
  It is diagnostic cost-center evidence, not an idle-host rerun or performance
  pass, and it does not change the official benchmark result. Stacked unmerged
  [PR #94](https://github.com/guilhermebmichelin-create/fruitboard/blob/588867bca154e798f189f6c99de8a2668005d141/docs/review/phase-2-integration/performance-followup/optimization-20260909.md)
  removes one duplicate native attribute query with safety evidence; its
  contended whole-scan comparison (median 11,735.5 ms to 11,517 ms; p95/max
  12,188 ms to 17,682 ms on an outlier) now has exact-head green via dispatch
  (`34352810308`/`34352813397`) but still needs a quiet-host A/B rerun; it is
  not an end-to-end p95 win and does not qualify the 10 s target. Scale
  options are proposed in draft
  [PR #96](https://github.com/guilhermebmichelin-create/fruitboard/pull/96)
  without applying either F1 option or authorizing F3 implementation. Unmerged
  draft [PR #98](https://github.com/guilhermebmichelin-create/fruitboard/pull/98)
  head `a3e738b` (timed binaries `1d7c298`/`b35c01a` precede the later
  cleanup) independently validates PR #94 (whole-scan stats match;
  ancestor candidate corrected to `7,043.2 ms`; identical
  fixtures/toolchains/profiles/flags; no nested-timer double counting;
  instrumented vs uninstrumented separation; single-query saving is tens of
  milliseconds) with a contended-host 1+10+3 A/B on identical `custom-9995`
  where both sides fail the provisional 10 s p95. It removes only 8 lines of
  dead diagnostic scaffolding plus a docs-only correction with an unexecuted
  §8 quiet-host plan, and recommends revise-then-keep as cleanup pending
  Agent 1 review; Foundation `34352820062` on `15b7f17` is green while
  packaging `34352822867` failed on the recurring hosted sidecar timeout and
  new head `a3e738b` has no exact-head CI yet. It is a limited validation
  result, not a qualification, and does not replace the required quiet-host
  A/B.
- **Platform qualification:** The re-run is a same-host, laptop-class
  Windows 11 measurement with background DriveFS, antivirus, agent, and
  sibling-build load. It is not the idle-host isolation required by the
  pending F2-rerun decision and transfers to no other host class.
- **Owner acceptance:** The starting budgets are owner-accepted as
  provisional in PR #57. F1 (quota versus fixture), F2 (budget/host/rerun or
  optimization), and F3 (100k qualification) have no posted owner decision.
  The [decision brief](budget-decision-brief.md) and [re-run report](benchmark-triage-20260908/rerun-report.md) quote the exact sentences still required.
- **Reconciled status/gate:** Measured, not qualified. Resolve F1-F3 with
  explicit owner decisions and any required re-measurement or plan amendment;
  do not promote the PR #93 diagnostic to an idle-host qualification.

### P2-12 - Complete visible journey works after integration

- **Implementation:** PR #80 records the historical checkpoint and PRs
  #81/#82/#85/#86/#88/#89 update the merged implementation and evidence
  boundary. The current index and this report are the status reconciliation,
  not a replacement for the journey record.
- **Automated tests:** The hidden worker/driver transcript in the historical
  checkpoint and the current exact-commit CI establish automated and
  contract-level evidence. They do not establish an installed-app journey.
- **Installed-app observations:** The unmerged [PR #92 checklist and run
  record](https://github.com/guilhermebmichelin-create/fruitboard/blob/49a5e649c688ae767c9189801dd033f2288b61f5/docs/review/phase-2-integration/installed-journey/run-20260908.md)
  record selection/cancel, persisted settings and paging, watcher convergence,
  cancellation retention, and interrupted-work recovery from a package built
  at merged `main` `0b7612d`. Durable queued observation and independent
  watcher-burst counting remain unverified there and are addressed
  independently. D1/D2/D3 repairs are verified in still-unmerged [PR
  #95](https://github.com/guilhermebmichelin-create/fruitboard/blob/bf0aeac38ac46dbb90990611b940429e232f67ef/docs/review/phase-2-integration/installed-journey/run-20260909.md)
  with an additive replay; that draft remains unmerged. Neither evidence
  document is merged and neither itself accepts P2-12.
- **Performance qualification:** P2-11 remains unresolved with F1-F3 open;
  no performance decision can complete P2-12.
- **Platform qualification:** #47 DriveFS and #48 FAT32/cross-volume remain
  open and unverified. The #89 no-target survey is not a pass.
- **Owner acceptance:** No explicit owner acceptance of the P2-12 checkpoint
  or Phase 2 was found. Issue #41 and epic #33 remain open.
- **Reconciled status/gate:** Pending. Review verified D1/D2/D3 repairs in
  still-unmerged PR #95 (rebuilding from merged `main` where required; no new
  Agent 1 head), review the automated PR #97 fences at `d699ecd` without
  counting them as installed observation, capture the installed queued
  snapshot and burst timing through installed runs, resolve performance and
  platform decisions (including the PR #96 proposal at `9aa97ab` with its
  §2.9 review correction, or dated deferral, plus the required quiet-host
  rerun), and obtain the owner's criterion-by-criterion acceptance.

## Owner decision packet

The following are recommendations and consequences for the owner; none is an
approval or an acceptance decision.

1. **F1 — exact observations or coordinated quota (details in PR #96).**
   Recommend defining the accepted fixture as exactly 10,000 observations,
   including aliases: 9,995 FLP-named files plus five hardlink alias locations
   (`custom-9995`, seed 0, manifest
   `a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a`; command
   `node scripts/run-benchmark.mjs --size custom --files 9995 --seed 0 --out
   <report>.json`). This is the narrowest change and preserves the
   10,000-record quota, but changes the fixture definition and requires a fresh
   accepted-protocol run (1 warm-up + 10 measured with median/max/p95,
   first-discovery, stop latency, and working-set reported). The alternative is
   a coordinated quota change above 10,005 across the worker, staging fences,
   storage checks, plan buffer, and tests; it preserves the 10,000-FLP fixture
   but requires implementation/configuration changes and remeasurement. Do not
   raise only one limit. Neither option is applied here; see PR #96 §1 for the
   full validation consequences.
2. **F3 — bounded 100,000-entry design or dated deferral (design in PR
   #96).** Recommend reviewing the bounded chunked-snapshot proposal (durable
   coverage ledger, generation/lease fencing across chunks, explicit
   memory/disk/path-byte/record bounds, one atomic authoritative publication,
   missing-file gating on complete coverage, cancel/crash/restart/expiry/
   cleanup/backup behavior, migration slices, and adversarial tests in PR #96
   §2) against the coordinated quota increase and the dated deferral, with the
   §2.9 independent-review correction as part of the review package: paged
   reads are not a bounded identity algorithm (two-pass index, temp-table
   assignment, or provisional-plus-merge still to be chosen with memory
   evidence), the coverage denominator rule is undecided, cross-page
   hardlink/rename/conflict handling has no mechanism yet, and publication
   lock duration under the single-connection `DELETE`-journal architecture is
   unmeasured. The chunked direction protects private-memory bounds but
   requires a new storage/protocol design with the listed tests. The
   lower-cost alternative is an explicit dated deferral that keeps the current
   10,000-entry contract; the consequence is that the 100,000-entry target
   remains unqualified until that checkpoint. This reconciliation authorizes
   none of the three; PR #96 is a proposal only.
3. **#47 — exact DriveFS run and authorization.** Run the complete procedure
   once in **Mirror files** and once in **Stream files**, reading the active mode
   from Drive for Desktop Preferences. Use only the documented disposable
   leaves `G:\fruitboard-drivefs-<run-id>-enum`,
   `G:\fruitboard-drivefs-<run-id>-burst`, and
   `G:\fruitboard-drivefs-<run-id>-gap`. Before starting, the owner/operator
   must authorize cloud-synchronized synthetic create/rename/delete and cleanup;
   switching modes requires authorization for the full resync; the gap case
   additionally requires authorization to pause/disconnect and verify resume.
   No personal project contents may be traversed, opened, hashed, or parsed.
4. **#48 — genuine FAT32 alongside NTFS.** Recommend an already disposable,
   writable genuine FAT32 USB volume or dedicated VHD with a drive letter,
   alongside the existing disposable leaf on `C:` NTFS. Record source/type,
   volume serial, filesystem, and allocation unit size; use a leaf such as
   `<LETTER>:\fruitboard-48-fat32-<run-id>` and authorize synthetic operations
   and cleanup on both volumes. Do not substitute the system FAT32 partition,
   the `G:` DriveFS virtual mount, a network share, or the no-media USB device.
5. **Remaining acceptance after fixes and verification.** Review verified D1
   native labeling, D2 cancelled Retry, and D3 exhausted Retry in still-unmerged
   PR #95, rebuilding from then-merged `main` where required; retain merged PR
   #92's first-attempt sidecar timeout and green rerun attempt 2 plus the
   recurring docs-only sidecar timeouts on PR #98 `15b7f17` (`34352822867`),
   prior head `986ca61` (`34353724295`), and merged #101 `c3d3292`
   (`34384925678`, build plus package succeeded) in the provenance with retry
   disposition (prior #91 heads `225ce5b` green `34380337434`/`34380337398`,
   `322471f` green `34356977837`/`34356976441`, `8437906` green
   `34386309754`/`34386309771`; validation-only #99 `9c211ad` green
   `34360568998`/`34360569032`; merged #100 `7980b75` and #103 `55668da`).
   `9c211ad` is validation-only on PR #99 branch (never merges), not merged
   `main`.
   Keep exact check links in the handoff. Review automated PR #97 fences
   at `d699ecd` without counting them as installed observation; merged
   #101/#102 are the historical stall baseline and this PR's fixed validation
   (S1–S4 PASS, S5 PARTIAL, S6 PASS) is installed evidence distinct from
   automated #104 `41ac09b` — do not infer beyond those records.
   Keep exact check links in the handoff. Resolve performance qualification
   with the owner-selected quiet-host A/B (stacked PR #94 exact-head green plus
   PR #98 `a3e738b` revise-then-keep cleanup pending Agent 1 review) or
   optimization/budget path, then close F1/F3 via merged proposal `326fb0a`
   (proposal only, not an approved design) or dated deferral, complete any
   scoped #47/#48 runs (Mirror/Stream consent; genuine FAT32 volume) or
   explicit scope decisions without inventing consent or availability, and
   decide the P2-10 static-versus-runtime boundary (static guards plus fixture
   equality vs a runtime content-read spy). Finally, obtain explicit acceptance for P2-01
   through P2-12; do not infer it from merged commits, green checks,
   automated tests, or these recommendations. Refresh this reconciliation
   after sibling fix PRs land and before any final merge/acceptance review.

## Decision table (2026-09-09 final refresh; single table)

 One row per pending choice. Recommendations state consequences; none is an
 approval, acceptance, merge, budget change, or close.

| Area | State at this refresh | Recommendation and consequence |
| --- | --- | --- |
| Constituent merge readiness | Merged: #82/#85–#86/#88–#90/#92–#93/#96/#100–#103 on `main` `55668da` (latest push `34397163352` 8/9 with new security failure; priors `34392062754`/`34391314350`/`34392921562`/`34394018328`/`34394817891`/`34395615910`/`34396496197` success). #96 merged as proposal only, not an approved scale design. #101/#102 are historical stall failure evidence. Unmerged: #94 `588867b` green via dispatch; #95 `bf0aeac` green; #97 `d699ecd` green slice; #98 `a3e738b` no exact-head CI (history `15b7f17` Foundation green, packaging fail); #99 `9c211ad` validation-only green all ten (never merges); #104 `41ac09b` green all ten (`34392610749`/`34392610789`, automated only, `pnpm check` still required); this PR adds fixed validation S1–S4 PASS / S5 PARTIAL / S6 PASS. Security remediation in Agent 1 lane. No approvals. | Agent 1: land security fix first (blocks required checks); then scanner track (#95 rebuild, #97 follows #95, #98 review, #94 quiet-host A/B, #104 after `pnpm check` plus S5 disposition); #99 never merges; #101/#102 historical; #91 merges last. Merging nothing here. |
| F1 fixture definition | 10,005 vs 10,000 quota reproduced; `custom-9995` comparison-only. No owner decision. | F1-A exact-10,000 fixture + fresh run (narrowest) vs F1-B coordinated quota + remeasure. Recommend F1-A. Apply neither here. |
| Performance qualification | Warm p95 14,267 ms vs provisional 10 s. #94 contended only; #98 confirms both sides fail 10 s contended. No idle-host evidence. | Quiet-host A/B per §8 plan after Agent 1 reviews `a3e738b`, before any re-budget. |
| F3 scale choice versus dated deferral | 100k unmeasured. Merged proposal #96 `326fb0a` details chunked snapshot; six mechanisms unresolved. Not an approved design. | F3-A chunked snapshot vs F3-B quota to 100k vs F3-C dated deferral. Authorize none here. |
| #47/#48 platform prerequisites | #47 Mirror/Stream authorizations absent; #48 no genuine FAT32 target per #89. Runbooks in #88; blockers merged in #90. | Authorize exact runs or post explicit scope exclusions; invent neither here. |
| P2-10 boundary | Static guards plus fixture equality merged; runtime spy undecided. | Decide static-vs-runtime explicitly; acceptance remains. |
| Remaining criterion-level acceptance | P2-01–P2-10 Partial/present, P2-11 measured-not-qualified, P2-12 Pending, P2-08 Partial (prior `Complete` disputed). Historical stall baseline plus fixed S1–S4 PASS / S5 PARTIAL / S6 PASS in this PR; P2-10 undecided; no acceptance found. | Keep implemented/automated/installed/acceptance separate (table above), decide P2-10, dispose S5, then seek criterion acceptance. Phase 2 stays open. |

## Remaining gates and handoff (Agent 1 merges last)

1. Keep evidence linked to exact commits (merged #90 `f487aa1`, #100 `7980b75`,
   #92 `300c2a4`, #93 `106335a`, #102 `892920d`, #96 `326fb0a` as proposal
   only, #101 `67bdc76`, #103 `55668da`; open #95 `bf0aeac`, #94 `588867b`,
   #97 `d699ecd`, #98 `a3e738b`, validation-only #99 `9c211ad`, fix #104
   `41ac09b`, fixed validation in this PR; prior #91 greens `225ce5b`,
   `322471f`, `8437906` plus `986ca61` fail preserved). Do not relabel
   unmerged observations, proposals, validations, or fixes as merged evidence.
   #99 green never implies constituents are safe. #99 never merges. Do not
   describe merged #96 as an approved scale design.
2. Agent 1: land the security remediation first (required-check blocker), then
   review #95 (rebuild from merged `main` where required), #97 (automated only),
   #98 review, #94 quiet-host A/B, and #104 at `41ac09b` (needs `pnpm check`
   plus S5 successor-vs-same-job disposition; automated #104 evidence stays
   distinct from installed fixed S1–S6). Keep #101/#102 as historical failure
   evidence.
3. Smallest remaining decisions (no new benchmarks, platform surveys, or scale
   implementation here): performance qualification (quiet-host A/B), F1/F3
   choices (exact fixture vs quota; proposal vs deferral), platform
   prerequisites (#47 Mirror/Stream, #48 FAT32), P2-10 static-versus-runtime
   boundary, and criterion-by-criterion phase acceptance. Obtain explicit
   acceptance; do not infer it from merges, green checks, or summaries.
4. Hand this PR to Agent 1 for merge last after constituents land and required
   checks are green. Merging nothing here.

Until those gates are resolved, P2-03 through P2-08 remain behind the
production-scanning gate, the Phase 2 epic remains open, and no Phase 2
acceptance is declared.

## 2026-09-10 final refresh addendum (live `cccaa67`; #104 merged; #106 canonical; #91 duplicate removed)

History above is preserved and not rewritten, including failed replays
(`986ca61`/`15b7f17`/`c3d3292` packaging sidecar-timeout fails with retry
disposition; `90c988d` Prettier fail) and the unpublished `cccd6da` claims
already superseded by actual merges. This addendum supersedes the stale
baseline, CI, and open-draft claims with live verification. No benchmark,
scale implementation, platform survey, gate change, or acceptance is added
here. Agent 1 owns merges; the owner retains acceptance and decision
authority; branch protection is unchanged.

- Live baseline: `origin/main` is now `cccaa67867250ca6708d1aab43a6dbcb758c8a9d`
  (PR #104, merged 2026-09-10). Since #91 head `f847fd1` (base `55668da`),
  main merged #105 (`f63a1d3`; `smol-toml` 1.7.1 fix for GHSA-7w5x-hrqm-74c2),
  #94 (`d342ec1`; remove duplicate enumeration metadata query), #98
  (`7e0e9f6`; contended A/B validation with diagnostic cleanup), #95
  (`ee720af`; Library scan actions aligned with durable state), #97
  (`ff5d8ee`; durable-queue and watcher-burst verification), and #104
  (`cccaa67`; skip failed retry while the root active slot is owned, 2 files:
  `crates/storage-sqlite/src/execution.rs` +17 plus
  `scan_console_host_recovery_tests.rs` +244; regression fails pre-fix and
  passes post-fix).
- Current CI: Foundation `34426244554` on `ff5d8ee` success (all 9 jobs);
  Foundation `34426947016` on `cccaa67` in progress at publish (not cited as
  green); Foundation `34399884756` on `f63a1d3` success (security fixed,
  resolving `34397163352`). The #94 merge-commit run `34424255475` was
  cancelled by the next push; its PR-branch dispatch (`34352810308` /
  `34352813397`) was green and the tree is covered by `34426244554`.
  Foundation `34424822082` on `7e0e9f6` success; Foundation `34425398811` on
  `ee720af` success. #104 head `00bb3e8` packaging `34426552044` success with
  Foundation `34426552063` in progress at publish.
- #106 canonical (open, rebased onto `cccaa67` as `0ef37e9`): publishes the
  corrected 348-line
  `installed-journey/run-20260909-fixed-validation.md`. Corrections preserved:
  tested `ded02ac` is unmerged (not PR #104 head, not `main`); PR #104 fix is
  patch-identical (`patch-id 7aa01a13c7450c2981011b315ac57affd5f06f18` for the
  two fix files; pre-fix base blob `7241dc58` identical); only the lock helper
  `90c988d` is merged (PR #103); suite counts are 87 (desktop) / 76 / 56 per
  the PR #104 body (92 was the integration-base count); lock hashes are
  observed in the `ded02ac` worktree (PR #104 lists none); unmerged-candidate
  vs merged-helper vs remote CI stay separate; merge recommendation targets
  the `main`-line head (now `cccaa67`), never `ded02ac`. Prior head `1c8be9d`
  green (`34423574124` / `34423574177`); new head CI (`34427024157` /
  `34427024185`) in progress at publish. This PR (#91) carries no copy: the
  older 314-line duplicate added at `f847fd1` is deleted here. No overwrite of
  #106, no two competing final reports.
- S5 disposition (Agent 2; no owner acceptance invented): #106 S5 PARTIAL.
  Successor convergence demonstrated (same job stayed `interrupted 1/4` after
  120 s; successor completed `1/4` with committed rows intact; worker
  converged; no wedge; repro IDs `s5-job.txt` / `s5-db-copy.db` / transcript
  preserved). Strict same-job `interrupted` to `completed` not demonstrated.
  Prior `run-20260908.md` remains the only same-job running-recovery evidence
  and is not superseded. Tracked as open issue #107: accept
  successor-recovery as the hard-kill contract or implement same-job requeue.
  #104 merged regardless; S5 does not block scanner readiness.

### Current summary (short)

- Merged `cccaa67` includes the queue-stall fix; #94/#95/#97/#98 are merged;
  #105 resolved the security advisory.
- Evidence classes stay separate. P2-08 remains Partial (PR #85's older
  `Complete` row superseded; no full-criterion acceptance recorded). Merged
  #96 is a proposal only. Merged #101/#102 are historical stall failure
  evidence.
- Installed fix evidence is the #106 canonical report (S1–S4 PASS, S6 PASS,
  S5 PARTIAL tracked as #107). Production scanning stays hidden until P2-03
  through P2-08 have integrated evidence. Phase 2 is not accepted.

### P2-01–P2-12 (implementation | automated evidence | installed evidence | remaining acceptance)

| ID | Implementation (merged `main`) | Automated evidence | Installed evidence | Remaining acceptance |
| --- | --- | --- | --- | --- |
| P2-01 | #34/#35 picker and settings | Client and Windows CI; #58 rendered evidence (fake-adapter, labeled) | #92 selection/cancel, persistence, restart; #54 picker record | Criterion-specific owner acceptance |
| P2-02 | #59 core, #64 enumeration, #70 worker | Deterministic and Windows fixture tests | #92 add/modify/rename and remove/restore convergence | Platform qualification; owner acceptance |
| P2-03 | #62/#70 plus #85 `p2_03_*` fault cases | Fault-injection tests green | #92 cancellation and unavailable retention | Denied/limit cases; owner acceptance |
| P2-04 | #60/#61/#70 plus #82 recovery plus #104 active-slot skip | `migration` and `scan-execution-windows` green; #97 convergence/burst/stale fences; #104 regression | #92 persistence/cancel/recovery; #101/#102 historical stall; #106 S1/S4/S6 PASS plus S5 PARTIAL (#107) | End-to-end acceptance incl. S5 disposition; owner acceptance |
| P2-05 | #60/#62 plus #85 disable/remove/lease/staging; #82 ordering | Disable/remove and lease tests green | #92 disable-while-running retention | Queued-operation coverage; owner acceptance |
| P2-06 | #62 plus #85 crash/rollback/backup/migration | Crash/recovery and backup tests green | #92 clean restart and interrupted recovery | Owner acceptance |
| P2-07 | #59/#62 plus #85 alias and rename tests | Alias and rename tests green | #92 rename convergence; no installed hardlink-alias qualification | #48 qualification; owner acceptance |
| P2-08 | #73/#77/#78/#81/#82 IPC/adapter/lifecycle/supervision; #95 alignment | Feature-on CI green; IPC/adapter/lifecycle/host-recovery tests; #104 regression | #92 journey; #101/#102 historical stall; #106 S1–S4 PASS plus S6 PASS with S5 PARTIAL; `run-20260909.md` | Durable queued state; full-criterion owner acceptance (Partial) |
| P2-09 | #68/#69/#71/#81/#82 watcher/coalescing/generation; #97 verification | Coalescing/overflow/stale tests; #97 automated counts; #104 sweep/claim regression | #92 follow-ups; #101 burst NOT OBSERVED (historical); #106 S3 PASS under isolation | Burst timing, real overflow timing, #47 DriveFS evidence; owner acceptance |
| P2-10 | #81 static no-parser/no-content-I/O guards plus fixture source-byte equality | Static guards and preservation checks green | Native-adapter independence observed (no parser content) | Runtime content-read spy decision; owner acceptance |
| P2-11 | #67/#72 harness, #86 re-run, #93 diagnostics, #94 optimization, #98 validation | F1 reproduced (10,005 vs 10,000 quota); F2 warm p95 14,267 ms vs 10 s contended (both sides fail); F3 unmeasured | No installed perf claim | F1 fixture, F2 qualification, F3 scope; any amended budget |
| P2-12 | #80 checkpoint plus merged implementation/CI map | Merged CI map (`34426244554` success; `34426947016` in progress); #99 validation-only green (historical) | #92 observations; #101/#102 historical stall; #106 S1–S6 (S5 PARTIAL) | Platform/performance/P2-10 decisions plus criterion-by-criterion final acceptance; Pending |

### Remaining decisions only

1. F1 fixture: exact 10,000-observation fixture plus fresh run, or coordinated
   quota change with remeasurement. Authorize neither here.
2. F2 qualification: quiet-host A/B per the recorded plan before any
   re-budget. Contended numbers stay diagnostic only.
3. F3 scope: bounded chunked-snapshot proposal (merged `326fb0a`, mechanisms
   unresolved), coordinated quota increase, or explicit dated deferral.
   Authorize none here.
4. Platform qualification/exclusions: #47 Mirror/Stream authorizations with
   exact disposable leaves, or explicit scope exclusions; #48 disposable
   genuine FAT32 volume or VHD with recorded identity, or explicit exclusion.
   Invent neither here.
5. P2-10 boundary: static guards plus fixture equality vs runtime
   content-read spy. Decide explicitly.
6. Final acceptance: explicit criterion-by-criterion acceptance for P2-01
   through P2-12 (including S5 successor-vs-same-job disposition via #107).
   Do not infer from merges, green checks, or summaries.

### Handoff (Agent 1 merges)

- Checks for this head below (docs/privacy/scripts/diff); CI Foundation plus
  packaging pending at publish not cited as green until runs complete.
- Merge order: Agent 1 merges open PR #106 (canonical fixed validation) on
  green, then this PR #91 last after required checks are green. PR #99 never
  merges. Merging nothing here.

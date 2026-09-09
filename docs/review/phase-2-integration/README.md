# Phase 2 integration evidence index (#41)

Status: **reconciled 2026-09-08; Phase 2 is not accepted.** This is the
current summary index for P2-01 through P2-12. The detailed evidence ledger,
GitHub provenance, acceptance conflict, and remaining gates are in the
[2026-09-08 reconciliation report](reconciliation-2026-09-08.md).

This index separates merged implementation and automated-test evidence from
installed-app observations, performance qualification, platform qualification,
and owner acceptance. A passing CI run or a merge record does not supply the
last three classes. The installed observations below were captured from an
application built from `main`; the observation record and the platform and
profiling reports remain in unmerged sibling PRs and are linked as provisional
evidence until those PRs land.

## Current merged baseline

Fetched origin/main is
`0b7612db3570e6235d4d2c86a678dd9004264f30` (PR #89). The recent material
provenance is:

| Merged PR | Commit | Current contribution |
| --- | --- | --- |
| [#82](https://github.com/guilhermebmichelin-create/fruitboard/pull/82) | `05d39ff` | Native watcher supervision, root mapping, reconnect/coverage handling, shutdown and restart recovery |
| [#85](https://github.com/guilhermebmichelin-create/fruitboard/pull/85) | `c05f4b6` | Worker-level fault, atomicity, recovery, alias, and stale-publication coverage; its P2-08 row update is reconciled below |
| [#86](https://github.com/guilhermebmichelin-create/fruitboard/pull/86) | `4218e41` | New benchmark re-run and profiling notes; no budget decision |
| [#88](https://github.com/guilhermebmichelin-create/fruitboard/pull/88) | `c8d8255` | Executable #47/#48 manual plans; no platform qualification |
| [#89](https://github.com/guilhermebmichelin-create/fruitboard/pull/89) | `0b7612d` | #48 survey: no qualifying writable FAT32/cross-volume target on the host |

The exact-current-commit [Foundation CI run](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34293250231) is successful for `0b7612d` across all nine Foundation jobs. The `windows-packaging-smoke` workflow is pull-request-triggered; its latest relevant exact PR-head evidence is [PR #89 run 34292846032](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34292846032), on head `2896095`, not a post-merge push run.

## Companion evidence under review

The following exact heads and checks were live when this index was reconciled;
none of these sibling PRs was merged by this update.

| PR/head | Evidence | Exact checks and status |
| --- | --- | --- |
| [#90 `17e571742634eceb16f09b166edd3d77b57fcf61`](https://github.com/guilhermebmichelin-create/fruitboard/commit/17e571742634eceb16f09b166edd3d77b57fcf61) | [#47 report](https://github.com/guilhermebmichelin-create/fruitboard/blob/17e571742634eceb16f09b166edd3d77b57fcf61/docs/research/platform-followup/p2-47-drivefs-host-evidence-20260908.md) and [#48 report](https://github.com/guilhermebmichelin-create/fruitboard/blob/17e571742634eceb16f09b166edd3d77b57fcf61/docs/research/platform-followup/p2-48-fat32-identity-evidence-20260908.md) | [Foundation run 34296963183](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34296963183) and [packaging run 34296963219](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34296963219) passed |
| [#92 `49a5e649c688ae767c9189801dd033f2288b61f5`](https://github.com/guilhermebmichelin-create/fruitboard/commit/49a5e649c688ae767c9189801dd033f2288b61f5) | [Installed run record](https://github.com/guilhermebmichelin-create/fruitboard/blob/49a5e649c688ae767c9189801dd033f2288b61f5/docs/review/phase-2-integration/installed-journey/run-20260908.md), captured from `main` `0b7612d` | [Foundation run 34299680195](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34299680195) passed; [packaging attempt 1](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34299680193/job/102303731791) failed with `The installed sidecar smoke timed out`, and [rerun attempt 2](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34299680193/job/102314720612) passed; retain the incident without inferring a product fix or acceptance |
| [#93 `59faefc2806a725368a59e7b6fc9be7f863f4fec`](https://github.com/guilhermebmichelin-create/fruitboard/commit/59faefc2806a725368a59e7b6fc9be7f863f4fec) | [Filesystem-port diagnostic](https://github.com/guilhermebmichelin-create/fruitboard/blob/59faefc2806a725368a59e7b6fc9be7f863f4fec/docs/review/phase-2-integration/performance-followup/README.md) | [Foundation run 34301751666](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34301751666) and [packaging run 34301751625](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34301751625) passed; the profile is contended shared-host diagnostics, not an idle-host performance pass |

The installed evidence is therefore not absent, but it is not yet merged or
accepted. It observes persistence, paging, watcher follow-up convergence,
cancellation retention, and interrupted-work recovery. Durable queued state and
independent watcher-burst counting remain unverified. D1 native labeling, D2
cancelled Retry, and D3 exhausted Retry remain defects until verified fixes and
a fresh installed verification exist.

## Acceptance ID evidence map

| ID | Reconciled status | Merged implementation and automated evidence | Missing or separate evidence |
| --- | --- | --- | --- |
| P2-01 | Evidence present; not promoted | #34/#35 picker and settings work; #54 installed picker record; #58 rendered keyboard/narrow evidence; client and Windows CI | Installed #92 selection/cancel, settings persistence, and restart observation; criterion-specific owner acceptance |
| P2-02 | Partial | #59 reconciliation core, #64 Windows enumeration, #70 hidden worker; deterministic and Windows fixture tests | Installed #92 add/modify/rename and remove/restore convergence; broader platform qualification |
| P2-03 | Partial | #62/#70 behavior plus #85 `p2_03_*` fault cases compare committed rows and the success marker byte-for-byte and assert no publication | Installed #92 cancellation and unavailable-root retention observed; denied/limit cases and owner acceptance remain |
| P2-04 | Partial | #60/#61/#70 durable state-machine and worker coverage; #82 host recovery tests; green `migration`, `scan-execution-windows`, and Windows feature-on CI | Installed #92 persistence, cancellation, and interrupted recovery observed; durable queued state, Retry defects, and end-to-end acceptance remain |
| P2-05 | Partial | #60/#62 and #85 disable/remove, lease, staging, re-add, and stale-publication tests; #82 transaction notification ordering | Installed #92 disable-while-running retention observed; queued-operation coverage and owner acceptance remain |
| P2-06 | Partial | #62 plus #85 crash-before-stage, crash-after-stage, apply rollback, backup recovery, and migration fixtures | Installed #92 clean restart and interrupted-work recovery observed; owner acceptance remains |
| P2-07 | Partial | #59/#62 plus #85 hardlink alias and conservative rename/replacement tests | Installed #92 rename path convergence observed, but no installed hardlink-alias qualification; #48 remains unverified |
| P2-08 | Partial; prior `Complete` claim is not accepted | #73/#77/#78/#81/#82 typed IPC, native adapter, responsiveness, lifecycle, and host supervision evidence; feature-on CI is green | Installed #92 Scan/Cancel/Library/persistence journey observed; D1/D2/D3, durable queued state, the retained packaging-timeout incident, and explicit full-criterion owner acceptance remain |
| P2-09 | Partial | #68/#69/#71/#81/#82 watcher, coalescing, coverage-loss, generation, reconnect, and shutdown tests | Installed #92 watcher follow-up and convergence observed; independent burst counting, real overflow timing, and #47 DriveFS evidence remain |
| P2-10 | Partial | #81 static no-parser/no-content-I/O guards and synthetic-fixture source-byte equality checks | Runtime content-read spy decision; installed-app independence; owner acceptance |
| P2-11 | Measured; not qualified | #67/#72 harness and #86 re-run. F1 is reproduced; F2 remains over target; F3 remains unmeasured; unmerged #93 adds contended filesystem-port diagnostics | F1/F2/F3 owner decisions, required idle-host work, and any amended budget or qualification |
| P2-12 | Pending | #80 checkpoint and subsequent merged implementation/CI map; unmerged #92 records installed observations captured from `main` | Defect fixes and verification, durable queued/burst gaps, platform/performance decisions, and explicit owner acceptance |

## P2-08 acceptance conflict

The documentation change merged in [PR #85](https://github.com/guilhermebmichelin-create/fruitboard/pull/85) says `P2-08 Pending -> Complete` based on the #77/#78 merge-train evidence. That statement conflicts with the same PR body, which says that no acceptance ID status is promoted and that owner review is required. It also conflicts with [PR #82](https://github.com/guilhermebmichelin-create/fruitboard/pull/82), whose body says no acceptance issue is closed, and with the accepted plan's rule that cross-issue criteria require integrated evidence. The owner comment on [PR #63](https://github.com/guilhermebmichelin-create/fruitboard/pull/63) accepts only the UI-only Library seam. The installed-app table is now populated in the unmerged [PR #92 record](https://github.com/guilhermebmichelin-create/fruitboard/blob/49a5e649c688ae767c9189801dd033f2288b61f5/docs/review/phase-2-integration/installed-app-journey-checklist.md), but it records D1/D2/D3 and does not itself promote P2-08.

No explicit owner post or review accepting the full P2-08 criterion was found.
The reconciled status is therefore **Partial**: the native implementation and
automated seam evidence are merged, but installed-app qualification and owner
acceptance are not established.

## Standing gates

- Fake-adapter renders remain fake-adapter evidence. They do not fill the
  installed-app column.
- The [installed-app checklist](installed-app-journey-checklist.md) was executed on an unsigned package built from `main` `0b7612d`; the unmerged [PR #92 checklist](https://github.com/guilhermebmichelin-create/fruitboard/blob/49a5e649c688ae767c9189801dd033f2288b61f5/docs/review/phase-2-integration/installed-app-journey-checklist.md) records persistence, paging, watcher convergence, cancellation retention, and interrupted-work recovery. Durable queued observation and independent watcher-burst counting remain unverified; D1/D2/D3 remain defects until fixed and reverified.
- The current performance source is the [2026-09-08 re-run](benchmark-triage-20260908/rerun-report.md), not only the historical 2026-09-07 report. It was recorded at pre-#85 baseline `51f45af`, reproduces F1 (10,005 observations versus the 10,000-record quota), measures a 14,267 ms warm p95 against the provisional 10 s target, and leaves F3 unmeasured. Unmerged [PR #93](https://github.com/guilhermebmichelin-create/fruitboard/blob/59faefc2806a725368a59e7b6fc9be7f863f4fec/docs/review/phase-2-integration/performance-followup/README.md) adds contended diagnostic profiling; it is not an idle-host performance pass and no budget changed.
- [#47](https://github.com/guilhermebmichelin-create/fruitboard/issues/47) and [#48](https://github.com/guilhermebmichelin-create/fruitboard/issues/48) remain open. The unmerged [PR #90 reports](https://github.com/guilhermebmichelin-create/fruitboard/commit/17e571742634eceb16f09b166edd3d77b57fcf61) specify the exact missing mode/consent and genuine-FAT32 prerequisites; neither report is a platform qualification or owner scope exclusion.
- PR #92's Foundation CI and current packaging rerun are green. Its first packaging attempt recorded the [sidecar timeout](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34299680193/job/102303731791); retain that incident in the handoff, and do not treat any packaging check as installed behavior or owner acceptance.
- Production scanning remains hidden until P2-03 through P2-08 have
  integrated evidence. The epic remains open until the owner accepts the
  checkpoint.

Historical checkpoint, continuation, acceptance-preparation, original benchmark,
post-merge, and platform-research files are preserved as historical records.

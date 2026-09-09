# Phase 2 integration evidence index (#41)

Status: **reconciled 2026-09-08; Phase 2 is not accepted.** This is the
current summary index for P2-01 through P2-12. The detailed evidence ledger,
GitHub provenance, acceptance conflict, and remaining gates are in the
[2026-09-08 reconciliation report](reconciliation-2026-09-08.md).

This index separates merged implementation and automated-test evidence from
installed-app observations, performance qualification, platform qualification,
and owner acceptance. A passing CI run or a merge record does not supply the
last three classes. Local or unmerged agent work remains provisional and is
not used as merged evidence.

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

## Acceptance ID evidence map

| ID | Reconciled status | Merged implementation and automated evidence | Missing or separate evidence |
| --- | --- | --- | --- |
| P2-01 | Evidence present; not promoted | #34/#35 picker and settings work; #54 installed picker record; #58 rendered keyboard/narrow evidence; client and Windows CI | Combined-commit installed selection/cancel/restart observation; criterion-specific owner acceptance |
| P2-02 | Partial | #59 reconciliation core, #64 Windows enumeration, #70 hidden worker; deterministic and Windows fixture tests | Installed add/modify/rename convergence; broader platform qualification |
| P2-03 | Partial | #62/#70 behavior plus #85 `p2_03_*` fault cases compare committed rows and the success marker byte-for-byte and assert no publication | Installed cancellation/offline/denial/limit retention; owner acceptance |
| P2-04 | Partial | #60/#61/#70 durable state-machine and worker coverage; #82 host recovery tests; green `migration`, `scan-execution-windows`, and Windows feature-on CI | Installed retry/cancel behavior and remaining end-to-end acceptance |
| P2-05 | Partial | #60/#62 and #85 disable/remove, lease, staging, re-add, and stale-publication tests; #82 transaction notification ordering | Installed disable/remove-while-running observation; owner acceptance |
| P2-06 | Partial | #62 plus #85 crash-before-stage, crash-after-stage, apply rollback, backup recovery, and migration fixtures | Installed restart/interrupted-work observation; owner acceptance |
| P2-07 | Partial | #59/#62 plus #85 hardlink alias and conservative rename/replacement tests | Installed alias observation; #48 non-NTFS identity qualification or explicit owner scope decision |
| P2-08 | Partial; prior `Complete` claim is not accepted | #73/#77/#78/#81/#82 typed IPC, native adapter, responsiveness, lifecycle, and host supervision evidence; feature-on CI is green | Installed Scan/Cancel/Retry/Library journey and explicit full-criterion owner acceptance |
| P2-09 | Partial | #68/#69/#71/#81/#82 watcher, coalescing, coverage-loss, generation, reconnect, and shutdown tests | Installed watcher observation, real overflow timing, and #47 DriveFS evidence |
| P2-10 | Partial | #81 static no-parser/no-content-I/O guards and synthetic-fixture source-byte equality checks | Runtime content-read spy decision; installed-app independence; owner acceptance |
| P2-11 | Measured; not qualified | #67/#72 harness and #86 re-run. F1 is reproduced; F2 remains over target; F3 remains unmeasured | F1/F2/F3 owner decisions and any amended budget or qualification |
| P2-12 | Pending | #80 checkpoint and subsequent merged implementation/CI map; current report keeps provenance current | Installed-app table, platform/performance decisions, and explicit owner acceptance |

## P2-08 acceptance conflict

The documentation change merged in [PR #85](https://github.com/guilhermebmichelin-create/fruitboard/pull/85) says `P2-08 Pending -> Complete` based on the #77/#78 merge-train evidence. That statement conflicts with the same PR body, which says that no acceptance ID status is promoted and that owner review is required. It also conflicts with [PR #82](https://github.com/guilhermebmichelin-create/fruitboard/pull/82), whose body says no acceptance issue is closed, and with the accepted plan's rule that cross-issue criteria require integrated evidence. The owner comment on [PR #63](https://github.com/guilhermebmichelin-create/fruitboard/pull/63) accepts only the UI-only Library seam. The installed-app observation table remains empty.

No explicit owner post or review accepting the full P2-08 criterion was found.
The reconciled status is therefore **Partial**: the native implementation and
automated seam evidence are merged, but installed-app qualification and owner
acceptance are not established.

## Standing gates

- Fake-adapter renders remain fake-adapter evidence. They do not fill the
  installed-app column.
- The [installed-app checklist](installed-app-journey-checklist.md) remains
  preparation-only and intentionally untouched; its observation table is empty.
- The current performance source is the [2026-09-08 re-run](benchmark-triage-20260908/rerun-report.md), not only the historical 2026-09-07 report. It was recorded at pre-#85 baseline `51f45af`, reproduces F1 (10,005 observations versus the 10,000-record quota), measures a 14,267 ms warm p95 against the provisional 10 s target, and leaves F3 unmeasured. It is not an exact-current-`origin/main` performance qualification; no budget changed.
- [#47](https://github.com/guilhermebmichelin-create/fruitboard/issues/47) and [#48](https://github.com/guilhermebmichelin-create/fruitboard/issues/48) remain open. Their plans and the #48 no-target survey are not platform qualification or owner scope exclusions.
- Production scanning remains hidden until P2-03 through P2-08 have
  integrated evidence. The epic remains open until the owner accepts the
  checkpoint.

Historical checkpoint, continuation, acceptance-preparation, original benchmark,
post-merge, and platform-research files are preserved as historical records.

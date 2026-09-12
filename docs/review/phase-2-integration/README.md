# Phase 2 integration evidence index (#41)

Status: **reconciled 2026-09-08, final 2026-09-10 refresh against live
`b2fb62c`; Phase 2 is not accepted.** This is the current summary index for
P2-01 through P2-12. The detailed evidence ledger, GitHub provenance,
acceptance conflict, decision table, and remaining gates are in the
[2026-09-08 reconciliation report](reconciliation-2026-09-08.md) with its
final 2026-09-10 addendum. Merged PR #96 (`326fb0a`) is a bounded-scan
proposal only, not an approved scale design. Merged PR #101 (`67bdc76`) plus
merged PR #102 (`892920d`) are historical stall failure evidence. Merged
PR #104 (`cccaa67`) carries the queue-stall fix; fixed installed validation
lives in merged PR #106 (canonical corrected report; this PR carries no
duplicate copy) with S5 PARTIAL tracked as open issue #107. Automated and
installed evidence stay distinct.

This index separates implementation, automated evidence, installed evidence,
and remaining acceptance. A passing CI run or a merge record does not supply
acceptance. Installed observations from merged PR #92 plus the historical
PRs #101/#102 baseline and the #106 canonical fixed validation are linked as
versioned records; platform and profiling reports from merged PRs #90/#93 are
merged evidence, not provisional.

## Current merged baseline

Merged `origin/main` is now
`b2fb62c36a057985ab0eba02458f037fcb96c215` (PR #106, merged 2026-09-10).
The material provenance is:

| Merged PR | Commit | Current contribution |
| --- | --- | --- |
| [#82](https://github.com/guilhermebmichelin-create/fruitboard/pull/82) | `05d39ff` | Native watcher supervision, root mapping, reconnect/coverage handling, shutdown and restart recovery |
| [#85](https://github.com/guilhermebmichelin-create/fruitboard/pull/85) | `c05f4b6` | Worker-level fault, atomicity, recovery, alias, and stale-publication coverage; its P2-08 row update is reconciled below |
| [#86](https://github.com/guilhermebmichelin-create/fruitboard/pull/86) | `4218e41` | New benchmark re-run and profiling notes; no budget decision |
| [#88](https://github.com/guilhermebmichelin-create/fruitboard/pull/88) | `c8d8255` | Executable #47/#48 manual plans; no platform qualification |
| [#89](https://github.com/guilhermebmichelin-create/fruitboard/pull/89) | `0b7612d` | #48 survey: no qualifying writable FAT32/cross-volume target on the host |
| [#90](https://github.com/guilhermebmichelin-create/fruitboard/pull/90) | `f487aa1` | Blocked #47/#48 platform follow-up survey; push `34392062754` success |
| [#100](https://github.com/guilhermebmichelin-create/fruitboard/pull/100) | `7980b75` | Packaging fix: outer 30s vs bounded 9.25s inner budget plus stage diagnostics and policy test; push `34391314350` success |
| [#92](https://github.com/guilhermebmichelin-create/fruitboard/pull/92) | `300c2a4` | Installed Windows journey (`run-20260908.md`); push `34392921562` success |
| [#93](https://github.com/guilhermebmichelin-create/fruitboard/pull/93) | `106335a` | Contended benchmark diagnostics; push `34394018328` success |
| [#102](https://github.com/guilhermebmichelin-create/fruitboard/pull/102) | `892920d` | Independent stall confirmation of #101 baseline; push `34394817891` success; historical failure evidence |
| [#96](https://github.com/guilhermebmichelin-create/fruitboard/pull/96) | `326fb0a` | Bounded-scan proposal only, not an approved scale design; push `34395615910` success |
| [#101](https://github.com/guilhermebmichelin-create/fruitboard/pull/101) | `67bdc76` | Final installed regression at frozen `9c211ad`; push `34396496197` success; historical failure evidence |
| [#103](https://github.com/guilhermebmichelin-create/fruitboard/pull/103) | `55668da` | Isolation: exclusive host lock for installed validation |
| [#105](https://github.com/guilhermebmichelin-create/fruitboard/pull/105) | `f63a1d3` | Security fix: `smol-toml` 1.7.1 override for GHSA-7w5x-hrqm-74c2; Foundation `34399884756` success |
| [#94](https://github.com/guilhermebmichelin-create/fruitboard/pull/94) | `d342ec1` | Enumeration: remove duplicate metadata query; PR-branch dispatch green, merge-commit run cancelled by next push |
| [#98](https://github.com/guilhermebmichelin-create/fruitboard/pull/98) | `7e0e9f6` | Validation of #94 with contended A/B and diagnostic cleanup; Foundation `34424822082` success |
| [#95](https://github.com/guilhermebmichelin-create/fruitboard/pull/95) | `ee720af` | Library scan actions aligned with durable state; Foundation `34425398811` success |
| [#97](https://github.com/guilhermebmichelin-create/fruitboard/pull/97) | `ff5d8ee` | Durable-queue and watcher-burst verification; Foundation `34426244554` success |
| [#104](https://github.com/guilhermebmichelin-create/fruitboard/pull/104) | `cccaa67` | Queue-stall fix: skip failed retry while root active slot is owned (2 files; regression fails pre-fix, passes post-fix) |
| [#106](https://github.com/guilhermebmichelin-create/fruitboard/pull/106) | `b2fb62c` | Fixed installed validation (docs-only, canonical report; S5 PARTIAL as #107, no acceptance); PR CI Foundation `34427283235` + Packaging `34427283311` success, post-merge `34427729043` success |

Latest main Foundation run
`34427729043` on `b2fb62c` is success (post-merge, all 9 jobs). Foundation `34426947016`
on `cccaa67` is success (post-merge). The earlier `55668da`
security failure `34397163352` is resolved by merged #105 (`34399884756`
success). Branch protection is unchanged.

## Companion evidence: merged vs under review

PR #99 is validation-only and never merges. Agent 1 owns merges per explicit
user authorization. Historical failed replays stay intact.

Merged (with push CI where applicable):

| PR | Evidence | Checks and status |
| --- | --- | --- |
| [#90 `f487aa1`](https://github.com/guilhermebmichelin-create/fruitboard/pull/90) | #47/#48 blocker reports | Push `34392062754` success |
| [#100 `7980b75`](https://github.com/guilhermebmichelin-create/fruitboard/pull/100) | Packaging fix: outer 30s plus stage diagnostics and policy test | Push `34391314350` success; exact-head `34357665979`/`34357665859` green |
| [#92 `300c2a4`](https://github.com/guilhermebmichelin-create/fruitboard/pull/92) | Installed `run-20260908.md` | Push `34392921562` success; attempt-1 timeout retained, rerun-2 passed |
| [#93 `106335a`](https://github.com/guilhermebmichelin-create/fruitboard/pull/93) | Contended diagnostics | Push `34394018328` success |
| [#102 `892920d`](https://github.com/guilhermebmichelin-create/fruitboard/pull/102) | Stall confirmation of #101 baseline | Push `34394817891` success; historical failure evidence |
| [#96 `326fb0a`](https://github.com/guilhermebmichelin-create/fruitboard/pull/96) | Bounded-scan proposal only, not an approved scale design | Push `34395615910` success |
| [#101 `67bdc76`](https://github.com/guilhermebmichelin-create/fruitboard/pull/101) | Final regression at frozen `9c211ad` | Push `34396496197` success; `c3d3292` 9/10 (packaging sidecar-timeout fail) as history |
| [#103 `55668da`](https://github.com/guilhermebmichelin-create/fruitboard/pull/103) | Isolation: exclusive host lock | Merged; `90c988d` 9/10 Prettier failure superseded |
| [#105 `f63a1d3`](https://github.com/guilhermebmichelin-create/fruitboard/pull/105) | Security fix: `smol-toml` 1.7.1 override | Foundation `34399884756` success; resolves `34397163352` failure |
| [#94 `d342ec1`](https://github.com/guilhermebmichelin-create/fruitboard/pull/94) | Enumeration optimization | PR-branch dispatch `34352810308`/`34352813397` green; merge-commit run cancelled by next push; tree covered by `34426244554` |
| [#98 `7e0e9f6`](https://github.com/guilhermebmichelin-create/fruitboard/pull/98) | PR #94 validation with diagnostic cleanup | Foundation `34424822082` success; contended A/B fails 10 s p95 on both sides, no qualification |
| [#95 `ee720af`](https://github.com/guilhermebmichelin-create/fruitboard/pull/95) | Library scan actions aligned with durable state | Foundation `34425398811` success; adds `installed-journey/run-20260909.md` |
| [#97 `ff5d8ee`](https://github.com/guilhermebmichelin-create/fruitboard/pull/97) | Durable-queue and watcher-burst verification | Foundation `34426244554` success; automated evidence, not installed observation |
| [#104 `cccaa67`](https://github.com/guilhermebmichelin-create/fruitboard/pull/104) | Queue-stall fix plus regression | Merged; 2 files; regression fails pre-fix, passes post-fix; S5 tracked as #107 |

Under review (open):

| PR/head | Evidence | Exact checks and status |
| --- | --- | --- |
| [#99 `9c211ad`](https://github.com/guilhermebmichelin-create/fruitboard/pull/99) (validation-only, never merges) | Full-stack integration validation | Foundation `34360568998` plus packaging `34360569032` success on the frozen head; historical combined-build evidence only |

Implementation, automated evidence, installed evidence, and remaining
acceptance are kept distinct. Merged #92 plus merged #101/#102 (historical
stall baseline) and the #106 canonical fixed validation supply installed
evidence, but none supplies acceptance. Durable queued state in #92 remains
unverified there; merged #97 counts are automated evidence and do not fill
the installed column. Merged #95 aligns Library scan actions with durable
state; merged #104 (`cccaa67`) carries the queue-stall fix (regression fails
pre-fix, passes post-fix; S5 tracked as #107); the #106 canonical fixed
validation (S1–S4 PASS, S5 PARTIAL with successor convergence and repro IDs,
S6 PASS) carries installed evidence and stays distinct. Merged
docs/packaging/isolation work (#90, #93, #100, #103, #105) stays separate
from merged scanner work (#94, #95, #97, #98, #104); #99 green never implies
constituents are safe. Prior #91 heads `f847fd1` (`34423093748`/`34423093848`
green), `225ce5b` green (`34380337434`/`34380337398`), `322471f` green
(`34356977837`/`34356976441`), `8437906` green (`34386309754`/`34386309771`),
and prior `986ca61` (Foundation green with packaging sidecar-timeout fail)
are preserved with retry disposition, not a product regression. Unpublished
`cccd6da` sixth-refresh history is preserved in its worktree; its stale
`#103 pending` / `ded02ac`-only / `0b7612d`-baseline claims are superseded
here by actual merges.

The installed-app defect repair replay for PR #92 is recorded additively in
[`installed-journey/run-20260909.md`](installed-journey/run-20260909.md). The
original installed observations remain preserved on PR #92 and are not
overwritten by that follow-up record.

The 2026-09-12 #107 queued/restart recovery is preserved in the installed
journey addendum, with the historical driver's provenance kept separate from
the locked follow-up driver. The remaining denied-traversal, unchanged-bound
`ResourceLimit`, and in-root hardlink cases are recorded separately in
[`installed-journey/run-20260912-ntfs-cases.md`](installed-journey/run-20260912-ntfs-cases.md).
That record is local NTFS evidence only; it does not qualify FAT32, DriveFS,
network shares, performance, or Phase 2 acceptance.

## Acceptance ID evidence map (implementation | automated | installed | remaining)

| ID | Implementation (merged `main`) | Automated evidence | Installed evidence | Remaining acceptance |
| --- | --- | --- | --- | --- |
| P2-01 | #34/#35 picker and settings work | Client and Windows CI; #58 rendered keyboard/narrow evidence (fake-adapter, labeled) | #92 selection/cancel, settings persistence, restart observation; #54 picker record | Criterion-specific owner acceptance |
| P2-02 | #59 reconciliation core, #64 Windows enumeration, #70 hidden worker | Deterministic and Windows fixture tests | #92 add/modify/rename and remove/restore convergence | Broader platform qualification; owner acceptance |
| P2-03 | #62/#70 behavior plus #85 `p2_03_*` fault cases (committed rows and success marker byte-for-byte, no publication) | Fault-injection tests green | #92 cancellation and unavailable-root retention observed | Denied/limit cases; owner acceptance |
| P2-04 | #60/#61/#70 durable state machine plus #82 host recovery plus #104 active-slot skip | `migration` and `scan-execution-windows` green; #97 mid-scan convergence, idle-burst collapse, stale-generation fencing; #104 regression fails pre-fix, passes post-fix | #92 persistence, cancellation, interrupted recovery; #101/#102 historical stall; #106 canonical S1/S4/S6 PASS plus S5 PARTIAL (successor; same-job not demonstrated; #107) | End-to-end acceptance including S5 disposition; owner acceptance |
| P2-05 | #60/#62 plus #85 disable/remove, lease, staging, re-add, stale-publication; #82 ordering | Disable/remove and lease tests green | #92 disable-while-running retention observed | Queued-operation coverage; owner acceptance |
| P2-06 | #62 plus #85 crash-before/after-stage, rollback, backup, migration fixtures | Crash/recovery and backup tests green | #92 clean restart and interrupted-work recovery observed | Owner acceptance |
| P2-07 | #59/#62 plus #85 hardlink alias and rename/replacement tests | Alias and rename tests green | #92 rename convergence observed; no installed hardlink-alias qualification | #48 qualification; owner acceptance |
| P2-08 | #73/#77/#78/#81/#82 typed IPC, native adapter, responsiveness, lifecycle, supervision; #95 Library/durable-state alignment | Feature-on CI green; IPC/adapter/lifecycle/host-recovery tests green; #104 regression green | #92 journey; #101/#102 historical stall; #106 canonical S1–S4 PASS plus S6 PASS with S5 PARTIAL; `run-20260909.md` replay | Durable queued state; full-criterion owner acceptance (Partial, not Complete) |
| P2-09 | #68/#69/#71/#81/#82 watcher, coalescing, coverage-loss, generation, reconnect, shutdown; #97 verification | Coalescing/overflow/stale tests green; #97 counts automated only; #104 sweep/claim regression green | #92 follow-ups; #101 burst NOT OBSERVED (historical stall); #106 canonical S3 burst PASS under isolation | Installed burst timing, real overflow timing, #47 DriveFS evidence; owner acceptance |
| P2-10 | #81 static no-parser/no-content-I/O guards plus fixture source-byte equality | Static guards and preservation checks green | Installed-app independence observed via #92 native adapter (no parser content) | Runtime content-read spy decision; owner acceptance |
| P2-11 | #67/#72 harness, #86 re-run, #93 contended diagnostics, #94 optimization, #98 validation | F1 reproduced (10,005 vs 10,000 quota); F2 warm p95 14,267 ms vs 10 s (contended; both sides fail); F3 unmeasured | No installed perf claim | F1 fixture, F2 qualification (quiet-host A/B), F3 scope; any amended budget |
| P2-12 | #80 checkpoint plus merged implementation/CI map | Merged CI map (`34426244554` success; `34426947016` in progress); #99 validation-only green (historical) | #92 observations; #101/#102 historical stall; #106 canonical S1–S6 (S5 PARTIAL) | Platform/performance/P2-10 decisions plus criterion-by-criterion final acceptance; Pending |

## P2-08 acceptance conflict

The documentation change merged in [PR #85](https://github.com/guilhermebmichelin-create/fruitboard/pull/85) says `P2-08 Pending -> Complete` based on the #77/#78 merge-train evidence. That statement conflicts with the same PR body, which says that no acceptance ID status is promoted and that owner review is required. It also conflicts with [PR #82](https://github.com/guilhermebmichelin-create/fruitboard/pull/82), whose body says no acceptance issue is closed, and with the accepted plan's rule that cross-issue criteria require integrated evidence. The owner comment on [PR #63](https://github.com/guilhermebmichelin-create/fruitboard/pull/63) accepts only the UI-only Library seam. The installed-app table is populated in merged [PR #92](https://github.com/guilhermebmichelin-create/fruitboard/pull/92), but it records D1/D2/D3 and does not itself promote P2-08.

No explicit owner post or review accepting the full P2-08 criterion was found.
The reconciled status is therefore **Partial**: the native implementation and
automated seam evidence are merged, but installed-app qualification and owner
acceptance are not established.

## Standing gates

- Fake-adapter renders remain fake-adapter evidence. They do not fill the
  installed-app column.
- The [installed-app checklist](installed-app-journey-checklist.md) now carries
  merged #103 isolation (exclusive host lock shared by automated smoke and
  manual journey). Merged [PR #92](https://github.com/guilhermebmichelin-create/fruitboard/pull/92)
  records persistence, paging, watcher convergence, cancellation retention, and
  interrupted-work recovery. Durable queued observation and independent
  watcher-burst counting remain unverified there; merged #97 counts are
  automated evidence and do not fill the installed column. Merged #95 aligns
  Library scan actions with durable state; merged #104 fixes the retry sweep.
  The #106 canonical fixed validation adds S1–S4 PASS, S5 PARTIAL (tracked as
  #107), S6 PASS under isolation, with no acceptance claimed.
- The current performance source is the [2026-09-08 re-run](benchmark-triage-20260908/rerun-report.md), not only the historical 2026-09-07 report. It was recorded at pre-#85 baseline `51f45af`, reproduces F1 (10,005 observations versus the 10,000-record quota), measures a 14,267 ms warm p95 against the provisional 10 s target, and leaves F3 unmeasured. Merged [PR #93](https://github.com/guilhermebmichelin-create/fruitboard/pull/93) adds contended diagnostic profiling; merged #94 removes the duplicate query; merged #98 validates with contended A/B (both sides fail 10 s p95). Neither is an idle-host performance pass and no budget changed. Quiet-host A/B still required. Scale options are in merged PR #96 (`326fb0a`) as a proposal only, not an approved scale design.
- [#47](https://github.com/guilhermebmichelin-create/fruitboard/issues/47) and [#48](https://github.com/guilhermebmichelin-create/fruitboard/issues/48) remain open. Merged [PR #90](https://github.com/guilhermebmichelin-create/fruitboard/pull/90) specifies the exact missing mode/consent and genuine-FAT32 prerequisites; neither report is a platform qualification or scope exclusion, and no new consent or volume is invented here.
- PR #92's Foundation CI and packaging rerun attempt 2 are green. Its first packaging attempt recorded the [sidecar timeout](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34299680193/job/102303731791); retain that incident in the handoff, and do not treat any packaging check as installed behavior or owner acceptance. The same recurring hosted sidecar-timeout signature recurs on docs-only heads PR #98 `15b7f17` (`34352822867`), prior #91 `986ca61` (`34353724295`), and PR #101 `c3d3292` (`34384925678`, build plus package succeeded in each case) with retry disposition; PR #101 is therefore 9/10, not green.
- Production scanning remains hidden until P2-03 through P2-08 have
  integrated evidence. The epic remains open until the owner accepts the
  checkpoint.

Historical checkpoint, continuation, acceptance-preparation, original benchmark,
post-merge, and platform-research files are preserved as historical records.

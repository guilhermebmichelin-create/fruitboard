# Phase 2 closeout acceptance packet - 2026-09-12

Status: **coordination and decision input only; Phase 2 is not accepted.** This
is the current acceptance summary for P2-01 through P2-12. It does not merge a
pull request, post an issue message, close an issue, amend a budget or fixture,
change the S5 contract, activate production scanning, or grant owner
acceptance.

## Source review and document authority

The unpublished packet
`acceptance-packet-2026-09-10.md` was recovered from the existing
`agent1-phase2-closeout` worktree and reviewed before reuse. The older
`acceptance-prep-2026-09-08.md`, `continuation-2026-09-08.md`,
`checkpoint-2026-09-07.md`, and the dated reconciliation report remain
historical records. They are not rewritten here, and their old baselines,
open-PR descriptions, and planning checkboxes must not be read as current
GitHub state.

The active sources are now:

| Source                                                                          | Authority in this closeout                                              |
| ------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| [Live GitHub baseline](https://github.com/guilhermebmichelin-create/fruitboard) | Current branch, PR, issue, and CI state below                           |
| [Phase 2 execution plan](../../PHASE_2_EXECUTION_PLAN.md)                       | Binding contracts and provisional budgets; no silent budget change      |
| [Reconciliation report](reconciliation-2026-09-08.md)                           | Preserved historical ledger and dated addenda                           |
| [This packet](acceptance-packet-2026-09-12.md)                                  | Current evidence classes, decisions, coordination, and proposed updates |

The S5 regression and its preserved 2026-09-10 disposition were published by
merged PR [#110](https://github.com/guilhermebmichelin-create/fruitboard/pull/110)
at head `e209b8ad46adfc2d66d2c2b18288d0ef254eeb8a`, with merge commit
`e2948f1bcc03af162ff95ba06ddd6033f2547da6`. The September 12 driver,
installed report, checklist addendum, and index changes remain installed
evidence from canonical source `19585dae7bef9ffdec06b71e0627df1b3f7ceb2f`;
they are merged as records, not owner acceptance. The dirty main checkout's
source copies remain untouched.

PR [#111](https://github.com/guilhermebmichelin-create/fruitboard/pull/111) was
merged at head `606635c06b6eabf345c3e91fa95409eee654afea` with merge commit
`914d7bd2475a11a5e7286086f15bce1d4bd148a5`. It is a mixed product-and-evidence
change, not an evidence-only PR: its local-NTFS report records denied
traversal, unchanged-bound `ResourceLimit`, and in-root hardlink observations,
while tested source `05fb35c153dd3f177b900292d39998da3774b5e4` changes durable
product-owned diagnostics and desktop scan-console mapping for `access_denied`,
`resource_limit`, `unsupported`, `unavailable`, and `worker_failed`. The
installed run used `05fb35c` and was not rerun for the later hardening revision.
The product changes are in merged `main`, but the installed observations remain
tied to their tested source and are not owner acceptance.

The #111 history also contains recovered S5 commit `de396e7`. It has the same
parent, tree, and stable patch ID as the canonical S5 source
`19585dae7bef9ffdec06b71e0627df1b3f7ceb2f`; it is preserved as historical
provenance, not counted as a second recovery experiment or duplicate recovery
commit. The current #111/#112 source stack already keeps the canonical S5 patch
once, so duplicate-S5 removal is complete and is not a future rebase action.
Historical replay descriptions below preserve the original transplant/drop
language as evidence only.

The September 12 installed verification used canonical source
`19585dae7bef9ffdec06b71e0627df1b3f7ceb2f`, whose diff from `3ebac5f` is
test/documentation only. The additive report records queued state, queued
disable/remove, and a genuine same-job restart recovery; those observations
are now published in merged #110, remain installed evidence, and are not
owner-accepted.

The separate #111 NTFS run used tested source
`05fb35c153dd3f177b900292d39998da3774b5e4` and its report was published in
reviewed revision `ef085229471301043a50f4b668901d9c956fb407`.
The later hardening revision adds durable/API validation and harness cleanup
without rerunning the installed cases. The tested source includes the focused
product correction which persists fixed durable diagnostics and maps them to
typed desktop scan-console presentation; that correction is now in merged
`main`, while the installed observations remain historical tested-source
evidence. Owner acceptance remains a separate decision.

PR [#112](https://github.com/guilhermebmichelin-create/fruitboard/pull/112)
merged at head `386b4c9808bca0853dbe71757f121d4106b6d82e` with merge commit
`00884ba87c47a24f3ad75aa31f34ef7efe4a5bb8`, based on the verified merged #111
baseline. Its diagnostic revisions are now in main; they remain diagnostic and
do not establish performance qualification. PR [#114](https://github.com/guilhermebmichelin-create/fruitboard/pull/114)
merged at head `0aa9020c5f524de7f7d0f78226f500f5e021baf3` with merge commit
`71848732216d4ab4e13e73c820b2b8e4d17bddbe4`; it adds the qualification runbook
and report validator, but no qualification run. Both merge states were
verified on GitHub; neither establishes owner acceptance.

The #112 disposition is explicit:
the original `3ebac5f` current-main `Failed`/`Partial` artifact remains
unexplained and was not reproduced. It adds bounded, path-free
`partial_class` coverage and a sanitized benchmark protocol; later passes do
not establish the original cause or performance acceptance. It does not
change limits, budgets, safety checks, storage schema, or the S5 contract.

Agent 2 found no product or contract defect in the S5 follow-up and did not
rerun S5. Its local
validation passed storage tests (77), desktop feature-on unit tests (92),
desktop feature-on clippy, driver syntax, Markdownlint, privacy, and format
checks under Rust/Cargo `1.98.1`. The full desktop Cargo command reached 92
passing unit tests but failed its local Rustdoc phase with `E0463` missing
extern crates; `pnpm check` stopped at the unavailable Node `24.20.0`/`uv`
environment. The live checks in the dated status table are GitHub check
provenance only; they do not constitute an approval, acceptance, or merge.

Agent 3's strict quiet-host preflight was completed fail-closed in draft PR
[#108](https://github.com/guilhermebmichelin-create/fruitboard/pull/108) at
`2561d3f`: DriveFS activity, an unverified Defender exclusion, sibling
activity, and missing 60-second CPU/disk idle proof prevented a qualifying
quiet-host run. The branch later published normal-configuration diagnostic
evidence at `a44653b`: before/candidate/current-main medians are 14,879 / 10,120
/ 14,982 ms and nearest-rank p95 values are 44,778 / 10,391 / 25,601 ms;
current-main has 9/10 authoritative iterations plus one retained partial
failure. The separate before, candidate, and current-main phase profiles
identify `next_entry` plus `read_metadata` at about 84% of scan wall time and
over 96% of timed filesystem-port time. These results are not quiet-host
qualification or acceptance evidence; the 10-second target, promotion, quota
changes, and historical classifications remain unchanged.

## Live GitHub baseline

The verified source baseline is
`71848732216d4ab4e13e73c820b2b8e4d17bddbe`, the squash merge commit for #114
after #112's merge commit `00884ba87c47a24f3ad75aa31f34ef7efe4a5bb8`; it also
contains #111's merge commit `914d7bd2475a11a5e7286086f15bce1d4bd148a5` and
the #110 merge commit `e2948f1bcc03af162ff95ba06ddd6033f2547da6`. See the
[dated live review status table](README.md#live-review-status-2026-09-13) for
the exact OIDs and current PR state. GitHub reports #110, #111, #112, and #114
as merged at this snapshot.
The source-stack state is not inferred from a synthetic candidate or an
installed evidence source.

Main protection is unchanged. It requires pull requests and ten named
contexts with strict up-to-date branches. The nine-job Foundation run for the
merged baseline is not described as the ten-context protection result.

## Source-stack and installed-source boundaries

The merged source sequence is #110 -> #111 -> #112 -> #114. #112's diagnostic
follow-up and #114's qualification-runbook/validator changes are in the merged
baseline, but no qualification run or owner acceptance is recorded. PR #113 is
a separate older synthetic combined
candidate at `e90b03cc0bddd1a449e817ea2d89708a85c82ef1`, based on the earlier
`main` baseline, not a refresh of the current source state. Its own green
checks do not validate an unmerged newer stack. This distinction is a review
dependency, not an acceptance statement.

The independent review artifact in the #112/#113 history reviewed earlier
candidate `37cd6c6ec1bef00af03456ddd6155cd0c704c8b8` and explicitly says it is
not an approval record for later candidate `e90b03cc0bddd1a449e817ea2d89708a85c82ef1`.
No GitHub review approval is inferred here; green checks are CI provenance only.

Agent 2's installed evidence used canonical S5 source
`19585dae7bef9ffdec06b71e0627df1b3f7ceb2f`, whose diff from merged `main` is
test/documentation-only. The #111 installed NTFS cases used tested product
source `05fb35c153dd3f177b900292d39998da3774b5e4` and reviewed report revision
`ef085229471301043a50f4b668901d9c956fb407`; the current #111 head was not used
to relabel those observations. The #111 product correction is now in merged
`main`, but no installed NTFS case was rerun on that merged source. The #92 and
the #106 installed records retain their recorded baselines. None of these sources
is owner acceptance.

## Merged implementation and evidence classes

The following merged commits are provenance, not acceptance. The classification
is intentional:

| PR / merge commit                     | Classification and contribution                                                                                                 |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| #82 / `05d39ff`                       | Merged watcher supervision, root mapping, reconnect, shutdown, and recovery implementation                                      |
| #85 / `c05f4b6`                       | Merged durability, fault, atomicity, alias, and stale-publication implementation/tests                                          |
| #90 / `f487aa1`                       | Merged blocker report for #47/#48; no platform qualification                                                                    |
| #92 / `300c2a4`                       | Merged installed journey record; observations are from its recorded build and remain installed evidence, not current-head proof |
| #93 / `106335a`                       | Merged contended benchmark/profile evidence; not quiet-host qualification                                                       |
| #94 / `d342ec1`                       | Merged duplicate metadata-query cleanup; no end-to-end p95 pass established                                                     |
| #95 / `ee720af`                       | Merged Library actions/durable-state alignment                                                                                  |
| #96 / `326fb0a`                       | Merged bounded chunked-snapshot proposal only; not an approved scale design                                                     |
| #97 / `ff5d8ee`                       | Merged automated durable-queue and watcher-burst verification; not installed observation                                        |
| #98 / `7e0e9f6`                       | Merged contended A/B validation and diagnostic cleanup; both sides miss the 10 s p95 target                                     |
| #100 / `7980b75`                      | Merged packaging-smoke deadline/diagnostic fix                                                                                  |
| #101 / `67bdc76` and #102 / `892920d` | Merged historical installed/stall-failure records; not current success evidence                                                 |
| #103 / `55668da`                      | Merged exclusive installed-test lock helper                                                                                     |
| #104 / `cccaa67`                      | Merged queue-stall fix for failed retry while a root active slot is owned; automated regression passes                          |
| #105 / `f63a1d3`                      | Merged `smol-toml` 1.7.1 security remediation                                                                                   |
| #106 / `b2fb62c`                      | Merged canonical fixed installed report; S1-S4 and S6 pass, S5 is partial; docs-only                                            |
| #91 / `3ebac5f`                       | Merged final reconciliation refresh; current live baseline, not a pending merge                                                 |

PR #99 is not a merge vehicle. Its green combined-build checks, like any
automated check, do not make its constituents safe or accepted. Agent 2's
canonical S5 source
`19585dae7bef9ffdec06b71e0627df1b3f7ceb2f` is unmerged test/documentation
provenance and is not listed as a merged implementation.

The merged implementation record also includes #110 at merge commit
`e2948f1bcc03af162ff95ba06ddd6033f2547da6` and #111 at merge commit
`914d7bd2475a11a5e7286086f15bce1d4bd148a5`. Their merge records do not
convert the historical installed observations into current-head evidence.

## P2-01 through P2-12 evidence ledger

Installed records have their own provenance. The #92 journey was recorded on an
older merged baseline, and the #106 fixed validation tested the unmerged,
patch-identical candidate `ded02ac`, not `3ebac5f`. Neither is silently
relabeled as an installed run on the current head. The #110 queued/restart
record used canonical source
`19585dae7bef9ffdec06b71e0627df1b3f7ceb2f`; the #111 NTFS record used
tested product source `05fb35c153dd3f177b900292d39998da3774b5e4` and was
published at reviewed revision
`ef085229471301043a50f4b668901d9c956fb407`. The product changes from both
merged PRs are now in main at `914d7bd`, but those installed observations have
not been silently relabeled as current-head runs. Tested-source, PR-head,
merged-code, CI, and owner-acceptance boundaries remain separate.

<!-- markdownlint-disable MD060 -->

| ID    | Merged implementation                                                              | Automated tests/evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Installed evidence                                                                                                                                                                                                                                           | Remaining acceptance                                                   |
| ----- | ---------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------- |
| P2-01 | #34/#35 picker, settings, and inline onboarding implementation                     | Client/Windows CI and #58 fake-adapter keyboard/narrow evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | #92 picker cancel/selection, persistence, and restart; #54 picker record                                                                                                                                                                                     | Owner decision on inline onboarding and criterion acceptance           |
| P2-02 | #59 reconciliation core, #64 enumeration, #70 worker                               | Deterministic reconciliation and Windows fixture tests                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | #92 add/modify/rename and remove/restore convergence                                                                                                                                                                                                         | Platform scope and owner acceptance                                    |
| P2-03 | #62/#70 safety path plus #85 fault cases                                           | Partial/offline/cancel/resource-limit fault-injection tests; #111 adds focused diagnostic mapping tests                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | #92 cancellation and unavailable-root retention; #111 `ef08522` records local-NTFS denied traversal and unchanged-bound `ResourceLimit` passes, tested from historical source `05fb35c`                                                                               | Denied/limited traversal and owner acceptance                          |
| P2-04 | #60/#61/#70 durable state, #82 recovery, #104 active-slot fix                      | Migration/scan-execution gates, #97 convergence/stale fences, and #104 regression                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | #92 persistence/cancel/recovery; #101/#102 historical stall; #106 S1/S4/S6 pass and S5 successor path; #110 queued/running/terminal states and genuine same-job restart recovery; #111 `de396e7` is tree-identical recovered S5 provenance, not new evidence | Installed S5 path disposition and owner acceptance                     |
| P2-05 | #60/#62/#85 lease, disable/remove, staging, and stale-publication paths            | Lease, disable/remove, and publication tests                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | #92 disable-while-running retention; September 12 queued disable and queued remove cancel before publication                                                                                                                                                 | Installed evidence is pending review and owner acceptance              |
| P2-06 | #62/#85 atomic publication, crash/rollback, backup, and migration paths            | Crash, recovery, backup, and migration tests                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | #92 clean restart/interrupted recovery; #106 did not establish a genuine crash with a running lease; September 12 hard-kill copy proves a running lease, then same job/retry chain recovery                                                                  | Correctly classified crash-recovery evidence and owner acceptance      |
| P2-07 | #59/#62/#85 identity, alias, rename, and replacement paths                         | Alias and rename tests                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | #92 rename convergence; #111 `ef08522` true in-root hardlink-alias result on local NTFS, tested from historical source `05fb35c`                                                                                                                                      | #48 decision/qualification and owner acceptance                        |
| P2-08 | #73/#77/#78/#81/#82 IPC, native adapter, lifecycle, and supervision; #95 alignment | Feature-on CI and IPC/adapter/lifecycle/recovery tests; #104 regression; #111 diagnostic-mapping tests                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | #92 journey; #106 S1-S4/S6 installed observations; #110 native/UI queued snapshot and queued invalidation; #111's tested-source `05fb35c` product changes for durable diagnostics and typed desktop error presentation; full criterion remains Partial                        | Full-criterion review remains Partial, not Complete                    |
| P2-09 | #68/#69/#71/#81/#82 watcher, coalescing, generation, and reconnect paths           | #97 automated queue/burst/fence counts and watcher tests                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | #92 follow-ups; #106 isolated burst pass; real overflow timing and DriveFS remain unverified                                                                                                                                                                 | Installed timing/overflow evidence, #47 decision, and owner acceptance |
| P2-10 | #81 static no-parser/no-content-I/O guards and fixture-byte equality               | Dependency/privacy checks, static guards, and preservation tests                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Native-adapter independence was observed; no runtime content-read spy was run                                                                                                                                                                                | Owner chooses static boundary or runtime spy gate                      |
| P2-11 | #67/#72 harness, #86 rerun, #93 profile, #94 cleanup, #98 validation               | F1 reproduced; historical contended warm p95 14,267 ms versus 10 s; Agent 3's strict quiet-host gate failed closed, while draft #108 `a44653b` records normal-config diagnostic medians 14,879 / 10,120 / 14,982 ms and p95 values 44,778 / 10,391 / 25,601 ms plus before/candidate/current-main profiles; current-main retains 9/10 authoritative iterations plus one `Failed`/`Partial` at 11,137 ms. #112 records the original Partial as unexplained and not reproduced, and adds bounded diagnostic coverage plus a sanitized protocol. Later passes do not establish its cause or performance acceptance; F3 remains unmeasured | No installed performance qualification                                                                                                                                                                                                                       | F1/F2/F3 decisions and any owner-approved remeasurement                |
| P2-12 | #80 checkpoint plus merged implementation/CI map and #91 refresh                   | Current exact-head Foundation run is green; historical PR #99 remains validation-only                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | #92 and #106 records plus September 12 report, with canonical S5 provenance preserved                                                                                                                                                                        | Decisions above, then explicit P2-01-P2-12 owner acceptance            |

The table deliberately does not use `Complete` as a synonym for “merged” or
“automated checks pass.” P2-08's older `Complete` wording is superseded by the
reconciled Partial disposition. No acceptance ID is promoted by this packet.

For the installed-evidence cells, `unmerged` qualifies the historical tested
source, not the current GitHub status of #111. The #111 product changes are in
merged main `914d7bd`; the local-NTFS observations remain tied to `05fb35c`.

<!-- markdownlint-enable MD060 -->

## S5: genuine crash recovery versus terminal follow-up

Current status: #110, #111, #112, and #114 are merged in main. #112 merged at
`00884ba87c47a24f3ad75aa31f34ef7efe4a5bb8`; #114 merged at
`71848732216d4ab4e13e73c820b2b8e4d17bddbe4`. The ordered records below retain
the historical review sequence; neither merge establishes qualification or
owner acceptance.

The #106 run must be read as two different state-machine paths:

1. It observed a process in `running`, but the harness had already sent a
   coalesced `scan_now` that returned `already_running`. That set
   `follow_up_requested`; the worker later finished the old run as terminal
   `interrupted` and created a queued successor.
2. `taskkill /F` happened after that terminal transition. After relaunch, the
   old job correctly stayed `interrupted` while the successor completed with
   committed rows. This is successor recovery from an already-terminal
   follow-up, not a crash recovery of a still-running lease.

Genuine crash recovery is different: restart finds a run still marked
`running`, `recover_interrupted_tx` marks that run interrupted with `restart`,
and the same job/retry chain is requeued for a fresh run. The existing
automated test `restart_requeues_the_interrupted_attempt_without_resetting_its_chain`
covers that state-machine rule. The #106 installed record does not prove that
path because its database copy had no running run before relaunch.

Agent 2's `19585da` pin replays the terminal follow-up path and asserts that the
old job is not resurrected and the successor is leaseable. It adds no product
fix. The September 12 installed record separately captures the genuinely
running-lease case: after an exact-PID hard kill, the same job and retry chain
reached attempt two, the old run became `interrupted/restart`, and a fresh run
completed with 8,000 published rows. It also records queued disable/remove
with `runId: null` before mutation. The report and driver are published in
merged PR #110; the reviewed evidence still requires owner acceptance and does
not promote P2-04,
P2-05, P2-06, or P2-08.

The #111 historical review record repeats the recovered S5 tree in `de396e7`, but
that commit has the same parent, tree, and stable patch ID as `19585da`; it is
not another recovery run. The current dependency stack has already retained #110's `19585da`
canonical and removed the duplicate from the live #111/#112 source ancestry.
Do not drop or replay `de396e7` during future updates; preserve it only as
historical provenance. Shared checklist/index files already carry the #110
queued/restart rows together with the #111 denied, ResourceLimit, and hardlink
rows. Historical replay SHAs and their original descriptions remain cited
below.

The #111 product portion is separately reviewable: tested source `05fb35c`
persists fixed worker/storage diagnostic codes and the desktop scan console maps
those durable codes to typed user-facing errors. It is not evidence-only. Its
product changes are merged in `914d7bd`; the installed observations remain tied
to `05fb35c`, and no owner acceptance follows from its green checks.

No additional S5 or NTFS run is requested for this stack. A future run would
require a newly identified evidence defect and separate owner authorization; it
must preserve the distinction between a terminal follow-up and a genuine
running-lease recovery, and must not invent a product/contract change.

## Ordered host windows

The host is a shared resource. Agent 3's strict quiet-host window is
non-qualifying, and its later normal-config measurements are diagnostic only;
the Agent 2 follow-up remains coordinated below:

| Order | Owner                      | Window and required controls                                                                                                                                                                                                                                                                                                                                                                    |
| ----- | -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1     | Agent 3                    | **Quiet-host gate failed closed at `2561d3f`; PR #108's later normal-config evidence is diagnostic only.** DriveFS, Defender verification, sibling activity, and idle-proof gates were ineligible for qualification. No quiet-host claim or promotion follows.                                                                                                                                  |
| 2     | Agent 2                    | **Completed evidence follow-up in PR #110.** The existing S5 regression and September 12 installed artifacts are isolated and published; no product/contract defect was found and no S5 rerun was performed. Its recorded checks are not a GitHub approval or owner acceptance.                                                                                                                 |
| 3     | Agent 1 / Agent 3 review   | **Review PR #111 as the mixed NTFS/product follow-up.** Its reviewed report revision is the unmerged NTFS evidence source recorded above; review durable diagnostic persistence and typed desktop error mapping separately from the local-NTFS observations, and do not replay `de396e7`.                                                                                                       |
| 4     | Agent 2 independent review | **Review the older combined candidate after comparing it with the current #110 → #111 → #112 source stack.** Check that #111's durable diagnostic propagation remains intact alongside #112's `partial_class` reporting and sanitized benchmark protocol. This is a code/evidence gate only; it does not establish the original Partial's cause, performance acceptance, or Phase 2 acceptance. |

The ordered table preserves the historical coordination plan. Current GitHub
state is #110, #111, #112, and #114 merged in main; #112's diagnostics and
The #114 runbook/validator remain non-qualification evidence. The #111 report
revision is historical installed evidence even though its product changes are
merged; it is not a current-head rerun.

Before and throughout Agent 3's measurement, and during any Agent 2
reproduction window, pause heavy Rust, Tauri, Cargo, pnpm, packaging,
benchmark, and installed validation. Do not start a sibling build, installed
run, or other host-intensive task until the active window's samples and logs
are complete. If any quiet-host preflight fails, abort and record the window as
non-qualifying; do not relabel the contended data as quiet-host evidence.

Neither window may delete `storage/owner.lock`, kill an unrelated process, or
overwrite another worktree's smoke data. A lock acquisition failure is a
fail-closed result that records the foreign owner.

## Owner decision list

These are recommendations and evidence boundaries, not decisions made by this
packet.

| Decision                  | Recommendation                                                                                                                                                           | Alternative                                                                                           | Code required                                                                                      | Evidence required                                                                                                                                                                                         | Acceptance criterion affected   |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------- |
| F1 fixture alignment      | Define qualification as exactly 10,000 scanner observations: `custom-9995`, seed `0`, and manifest `a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a`.   | Retain the 10,005-observation fixture and raise every coupled bound together.                         | No for alignment; yes for coordinated worker, staging, path-byte, buffer, and publication changes. | Rerun the accepted protocol with the recorded fixture; remeasure all changed bounds if the alternative is chosen.                                                                                         | P2-11 / F1                      |
| F2 qualifying measurement | Authorize an eligible quiet-host A/B before optimization or target/budget change.                                                                                        | Defer measurement or make a named host-class/target decision; optimize only after eligible data.      | No code for measurement; optimization requires a separate owner-approved change.                   | AC/high-performance mode, verified DriveFS pause and AV exclusion, no sibling load, 60 seconds idle proof, pinned release build, one warm-up, ten unfiltered iterations, median/maximum/nearest-rank p95. | P2-11 / F2                      |
| F3 scale                  | Make a dated deferral and retain the current 10,000-entry contract.                                                                                                      | Resolve merged proposal #96 or coordinate an end-to-end 100,000-entry limit increase.                 | No for deferral; yes for chunking or coordinated limit changes.                                    | For implementation: private memory, disk, latency, cleanup, cancellation, crash, publication, and correctness evidence.                                                                                   | P2-11 / F3                      |
| DriveFS/FAT32 scope       | Keep DriveFS Mirror/Stream and FAT32/cross-volume identity unverified, or record explicit Phase 2 scope exclusions.                                                      | Authorize the exact #47 Drive UI/synced-location run and #48 genuine writable FAT32 USB/VHD run.      | No for an exclusion; code only for a defect found in a qualified run.                              | Mode capture, disposable synced leaves and cloud/pause-resume consent for #47; drive letter, serial, filesystem, allocation unit size, and cleanup consent for #48.                                       | P2-02, P2-07, and P2-09         |
| Watcher timing/overflow   | Accept the synthetic/local watcher evidence only within an explicit scope boundary.                                                                                      | Authorize controlled local-NTFS OS-buffer timing and in-scope DriveFS runs.                           | No for a scope decision; yes only for a discovered behavioral defect.                              | Timestamped OS-buffer overflow/timing evidence and DriveFS observations for the alternative; current tests do not supply either.                                                                          | P2-09                           |
| P2-10 evidence boundary   | Accept static no-parser/no-content-I/O guards plus fixture source-byte equality for the filesystem-only MVP.                                                             | Require a runtime content-read spy gate on the integrated path.                                       | No for the static boundary; test/CI code is required for the spy.                                  | Dependency/privacy checks, static guards, byte equality; or a retained runtime-spy test and log. The installed native observation is not a spy.                                                           | P2-10                           |
| Inline onboarding         | Accept the already merged inline Preferences entry path.                                                                                                                 | Defer a separate first-run route to a named later phase.                                              | No for acceptance; UI/product code is required for a different route.                              | Owner review of the existing entry path and criterion mapping.                                                                                                                                            | P2-01                           |
| #107 disposition          | Record #106 as terminal-follow-up successor convergence and Agent 2's September 12 evidence as genuine running-lease same-job recovery; no defect found and no S5 rerun. | Request code or a new run only if review identifies a specific product, contract, or evidence defect. | No current code change; a defect-specific fix may require code.                                    | Agent 2's recorded final verification and the preserved #106/#110 state distinctions; no GitHub approval is claimed.                                                                                      | P2-04 (with P2-05/P2-06 review) |

## Proposed issue updates (not posted)

These drafts are for the owner/maintainer. No GitHub message or issue mutation
was made by this task.

- **#33:** Keep the epic open. Link this packet after it is merged, state that
  implementation and evidence are present but F1/F2/F3, platform scope,
  P2-10, onboarding, S5 disposition, and owner acceptance remain open. Note
  that merged #111 is a mixed product-and-evidence change, not evidence-only.
- **#36, #37, #38, and #40:** Replace “implementation pending” wording with
  “implementation merged; acceptance evidence is classified in the 2026-09-12
  packet.” Keep each open until the owner accepts its criterion and any named
  installed/platform gap is resolved or explicitly scoped out.
- **#39:** Record that the filesystem-only/no-parser direction is implemented,
  then record the owner's static-boundary versus runtime-spy choice. Do not
  claim a runtime spy before it exists.
- **#41:** Keep the aggregator open and link this packet plus the [dated live
  status table](README.md#live-review-status-2026-09-13). State that the
  current source stack is #110 → #111 → #112 → #114, while #113 is an older combined
  candidate; its checks are not acceptance. Preserve the canonical S5 source
  `19585dae7bef9ffdec06b71e0627df1b3f7ceb2f`, the tested #111 product
  source `05fb35c153dd3f177b900292d39998da3774b5e4`, and the report revision
  `ef085229471301043a50f4b668901d9c956fb407`. State that the original Partial
  remains unexplained and not reproduced, #112 adds bounded diagnostics
  without establishing cause or performance acceptance, and #111 is mixed
  product/evidence work. Include the duplicate-S5 mapping and independent
  Agent 2 review boundary. It is not a Phase 2 acceptance statement.
- **#47:** Request the missing DriveFS UI mode capture, disposable synced
  leaves, and cloud/pause-resume consent, or record an explicit owner scope
  exclusion. Do not report another blocked inventory as a run.
- **#48:** Request a genuine disposable FAT32 USB/VHD with recorded identity
  metadata and cross-volume consent, or record an explicit scope exclusion.
  Do not use DriveFS, the system partition, or the no-media device as a proxy.
- **#107:** Record that #106 demonstrated successor convergence after a
  terminal `follow_up_requested` transition, while the September 12 evidence
  separately demonstrates same-job recovery from a genuinely running lease.
  Agent 2 found no defect and published the isolated test/report artifacts in
  merged #110. #111's `de396e7` is tree- and patch-identical S5 recovery
  provenance, not a second run; keep the canonical full S5 source above when rebasing the
  stack. No additional S5 or NTFS run is requested unless a specific new
  evidence defect is identified and separately authorized.
  Require a product/contract change only if review establishes a genuine
  defect.

No issue is closed, and no owner acceptance is inferred by these proposals.

## Current dependency and merge order

GitHub verification records main at `71848732216d4ab4e13e73c820b2b8e4d17bddbe`,
with #110 squash-merged as `e2948f1bcc03af162ff95ba06ddd6033f2547da6`, #111
squash-merged as `914d7bd2475a11a5e7286086f15bce1d4bd148a5`, #112
squash-merged as `00884ba87c47a24f3ad75aa31f34ef7efe4a5bb8`, and #114
squash-merged as this main SHA. #109 is the documentation-only publication
vehicle and must be checked against the exact main head immediately before its
guarded merge. PR #113 is a historical synthetic candidate, not a merge
prerequisite.

## Historical dependency and merge order

The dated status table referenced by this historical section records the
historical dependency facts. The historical source stack was #110 -> #111 -> #112, with #112
as its head. PR #113 is an older synthetic combined candidate based on
`main`; it is not the current source-stack head. The installed evidence remains
tied to canonical S5 source
`19585dae7bef9ffdec06b71e0627df1b3f7ceb2f` and to #111's tested product
source `05fb35c153dd3f177b900292d39998da3774b5e4` where applicable.

The historical owner-ready source sequence was:

1. Review/mark PR #110 ready and have the owner squash-merge its current head
   `e209b8ad46adfc2d66d2c2b18288d0ef254eeb8a`. Its installed observations and
   green checks remain evidence, not acceptance.
2. After fetching and verifying the actual `main` produced by that squash
   merge, update #111 by rebasing only its own commits (the commits after its
   old parent `e209b8a`) onto actual `main`; exclude #110's old-parent commits.
   Preserve the existing #110/#111 installed-evidence union, run fresh checks
   on the resulting #111 SHA, and leave review/merge authority with the owner.
3. After #111 is actually squash-merged, fetch and verify actual `main`, then
   update #112 by rebasing only its own commits (the commits after its old
   parent `a3b90a4`) onto actual `main`; exclude the old #110/#111 parents.
   Preserve canonical S5 and the installed-evidence union, run fresh checks on
   the resulting #112 SHA, and leave review/merge authority with the owner.
4. After the source merges, update #109 once against that verified baseline and
   refresh only live-head references. Keep this factual publication separate
   from unresolved Phase 2 acceptance. PR #113 remains the older historical
   candidate; do not treat it as the next merge vehicle.

Keep PR #108's performance record diagnostic: its quiet-host gate failed closed,
the retained current-main `Failed`/`Partial` iteration is unexplained, and the
PR #112 record did not reproduce it. Any budget, fixture, platform, contract, or P2
acceptance change needs its own owner decision and evidence.

This packet is documentation and coordination only. It does not constitute
issue comments or closures, owner acceptance, a budget or scope decision,
production scanning, or Phase 3 activation. The separately guarded #109 merge
publishes these facts and does not change those boundaries.

## Cross-PR integration audit

This is a historical audit of the earlier synthetic replay. Its frozen commit
references and validation results are preserved unchanged as evidence; they do
not override the current source-stack and candidate rows in the dated status
table.

The isolated checkout at
`%TEMP%\fruitboard-phase2-integration-20260912`
was created from verified `origin/main` `3ebac5f7a76c3425620ceba59e6078b32fb6cd85`.
It replayed PR #110's `19585da` and `e209b8a` first, then PR #111's later
harness, product, and report commits through current head `f8140349`. PR #111's
recovered `de396e7` was not replayed: it has the same parent, tree, and stable
patch ID as `19585da`. The later `2eb629f` replay was empty after the
shared-file resolution and was skipped, so no recovery work was duplicated.

The replay had real conflicts in the checklist and installed-journey README;
the resolution retained #110's queued/running/restart/disable/remove entries
and #111's denied-traversal, unchanged-bound `ResourceLimit`, hardlink, product
diagnostic, and report-index entries. The add/add queued report conflict was
resolved to the canonical #110 bytes. The pre-#112 synthetic combined checkout
head was `f7ebbf4e61ffef9501c4804763f1c6e469e6c486` before #112. The existing
integration branch then cherry-picked #112's `eace2e6` as `879010d`. The final
candidate is now published as draft #113 at frozen SHA
`5cc8fb549966ebd53c135540b2f4c68339269680`. It is a synthetic review
candidate, not a merge commit on GitHub.

The combined audit began only after Agent 2's liaison reported no active
reproduction window, lock, or matching Cargo/NTFS process. No S5 or NTFS
experiment was rerun. The individual green checks on PR heads #108 `a44653b`,
PR #110 `e209b8a`, PR #111 `f8140349`, and PR #112 `eace2e6` remain per-PR
evidence; they are not combined integration evidence. The pre-#112 combined
validation at `f7ebbf4` is historical for #110/#111 only and is superseded as
combined evidence by the published #112 candidate.

## Validation and non-actions

The owned #109 branch is documentation-only. The final publication checks are:

- Targeted Markdownlint v0.23.2 on the five owned documents: **pass**, 0
  issues.
- Prettier v3.9.6 check on the five owned documents: **pass**, all matched.
- `node scripts/verify-repository-privacy.mjs`: **pass** (295 files).
- `git diff --check`: **pass** for the owned documentation diff.
- GitHub CI for the resulting #109 head: verified after push; all ten required
  contexts must be successful before handoff.

The historical combined-audit checks above are not relabeled as checks newly
run by this packet. Agent 2's recorded storage, desktop, driver, build, and
installed results are likewise cited as prior evidence, not rerun here. No
full source build, installed run, performance run, issue mutation, approval,
Phase 2 acceptance, or production activation was performed by this
documentation slice. The owner-authorized #109 publication merge is guarded
by the exact pushed head and required CI; it does not accept Phase 2.

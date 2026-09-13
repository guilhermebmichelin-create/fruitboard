# Phase 2 integration evidence index (#41)

Status: **final factual refresh verified 2026-09-13; Phase 2 is not accepted.**
This index separates merged implementation, automated tests, installed
observations, and owner acceptance. The [current acceptance packet](acceptance-packet-2026-09-12.md)
owns the closeout decisions and coordination. The dated checkpoint,
reconciliation, benchmark, and journey reports remain historical evidence and
are not rewritten here.

## Live review status 2026-09-13

This is the current table for the verified source baseline and PR state used by
this refresh. A row says **merged** only when GitHub reports the PR as merged;
an open or draft row is not a merged dependency. The `main` row is the source
baseline before this documentation PR's squash merge, so the final post-merge
`main` SHA is reported separately in the handoff.

<!-- markdownlint-disable MD060 -->

| Item    | Verified head or boundary                                                                            | Live state, dependency, and check result                                                                                                                          |
| ------- | ---------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `main`  | `71848732216d4ab4e13e73c820b2b8e4d17bddbe`                                                          | Source baseline for this refresh; includes merged #110, #111, #112, and #114; final post-#109 main is reported separately                         |
| PR #108 | `a44653bd27baaa5d2878c5a3bd118470cb6909dd`                                                          | Open draft; diagnostic performance evidence only; its quiet-host gate failed closed; no qualification or acceptance claim                                          |
| PR #109 | This documentation refresh                                                                          | Documentation-only publication vehicle; exact pushed head and guarded GitHub merge are verified separately                                                       |
| PR #110 | head `e209b8ad46adfc2d66d2c2b18288d0ef254eeb8a`; merge `e2948f1bcc03af162ff95ba06ddd6033f2547da6` | **Merged**; queued/running/terminal and same-job restart evidence is installed evidence from canonical source `19585da`; acceptance remains separate                 |
| PR #111 | head `606635c06b6eabf345c3e91fa95409eee654afea`; merge `914d7bd2475a11a5e7286086f15bce1d4bd148a5` | **Merged**; mixed NTFS/product change; installed NTFS observations remain tied to tested source `05fb35c` and report `ef085229`, not to a rerun on current `main` |
| PR #112 | head `386b4c9808bca0853dbe71757f121d4106b6d82e`; merge `00884ba87c47a24f3ad75aa31f34ef7efe4a5bb8` | **Merged**; diagnostic/performance follow-up is in main; no performance qualification or Phase 2 acceptance                            |
| PR #113 | `e90b03cc0bddd1a449e817ea2d89708a85c82ef1`                                                          | Open and ready; older synthetic combined candidate, superseded by the source baseline; its checks are historical candidate evidence, not a merge or acceptance                 |
| PR #114 | head `0aa9020c5f524de7f7d0f78226f500f5e021baf3`; merge `71848732216d4ab4e13e73c820b2b8e4d17bddbe` | **Merged**; executable qualification runbook/validator is in main; no qualification run or acceptance                            |

<!-- markdownlint-enable MD060 -->

The required contexts are `docs-policy`, `client`, `rust-portable`,
`migration`, `windows-foundation`, `security`, `windows-packaging-smoke`,
`enumeration-windows`, `filesystem-watcher-windows`, and
`scan-execution-windows`. Green checks establish CI provenance only; they do
not establish owner acceptance. The merged source sequence includes #110, #111,
the #112 and #114 changes. The executable runbook is prepared but no qualification run or
owner acceptance has been recorded; GitHub verification, not ancestry alone,
determines merge status.
PR #99 is closed without merge and remains validation-only history. Open issues
remain #33, #36-#41, #47, #48, and #107.

## Historical live review status 2026-09-12

This earlier table is preserved for provenance. Its heads, dependencies, and
open/merged wording do not override the current table above. The head OIDs
were resolved from the fetched Git refs and cross-checked against GitHub at the
time; historical evidence references elsewhere are intentionally preserved.

| Item    | Head or source                             | Live state, dependency, and check result                                                                                                                              |
| ------- | ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `main`  | `3ebac5f7a76c3425620ceba59e6078b32fb6cd85` | Merged PR #91 baseline; Foundation run 34428758835 passes its nine jobs, with `windows-packaging-smoke` as the separate tenth required context                        |
| PR #108 | `a44653bd27baaa5d2878c5a3bd118470cb6909dd` | Open draft; performance diagnostics only; ten required contexts successful                                                                                            |
| PR #109 | This publication commit                    | Open draft documentation branch; exact post-push head is reported in the publication handoff; no acceptance follows                                                   |
| PR #110 | `e209b8ad46adfc2d66d2c2b18288d0ef254eeb8a` | Open draft; Agent 2 queued/restart evidence; based on `main`; ten required contexts successful                                                                        |
| PR #111 | `a3b90a461f32a0ab2e49f42840da7fbdeceb6673` | Open draft; mixed NTFS/product follow-up based on PR #110; ten required contexts successful                                                                           |
| PR #112 | `9da0272fb2fefac9775028ed6bf8013459c10cad` | Open draft; benchmark diagnostics based on PR #111; current source-stack head; ten required contexts successful                                                       |
| PR #113 | `e90b03cc0bddd1a449e817ea2d89708a85c82ef1` | Open, ready for review; older synthetic combined candidate based on `main`, not refreshed onto the current #110 → #111 → #112 stack; ten required contexts successful |

Agent 1's earlier coordination is preserved in this historical table. The
current source state is in the 2026-09-13 table above.

The ten required contexts are `docs-policy`, `client`, `rust-portable`,
`migration`, `windows-foundation`, `security`, `windows-packaging-smoke`,
`enumeration-windows`, `filesystem-watcher-windows`, and
`scan-execution-windows`. Green checks establish check provenance only; they
do not establish owner acceptance. PR #99 is closed without merge and remains
validation-only history. Open issues remain #33, #36–#41, #47, #48, and #107.

## Merged contributions

These records establish provenance, not owner acceptance:

| PR / commit                        | Classification                                                                      |
| ---------------------------------- | ----------------------------------------------------------------------------------- |
| #82 / `05d39ff`                    | Watcher supervision, root mapping, reconnect, shutdown, and recovery implementation |
| #85 / `c05f4b6`                    | Durability, fault, atomicity, alias, and stale-publication implementation/tests     |
| #90 / `f487aa1`                    | #47/#48 blocker report; no platform qualification                                   |
| #92 / `300c2a4`                    | Installed journey record from its recorded baseline                                 |
| #93 / `106335a`                    | Contended benchmark/profile evidence                                                |
| #94 / `d342ec1`                    | Duplicate metadata-query cleanup                                                    |
| #95 / `ee720af`                    | Library/durable-state alignment                                                     |
| #96 / `326fb0a`                    | Bounded chunked-snapshot proposal only                                              |
| #97 / `ff5d8ee`                    | Automated durable-queue/watcher-burst verification                                  |
| #98 / `7e0e9f6`                    | Contended A/B validation; no 10 s p95 qualification                                 |
| #100 / `7980b75`                   | Packaging-smoke deadline and diagnostics fix                                        |
| #101 / `67bdc76`, #102 / `892920d` | Historical installed/stall-failure records                                          |
| #103 / `55668da`                   | Exclusive installed-test lock helper                                                |
| #104 / `cccaa67`                   | Queue-stall fix and automated regression                                            |
| #105 / `f63a1d3`                   | `smol-toml` 1.7.1 security remediation                                              |
| #106 / `b2fb62c`                   | Canonical fixed installed report; S5 partial; docs-only                             |
| #91 / `3ebac5f`                    | Final reconciliation refresh and current baseline                                   |

| #110 / `e2948f1`                  | Queued/running/terminal, queued disable/remove, and genuine same-job restart evidence; merged, no acceptance decision |
| #111 / `914d7bd`                  | Mixed NTFS/product hardening; merged, while installed NTFS observations remain tied to `05fb35c` and `ef085229` |

PR #99 (`9c211ad`) is closed and never merged. Agent 2's canonical S5 source
is the Git-resolved commit
`19585dae7bef9ffdec06b71e0627df1b3f7ceb2f`; it is an unmerged
test/documentation-only pin relative to `main`, not merged product code. A merged commit, green
automated check, or installed observation does not by itself provide owner
acceptance.

## Evidence and tested-source boundaries

The current PR heads are kept in the [dated live review status table](#live-review-status-2026-09-13).
The classifications below intentionally do not duplicate those changing OIDs.

| PR   | Classification                                                                                                                                                                                                                                                                                                                                            |
| ---- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| #108 | Agent 3 normal-configuration performance diagnostics; the strict quiet-host preflight failed closed and no qualification is claimed                                                                                                                                                                                                                       |
| #109 | This closeout packet and coordination index                                                                                                                                                                                                                                                                                                               |
| #110 | Agent 2 queued/running/terminal, queued disable/remove, and genuine same-job restart evidence; no product fix                                                                                                                                                                                                                                             |
| #111 | Mixed product-and-evidence change: local-NTFS denied traversal, unchanged-bound `ResourceLimit`, and in-root hardlink evidence, plus the durable-diagnostic and desktop scan-console correction at tested source `05fb35c153dd3f177b900292d39998da3774b5e4`; reviewed report revision `ef085229471301043a50f4b668901d9c956fb407`; it is not evidence-only |
| #112 | Correctness/diagnostics follow-up: the original current-main `Partial` remains unexplained and was not reproduced; bounded `partial_class` coverage and sanitized benchmark protocol were added, with no performance qualification or acceptance claim                                                                                                    |
| #113 | Older synthetic combined candidate; its relationship to the current source stack is described below and in the dated status table                                                                                                                                                                                                                         |

The #111 branch also carries recovered S5 commit `de396e7`. Its tree and
stable patch ID are identical to #110's
`19585dae7bef9ffdec06b71e0627df1b3f7ceb2f`, so it is historical provenance,
not a second recovery result. The integration strategy keeps the canonical
S5 patch once and does not duplicate the recovery work.

## P2-01 through P2-12

The #92 installed journey was recorded on an older merged baseline. The #106
fixed validation tested unmerged candidate `ded02ac`, patch-identical to the
fix in #104, not the then-current `3ebac5f`. The September 12 queued/restart report
was verified from
`19585dae7bef9ffdec06b71e0627df1b3f7ceb2f`, whose diff from `3ebac5f` is
test/documentation only. The queued-state, queued disable/remove, and genuine
same-job restart observations are published in merged PR #110, but remain
historical installed evidence and owner-unaccepted. The separate NTFS report
in merged PR #111 was published at reviewed revision
`ef085229471301043a50f4b668901d9c956fb407`; it was tested from source
`05fb35c153dd3f177b900292d39998da3774b5e4`. That source changes durable
product-owned diagnostics and desktop scan-console mapping for `access_denied`,
`resource_limit`, `unsupported`, and `unavailable`. The installed observations
remain tied to their tested source, while the corresponding product correction
is now in merged main; neither is owner acceptance.

<!-- markdownlint-disable MD060 -->

| ID    | Merged implementation                                        | Automated evidence                                                                                                                                                                                                                                                                                                                                                                                                                              | Installed evidence                                                                                                                                                                                                   | Remaining                                      |
| ----- | ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| P2-01 | #34/#35 picker, settings, and inline onboarding              | Client/Windows CI; #58 fake-adapter keyboard/narrow evidence                                                                                                                                                                                                                                                                                                                                                                                    | #92 picker, cancel, persistence, restart; #54 picker record                                                                                                                                                          | Onboarding decision and owner acceptance       |
| P2-02 | #59 core, #64 enumeration, #70 worker                        | Deterministic and Windows fixture tests                                                                                                                                                                                                                                                                                                                                                                                                         | #92 add/modify/rename and remove/restore                                                                                                                                                                             | Platform scope and owner acceptance            |
| P2-03 | #62/#70 safety path and #85 fault cases                      | Partial/offline/cancel/resource-limit fault tests                                                                                                                                                                                                                                                                                                                                                                                               | #92 cancellation and unavailable-root retention; #111 `ef08522` local-NTFS denied traversal and unchanged-bound `ResourceLimit` pass, tested from historical source `05fb35c`                                                 | Denied/limited traversal and owner acceptance  |
| P2-04 | #60/#61/#70 durability, #82 recovery, #104 active-slot fix   | Migration/worker gates; #97 fences; #104 regression                                                                                                                                                                                                                                                                                                                                                                                             | #92 persistence/cancel/recovery; #106 S1-S4/S6 pass, S5 successor path; #110 queued/running/terminal states and genuine same-job restart; #111 `de396e7` is tree-identical recovered S5 provenance, not new evidence | S5 disposition and owner acceptance            |
| P2-05 | #60/#62/#85 lease, disable/remove, staging                   | Lease and stale-publication tests                                                                                                                                                                                                                                                                                                                                                                                                               | #92 disable-while-running retention; September 12 queued disable/remove                                                                                                                                              | Evidence review and owner acceptance           |
| P2-06 | #62/#85 atomic publication and crash/backup paths            | Crash, recovery, backup, migration tests                                                                                                                                                                                                                                                                                                                                                                                                        | #92 restart/recovery; September 12 genuine running-lease hard-kill recovery                                                                                                                                          | Correct crash-path review and owner acceptance |
| P2-07 | #59/#62/#85 identity, alias, rename paths                    | Alias and rename tests                                                                                                                                                                                                                                                                                                                                                                                                                          | #92 rename; #111 `ef08522` true in-root hardlink alias result, local NTFS only, tested from historical source `05fb35c`                                                                                                       | #48 decision/qualification                     |
| P2-08 | #73/#77/#78/#81/#82 UI/native/lifecycle; #95 alignment       | Feature-on CI and IPC/adapter/lifecycle tests; #111 tests durable diagnostic mapping                                                                                                                                                                                                                                                                                                                                                            | #92 journey; #106 S1-S4/S6; #110 native/UI queued snapshot and invalidation; #111's tested-source `05fb35c` product changes for durable diagnostics and typed desktop error presentation                                              | Full criterion remains Partial                 |
| P2-09 | #68/#69/#71/#81/#82 watcher/coalescing/reconnect             | #97 automated queue/burst/fence tests                                                                                                                                                                                                                                                                                                                                                                                                           | #92 follow-ups; #106 isolated burst pass                                                                                                                                                                             | Overflow timing and #47 scope                  |
| P2-10 | #81 static no-parser/content-I/O guards and fixture equality | Dependency/privacy, static, preservation checks                                                                                                                                                                                                                                                                                                                                                                                                 | Native-adapter independence only; no runtime spy                                                                                                                                                                     | Static-versus-runtime decision                 |
| P2-11 | #67/#72/#86/#93/#94/#98 benchmark work                       | F1 reproduced; contended warm p95 14,267 ms vs 10 s; #108 normal-config current-main retains 9/10 authoritative iterations plus one `Failed`/`Partial` at 11,137 ms; Agent 3's quiet-host window failed closed; #112 adds bounded `partial_class` reporting and a sanitized protocol. The original Partial remains unexplained and was not reproduced; later passes do not establish its cause or performance acceptance. F3 remains unmeasured | No installed performance qualification                                                                                                                                                                               | F1/F2/F3 decisions                             |
| P2-12 | #80 checkpoint, merged map, #91 refresh                      | Run 34428758835 passes all nine Foundation jobs                                                                                                                                                                                                                                                                                                                                                                                                 | #92/#106 plus September 12 report with canonical S5 provenance                                                                                                                                                       | Decisions, then explicit owner acceptance      |

<!-- markdownlint-enable MD060 -->

In the P2-03, P2-07, and P2-08 rows, `05fb35c` identifies the source used for
the installed observations; it does not describe the current merge state.
The related #111 product changes are merged at `914d7bd`, while the installed
observations remain historical. P2-08's historical `Complete` wording is not carried forward: the current
disposition is Partial because the full installed/owner boundary was not
established. No P2 acceptance ID is promoted here.

## S5 disposition

The #106 record observed a running process, but the harness had already issued
an `already_running` coalesced follow-up. The old run therefore finished as
terminal `interrupted/follow_up_requested` before `taskkill`; after relaunch,
the queued successor completed and the old job stayed interrupted. That is the
expected terminal-follow-up path.

A genuine crash is the separate case where restart finds a run still marked
`running`; recovery marks it `restart` and requeues the same job/retry chain.
The current automated state-machine test covers that rule. Agent 2's
`19585dae7bef9ffdec06b71e0627df1b3f7ceb2f` pin covers the terminal-follow-up
path and adds no product fix. The September 12 installed report separately
records a genuinely running lease at exact-PID kill, same-job/retry-chain
recovery at attempt two, queued disable/remove with `runId: null`, and
terminal queued/running states. Those artifacts are isolated in merged PR #110;
Agent 2 found no defect and no contract change is proposed. #111's reviewed
`de396e7` has the same parent, tree, and stable patch ID as the canonical S5
commit, so the integration plan retains only one recovery patch while
preserving the historical provenance. No additional S5 or NTFS run is
requested for this stack; a future run would require a newly identified
evidence defect and separate owner authorization.

## Dependency and decision coordination

The verified current source baseline is main `71848732216d4ab4e13e73c820b2b8e4d17bddbe`,
including merged #110 (`e2948f1`), #111 (`914d7bd`), #112 (`00884ba`), and
the merged #114. The owner retains Phase 2 acceptance authority; the historical
coordination text below is preserved for provenance.

The [closeout packet's concise decision table](acceptance-packet-2026-09-12.md#owner-decision-list)
is the single owner decision list. It covers F1/F2/F3, DriveFS/FAT32 scope,
watcher timing/overflow, P2-10, inline onboarding, and #107 without converting
any recommendation into acceptance.

### Historical source-stack coordination

The historical source dependencies were #110 → #111 → #112: #111 depended on
the #110 evidence carrier, and #112 depends on #111's mixed product/evidence
tip. PR #112 was the source-stack head in the dated status table. The
PR #113 branch is a separate, older synthetic combined candidate based on `main`; its
green checks and candidate artifacts do not refresh or replace the current
stack. The #110 installed evidence used canonical S5 source
`19585dae7bef9ffdec06b71e0627df1b3f7ceb2f`; #92 and #106 remain historical
or separately recorded installed sources.

Agent 3's first quiet performance window failed closed before measurement in
PR #108 at `2561d3f`; no quiet-host qualification was claimed. Its later
normal-config current-main series retains the `Failed`/`Partial` finding above.
PR #112 did not reproduce the original result; its `partial_class` field and
sanitized protocol identify future occurrences without explaining the
historical one. Agent 2's final follow-up is isolated in PR #110, with no S5
rerun and no product defect identified. No heavy Cargo, Rust, Tauri, pnpm,
packaging, benchmark, or installed validation was run during Agent 2's
reproduction window. Prebuild artifacts outside a measurement window and
pause those heavy tasks throughout any future measurement. Do not overlap
windows or relabel a failed quiet preflight as qualification. No new S5 or
NTFS run is requested absent a specific evidence defect.

## Integration and merge readiness

For the current publication baseline, #110, #111, #112, and #114 are merged in
main `71848732216d4ab4e13e73c820b2b8e4d17bddbe`. #114 adds the executable
qualification runbook and validator, but no qualification run or acceptance.
The historical readiness sequence below is not a request to replay old source
heads or to merge an Agent 1-owned PR again.

Code/evidence review and Phase 2 acceptance are separate gates. The current
source-stack order is #110 -> #111 -> #112 -> #114 in the merged baseline
documented above. PR #113 is an older synthetic combined candidate based on `main`, not
the current source-stack head; its checks and independent review are evidence
for that candidate only.
Installed evidence remains tied to its recorded sources, including canonical
S5 `19585dae7bef9ffdec06b71e0627df1b3f7ceb2f` and the tested #111 product
source `05fb35c153dd3f177b900292d39998da3774b5e4`. The product changes are
merged in `914d7bd`; the installed observations are not a current-main rerun.

The earlier handoff recorded live heads #110 `e209b8ad46adfc2d66d2c2b18288d0ef254eeb8a`
on `main` `3ebac5f7a76c3425620ceba59e6078b32fb6cd85`, #111
`a3b90a461f32a0ab2e49f42840da7fbdeceb6673` based on #110, and #112
`9da0272fb2fefac9775028ed6bf8013459c10cad` based on #111. Those heads are
historical and do not override the current table above. The #113 candidate is
`e90b03cc0bddd1a449e817ea2d89708a85c82ef1`, based directly on `main`; it is
not the current stack head.

The current source stack already contains #110's one canonical S5 patch
`19585dae7bef9ffdec06b71e0627df1b3f7ceb2f`. Recovered `de396e7` is historical
provenance only; duplicate-S5 removal is already complete in the current stack
and is not a future rebase action. Historical replay descriptions retain their
original SHAs and wording below for provenance.

The earlier owner-ready sequence was:

1. Review/mark #110 ready and have the owner squash-merge `e209b8a`.
2. After fetching and verifying the actual new `main`, rebase only #111's own
   commits (the commits after its old parent `e209b8a`) onto that `main`,
   excluding #110's commits. Preserve the #110/#111 installed-evidence union,
   run the required checks on the resulting SHA, and leave review/merge to the
   owner.
3. After #111 is actually squash-merged, fetch and verify the resulting
   `main`, then rebase only #112's own commits (the commits after its old
   parent `a3b90a4`) onto that `main`, excluding the old #110/#111 parents.
   Preserve the canonical S5 and installed-evidence union, run fresh checks,
   and leave review/merge to the owner.
4. After the source merges, update #109 once against that verified baseline
   and refresh only live-head references. Keep this factual publication
   separate from unresolved Phase 2 acceptance.

Agents prepare and verify changes. The owner merges pull requests and makes
acceptance, scope, budget, and issue-closure decisions. #108 remains
diagnostic evidence only and carries no performance acceptance; #113 remains
historical until the owner decides its disposition. None of these steps accepts
Phase 2, changes budgets or scope, closes issues, or activates production
scanning.

## Standing gates

- #47/#48 remain unverified until their prerequisites exist or the owner makes
  an explicit scope decision.
- Historical reports retain their original baselines and outcomes; they are
  not current-state claims.
- Production scanning remains hidden until P2-03 through P2-08 have integrated
  evidence and the owner accepts Phase 2.
- The epic and child issues remain open. No message, merge, closure, acceptance,
  or production scanning action is authorized by this index.

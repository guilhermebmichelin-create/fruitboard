# Phase 2 integration evidence index (#41)

Status: **Phase 2 accepted with known gaps on 2026-09-14; P2-08 remains Partial.** The [2026-09-14 acceptance record](acceptance-2026-09-14.md) owns the decision; see the [post-acceptance addendum](#post-acceptance-addendum-2026-09-15) for the current state.
This index separates merged implementation, automated tests, installed
observations, measured-source provenance, and owner acceptance. The [current
acceptance packet](acceptance-packet-2026-09-12.md) owns the decision list and
proposed issue/PR dispositions. The dated checkpoint, reconciliation,
benchmark, and journey reports remain historical evidence and are not
rewritten here. The current-only scanner qualification is recorded in [the
2026-09-13 evidence](qualification-evidence-20260913.md).

## Live review status 2026-09-13

This is the current table for the fetched publication baseline and PR state
used by this refresh. A row says **merged** only when GitHub reports the PR as
merged; an open or draft row is not a merged dependency. The current
publication boundary is `origin/main` at `83da093672b5e2154097c897af533821b04f2352`,
the merge of PR #116. The scanner measurement is deliberately separate: its
exact measured source is `69f27f64f26aa657182a9260cc8e78f28a5838fb`, before the
documentation-only #115/#116 publication advances.

<!-- markdownlint-disable MD060 -->

| Item    | Verified head or boundary                                                                         | Live state, dependency, and check result                                                                                                                          |
| ------- | ------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `main`  | `83da093672b5e2154097c897af533821b04f2352`                                                        | **Current publication boundary**; PR #116 is merged. The measured scanner source remains `69f27f6`; no timing was rerun after the publication-only advances       |
| PR #108 | `a44653bd27baaa5d2878c5a3bd118470cb6909dd`                                                        | Open draft; diagnostic performance evidence only; its quiet-host gate failed closed; no qualification or acceptance claim                                         |
| PR #109 | head `659189528d4cb30568fa7ebdd12c91b454052074`; merge `69f27f64f26aa657182a9260cc8e78f28a5838fb` | **Merged**; documentation-only factual refresh that established the measured-source boundary used by the dated qualification                                      |
| PR #110 | head `e209b8ad46adfc2d66d2c2b18288d0ef254eeb8a`; merge `e2948f1bcc03af162ff95ba06ddd6033f2547da6` | **Merged**; queued/running/terminal and same-job restart evidence is installed evidence from canonical source `19585da`; acceptance remains separate              |
| PR #111 | head `606635c06b6eabf345c3e91fa95409eee654afea`; merge `914d7bd2475a11a5e7286086f15bce1d4bd148a5` | **Merged**; mixed NTFS/product change; installed NTFS observations remain tied to tested source `05fb35c` and report `ef085229`, not to a rerun on current `main` |
| PR #112 | head `386b4c9808bca0853dbe71757f121d4106b6d82e`; merge `00884ba87c47a24f3ad75aa31f34ef7efe4a5bb8` | **Merged**; diagnostic/performance follow-up is in main; no performance qualification or Phase 2 acceptance                                                       |
| PR #113 | `e90b03cc0bddd1a449e817ea2d89708a85c82ef1`                                                        | Open and ready; older synthetic combined candidate, superseded by the source baseline; its checks are historical candidate evidence, not a merge or acceptance    |
| PR #114 | head `0aa9020c5f524de7f7d0f78226f500f5e021baf3`; merge `71848732216d4ab4e13e73c820b2b8e4d17bddbe` | **Merged**; executable qualification runbook/validator is in main; the dated current-only run is recorded separately and is non-qualifying                        |
| PR #115 | head `c868a91a7fd74a969a9910dd89be212679f05707`; merge `f811cf3cf6c2e0bc4e3cd161bdcd9e063d53eb35` | **Merged**; documentation-only agent disk/cache rules; no scanner or acceptance change                                                                            |
| PR #116 | head `484c5eb2ab3bcf5bca1d2ad419009de5ec61046a`; merge `83da093672b5e2154097c897af533821b04f2352` | **Merged**; publishes the current-only qualification evidence and records the already-approved `custom-9995`/`repository-minimum` run; no Phase 2 acceptance      |

<!-- markdownlint-enable MD060 -->

The required contexts are `docs-policy`, `client`, `rust-portable`,
`migration`, `windows-foundation`, `security`, `windows-packaging-smoke`,
`enumeration-windows`, `filesystem-watcher-windows`, and
`scan-execution-windows`. Green checks establish CI provenance only; they do
not establish owner acceptance. GitHub verification, not ancestry alone,
determines merge status. Ten required checks passed on merged PR #116; that is
CI provenance only. The executable runbook was executed once against measured
source `69f27f6`, and its non-qualifying result is recorded in the dated
evidence. PR #99 is closed without merge and remains validation-only history.
GitHub currently leaves issues #33, #36-#41, #47, #48, and #107 open.

## Unmerged evidence addendum 2026-09-14

This addendum records the 2026-09-14 evidence state without rewriting the
2026-09-13 tables or making any acceptance decision. The current publication
boundary advanced to `origin/main`
`adab234b9b17b45f87d30460fa643818161e7cf3`, the merge of PR #118. The measured
scanner source remains `69f27f64f26aa657182a9260cc8e78f28a5838fb`; no timing was
rerun after that documentation-only merge. An open draft is not a merged
dependency, and neither PR below records an owner decision or promotes an
acceptance ID.

<!-- markdownlint-disable MD060 -->

| Item    | Head                                       | Live state, dependency, and check result                                                                                                                                                               |
| ------- | ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `main`  | `adab234b9b17b45f87d30460fa643818161e7cf3` | **Current publication boundary**, the merge of PR #118 (acceptance-gap refresh); no scanner rerun follows that documentation-only advance                                                              |
| PR #121 | `1342971cdcfe323dfdb8b8af8a8132b7ef2ed5a1` | Open draft, **UNMERGED**, based on `adab234`; P2-08 current-head installed retry/presentation/accessibility evidence; no owner acceptance follows                                                      |
| PR #119 | `f64decd5a4ae272c060ed647c4bb2ae077fa3c56` | Open draft, **UNMERGED**, based on `adab234`; opt-in feature-gated diagnostics phase profile with regression tests and a proposed CI feature gate; diagnostic investigation only, no performance claim |

<!-- markdownlint-enable MD060 -->

### PR #121 P2-08 installed evidence (unmerged)

The [P2-08 current-head run record](installed-journey/run-20260914-p2o8-retry-presentation.md)
was executed once against installed `adab234` and is published only on the
unmerged #121 branch. Raw logs, database copies, and screenshots stay outside
Git. The run recorded:

- D2: after cancellation the durable job is `cancelled`,
  `retryAvailable=false`; the installed UI offers no rejected Retry, and the
  enabled `Scan now` converges to `completed` (8,000 files observed).
- D3: after the unavailable-root retry budget is exhausted (`failed`,
  `attempt=4/4`, `last_error_code=unavailable`), the UI again offers no Retry;
  restoring the root and running `Scan now` converges (10 files observed) while
  prior committed results stay visible.
- #111 presentation: the durable database and installed UI agree on typed
  `cancelled` and `unavailable`; `queued`/`running`/`completed` render honest
  counters with no fabricated percentage. The installed mount is
  `data-review-adapter="native"` with no fake-adapter badge (D1 fixed).
- Accessibility: desktop and 390x844 narrow captures, keyboard focus traces,
  and an AX-tree capture for cancelled/completed/failed-unavailable; axe
  reported exactly one moderate `region` finding per captured state and no
  critical or serious violations. This is a minor finding for owner review,
  not a defect claim.

P2-08 remains **Partial** with narrowed gaps. D2/D3 converged on current head
pending the owner's policy-versus-defect review, and the typed presentation is
revalidated for `cancelled`/`unavailable`. The `access_denied`,
`resource_limit`, `unsupported`, and `worker_failed` presentations were not
produced because this run did not rerun the NTFS, overflow, or malformed
cases; `queued`/`running` lack a keyboard trace and axe run, and `interrupted`
was observed only durably. The full criterion and its acceptance remain the
owner's decision; nothing in #121 promotes it.

### PR #119 diagnostics investigation (unmerged)

The [warm-reconciliation diagnostic investigation](performance-followup/benchmark-investigation-20260913/README.md)
is published only on the unmerged #119 branch. It adds an opt-in,
feature-gated phase counter to `scan-execution`, exposes it through the
existing `profile-fs-calls` example, adds diagnostics regression tests, and
adds a workflow step that would gate that feature build. Its measurements are
diagnostic context only: they do not qualify performance, change a target,
explain the historical `Partial`, or approve the proposed ancestor-validation
fast path. The 2026-09-13 10,173 ms non-qualifying warm-p95 result is
unchanged, and no performance claim is recorded from #119.

The ten required contexts for both PRs, GitHub merge verification, and owner
review remain separate gates. This addendum is evidence state only; no owner
decision is recorded as made and Phase 2 remains unaccepted.

## Merged-state correction 2026-09-14

The [Unmerged evidence addendum 2026-09-14](#unmerged-evidence-addendum-2026-09-14)
recorded its boundary and merge state as verified before the #117/#121/#120/#122
merges. GitHub now reports those states superseded; this correction is added
rather than rewriting the dated addendum rows.

- Publication boundary: `origin/main` is
  `3fcaa4ade3099105594b48eadc785731608c2824` (merge of PR #122), not `adab234`
  (merge of PR #118). The later merges are documentation-only, so the measured
  scanner source remains `69f27f64f26aa657182a9260cc8e78f28a5838fb` and no
  scanner timing was rerun.
- PR #117 merged as `9ebcc9ffcdd94300b9ad2af22281be4ec8c187db`; PR #121 merged
  as `13c0ba2af88673746a96e787fae315d2eca920a0`; PR #120 merged as
  `fea3a8763d039c9855a368f7072a4f41acef8d85`; PR #122 merged as
  `3fcaa4ade3099105594b48eadc785731608c2824`.
- The addendum's PR #121 row is corrected: it is **MERGED**, not UNMERGED. Its
  ten required contexts passed on the updated head
  `e04ce92d0e69b9defd69b5615c0b047aac23ebfc` (original head
  `1342971cdcfe323dfdb8b8af8a8132b7ef2ed5a1`), and the
  [P2-08 current-head run record](installed-journey/run-20260914-p2o8-retry-presentation.md)
  now publishes on `main` rather than only on the unmerged branch.
- The addendum's PR #119 row remains accurate: still an open draft,
  **UNMERGED** at `f64decd5a4ae272c060ed647c4bb2ae077fa3c56`, based on
  `adab234`; its
  [diagnostic investigation](performance-followup/benchmark-investigation-20260913/README.md)
  still publishes only on the #119 branch.
- The "ten required contexts for both PRs ... remain separate gates" sentence
  now applies only to #119 and the still-open owner review. P2-08 remains
  **Partial** with the same narrowed gaps; no acceptance ID is promoted and no
  owner decision is recorded as made.

## Post-acceptance addendum 2026-09-15

Status: **Phase 2 accepted with known gaps on 2026-09-14; P2-08 remains
Partial.** This addendum records the post-acceptance state without rewriting
any dated row above. The owner decision is
[acceptance-2026-09-14.md](acceptance-2026-09-14.md); it was published by
merged PR #132 (`1454fb3`) after the owner closed the Phase 2 evidence
aggregator #41 COMPLETED on `2026-09-14T04:31:53Z`.

That record disposes P2-01 through P2-12: P2-01 through P2-07 and P2-09
through P2-12 are accepted (P2-11 carried with performance non-qualification;
P2-12 as the phase checkpoint), with the recorded platform, provenance, and
evidence boundaries carried forward. P2-08 stays **Partial** with four narrowed
gaps. Acceptance is documentation only: it does not activate production
scanning, promote P2-08, amend a budget, quota, fixture, or target, or start
Phase 3; the bounded Rust-parser spike is the stated next step.

The publication boundary is `origin/main`
`274d155b7a3f6c86ea5009a6dcb172596d6d3dfa`, the merge of PR #142, with
Foundation CI `success` on that head. The measured scanner source remains
`69f27f64f26aa657182a9260cc8e78f28a5838fb`; no timing was rerun after the
documentation-only advances, so the non-qualifying warm nearest-rank p95 remains
`10,173 ms` against the unchanged `10,000 ms` target.

### Post-acceptance merges

GitHub reports these pull requests merged; they publish documentation and
evidence only and promote no criterion.

<!-- markdownlint-disable MD060 -->

| PR   | Merge     | Scope                                                                                                                                                                             |
| ---- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| #137 | `b76bac7` | [Phase 2 residual close-out packet](residual-closeout-20260914.md) (2026-09-14); record only, no GitHub mutation                                                                  |
| #139 | `eeea1b6` | [Non-NTFS scanner scope decision](../../research/non-ntfs-scope-decision-20260915.md); local NTFS is the supported scope, DriveFS/FAT32/cross-volume excluded, #47/#48 stay open |
| #140 | `c72fbc4` | `AGENTS.md` plain-language summary requirement for generated prompts and reviews                                                                                                  |
| #141 | `7c7c477` | [Bounded Rust-parser fixture-manifest proposal](../../research/parser-fixture-manifest-proposal-20260914.md) for #138                                                            |
| #142 | `274d155` | [Phase 2 residual disposition drafts](residual-disposition-drafts-2026-09-14.md) (2026-09-14); record only, no issue/PR mutation                                                  |

<!-- markdownlint-enable MD060 -->

PR #119 (`58538cd`) is also now **MERGED**. The "Unmerged evidence addendum
2026-09-14" and "Merged-state correction 2026-09-14" rows above that describe it
as an open draft are superseded by that merge. Its feature-gated phase
diagnostics remain diagnostic context only: they do not explain the historical
`Partial`, qualify performance, or approve the ancestor-validation fast path.

### Observed issue state (recorded, not decided)

GitHub reports the following at recording time; this index mutates no issue.

<!-- markdownlint-disable MD060 -->

| Item     | Observed state                          | Note                                                                          |
| -------- | --------------------------------------- | ----------------------------------------------------------------------------- |
| #41      | CLOSED COMPLETED `2026-09-14T04:31:53Z` | Phase 2 evidence aggregator; acceptance-record provenance                     |
| #33      | CLOSED `2026-09-15T02:41:05Z`           | Phase 2 epic; was open at acceptance                                          |
| #107     | CLOSED COMPLETED `2026-09-15T01:46:45Z` | S5 restart-contract follow-up; was carried open at acceptance                 |
| #36/#37/#39 | CLOSED as accepted (comments 2026-09-15) | P2-02/P2-03/P2-07/P2-09/P2-10/P2-11 accepted; platform and perf boundaries carried |
| #38/#40  | OPEN                                    | Keep-open rationale posted 2026-09-15; carry the P2-08 evidence-only residual, no code fix ordered |
| #47, #48 | OPEN                                    | Non-NTFS scope excluded by #139, but kept open as unverified-territory markers |
| #138     | OPEN                                    | Spike open; fixture-manifest plan approved 2026-09-15 (merged #141); manifest PR authorized, spike starts after its merge |

<!-- markdownlint-enable MD060 -->

All dated 2026-09-13 and 2026-09-14 rows above are preserved verbatim. This
addendum is record-only: it rewrites no dated row, merges or closes nothing, and
records no acceptance decision beyond the owner's 2026-09-14 acceptance.

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

| #110 / `e2948f1` | Queued/running/terminal, queued disable/remove, and genuine same-job restart evidence; merged, no acceptance decision |
| #111 / `914d7bd` | Mixed NTFS/product hardening; merged, while installed NTFS observations remain tied to `05fb35c` and `ef085229` |
| #112 / `00884ba` | Bounded diagnostic coverage for the unexplained benchmark Partial; merged, no performance or acceptance decision |
| #114 / `7184873` | Executable qualification runbook and validator; merged, no qualification or acceptance decision |
| #115 / `f811cf3` | Agent disk/cache workflow rules; merged documentation-only change |
| #116 / `83da093` | Current-only qualification evidence publication; merged documentation-only change, no Phase 2 acceptance |

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

This is the current gap ledger at publication baseline `83da093`. It separates
implementation, automated evidence, installed evidence, the remaining defect or
evidence gap, and the decision still reserved to the owner. The installed
records from #92, #106, #110, and #111 retain their own tested-source provenance;
later merge does not turn them into current-head runs.

<!-- markdownlint-disable MD060 -->

| ID              | Merged implementation                                                                                    | Automated tests/evidence                                                                                                    | Installed evidence                                                                                                                                                                                                            | Remaining defect/evidence gap                                                                                                                                                                                                                                                                                                                | Owner decision                                                                                                                                                             |
| --------------- | -------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P2-01           | #34/#35 picker, settings, and inline Preferences onboarding                                              | Client/Windows CI; #58 fake-adapter keyboard, narrow, and axe evidence                                                      | [#92 root selection and settings](installed-journey/run-20260908.md) and #54 picker record                                                                                                                                    | No current-head installed accessibility capture for the complete Preferences path; no product defect is recorded.                                                                                                                                                                                                                            | Recommended scope decision: accept the existing inline entry path. A separate first-run route is later UI work, not a current acceptance prerequisite.                     |
| P2-02           | #59 reconciliation core, #64 enumeration, #70 worker                                                     | Deterministic reconciler and Windows enumeration/fixture tests                                                              | [#92 installed convergence](installed-journey/run-20260908.md) covers add, modify, rename, remove, and restore                                                                                                                | Installed observations are historical local-NTFS evidence. DriveFS and non-NTFS behavior remain unverified under #47/#48.                                                                                                                                                                                                                    | Accept the local-NTFS/filesystem-only boundary, or keep #47/#48 in scope and run their platform evidence.                                                                  |
| P2-03           | #62/#70 safety path, #85 fault cases, and #111 diagnostic hardening                                      | Partial/offline/denied/cancel/resource-limit fault tests and typed diagnostic mapping                                       | [#92 retention](installed-journey/run-20260908.md) plus [#111 NTFS cases](installed-journey/run-20260912-ntfs-cases.md) from tested source `05fb35c`                                                                          | The installed #111 observations were not rerun on merged `914d7bd`; mid-watch ACL, network, and DriveFS cases remain absent.                                                                                                                                                                                                                 | Accept local-NTFS evidence with explicit platform limits, or authorize the named platform/ACL work.                                                                        |
| P2-04           | #60/#61/#70 durability, #82 recovery, and #104 active-slot fix                                           | Migration/worker gates, #97 fences, and #104 regression                                                                     | [#92 recovery](installed-journey/run-20260908.md) plus [#110 queued/running/terminal and same-job recovery](installed-journey/run-20260912-queued-restart.md)                                                                 | No S5 product or contract defect was found. #107 still needs the owner’s disposition of terminal-follow-up versus genuine running-lease recovery.                                                                                                                                                                                            | Record the two S5 paths and close #107 as no defect after review, or request defect-specific work; do not rerun S5 by default.                                             |
| P2-05           | #60/#62/#85 lease, disable/remove, staging, and stale-publication paths                                  | Lease, disable/remove, and publication tests                                                                                | #92 disable-while-running retention plus #110 queued disable/remove before publication                                                                                                                                        | No stale-publication defect is identified. The installed evidence is historical-source evidence, not a current-head replay.                                                                                                                                                                                                                  | Accept the evidence union, or identify the exact current-head installed scenario required; no general rerun is implied.                                                    |
| P2-06           | #62/#85 atomic publication, crash/rollback, backup, and migration paths                                  | Crash, recovery, backup, migration, and atomic-publication tests                                                            | #92 interrupted recovery plus #110 hard-kill with a genuinely running lease and same-job retry-chain recovery                                                                                                                 | The running-lease gap is covered by #110; remaining work is review of evidence provenance and owner acceptance, not another crash run.                                                                                                                                                                                                       | Accept the combined automated and installed evidence; request a new run only for a specific evidence defect.                                                               |
| P2-07           | #59/#62/#85 identity, alias, rename, and replacement paths                                               | Alias, rename, and per-location transition tests                                                                            | #92 rename plus [#111 in-root hardlink aliases](installed-journey/run-20260912-ntfs-cases.md), local NTFS, tested from `05fb35c`                                                                                              | FAT32 and cross-volume identity remain unverified; the installed alias run is not a current-head run.                                                                                                                                                                                                                                        | Accept local-NTFS identity and exclude #48, or authorize a genuine FAT32/cross-volume run.                                                                                 |
| P2-08 (Partial) | #73/#77/#78/#81/#82 UI/native lifecycle and supervision, #95 alignment, and #111 diagnostic presentation | Feature-on CI, typed IPC/client/native, lifecycle/recovery, axe, keyboard/narrow fake-adapter, and diagnostic-mapping tests | [#92 journey](installed-journey/run-20260908.md), [#106 fixed validation](installed-journey/run-20260909-fixed-validation.md), and [#110 queued/restart evidence](installed-journey/run-20260912-queued-restart.md)           | The exact unmet requirements are listed below: explicit Retry did not converge after cancellation or exhausted unavailable-root retry; #111’s merged diagnostic presentation has no current-head installed revalidation; fake-adapter keyboard/narrow captures are not installed-native proof; and no owner has accepted the full criterion. | Decide whether the recorded Retry failures require a product fix, and whether the existing evidence boundary is sufficient. Do not promote the criterion from this report. |
| P2-09           | #68/#69/#71/#81/#82 watcher, coalescing, generation, and reconnect paths                                 | #97 synthetic burst/coverage-loss/fence counts and watcher tests                                                            | #92 follow-ups and #106 isolated burst convergence; no real overflow timing                                                                                                                                                   | No controlled OS-buffer overflow/timing record and no DriveFS watcher evidence exist. Installed burst convergence is not a timing qualification.                                                                                                                                                                                             | Accept deterministic/local-NTFS watcher scope, or authorize the separate overflow/timing and #47 runs.                                                                     |
| P2-10           | #81 static no-parser/no-content-I/O guards and fixture equality                                          | Dependency/privacy checks, static guards, and source-byte preservation tests                                                | Native adapter independence was observed; no runtime content-read spy was run                                                                                                                                                 | Static evidence does not observe every integrated runtime file-open call. A runtime spy remains an optional evidence-strength choice.                                                                                                                                                                                                        | Recommended scope decision: accept static guards plus byte equality. Alternative: require a runtime content-read spy, which is technical test work.                        |
| P2-11           | #67/#72/#86/#93/#94/#98/#114 benchmark and validator work                                                | Harness, validator, resource-limit, and benchmark diagnostics                                                               | [Current-only qualification](qualification-evidence-20260913.md) measured source `69f27f6`; 10/10 scans were authoritative, but warm nearest-rank p95 was 10,173 ms versus 10,000 ms. No installed performance qualification. | F2 is a recorded non-qualifying result; F3/100,000-entry memory is unmeasured. The approved fixture/profile are not pending selections.                                                                                                                                                                                                      | Decide how to treat the 100,000-entry proposal and the existing 10,000-entry contract; no A/B or strict-profile rerun is requested.                                        |
| P2-12           | #80 checkpoint and the merged implementation/CI map                                                      | PR #116’s ten required checks passed; this is CI provenance, not acceptance                                                 | #92/#106/#110 installed records retain tested-source provenance                                                                                                                                                               | No single current-head installed journey covers every criterion, and P2-08, platform scope, P2-10, performance, and owner acceptance remain open.                                                                                                                                                                                            | Review the rows and record explicit owner acceptance for P2-01 through P2-12; keep production scanning hidden until the gate is met.                                       |

<!-- markdownlint-enable MD060 -->

### Why P2-08 remains Partial

P2-08 is not held back merely because its implementation PRs are separate or
because owner acceptance is missing. The current evidence leaves these
individual requirements unmet:

1. The installed journey's explicit Retry requirement is not demonstrated.
   After cancellation, Retry returned `The scan state changed. Refresh the
status and try again.`; after an unavailable root exhausted its automatic
   retry chain, restoring the root and selecting Retry also did not reconcile.
   These are the recorded D2/D3 observations in [the #92 run](installed-journey/run-20260908.md),
   not a new claim that the merged code is defective.
2. The typed durable diagnostic and desktop error-presentation correction from
   #111 is merged at `914d7bd`, but the installed NTFS observations were tested
   from `05fb35c`. No installed current-head run demonstrates that the merged
   presentation is the one exercised by the user journey.
3. The planned evidence includes keyboard, narrow-layout, and accessibility
   coverage. The available rendered captures are explicitly fake-adapter
   evidence; the installed records do not provide a current-head capture of
   the complete scan-console state set. This is an evidence boundary, not proof
   of an accessibility defect.
4. The criterion requires the complete Scan/Cancel/Retry and Library journey to
   be usable, honest, and persistent. #110 closes the durable queued-state and
   running-lease restart observations, but it does not close the Retry or
   current-head presentation gaps above.

The owner must decide whether D2/D3 represent an intended exhausted/cancelled
retry policy or require a product correction, and whether the existing
fake-adapter plus historical installed evidence is sufficient for this
criterion. Until then, `Partial` is the evidence disposition; it is not a
defect declaration or an owner decision.

In the P2-03, P2-07, and P2-08 rows, `05fb35c` identifies the source used for
the installed observations; it does not describe the current merge state. The
related #111 product changes are merged at `914d7bd`, while the installed
observations remain historical. P2-08's historical `Complete` wording is not
carried forward because the individual Retry, current-head presentation, and
installed accessibility evidence gaps above remain unresolved. No P2
acceptance ID is promoted here.

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

The verified current publication baseline is `origin/main`
`83da093672b5e2154097c897af533821b04f2352`, merged by PR #116 after #115.
The exact measured source remains `69f27f64f26aa657182a9260cc8e78f28a5838fb`;
The #115 and #116 changes are documentation-only publication advances after that run. The
owner retains Phase 2 acceptance authority; the historical coordination text
below is preserved for provenance.

The [closeout packet's concise decision table](acceptance-packet-2026-09-12.md#current-owner-decision-list---2026-09-13)
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

For the current publication boundary, #110, #111, #112, #114, #115, and #116
are merged in `origin/main` `83da093672b5e2154097c897af533821b04f2352`.
PR #116 publishes the current-only qualification evidence; the exact measured
source remains `69f27f64f26aa657182a9260cc8e78f28a5838fb`. The run is recorded
in [the dated evidence](qualification-evidence-20260913.md) and is
non-qualifying. Neither the run nor this documentation changes Phase 2
acceptance.
The historical readiness sequence below is not a request to replay old source
heads or to merge an Agent 1-owned PR again.

Code/evidence review and Phase 2 acceptance are separate gates. The merged
implementation/evidence sequence is #110 -> #111 -> #112 -> #114; #115 and
The #116 changes are documentation-only publication advances. PR #113 is an older
synthetic combined candidate based on the earlier `main`, not the current
source-stack head; its checks and independent review are evidence for that
candidate only.
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

The following owner-ready sequence is historical and closed by the merges above;
it is not a current action list:

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

The 2026-09-13 qualification record already used the owner-approved
`custom-9995` fixture and `repository-minimum` profile. Those are recorded
choices, not pending selections. The historical strict-profile and A/B
recommendations are not requirements for this refresh and do not authorize a
new measurement.

Agents prepare and verify changes. The owner merges pull requests and makes
acceptance, scope, budget, and issue-closure decisions. #108 remains
unexplained historical diagnostic evidence only and carries no performance
acceptance; #113 remains a superseded historical candidate, with closure
without merge proposed but not applied. None of these steps accepts Phase 2,
changes budgets or scope, closes issues, or activates production scanning.

## Standing gates

- #47/#48 remain unverified until their prerequisites exist or the owner makes
  an explicit scope decision.
- Historical reports retain their original baselines and outcomes; they are
  not current-state claims.
- Production scanning remains hidden until P2-03 through P2-08 have integrated
  evidence and the owner accepts Phase 2.
- The epic and child issues remain open. No message, merge, closure, acceptance,
  or production scanning action is authorized by this index.

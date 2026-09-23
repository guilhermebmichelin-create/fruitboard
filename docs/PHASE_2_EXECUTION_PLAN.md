# Phase 2 execution contracts and acceptance plan

Status: **Accepted for implementation; not an implementation or measured
performance claim.** Owner checkpoint: 2026-09-06 accepted the shared contracts
and proposed starting budgets below. Inline Preferences onboarding remains a
separate pending decision. Design baseline: main `82bc651` (PR #61), 2026-09-06. This
document makes the accepted filesystem-only direction executable. Do not
silently relax safety gates to meet a target. ROADMAP remains the milestone
sequence; this document owns the detailed Phase 2 contracts and acceptance IDs
referenced by issues.

## Current status

The [dated live review status table](review/phase-2-integration/README.md#live-review-status-2026-09-13)
is the current-head authority for `main` and PRs #108-#116. The current
publication boundary is fetched `origin/main`
`83da093672b5e2154097c897af533821b04f2352`, the merge of #116. The measured
source boundary for the scanner run is separately
`69f27f64f26aa657182a9260cc8e78f28a5838fb`, before the documentation-only
`#115`/`#116` publication advances. The executable qualification runbook was
executed once; its current-only result is recorded in [the dated evidence](review/phase-2-integration/qualification-evidence-20260913.md)
and is non-qualifying on the unchanged warm-p95 target. Installed records
retain their tested-source provenance. The owner-approved `custom-9995` and
`repository-minimum` choices were used and are not pending re-selection; the
historical strict-profile/A/B recommendations are not current requirements.
Green checks establish CI provenance only; they do not establish owner
acceptance.

Agent 2's final S5 verification found no product or contract defect and did not
rerun S5. Its recorded installed evidence is now published by merged #110; the
packet records the disposition and the owner decision list. The original
benchmark `Partial` remains unexplained, and no performance result is
qualified.

## Historical status snapshot

The following earlier snapshot is retained for provenance. Its head references
and planning language do not override the dated status table above.

Current live refresh: `origin/main` is
`3ebac5f7a76c3425620ceba59e6078b32fb6cd85` (PR #91, merged 2026-09-10).
Since PR #89 (`0b7612d`), main has merged subsequent PRs: #90, #100, #92,
and #93. The next merges were #102, #96 (proposal only), and #101
(historical stall baseline).
The remaining merges are #103, #105, #94, #98, #95, #97, #104, #106, and
PR #91. Foundation CI run `34428758835` is successful
on the exact current head with all nine Foundation jobs. Draft PRs #108
(`a44653b`), #109 (closeout), #110 (`e209b8a`), #111 (`f8140349`), and #112
(`eace2e6`) are open
and unmerged; PR #99 is closed without merge and remains validation-only
history. Implemented behavior, automated evidence, installed evidence, and
owner acceptance stay separate.

Merged #92 records the installed journey (`run-20260908.md`): persistence,
paging, watcher follow-up convergence, cancellation retention, and
interrupted-work recovery were observed. Merged #95 (`ee720af`) aligns
Library scan actions with durable state (Foundation `34425398811` success).
Merged #97 (`ff5d8ee`) adds durable-queue and watcher-burst verification
(Foundation `34426244554` success). Merged #94 (`d342ec1`) removes the
duplicate enumeration metadata query; merged #98 (`7e0e9f6`) validates that
candidate with contended A/B and diagnostic cleanup (Foundation `34424822082`
success). Merged #104 (`cccaa67`) skips failed retry while the root active
slot is owned (2 files; regression fails pre-fix, passes post-fix). Fixed
installed validation lives in merged PR #106 (canonical corrected report; this
plan carries no duplicate copy): S1-S4 PASS, S6 PASS, S5 PARTIAL (successor
convergence after an already-terminal follow-up; a genuine crash with a
running lease at restart is a separate path. The existing automated same-job
requeue test covers that path; issue #107 tracks disposition and any evidence
gap. No contract change is proposed unless Agent 2 establishes a defect, and
no acceptance is claimed. The current criterion-by-criterion ledger is in the
[2026-09-08 reconciliation
report](review/phase-2-integration/reconciliation-2026-09-08.md) with its
final 2026-09-10 addendum and P2-01-P2-12 table. Historical runs (including
`986ca61`/`15b7f17`/`c3d3292` packaging sidecar-timeout fails with retry
disposition, and the unpublished `cccd6da` claims superseded by actual merges)
stay in that report's historical sections.

The following historical review-head summary is deliberately classified
separately and does not override the current status above:
PR #110 carries the queued/restart evidence from tested `19585da`; PR #111 carries
the local-NTFS denied-traversal, unchanged-bound `ResourceLimit`, and hardlink
evidence plus the durable-diagnostic and desktop scan-console correction from
tested source `05fb35c`. The J run is not evidence from merged `3ebac5f`, and
PR #111 is not evidence-only documentation. Its recovered S5 `de396e7` has the
same parent, tree, and stable patch ID as #110's `19585da`; the integration
strategy keeps `19585da` canonical and does not replay a second recovery patch.

The current [closeout packet](review/phase-2-integration/acceptance-packet-2026-09-12.md)
owns the live coordination summary and proposed issue updates. Validation-only
PR #99 (`9c211ad`) is closed, never merged, and never implies constituent
safety; it stays historical. Merged #96 stays a proposal only. Merged #101/#102
stay historical stall failure evidence. Prior #91 heads stay as history.

<!-- markdownlint-disable MD060 -->

| State                                                   | Work                                                                                               | Evidence or next action                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Delivered                                               | Phase 1 foundation, accepted; #34 native picker/root storage                                       | PRs #20-32 and #44; installed picker evidence in `docs/review/issue-34/` via #54                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| Delivered                                               | #35 persistent rename/enabled settings, inline onboarding, failure/focus fixes                     | PRs #55/#56/#58; CI and regression tests; not scanner execution                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Delivered                                               | #35 rendered desktop/narrow keyboard verification                                                  | [Evidence](review/issue-35/README.md) and PR #58; fake-adapter rendering is labeled separately from installed-app evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Partially evidenced (acceptance pending)                | #36 bounded reconciliation, #64 Windows enumeration, and #70 scan worker                           | PRs #59/#64/#70; deterministic reconciliation, bounded handle-bound enumeration, and the end-to-end worker are merged; the installed #92 run observed add/modify/rename and remove/restore convergence; merged #111 adds the denied-traversal and unchanged-bound `ResourceLimit` implementation/evidence, while those installed observations remain tested from `05fb35c` on local NTFS; P2-02/P2-03 acceptance remains open                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Partially evidenced (acceptance pending)                | #38 durable scan execution foundation                                                              | PRs #60/#61/#70/#85 and [durable execution evidence](PHASE_2_DURABLE_EXECUTION.md); durable state-machine and worker fault coverage is merged; the installed #92 run observed persisted settings, cancellation retention, and interrupted-work recovery. Merged #95 aligns Library scan actions with durable state; merged #97 adds durable-queue and watcher-burst verification; merged #104 skips failed retry while the root active slot is owned. Merged #110 publishes queued/running/terminal state and genuine same-job restart evidence tested from canonical `19585da`; merged #111 retains the recovered `de396e7` only as historical provenance. P2-04/P2-05 and owner review remain open                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Decision pending                                        | Inline Preferences onboarding instead of a separate first-run route                                | Implementation documented; owner review remains separate from the accepted scanner contracts                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| Partially evidenced (acceptance pending)                | #40 staging/publication/read model plus #77 responsive staged batches and #85 durability close-out | PR #62, PR #77, and PR #85; atomic publication, bounded batches, crash/recovery, fault-injection, alias, and stale-publication tests are merged; the installed #92 run observed cancellation/unavailable retention and interrupted recovery. Merged #110 adds queued disable/remove and a genuine running-lease hard-kill/restart record; merged #111 adds local-NTFS hardlink evidence and durable diagnostic/desktop error presentation, while the installed NTFS source remains `05fb35c`. Installed alias qualification and P2-03/P2-06/P2-07 acceptance remain open                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Implemented (feature-gated; acceptance pending)         | #73 scan-console IPC, #78 native Library adapter, and #82 native watcher host                      | PRs #73/#77/#78/#81/#82 plus merged #95 Library/durable-state alignment and #111 diagnostic mapping; six typed commands, scoped permissions, native supervision, root mapping, shutdown recovery, and feature-on CI are merged; the installed #92 run observed paging, persistence, cancellation retention, and native work. P2-08/full-criterion acceptance remains open                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Implemented (automated evidence; qualification pending) | #37 watcher foundation, durable follow-up adapter, and native supervisor                           | PRs #68/#69/#71/#81/#82 plus merged #97 durable-queue/watcher-burst verification provide bounded coalescing, coverage-loss handling, generation fencing, reconnect backoff, and joined host loops; the installed #92 run observed watcher follow-ups and convergence, but installed burst timing, real overflow timing, and DriveFS evidence remain open; merged #97 counts are automated evidence and do not fill the installed column                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| Partially evidenced (acceptance pending)                | #39 no-parser boundary and additional #37 guards                                                   | Merged PR #81 adds static no-parser/no-content-I/O policy guards plus synthetic-fixture source-byte preservation assertions; these checks are not an installed-app proof or a runtime content-read spy                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| Measured (qualification and owner decision pending)     | #41/P2-11 benchmark and #79 budget/governance brief                                                | Original report remains historical; the [2026-09-08 re-run](review/phase-2-integration/benchmark-triage-20260908/rerun-report.md), recorded at pre-#85 baseline `51f45af`, reproduces F1, records warm p95 14,267 ms versus 10 s, and leaves F3 unmeasured. Merged [PR #93](https://github.com/guilhermebmichelin-create/fruitboard/pull/93) (`106335a`) adds contended filesystem-port profiling, not an idle-host performance pass. Merged #94 (`d342ec1`) removes the duplicate enumeration metadata query; merged #98 (`7e0e9f6`) validates that candidate with contended A/B and diagnostic cleanup (both sides fail 10 s p95; no qualification claimed). #108 records normal-config current-main at 9/10 authoritative with one retained `Failed`/`Partial` iteration at 11,137 ms; Agent 3's first quiet-host window failed closed before measurement at `2561d3f`; merged #112 records that original Partial as unexplained and not reproduced, and adds bounded diagnostic coverage plus a sanitized protocol. Later passes do not establish its cause or performance acceptance. F1/F3 scale options are in merged PR #96 (`326fb0a`) as a proposal only with its §2.9 correction, not an approved scale design |
| Recorded (owner acceptance pending)                     | #41/P2-12 checkpoint (#80)                                                                         | The checkpoint and reconciliation report remain historical records. The current [closeout packet](review/phase-2-integration/acceptance-packet-2026-09-12.md) refreshes the live baseline, evidence classes, open decisions, and coordination order without rewriting those reports                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| Next gates                                              | Owner decisions and remaining checks                                                               | Agent 3's first quiet-host preflight failed closed before measurement at `2561d3f`; merged #110 publishes the September 12 S5 evidence with no defect and no S5 rerun; merged #111 carries the mixed NTFS/product hardening, and merged #112 adds bounded `partial_class` reporting plus a sanitized benchmark protocol. The runbook/validator from merged #114 was used once on measured source `69f27f6`: 10/10 measured scans were authoritative, but warm p95 was 10,173 ms against the unchanged 10,000 ms target; see the dated qualification evidence. These checks and artifacts remain separate from Phase 2 acceptance. The owner-approved `custom-9995`/`repository-minimum` execution record is complete; no F1/F2 reselection or historical strict-profile/A/B rerun is requested. Resolve the F2 result disposition, F3 scale, platform scope, P2-10 evidence strength, inline onboarding, and criterion-by-criterion acceptance. Keep production scanning hidden until P2-03 through P2-08 have integrated evidence                                                                                                                                                                                        |
| Open/unverified                                         | DriveFS modes (#47), cross-volume/FAT32 identity (#48)                                             | Merged [PR #90](https://github.com/guilhermebmichelin-create/fruitboard/pull/90) (`f487aa1`) records the exact blockers and setup needed. No platform qualification or exclusion is claimed                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| Deferred                                                | Parsing (#39), logical grouping, Kanban, playback, sync, PWA                                       | Phase 2 is filesystem-only; bounded independent Rust-parser spike follows accepted MVP                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |

<!-- markdownlint-enable MD060 -->

## Visible completion scenario

1. In Preferences, select a disposable folder, give it a recognizable name, and
   leave it enabled. Cancellation changes nothing. Explain that no scan has run.
2. Choose **Scan now** for that root. Show queued/running status, counts and
   Cancel, not a fabricated percentage when total work is unknown.
3. In Library, inspect a paginated discovered-file list: filename, owning root,
   root-relative path, byte size, modified time, and present/missing state.
   Do not label records as grouped projects or display inferred FLP metadata.
4. Close and reopen the app. The last committed list and root settings survive;
   interrupted work is identified as interrupted and safely reconciled again.
5. Change the synthetic tree externally: add, rename, modify, remove, and restore
   files. Manual scans and later watcher-triggered reconciliation converge on
   the same result. Missing entries remain visible and reversible.
6. Cancel a scan or make the root unavailable. Retain the last committed list,
   label it as previous results, explain the safe failure, and offer retry.
   Never convert these failures into a list of missing files.

Ownership: #38 owns scan controls, root execution status and retry/cancel UX;
Issue #40 owns the read-only Library query/typed IPC/list UI; #36 owns reconciliation
decisions; #37 connects watcher triggers; #41 verifies the complete journey.
The Library scope is deliberately narrow: no cards, grouping, parser fields,
advanced search, file-opening authority, or content reads. Include loading,
empty, stale/offline, error, populated, keyboard and narrow-layout states.

## Shared contracts: #36 / #38 / #40

### Root revision and run identity

- Persist a monotonically increasing generation per root and a configuration
  revision. Every run carries root ID, generation, revision, run ID and a
  unique lease token. Database transactions allocate identities, not renderers.
- Changes affecting execution (disable, removal, future path/policy changes)
  invalidate active runs. Display-name edits do not invalidate traversal.
  IDs are not reused. Re-enabling requests a fresh generation.
- Availability (`unknown/available/unavailable/unsupported`) is an observation,
  distinct from queue state, outcome and freshness of the last successful scan.
  A stored observation is not proof of current reachability.

### Enumeration and authoritative completion

- #36 consumes an enumeration port and previous committed records; it produces
  bounded observations and decisions, never writes source files or invokes a
  parser. Discovery matches `.flp` case-insensitively; malformed FLP content is
  not diagnosed without parsing. Metadata/access failures are isolated and safe.
- Capture normalized root-relative locator, size, mtime and qualified filesystem
  identity. Reuse the researched Windows identity/normalization policy. Identity
  tuples are non-unique lookups: hardlink aliases retain distinct locations.
  Uncertain identity never proves a move or logical-project equivalence.
- Do not read file contents, hash files, hydrate placeholders, or follow nested
  reparse points in this MVP. Record policy exclusions distinctly from I/O
  failures; do not follow an alias outside the selected root or loop forever.
- A run is authoritative only when all in-policy directories were enumerated,
  required metadata succeeded, root/configuration/lease are still valid and no
  cancellation or observed invalidating event occurred. Completion is not an
  atomic filesystem snapshot: changes not observed during traversal are repaired
  by subsequent reconciliation, not covered by a snapshot-consistency claim.
- Access denial, root loss, failed metadata, resource-limit exhaustion or
  cancellation makes the run non-authoritative. Retain prior committed results.
  Never infer absence for excluded subtrees or paths whose coverage is uncertain.
  Policy changes cannot retroactively turn excluded locations into missing ones.

### Durable queue, cancellation and restart

- Persist `queued -> running -> completed|failed|cancelled|interrupted` with
  `cancellation_requested` for running work. Track retry attempts, not-before
  timestamp and a deduplicated follow-up request. Completed means applied, not
  merely enumerated. Terminal attempts are immutable; retries get fresh runs.
- Allow one running scan and at most one queued follow-up per root; initially
  one worker globally. Repeated manual/watcher triggers coalesce. A trigger
  during traversal marks the run invalidated and schedules one follow-up.
- Workers renew leases; all staging/final writes validate the token and root
  revision. An expired/replaced worker cannot commit even if it later returns.
- Persist cancellation before acknowledging it. Workers check between batches
  and before final apply. Disable/remove cancels queued work and invalidates
  running work in the same configuration transaction. Removal deletes only
  local tracked state per the retention policy, never source files.
- On process restart, prior-session running leases become interrupted, their
  staging is ineligible for apply, and the same job/retry chain is requeued with
  its persisted attempt budget and backoff. Enabled roots without eligible
  interrupted or queued work may get one deduplicated recovery scan. Cancelled
  outcomes take precedence over follow-up invalidation and are not implicitly
  revived. Exhausted chains are suppressed from recovery by durable state and
  attempt budget, regardless of diagnostic error code. Do not resume an
  abandoned enumeration cursor as authoritative.
- Retry transient errors with bounded exponential backoff; unavailable roots
  wait for explicit retry or a bounded reconnect/periodic trigger after the retry
  budget. User cancellation is not automatically retried. Surface safe codes and
  counters only in diagnostics; display paths solely in root/Library management.
- SQLite serialization defines the cancel/complete race: cancellation committed
  before apply prevents apply; completion committed first remains completed and
  a late cancellation reports that outcome without claiming rollback.

### Atomic application and read model

- #40 owns staging and publication. Stage batches by run ID outside the visible
  Library dataset; do not hold a write transaction throughout filesystem I/O.
  Set a staging quota and mark quota exhaustion non-authoritative.
- In one final transaction, validate generation/revision/lease/cancellation,
  apply observed file/location metadata, mark eligible unseen locations missing,
  record run completion, and advance the last-success marker. Any failure rolls
  back the entire publication. Unsuccessful runs publish no partial file list.
- The Library reads only committed generations, with bounded pages and stable
  ordering. A root's previous results remain identifiable while work runs or
  fails. IPC carries typed records and safe errors, not generic SQL access.
- Missing/restored is per location. Deleting one hardlink must not mark its
  other alias missing or collapse two aliases into one unique identity row.
  A rename with trustworthy identity updates location associations without
  inventing Phase 4 grouping; uncertain cases stay conservative.
- Define forward migrations for run/job/staging/read-model data; retain startup
  preference and root configuration. Keep WAL disabled. Backup contains a
  consistent database; recovery invalidates in-flight leases and staging before
  restarting work. Test failure at each durable boundary.
- Bound terminal-run history and staging cleanup; cleanup never deletes source
  files or historical missing locations. Specify history retention in the schema
  PR, independently of the existing diagnostic-log retention policy.

### Deterministic identity and per-path transitions

The first core slice compares one root's previous committed observations with
one completed enumeration. It is an isolated library, with no renderer command
or production traversal. A failed or incomplete enumeration returns no change
set, including no positive updates. Staging is disposable until atomic apply.

| Previous versus observed                                 | Decision                                                                                                                           |
| -------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Same normalized path, same identity/size/mtime           | No file change; run freshness advances separately                                                                                  |
| Same path, same qualified identity, changed size/mtime   | Modified filesystem metadata; no claim about content equality                                                                      |
| Same path, both qualified identities differ              | Replacement, retaining path history and assigning the observed physical association; never silently continue the old physical file |
| Same path, identity unavailable on either side           | Path continuity only, explicitly uncertain; changed metadata is modification evidence, never proof of identity                     |
| Old path absent, new path has same qualified identity    | Record old path missing and new path present; emit rename evidence only for an unambiguous one-old/one-new match                   |
| Two or more paths share an identity                      | Preserve every location; identity lookup is non-unique, and ambiguous alias changes do not prove a rename                          |
| Missing path observed again                              | Restore that location's presence; separately flag replacement if qualified identity changed                                        |
| Unseen path after incomplete/offline/cancelled traversal | No transition; previous committed state remains intact                                                                             |

A rename signal is advisory physical-locator evidence, not a logical-project
merge and not deletion of the old path's history. Cross-root moves, unsupported
filesystem IDs, identity reuse after a historical absence, names, sizes and
timestamps alone never establish a rename. Identity supplied by the enumerator
must already be qualified for the researched local NTFS scope. Metadata-only
comparison cannot detect same-size writes that preserve timestamps.

Normalization is supplied by the Windows boundary, not a generic lowercase
operation inside the core. Duplicate normalized paths invalidate a run rather
than letting enumeration order select a winner. Before production integration,
the boundary must reject paths outside the root, scope policy exclusions, and
prove traversal completion; callers cannot treat a renderer boolean as authority.

Retries allocate fresh generations, preserving a separate retry-chain attempt
count so restarting cannot reset the automatic retry budget. Lease deadlines
are checked in every staging and publication transaction as well as token
ownership; token equality alone is insufficient. The process-session ID fences
old workers across restarts regardless of wall-clock changes. Within a session,
use a monotonic deadline for worker liveness; durable timestamps schedule retry
eligibility after restart, never resurrect an old lease.

Root removal deletes configuration but retains file/location history as detached
from tracking (nullable root association plus former-root identity). It does not
mark detached locations missing. The removal transaction invalidates leases and
staging first; a re-added path has a fresh root ID and cannot receive old work.
The schema PR must implement this policy without cascading away file history.

## Current acceptance-gap ledger - 2026-09-13

The following ledger is the current implementation-to-acceptance map at
publication baseline `83da093`. It separates implementation, automated tests,
installed evidence, the remaining defect or evidence gap, and the owner
decision. Installed records retain their measured-source provenance; they are
not silently relabeled as current-head runs.

<!-- markdownlint-disable MD060 -->

| ID              | Merged implementation                                                                | Automated tests/evidence                                                                                          | Installed evidence                                                                                                  | Remaining defect/evidence gap                                                                                                                                                                                                                   | Owner decision                                                                                                             |
| --------------- | ------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| P2-01           | #34/#35 picker, settings, and inline Preferences onboarding                          | Client/Windows CI; #58 keyboard, narrow, and axe fake-adapter evidence                                            | #54 picker and #92 root/settings journey                                                                            | No current-head installed accessibility capture for the complete Preferences path; no product defect recorded.                                                                                                                                  | Recommended scope decision: accept the existing inline entry path; a separate first-run route is later UI work.            |
| P2-02           | #59 reconciliation, #64 enumeration, #70 worker                                      | Deterministic reconciliation and Windows enumeration/fixture tests                                                | #92 add/modify/rename/remove/restore convergence                                                                    | Installed evidence is historical local-NTFS evidence; DriveFS and non-NTFS remain unverified under #47/#48.                                                                                                                                     | Accept the local-NTFS/filesystem-only boundary, or authorize platform evidence.                                            |
| P2-03           | #62/#70 safety path, #85 fault cases, #111 diagnostic hardening                      | Partial/offline/denied/cancel/resource-limit tests and typed diagnostic mapping                                   | #92 retention and #111 NTFS cases tested from `05fb35c`                                                             | #111 installed cases were not rerun on merged `914d7bd`; mid-watch ACL, network, and DriveFS cases remain absent.                                                                                                                               | Accept local-NTFS limits, or authorize named ACL/platform work.                                                            |
| P2-04           | #60/#61/#70 durability, #82 recovery, #104 active-slot fix                           | Migration/worker gates, #97 fences, #104 regression                                                               | #92 recovery and #110 queued/running/terminal same-job recovery                                                     | No S5 product or contract defect found; #107 still needs owner disposition of the two recovery paths.                                                                                                                                           | Close #107 as no defect after review, or request defect-specific work; no default S5 rerun.                                |
| P2-05           | #60/#62/#85 lease, disable/remove, staging, stale publication                        | Lease, disable/remove, and publication tests                                                                      | #92 disable-while-running and #110 queued disable/remove                                                            | No stale-publication defect identified; installed evidence is historical-source evidence.                                                                                                                                                       | Accept the evidence union, or identify one exact current-head scenario; no general rerun.                                  |
| P2-06           | #62/#85 atomic publication, crash/rollback, backup, migration                        | Crash, recovery, backup, migration, and atomic-publication tests                                                  | #92 interrupted recovery and #110 running-lease hard-kill/retry recovery                                            | Running-lease gap is covered by #110; provenance review and acceptance remain.                                                                                                                                                                  | Accept combined evidence, or request a run for a specific evidence defect.                                                 |
| P2-07           | #59/#62/#85 identity, alias, rename, replacement                                     | Alias, rename, and per-location transition tests                                                                  | #92 rename and #111 in-root hardlink aliases on local NTFS from `05fb35c`                                           | FAT32/cross-volume identity unverified; alias run is not current-head.                                                                                                                                                                          | Accept local-NTFS identity and exclude #48, or authorize genuine FAT32/cross-volume evidence.                              |
| P2-08 (Partial) | #73/#77/#78/#81/#82 UI/native lifecycle, #95 alignment, #111 diagnostic presentation | Feature-on CI, typed IPC/client/native, lifecycle/recovery, axe, keyboard/narrow fake-adapter, diagnostic mapping | #92/#106 journey records and #110 queued/restart evidence                                                           | Explicit Retry did not converge after cancellation or exhausted unavailable-root retry; #111 presentation lacks current-head installed revalidation; fake-adapter captures are not installed-native proof; complete criterion remains unproven. | Decide whether D2/D3 need a product fix and whether the evidence boundary is sufficient; do not promote from Partial here. |
| P2-09           | #68/#69/#71/#81/#82 watcher, coalescing, generation, reconnect                       | #97 synthetic burst/coverage-loss/fence counts and watcher tests                                                  | #92 follow-ups and #106 isolated burst convergence; no real overflow timing                                         | No controlled OS-buffer overflow/timing record and no DriveFS watcher evidence; burst convergence is not timing qualification.                                                                                                                  | Accept deterministic/local-NTFS scope, or authorize separate overflow/timing and #47 work.                                 |
| P2-10           | #81 static no-parser/no-content-I/O guards and fixture equality                      | Dependency/privacy checks, static guards, source-byte preservation                                                | Native adapter independence observed; no runtime content-read spy                                                   | Static evidence does not observe every integrated runtime file-open call; spy is an optional evidence-strength choice.                                                                                                                          | Recommended: static guards plus byte equality. Alternative: authorize a runtime content-read spy test/CI gate.             |
| P2-11           | #67/#72/#86/#93/#94/#98/#114 benchmark and validator                                 | Harness, validator, resource-limit, and benchmark diagnostics                                                     | #116 current-only qualification measured source `69f27f6`: 10/10 authoritative, warm p95 `10,173 ms` vs `10,000 ms` | F2 result is non-qualifying; F3/100,000-entry memory is unmeasured. Approved fixture/profile are not pending selections.                                                                                                                        | Decide F3/10,000-entry contract treatment; no historical strict/A/B rerun is requested.                                    |
| P2-12           | #80 checkpoint and merged implementation/CI map                                      | #116 ten required checks passed; CI provenance only                                                               | #92/#106/#110 installed records retain tested-source provenance                                                     | No single current-head installed journey covers every criterion; P2-08, platform scope, P2-10, performance, and owner acceptance remain open.                                                                                                   | Review all rows and record explicit owner acceptance or exclusions; keep production scanning hidden until the gate is met. |

<!-- markdownlint-enable MD060 -->

### P2-11 performance disposition note - 2026-09-15

The canonical P2-01 through P2-12 ledger is the integration index's
[P2-01 through P2-12 section](review/phase-2-integration/README.md#p2-01-through-p2-12);
the acceptance-gap ledger above is the dated 2026-09-13 plan copy, and its
P2-11 row is not rewritten here.

Measured facts are unchanged: the current-only qualification on measured source
`69f27f6` returned 10/10 authoritative scans with a warm nearest-rank p95 of
`10,173 ms` against the unchanged `10,000 ms` target, so F2 stays
non-qualifying, and the 100,000-entry qualification set with its provisional
`<=128 MiB` working-memory F3 target stays **unmeasured**. No rerun,
optimization, or budget change is performed here.

Three explicit disposition options are on record for P2-11; the owner selects
between them. None is selected by this note, and the measured overage must
never become a pass by inaction or by an unapproved target change:

1. **Accept the 173 ms overage.** The owner explicitly decides that the
   `10,173 ms` warm-p95 result satisfies the existing `10,000 ms` target for
   the accepted local-NTFS scope. The decision is recorded against the
   unchanged target: it is not a target amendment, the overage stays visible,
   and F3/100,000-entry memory stays unmeasured.
2. **Retarget with evidence.** Amend the provisional scan-latency budget in
   [Proposed targets accepted as provisional budgets](#proposed-targets-accepted-as-provisional-budgets)
   only after new reviewable evidence (a fresh measured run or a recorded
   measurement analysis) plus explicit owner review. The existing `10,173 ms`
   result alone does not establish a replacement target, and CI correctness
   tests still may not assert hardware-sensitive absolute timing.
3. **Optimize.** Keep the `10,000 ms` target and run a bounded optimization
   with its own evidence and checks, followed by a fresh qualification run on
   the resulting source. The diagnostics from merged #119 and the proposed
   ancestor-validation fast path remain diagnostic/proposal history only;
   neither is approved optimization work here.

Under all three options, F1/F2/F3 stay as recorded, no quota, fixture, or
target change follows, and production scanning stays hidden. F3/100,000-entry
memory remains unmeasured either way.

### Why P2-08 remains Partial

P2-08 is not Partial merely because implementation PRs are separate or because
owner acceptance is missing. These individual requirements remain unmet:

1. The installed journey does not demonstrate a successful explicit Retry.
   After cancellation, Retry returned `The scan state changed. Refresh the
status and try again.` After an unavailable root exhausted automatic retry,
   restoring the root and selecting Retry also did not reconcile. These are the
   recorded D2/D3 observations, not a new claim that merged code is defective.
2. #111's typed durable diagnostic and desktop error presentation is merged at
   `914d7bd`, but its installed NTFS cases were tested from `05fb35c`; no
   current-head installed run demonstrates the merged presentation in the user
   journey.
3. The planned keyboard, narrow-layout, and accessibility coverage is available
   as fake-adapter rendered evidence. The installed records do not provide a
   current-head native capture of the complete scan-console state set. This is
   an evidence boundary, not proof of an accessibility defect.
4. The criterion requires the complete Scan/Cancel/Retry and Library journey to
   be usable, honest, and persistent. #110 closes queued-state and
   running-lease restart observations, but it does not close the Retry or
   current-head presentation gaps above.

The owner must decide whether D2/D3 describe intended exhausted/cancelled retry
policy or require a product correction, and whether the evidence boundary is
sufficient. Until then, Partial is the evidence disposition, not a defect
declaration or owner decision.

## Status-table addendum 2026-09-14

This addendum adds current rows without rewriting the ledger, the table, or any
historical section above. The current publication boundary advanced to
`origin/main` `adab234b9b17b45f87d30460fa643818161e7cf3`, the merge of PR #118;
the measured scanner source remains
`69f27f64f26aa657182a9260cc8e78f28a5838fb`. Both referenced PRs are
**UNMERGED** open drafts based on `adab234`. No owner decision is recorded as
made and no acceptance ID is promoted.

<!-- markdownlint-disable MD060 -->

| State                                                 | Work                                                                                                                        | Evidence or next action                                                                                                                                                                                                                                                           |
| ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Boundary (addendum)                                   | #118 acceptance-gap refresh merged as `adab234`                                                                             | Current `origin/main`, a documentation-only advance after measured source `69f27f6`, so the scanner timing is unchanged                                                                                                                                                           |
| Unmerged evidence (addendum, no acceptance)           | PR #121 P2-08 installed retry/presentation revalidation at `1342971`                                                        | Executed once on installed `adab234`: D2/D3 converged, typed `cancelled`/`unavailable` presentation revalidated, desktop/narrow/keyboard/AX captures with one moderate axe `region` finding; D2/D3 policy-versus-defect disposition and owner acceptance remain open              |
| P2-08 (addendum, still Partial)                       | Narrows the 2026-09-13 gap: D2/D3 observed on the current head; #111 presentation revalidated for `cancelled`/`unavailable` | Remaining gaps: `access_denied`, `resource_limit`, `unsupported`, and `worker_failed` presentations unproduced (NTFS/overflow/malformed cases not rerun); `queued`/`running` lack keyboard/axe; `interrupted` only durable; full criterion and owner acceptance remain unpromoted |
| Unmerged diagnostics (addendum, no performance claim) | PR #119 feature-gated `scan-execution` phase diagnostics and regression coverage at `f64decd`                               | Diagnostic investigation only; the 10,173 ms non-qualifying warm-p95 result is unchanged; the proposed ancestor-validation fast path is a proposal for separate gating, not approved; owner review remains open                                                                   |

<!-- markdownlint-enable MD060 -->

## Merged-state correction 2026-09-14

The [status-table addendum 2026-09-14](#status-table-addendum-2026-09-14) above
recorded its boundary and PR state as verified before the #117/#121/#120/#122
merges. GitHub now reports those states superseded; this correction is added
rather than rewriting the dated addendum or its rows.

- Publication boundary: `origin/main` is
  `3fcaa4ade3099105594b48eadc785731608c2824` (merge of PR #122), not `adab234`
  (merge of PR #118).
- PR #117 merged as `9ebcc9ffcdd94300b9ad2af22281be4ec8c187db`.
- PR #121 merged as `13c0ba2af88673746a96e787fae315d2eca920a0` (squash of
  updated head `e04ce92d0e69b9defd69b5615c0b047aac23ebfc`; the addendum's
  `1342971` was the pre-update head). Its P2-08 installed evidence is now
  published on `main`.
- PR #120 merged as `fea3a8763d039c9855a368f7072a4f41acef8d85`.
- PR #122 merged as `3fcaa4ade3099105594b48eadc785731608c2824`.
- PR #119 remains an open draft, **UNMERGED** at
  `f64decd5a4ae272c060ed647c4bb2ae077fa3c56`, so its addendum row is unchanged.

The merges are documentation-only advances after measured source `69f27f6`: no
scanner timing was rerun, P2-08 remains **Partial** with the same narrowed
gaps, and no acceptance ID is promoted or owner decision recorded by them.

## Owner acceptance 2026-09-14

The owner closed the Phase 2 evidence aggregator
[#41](https://github.com/guilhermebmichelin-create/fruitboard/issues/41) as
COMPLETED on 2026-09-14 and recorded Phase 2 as **accepted with known gaps**.
Epic #33 remains open. The [acceptance
record](review/phase-2-integration/acceptance-2026-09-14.md) owns the
criterion-by-criterion disposition; this plan is not rewritten by it.

<!-- markdownlint-disable MD060 -->

| State                                 | Work                                                          | Evidence or next action                                                                                                                                                                                                                                                                            |
| ------------------------------------- | ------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Accepted with known gaps (2026-09-14) | Phase 2 accepted; #41 closed COMPLETED; epic #33 remains open | [Acceptance record](review/phase-2-integration/acceptance-2026-09-14.md): P2-08 stays Partial, #107/#47/#48 stay open, F1/F2/F3 and the performance non-qualification carry forward, and the bounded Rust-parser spike is the next step. No budget, scope, or production-activation change follows |

<!-- markdownlint-enable MD060 -->

## Historical acceptance ownership and evidence snapshot - preserved

The original four-column ownership table remains below as historical contract
provenance. Its required-evidence wording is not a current gap classification;
the current ledger above controls this refresh.

| ID    | Acceptance criterion                                                         | Owner                       | Required evidence                                                                                          |
| ----- | ---------------------------------------------------------------------------- | --------------------------- | ---------------------------------------------------------------------------------------------------------- |
| P2-01 | Existing root settings/picker remain safe and accessible                     | #34/#35                     | Installed selection/cancel evidence; settings restart tests; rendered narrow/keyboard follow-up            |
| P2-02 | Unchanged tree causes no spurious file changes; add/modify/rename converge   | #36                         | Deterministic synthetic enumerator and qualified NTFS fixture tests                                        |
| P2-03 | Partial/offline/denied/cancelled/limited traversal never marks files missing | #36 + #40                   | Fault injection for every outcome; compare committed rows before/after                                     |
| P2-04 | Generations, dedup, leases, backoff and cancellation obey the contracts      | #38                         | Fake-clock/state-machine tests including stale workers and both cancel/commit orderings                    |
| P2-05 | Disable/remove while queued/running prevents stale publication               | #38 + #40                   | Concurrent operation tests; source marker hashes unchanged; re-enable fresh run                            |
| P2-06 | Atomic publication and restart/backup recovery preserve valid data           | #40                         | Crash before/after staging/apply, migration rollback and backup/recovery fixtures                          |
| P2-07 | Hardlink aliases and uncertain identity preserve per-path presence           | #40 + #36                   | Required two-location identity regression; rename/replacement tests; #48 limits stated                     |
| P2-08 | Scan/Cancel/Retry and Library list are usable, honest and persistent         | #38 controls; #40 list      | Typed IPC/client/native integration tests, axe, keyboard and desktop/narrow evidence                       |
| P2-09 | Watcher bursts/overflow/event loss converge through durable reconciliation   | #37                         | Synthetic burst/overflow/restart tests, bounded coalescing; #47 manual evidence for Drive claims           |
| P2-10 | No parsing, hydration or source mutation enters filesystem-only discovery    | #39 + #36                   | Dependency/privacy checks, content-read spy tests and source-byte preservation                             |
| P2-11 | Performance/resource limits are measured and safely enforced                 | #36/#38/#40; #41 aggregates | Reproducible benchmark report with environment, targets, failures and resource-limit tests                 |
| P2-12 | Complete visible journey works after integration                             | #41                         | Merged PR mapping, CI results, Windows interactive journey, explicit unverified modes and owner acceptance |

Each implementation PR cites its acceptance IDs and supplies tests in that PR.
Cross-issue criteria require integrated evidence before they count as complete.
Issue #41 aggregates evidence throughout development; it is not a late testing phase.

## Proposed targets accepted as provisional budgets

These are owner-accepted starting budgets, not measured promises. The existing
scanner contract and the current run are bounded at 10,000 observations. The
100,000-entry qualification set and corresponding memory target below remain
provisional F3 options, not a current implementation or acceptance requirement.
Record a reference Windows machine (CPU, RAM, storage, OS, power mode), pinned
release build and synthetic fixture generator/seed. Run 10 measured iterations after one warm-up; report
median, maximum and nearest-rank p95. Report first-run separately; do not call
it cold-cache unless OS cache state was actually controlled.

| Area              | Proposed target                                                                                                       | Failure behavior / measurement                                                                               |
| ----------------- | --------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| Baseline fixture  | Current contract: 10,000 FLP-named synthetic observations; 100,000-entry qualification set is a provisional F3 option | No private projects; no parser; include Unicode, long paths and aliases in correctness sets                  |
| NTFS scan latency | Baseline first discovery <=30 s; unchanged warm reconciliation p95 <=10 s                                             | Measure enumerate + stage + final apply; investigate misses, never skip safety checks                        |
| Cancellation      | UI acknowledgement <=250 ms; cooperative worker stop p95 <=1 s between responsive I/O calls                           | Blocking OS calls may exceed this; invalidate publication immediately, report observed stop latency honestly |
| Working memory    | Provisional F3 option: scanner incremental private memory <=128 MiB on the 100,000-entry set; not measured            | Measure against idle app; bounded streaming, no full-tree in-memory accumulation                             |
| Batch/staging     | <=512 records per batch; <=256 MiB staging per run                                                                    | Enforced quota failure retains prior results; benchmark disk use and cleanup                                 |
| Queue/leases      | One worker, one follow-up/root; 30 s lease, renewal every 5 s                                                         | Fake-clock expiry tests; lease validation before all writes                                                  |
| Retry budget      | Three automatic retries at 1/2/4 s, bounded jitter <=20%; then explicit retry                                         | Persist attempts; no tight loops on disconnected roots                                                       |
| Progress/list     | <=4 progress updates/s; <=200 records/page; baseline query p95 <=200 ms                                               | Test throttling and page order; measure IPC query separately from rendering                                  |
| Root budget       | Initially qualify <=100 configured roots                                                                              | Validate budget before claiming larger scale; never silently ignore excess roots                             |

If measurements reject a target, amend this table with evidence and owner review
before broadening support. CI correctness tests use deterministic clocks and
invariants, not hardware-sensitive absolute timing assertions. Numerical budget
approval does not waive correctness, accessibility or privacy acceptance.

## Research gates, sequencing and parser decision

Local NTFS contract design and implementation may proceed using recorded
findings. Drive modes and non-NTFS identity remain unverified under #47/#48.
Before claiming support, land their evidence or obtain an explicit owner scope
exclusion; do not reinterpret unavailable environments as passing tests.

The merged boundary, current PR heads, and exact-head CI results are maintained
in the [dated live review status table](review/phase-2-integration/README.md#live-review-status-2026-09-13).
Earlier boundaries such as `0b7612d`/PR #89 remain historical inputs. Native
watcher supervision, durability close-out coverage, the benchmark triage
re-run, the fixed installed report, and the final reconciliation are merged;
the current closeout packet records their evidence classes separately from
acceptance. The merged #111 product changes are now part of `main`, but the
installed NTFS observations remain tied to tested source `05fb35c` and report
revision `ef08522`; they were not rerun on the merged baseline. #112 and #114
are also merged in the current source baseline: #112 supplies diagnostics and
the merged #114 supplies the executable runbook/validator, but neither supplies a
qualification result or owner acceptance.

The current publication boundary is fetched `origin/main`
`83da093672b5e2154097c897af533821b04f2352`, after merged #110 -> #111 -> #112
-> #114 and documentation-only #115 -> #116. The scanner was measured from
separate source `69f27f64f26aa657182a9260cc8e78f28a5838fb`; #114's runbook and
the #116 publication do not turn that result into owner acceptance. The #113
branch is an older synthetic combined candidate at
`e90b03cc0bddd1a449e817ea2d89708a85c82ef1`, based on an earlier `main`, and is
superseded rather than a replacement for the current baseline.
The source history retains one canonical S5 source `19585da`; recovered
`de396e7` is historical provenance and requires no future deduplication step.
Agent 2's final verification found no S5 product or contract defect and did not
rerun S5. The approved `custom-9995`/`repository-minimum` choices are recorded
and used; do not request them again or reinstate historical strict-profile/A/B
requirements. Resolve F3/10,000-entry scope, #47/#48, P2-09 timing/overflow,
P2-10, inline onboarding, #107/S5, and the owner's criterion-by-criterion P2
checklist using the
[closeout packet](review/phase-2-integration/acceptance-packet-2026-09-12.md).

The source merge handoff has been verified for #110, #111, #112, and #114; #115
and #116 are documentation-only publication merges. Any future update to those
PRs must first fetch and verify actual `main`, preserve
the canonical S5 and installed-evidence union, and rerun the required checks on
its new head. This refresh is documentation-only and starts from the verified
`#116` baseline; its merge does not accept Phase 2. #108 remains unexplained
historical diagnostics, and #113 remains a superseded historical candidate.
The original benchmark `Partial` remains unexplained, performance remains
unqualified, and no budget, platform, or acceptance promotion follows from
merged implementation, CI, or a combined candidate. Keep production scanning
hidden until P2-03 through P2-08 have integrated evidence and owner acceptance.

Issue #39 records the chosen filesystem-only path and its no-parser evidence; it is
not blocked waiting for a parser and does not authorize a production adapter.
After Phase 2 owner acceptance, schedule the bounded independent Rust-parser
spike before Phase 3 selection. PyFLP remains an optional candidate subject to
compatibility, packaging and compatible licensing gates, not a requirement.
No new Phase 3 issues/code are needed to complete this planning PR.

## Current post-#152 handoff — 2026-09-21

This addendum is the current status correction; historical baselines and dated
tables above remain unchanged. `main` is now
`73775453e9e4021c7b6b4ade381fd87300f47c32`, the squash merge of #152, whose
reviewed product tree is `79da7e97f7b8481eab1f01661fe54ab0b33d98aa`. The
explicit Retry and cancellation-finalization fixes are therefore merged and
their previous owner-action item is complete.

The feature-enabled installed follow-up uses synthetic local NTFS roots only.
It passes ten native scenarios and the actual visible settings/Library journey,
including keyboard focus, Scan now, Cancel, typed denied/resource-limit copy,
and committed-row preservation. The retained evidence root is
`post152-autonomous-followup-20260921`; the final driver record is `pass: true`
and explicitly excludes FAT32, DriveFS, and network shares.

P2-08 remains **Partial**. The installed run now supplies visible
access-denied/resource-limit evidence and targeted axe passes for the exercised
Preferences, Library, running, cancelled, and keyboard-started states. It does
not supply a stable queued screen, `unsupported` or `worker_failed` visible
presentations, or a stable rendered interrupted state. Broader axe snapshots
still contain incomplete results, so the targeted passes are not blanket
accessibility clearance. Issues #38 and #40 remain open for that narrowed
evidence residual; #47/#48 remain excluded or unverified territory.

No performance qualification, non-NTFS support claim, parser research, full
P2-08 acceptance, production-scanning activation, or security-product
activation follows from this implementation or evidence refresh. The owner
still decides the remaining acceptance, fixture/parser, security, and scope
items listed in the final handoff.

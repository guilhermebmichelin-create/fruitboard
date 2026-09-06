# Phase 2 execution contracts and acceptance plan

Status: **Accepted for implementation; not an implementation or measured
performance claim.** Owner checkpoint: 2026-09-06 accepted the shared contracts
and proposed starting budgets below. Inline Preferences onboarding remains a
separate pending decision. Baseline: main `82bc651` (PR #61), 2026-09-06. This
document makes the accepted filesystem-only direction executable. Do not
silently relax safety gates to meet a target. ROADMAP remains the milestone
sequence; this document owns the detailed Phase 2 contracts and acceptance IDs
referenced by issues.

## Current status

| State | Work | Evidence or next action |
| --- | --- | --- |
| Delivered | Phase 1 foundation, accepted; #34 native picker/root storage | PRs #20-32 and #44; installed picker evidence in `docs/review/issue-34/` via #54 |
| Delivered | #35 persistent rename/enabled settings, inline onboarding, failure/focus fixes | PRs #55/#56/#58; CI and regression tests; not scanner execution |
| Delivered | #35 rendered desktop/narrow keyboard verification | [Evidence](review/issue-35/README.md) and PR #58; fake-adapter rendering is labeled separately from installed-app evidence |
| Delivered | #36 bounded reconciliation reference core | PR #59 and `crates/reconciliation/README.md`; deterministic in-memory foundation only, not filesystem enumeration or production scanning |
| Delivered | #38 durable scan execution foundation | PRs #60/#61 and [durable execution evidence](PHASE_2_DURABLE_EXECUTION.md); migrations, jobs, runs, leases and recovery fencing only |
| Decision pending | Inline Preferences onboarding instead of a separate first-run route | Implementation documented; owner review remains separate from the accepted scanner contracts |
| In progress | #40 durable staging/publication storage slice | [Publication evidence](PHASE_2_DURABLE_PUBLICATION.md); migration 004 and typed storage APIs; traversal, Library UI and production scanning remain open |
| Next | Integrate bounded #36 enumeration and the minimal Library journey | Keep production scanning hidden until incomplete-run, restart and atomic-publication tests pass |
| Blocked/unverified | DriveFS modes (#47), cross-volume/FAT32 identity (#48) | #42/#43 closed with partial findings, not blanket platform qualification |
| Deferred | Parsing (#39), logical grouping, Kanban, playback, sync, PWA | Phase 2 is filesystem-only; bounded independent Rust-parser spike follows accepted MVP |

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

| Previous versus observed | Decision |
| --- | --- |
| Same normalized path, same identity/size/mtime | No file change; run freshness advances separately |
| Same path, same qualified identity, changed size/mtime | Modified filesystem metadata; no claim about content equality |
| Same path, both qualified identities differ | Replacement, retaining path history and assigning the observed physical association; never silently continue the old physical file |
| Same path, identity unavailable on either side | Path continuity only, explicitly uncertain; changed metadata is modification evidence, never proof of identity |
| Old path absent, new path has same qualified identity | Record old path missing and new path present; emit rename evidence only for an unambiguous one-old/one-new match |
| Two or more paths share an identity | Preserve every location; identity lookup is non-unique, and ambiguous alias changes do not prove a rename |
| Missing path observed again | Restore that location's presence; separately flag replacement if qualified identity changed |
| Unseen path after incomplete/offline/cancelled traversal | No transition; previous committed state remains intact |

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

## Acceptance ownership and evidence

| ID | Acceptance criterion | Owner | Required evidence |
| --- | --- | --- | --- |
| P2-01 | Existing root settings/picker remain safe and accessible | #34/#35 | Installed selection/cancel evidence; settings restart tests; rendered narrow/keyboard follow-up |
| P2-02 | Unchanged tree causes no spurious file changes; add/modify/rename converge | #36 | Deterministic synthetic enumerator and qualified NTFS fixture tests |
| P2-03 | Partial/offline/denied/cancelled/limited traversal never marks files missing | #36 + #40 | Fault injection for every outcome; compare committed rows before/after |
| P2-04 | Generations, dedup, leases, backoff and cancellation obey the contracts | #38 | Fake-clock/state-machine tests including stale workers and both cancel/commit orderings |
| P2-05 | Disable/remove while queued/running prevents stale publication | #38 + #40 | Concurrent operation tests; source marker hashes unchanged; re-enable fresh run |
| P2-06 | Atomic publication and restart/backup recovery preserve valid data | #40 | Crash before/after staging/apply, migration rollback and backup/recovery fixtures |
| P2-07 | Hardlink aliases and uncertain identity preserve per-path presence | #40 + #36 | Required two-location identity regression; rename/replacement tests; #48 limits stated |
| P2-08 | Scan/Cancel/Retry and Library list are usable, honest and persistent | #38 controls; #40 list | Typed IPC/client/native integration tests, axe, keyboard and desktop/narrow evidence |
| P2-09 | Watcher bursts/overflow/event loss converge through durable reconciliation | #37 | Synthetic burst/overflow/restart tests, bounded coalescing; #47 manual evidence for Drive claims |
| P2-10 | No parsing, hydration or source mutation enters filesystem-only discovery | #39 + #36 | Dependency/privacy checks, content-read spy tests and source-byte preservation |
| P2-11 | Performance/resource limits are measured and safely enforced | #36/#38/#40; #41 aggregates | Reproducible benchmark report with environment, targets, failures and resource-limit tests |
| P2-12 | Complete visible journey works after integration | #41 | Merged PR mapping, CI results, Windows interactive journey, explicit unverified modes and owner acceptance |

Each implementation PR cites its acceptance IDs and supplies tests in that PR.
Cross-issue criteria require integrated evidence before they count as complete.
Issue #41 aggregates evidence throughout development; it is not a late testing phase.

## Proposed targets accepted as provisional budgets

These are owner-accepted starting budgets, not measured promises. Record a reference Windows
machine (CPU, RAM, storage, OS, power mode), pinned release build and synthetic
fixture generator/seed. Run 10 measured iterations after one warm-up; report
median, maximum and nearest-rank p95. Report first-run separately; do not call
it cold-cache unless OS cache state was actually controlled.

| Area | Proposed target | Failure behavior / measurement |
| --- | --- | --- |
| Baseline fixture | 10,000 FLP-named synthetic files across 1,000 directories; qualification set 100,000 entries | No private projects; no parser; include Unicode, long paths and aliases in correctness sets |
| NTFS scan latency | Baseline first discovery <=30 s; unchanged warm reconciliation p95 <=10 s | Measure enumerate + stage + final apply; investigate misses, never skip safety checks |
| Cancellation | UI acknowledgement <=250 ms; cooperative worker stop p95 <=1 s between responsive I/O calls | Blocking OS calls may exceed this; invalidate publication immediately, report observed stop latency honestly |
| Working memory | Scanner incremental private memory <=128 MiB on the 100,000-entry set | Measure against idle app; bounded streaming, no full-tree in-memory accumulation |
| Batch/staging | <=512 records per batch; <=256 MiB staging per run | Enforced quota failure retains prior results; benchmark disk use and cleanup |
| Queue/leases | One worker, one follow-up/root; 30 s lease, renewal every 5 s | Fake-clock expiry tests; lease validation before all writes |
| Retry budget | Three automatic retries at 1/2/4 s, bounded jitter <=20%; then explicit retry | Persist attempts; no tight loops on disconnected roots |
| Progress/list | <=4 progress updates/s; <=200 records/page; baseline query p95 <=200 ms | Test throttling and page order; measure IPC query separately from rendering |
| Root budget | Initially qualify <=100 configured roots | Validate budget before claiming larger scale; never silently ignore excess roots |

If measurements reject a target, amend this table with evidence and owner review
before broadening support. CI correctness tests use deterministic clocks and
invariants, not hardware-sensitive absolute timing assertions. Numerical budget
approval does not waive correctness, accessibility or privacy acceptance.

## Research gates, sequencing and parser decision

Local NTFS contract design and implementation may proceed using recorded
findings. Drive modes and non-NTFS identity remain unverified under #47/#48.
Before claiming support, land their evidence or obtain an explicit owner scope
exclusion; do not reinterpret unavailable environments as passing tests.

Sequence: accepted planning contracts -> #59 bounded #36 reconciliation core ->
PR #60 durable #38 execution foundation -> #40 staging/publication and read-model
slices against the contracts -> bounded enumeration and integrated manual scan
journey -> #37 watcher integration -> #41 owner checkpoint. Schema and port work
may precede consumers; issue numbering is not a strict coding order. Keep
production scanning hidden until P2-03 through P2-08 are integrated.

Issue #39 records the chosen filesystem-only path and its no-parser evidence; it is
not blocked waiting for a parser and does not authorize a production adapter.
After Phase 2 owner acceptance, schedule the bounded independent Rust-parser
spike before Phase 3 selection. PyFLP remains an optional candidate subject to
compatibility, packaging and compatible licensing gates, not a requirement.
No new Phase 3 issues/code are needed to complete this planning PR.

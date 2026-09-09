# Bounded chunked snapshot — scale design proposal (F1/F3)

Status: **design proposal only; not authorization to implement a new scan
protocol.** This document turns the F3 “bounded chunked snapshot” suggestion
from the [performance follow-up](performance-followup/README.md) into a
concrete, reviewable proposal, and records the exact F1 fixture alternative
alongside it. It changes no production code, shared fixture preset, quota,
budget, acceptance criterion, feature gate, or platform scope. Provisional
budgets in `docs/PHASE_2_EXECUTION_PLAN.md` remain unchanged. Production
scanning stays hidden until P2-03 through P2-08 have integrated evidence.
Epic #33 and issues #36–#41, #47, #48 remain open. No acceptance is promoted.

Evidence classes are kept separate throughout: merged code, unmerged draft
changes, automated tests, installed-app observations, and owner acceptance
decisions are different columns. A green check or a merged commit in one
column does not fill another.

## 0. Basis and boundaries

- Current merged baseline: `origin/main`
  `0b7612db3570e6235d4d2c86a678dd9004264f30` (PR #89); exact-commit
  Foundation CI run
  [34293250231](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34293250231)
  is green.
- Current contracts: `docs/PHASE_2_EXECUTION_PLAN.md` (“Shared contracts:
  #36 / #38 / #40”), including root revision/run identity, enumeration and
  authoritative completion, durable queue/cancellation/restart, and atomic
  application/read model. This proposal extends those contracts; it does not
  replace or relax them.
- Current enforced bounds (unchanged; cited from
  `crates/storage-sqlite/src/publication.rs:94-98`,
  `crates/scan-execution/src/lib.rs:103-110,700-706`):
  worker `EnumerationLimits.max_observations = MAX_STAGED_RECORDS = 10,000`;
  `MAX_STAGED_PATH_BYTES = 4 MiB`; `MAX_STAGED_BATCH_RECORDS = 512`;
  `PlanBuffer::CAPACITY = 10,000`; `MAX_LIBRARY_PAGE_SIZE = 200`.
  A 100,000-record set therefore needs 196 staging batches and about 500
  Library pages by arithmetic; those are planning values, not measurements.
- Measured inputs (historical records, not edited here): the 2026-09-08
  re-run at `51f45af` (F1 reproduced at 10,005 observations vs 10,000 quota;
  quota-fitting warm p95/max 14,267 ms vs provisional 10 s; F3 unmeasured),
  the contended filesystem-port diagnostic in unmerged PR #93
  (`59faefc2806a725368a59e7b6fc9be7f863f4fec`; 10.754 s of 12.157 s warm scan
  in filesystem-port calls), and the stacked native candidate in unmerged PR
  #94 (`588867bca154e798f189f6c99de8a2668005d141`, base
  `docs/41-performance-followup-20260908`) which has no exact-head CI yet and
  reports a contended median move 11,735.5 ms to 11,517 ms with p95/max
  worsened 12,188 ms to 17,682 ms by an outlier. None of these is an
  idle-host qualification.
- Sibling verification inputs (unmerged, not edited here; see the PR #91
  reconciliation for the full ledger): draft PR #97 head `aba1812`
  (`test/95-durable-queue-watcher-20260909`, base PR #95 `bf0aeac`) adds three
  deterministic automated integration tests (mid-scan convergence through the
  status/Library API, 5,000 + 5,000 idle-burst collapse to one queued job,
  stale-generation fencing for disabled/removed roots) with Foundation run
  `34339792661` and packaging run `34339792759` green; installed queued
  visibility and burst timing remain unverified there. Draft PR #98 head
  `15b7f17` (`perf/94-validation-20260909`) independently validates PR #94 on
  the shared contended host (all runs fail the 10 s p95; no qualification
  claimed) and removes 8 lines of dead diagnostic scaffolding; its CI is
  separately dispatched and Agent 1 review is requested. Neither input changes
  the chunked-snapshot design below; both are recorded here so this proposal
  is not read as contradicting them.
- Installed inputs (unmerged, not edited here): PR #92 head `49a5e649`
  (`run-20260908.md`) and PR #95 head `bf0aeac`
  (`run-20260909.md`, Foundation run
  [34309511415](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34309511415)
  and packaging run
  [34309511404](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34309511404)).
  D1–D3 repairs are verified in #95 but that PR remains an unmerged draft;
  durable queued snapshots and independent watcher-burst counts remain gaps
  addressed independently of this proposal.
- Platform inputs (unmerged, not edited here): PR #90 head `17e5717`
  records #47/#48 blockers. This proposal carries those prerequisites forward
  unchanged and invents no DriveFS consent, FAT32 availability, or platform
  exclusion.
- Out of scope for this proposal: parser selection, logical grouping, Kanban,
  playback, sync, PWA, signing/updater, macOS port, and any change to the
  10-second provisional target itself (F2 budget/host/rerun remains a separate
  owner decision with a required quiet-host ten-iteration rerun).

## 1. F1 — exact 10,000-observation fixture change vs quota alternative

Neither option is applied by this proposal. Both require a fresh
accepted-protocol run before any qualification claim.

### Option F1-A (recommended as narrowest): exact-10,000-observation fixture

- Definition: `custom-9995`, seed `0`: **9,995 FLP-named files + 5 hardlink
  alias locations = 10,000 observations**, plus 4 non-FLP files excluded from
  observations. Leaf directories 1,000; total directories 1,025; empty
  directories 10; hardlink groups 5; hardlink support `created` on Windows.
  Manifest SHA-256
  `a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a`
  (see PR #93 `provenance.json`).
- Exact generation (pinned Node 24.20.0; synthetic bytes only; no private
  data; disposable empty `--out` outside filesystem roots/home):
  `node scripts/run-benchmark.mjs --size custom --files 9995 --seed 0 --out <report>.json`,
  equivalent to `buildPlan({ sizeLabel: 'custom-9995', seed: '0', fileCount:
  9995, leafDirectoryCount: 1000 })` followed by `writePlan`.
  Accepted baseline for comparison remains `--size baseline --seed 0`
  (10,000 FLP + 5 aliases = 10,005 observations, manifest hash
  `8c3d85ec01299995208abfa450378b1b37c704e423593d7a4370ec465254afba`).
- What changes: fixture definition only. Worker, staging, storage, plan
  buffer, page, and test quotas stay at their current values. Alias locations
  continue to count as observations (hardlink aliases retain distinct
  locations per the accepted identity contract).
- Validation consequences if the owner selects F1-A:
  1. Run the accepted protocol fresh on the new fixture: 1 warm-up + 10
     measured fresh-process iterations, pinned release build timed
     separately, temp-generated fixtures with recorded seeds/hashes,
     working-set sampling, mid-run cancellation stop latency, and the budget
     pass/fail table per `scripts/benchmark-scan.md`.
  2. Report median, maximum, and nearest-rank p95 (for n=10, p95 is the max),
     first-discovery time, cooperative stop samples, and working-set
     increment. Report first-run separately; do not call it cold-cache unless
     OS cache state was controlled.
  3. Re-baseline F2 numbers on the new fixture; prior 14,267 ms p95 was
     measured on the quota-fitting set at `51f45af` and does not transfer
     automatically.
  4. Keep the accepted 10,000-FLP baseline manifest as a historical record;
     do not silently redefine “baseline” in old reports.
  5. No production code change is needed for F1-A itself, but the fixture
     preset documentation and any fixture-hash pins must be updated in the
     same PR that lands the decision, with a new manifest hash recorded.

### Option F1-B: coordinated quota increase above 10,005

- Definition: keep the accepted 10,000-FLP fixture (10,005 observations) and
  raise **every** coupled fence together to a single new value above 10,005
  (example value to be chosen by the owner; “10,005” is the floor, not the
  recommendation). Do not raise only one limit.
- Coupled fences that must move together:
  `WorkerConfig::default EnumerationLimits.max_observations`,
  `MAX_STAGED_RECORDS` (worker + storage + publication fences),
  `MAX_STAGED_PATH_BYTES` (re-evaluate whether 4 MiB still holds at the new
  record count), `PlanBuffer::CAPACITY`, staging batch handling
  (`MAX_STAGED_BATCH_RECORDS` stays 512 unless the owner also re-budgets
  batching), Library paging expectations, and every test that asserts the
  10,000 boundary (including `p2_03_*` resource-limit, staging-quota, and
  quota-exhaustion cases).
- Validation consequences if the owner selects F1-B:
  1. Implementation/configuration PR plus updated migration/backfill review
     if any persisted limit changes shape.
  2. Fresh accepted-protocol run on the unchanged 10,000-FLP fixture,
     reporting the same statistics as F1-A, plus staged-DB bytes, cleanup
     behavior, and private-memory increment at the new quota.
  3. Explicit record that aliases count toward quota at the new value.
  4. Higher cost than F1-A: touches worker, staging, storage, and tests, and
     still does not qualify 100,000 entries.

Recommendation for F1: F1-A is the smallest change that restores a
fence-fitting baseline without touching production quotas. F1-B is reserved
for the case where the owner requires the 10,000-FLP count to remain the
canonical baseline fixture.

## 2. F3 — bounded chunked snapshot proposal

Goal: qualify a 100,000-entry set without holding the full tree in private
memory and without weakening authoritative publication. The run enumerates in
bounded chunks, durably tracks per-chunk coverage, and publishes **once**,
atomically, after all chunks for the root generation succeed.

### 2.1 Durable staging and coverage schema, ownership

- Owner: `crates/storage-sqlite` (same crate that owns run/job/staging/read
  model today). No second write path; sync (when it later exists) continues
  to consume durable operations from the same mutation path.
- Proposed new durable state (names illustrative; exact DDL belongs to the
  schema PR):
  - `scan_chunk_coverage(run_id, chunk_index, coverage_key, state,
    enumerated_observations, path_bytes, started_at_ms, completed_at_ms)`:
    one row per chunk. `coverage_key` is an opaque, ordered partition
    descriptor (e.g., directory-shard range), never a raw path in logs.
    `state` is `pending | complete | failed | discarded`.
  - Chunk staging reuses the existing per-run staging shape keyed by
    `(run_id, chunk_index)`, subject to the per-chunk and per-run bounds in
    §2.3. Staging rows remain outside the visible Library dataset until the
    single final publication.
  - `scan_run` gains `chunk_count`, `chunk_size_target`, and
    `coverage_complete` markers so readers can distinguish “chunk 3 of 20
    complete” from “authoritative”.
- Coverage ledger rules:
  - The ledger is written in the same durable transactions as chunk staging;
    a chunk with failed staging cannot be marked `complete`.
  - Only `complete` chunks count toward coverage. `failed`/`discarded`
    chunks block publication until they are retried to `complete` or the run
    is terminally failed/cancelled with no publication.
  - The ledger is per `(run_id, generation, configuration_revision,
    lease_token)`; any generation/revision/lease invalidation discards all
    of that run’s chunks and coverage together (see §2.2).
- Migration implications: forward migration adds the coverage table and run
  columns with defaults that keep existing single-chunk runs valid
  (chunk_count 1, coverage trivially complete under the old path). Rollback
  on migration failure, backup/recovery inclusion, and startup-preference /
  root-configuration retention follow the existing Phase 2 migration rules.
  WAL stays disabled unless a separately reviewed change justifies enabling
  it.

### 2.2 Generation and lease fencing across chunks

- One root generation, one run ID, one lease token per snapshot attempt.
  Chunk workers do not mint generations; retries allocate fresh generations
  with a separate retry-chain attempt count so restarting cannot reset the
  automatic retry budget (existing contract preserved).
- Every chunk staging write and every coverage transition validates
  `(generation, configuration_revision, lease_token, lease deadline, session
  ID)` exactly as the current staging/publication transactions do. Token
  equality alone is insufficient; deadlines are checked against a monotonic
  clock within the session and durable timestamps schedule retry eligibility
  after restart.
- Lease renewal (currently 30 s lease, renewal every 5 s) covers the whole
  run, not per-chunk leases that could outlive the run. An expired or
  replaced worker cannot commit any chunk even if it later returns.
- Disable/remove in the same configuration transaction cancels queued work,
  invalidates the running generation, and discards that run’s chunks and
  coverage. Re-added paths get a fresh root ID and cannot receive old chunks.
- Process-session fencing: on restart, prior-session running leases become
  `interrupted`, their chunks/coverage are ineligible for publication, and
  the same job/retry chain is requeued with its persisted attempt budget and
  backoff. An abandoned enumeration cursor (chunk offset) is never resumed as
  authoritative; the rerun re-enumerates from chunk boundaries.
- Out-of-order chunk completion is allowed; publication order is by
  `chunk_index`, not arrival order. Duplicate chunk deliveries (retry +
  original both return) are fenced by `(run_id, chunk_index, lease_token)`;
  the loser is discarded.

### 2.3 Explicit memory, disk, path-byte, and record bounds

All bounds are enforced, not advisory. Quota exhaustion in any chunk makes
the run non-authoritative with prior committed results retained (existing
failure semantics preserved).

- Per-chunk in-memory: chunk observation buffer ≤ a new
  `MAX_CHUNK_OBSERVATIONS` (proposed starting value to be chosen in the
  schema PR; must be ≤ `PlanBuffer::CAPACITY` and small enough to hold the
  provisional 128 MiB private-memory budget at 100,000 entries — e.g., 2,000
  observations per chunk implies 50 chunks; exact value requires measurement
  and owner approval, not set here).
- Per-chunk staging writes: reuse `MAX_STAGED_BATCH_RECORDS = 512` batches;
  a chunk therefore needs `ceil(chunk_observations / 512)` batches.
- Per-run staging: total staged records across all chunks ≤ a new
  `MAX_SNAPSHOT_STAGED_RECORDS` (must be ≥ 100,000 to qualify the target;
  value and disk-budget approval belong to the owner decision, not this
  proposal). Total staged path bytes across all chunks ≤ a new
  `MAX_SNAPSHOT_STAGED_PATH_BYTES` (must be re-derived from measured
  path-length distribution at 100,000 entries; the current 4 MiB per-run cap
  is insufficient by construction and is not silently scaled here).
- Advisory plan buffer: `PlanBuffer::CAPACITY` (10,000) cannot hold a
  100,000-record change plan. The proposal pages the change plan from durable
  chunk staging in bounded pages (≤ `MAX_LIBRARY_PAGE_SIZE = 200` rows per
  read) rather than raising the in-memory plan buffer to 100,000. Paged reads
  alone do not constitute the bounded planning algorithm: the global identity
  decisions in §2.9 item 1 still require a demonstrated mechanism (options
  and validation there), not just page-sized reads.
- Library paging stays ≤ 200 records/page with stable ordering; 100,000 rows
  imply about 500 pages. Progress reporting stays ≤ 4 updates/s and never
  fabricates a percentage when total work is unknown.
- Disk/working-set measurement gates for any qualification run: private
  bytes vs idle app, staged-DB bytes, wall time (enumerate + stage + final
  apply split by phase), cleanup bytes reclaimed, and cancellation stop
  latency. CI correctness tests keep deterministic clocks and invariants, not
  hardware-sensitive absolute timing assertions.

### 2.4 One atomic authoritative publication

- Exactly one final transaction per run, owned by #40, after every chunk is
  `complete` and every coverage key for the root is accounted for:
  1. Revalidate generation/revision/lease/cancellation and that no
     invalidating watcher event arrived during chunking.
  2. Page through durable chunk staging in bounded reads, apply observed
     file/location metadata, and advance the change plan without loading all
     100,000 records into memory. The paged apply still requires the global
     identity mechanism of §2.9 item 1 and the lock-duration evidence of §2.9
     item 5; paging the reads does not by itself bound planning memory or
     shorten the write lock.
  3. Mark eligible unseen locations missing **only** if §2.5 coverage holds.
  4. Record run completion and advance the last-success marker.
- Any failure rolls back the entire publication. Unsuccessful runs publish no
  partial file list. Calling current publication once per chunk is explicitly
  forbidden: it would create multiple root generations and could mark records
  from other chunks missing.
- The Library continues to read only committed generations with bounded
  pages. A root’s previous results remain identifiable while chunking runs
  or fails.

### 2.5 Missing-file decisions only after complete root coverage

- A location becomes `missing` only after a **successfully completed,
  authoritative snapshot** for an available root that covers every in-policy
  directory. Coverage completeness is read from the durable ledger, not
  inferred from “no error was seen.”
- Non-authoritative outcomes (access denial, root loss, failed metadata,
  resource-limit/quota exhaustion in any chunk, cancellation, lease expiry,
  observed invalidating event, or any `failed`/`discarded` chunk) retain
  prior committed results and infer no absence — including for subtrees whose
  chunks did complete. Excluded subtrees (policy exclusions, nested reparse
  fences, unhydrated placeholders) never become missing by policy change or
  by another chunk’s success.
- Rename/replacement/alias semantics from the accepted contracts are
  unchanged: same-path identity transitions, non-unique identity lookups,
  per-location missing/restored, no Phase 4 grouping, duplicates invalidate
  rather than letting order pick a winner.

### 2.6 Cancellation, crash/restart, expiry, cleanup, backup

- Cancellation: persisted before acknowledgement. Workers check between
  chunks, between batches, and before final apply. UI acknowledgement budget
  (≤ 250 ms) and cooperative worker stop budget (p95 ≤ 1 s between responsive
  I/O calls) are unchanged; blocking OS calls may exceed the latter and must
  be reported honestly with publication invalidated immediately. Cancelled
  outcomes take precedence over follow-up invalidation and are not implicitly
  revived; a new `Scan now` starts a fresh generation/chain.
- Crash/restart: crash before chunk staging leaves no partial Library rows;
  crash after some chunks leaves durable chunks + coverage but ineligible for
  apply until the same chain is requeued and re-driven to complete coverage;
  crash during final apply rolls back the whole publication. Restart reaps
  prior-session leases to `interrupted`, discards ineligible staging per the
  existing `discard_staging_for_run` path extended to chunks, and schedules at
  most one deduplicated recovery scan per enabled root without eligible work.
- Expiry: chunk coverage rows carry `completed_at_ms` and a run-level
  `coverage_ttl`; stale partial snapshots (e.g., run abandoned past TTL or
  lease deadline without renewal) are discarded by a bounded cleanup sweep,
  never published. Durable timestamps schedule retry eligibility after
  restart; they never resurrect an old lease.
- Cleanup: bound terminal-run history and staging cleanup (chunk rows
  included). Cleanup deletes only local tracked staging/coverage/history; it
  never deletes source files or historical missing locations. History
  retention is specified in the schema PR, independently of the existing
  diagnostic-log retention policy (1 MiB × 5 files, 14 days).
- Backup: backups contain a consistent database including chunk/coverage
  rows. Recovery invalidates in-flight leases and chunk staging/coverage
  before restarting work, then follows the normal requeue path. Recovery
  tests must assert byte-identical committed data/markers after restore.

### 2.7 Migration implications, implementation slices, adversarial tests

- Migration: one forward migration for coverage/staging/run columns with
  rollback-on-failure tests, backup/recovery fixtures, and privacy tests, per
  the per-slice Phase 2 rule. Existing v0/v1, synthetic-future-migration,
  kill-during-migration, and backup-recovery coverage is extended, not
  replaced.
- Proposed slices (each a separately reviewable PR; order matters):
  1. Schema + migration + retention/cleanup DDL with unit tests (no chunk
     execution yet).
  2. Chunk partitioner + coverage ledger writer behind the existing
     enumeration port (deterministic fake-port tests; no renderer command).
  3. Chunk staging path with per-chunk/per-run quota enforcement and
     fencing tests.
  4. Single final atomic publication from paged chunk reads with
     missing-file gating tests.
  5. Cancellation/lease-expiry/restart/backup integration with fake-clock
     and crash-fixture tests.
  6. Observability (opaque chunk counters, safe codes) and IPC read-model
     paging validation; no raw paths in diagnostics.
- Adversarial tests (minimum set before any qualification claim):
  cancel mid-chunk and mid-publication (both cancel/commit orderings via
  SQLite serialization); crash before staging, after some chunks, and during
  final apply; lease expiry mid-snapshot and stale-worker return; duplicate
  and out-of-order chunk delivery; quota exhaustion in a middle chunk;
  path-byte exhaustion; uncovered-subtree missing-file attempt (must not
  publish); policy-exclusion vs I/O-failure distinction; disable/remove
  mid-snapshot with fresh re-add identity; backup taken mid-snapshot then
  recovered; history/cleanup sweep with terminal-run bound; 100,000-entry
  synthetic run with Unicode/long-path/alias cases and source-byte
  preservation hashes unchanged.

### 2.8 Comparison: chunked snapshot vs coordinated quota increase vs deferral

| Option | Design/code cost | Memory/disk risk | Evidence needed | What remains unqualified if chosen |
| --- | --- | --- | --- | --- |
| **A. Bounded chunked snapshot (this proposal)** | New storage/protocol design + 6 slices + adversarial tests; no budget relaxation | Lowest: bounded chunk memory, paged plan/publication, enforced per-run disk caps | Schema/migration tests, chunk/coverage/fencing tests, single-publication atomicity tests, cancel/crash/restart/backup tests, then a measured 100,000-entry protocol run with phase-split timings | Nothing structural, but qualification still requires the measured 100k run plus F1/F2 closure |
| **B. Coordinated end-to-end quota increase** | Raise worker, staging, path-byte, plan-buffer, and publication/reconciliation limits together + update all coupled tests | Highest: 100,000 records in memory/disk at once; must prove private-memory, staged-DB, latency, and cleanup budgets anew | Same 100k protocol run plus private-bytes, staged-bytes, wall-time, and cleanup measurements at the new quota | Cheaper in design, but risks OOM/slow-publication and still needs full re-qualification; raising one constant alone is insufficient |
| **C. Explicit dated deferral** | No code; owner records a checkpoint date and keeps the current 10,000-entry contract | None added | None beyond recording the decision | The 100,000-entry target remains explicitly unqualified until that date; do not describe the provisional budget as qualified |

“Raise the constants” (B) is suitable only if the owner accepts a new
private-memory, disk, and latency budget and updates every coupled fence with
remeasurement. Deferral (C) is the lower-cost path if 100,000 entries are not
required for Scanner MVP acceptance.

### 2.9 Independent review — unresolved mechanisms (2026-09-09)

Status: **reviewer-owned corrections to this proposal; no implementation,
migration, quota, budget, or acceptance change.** Each item below names what
the design asserts, what mechanism is still missing, the concrete options,
and the validation required before any 100,000-entry qualification claim.
“Paged reads” is a read pattern, not a planning or fencing algorithm, and is
not accepted as evidence for any item until its mechanism lands with tests.

1. **Bounded global identity and reconciliation planning across chunks —
   unresolved.** Current `plan_observations`
   (`crates/storage-sqlite/src/publication.rs:893-1001`) loads the full
   previous committed set (`select_root_locations`) and the full staged set
   (`select_staged_observations`) into memory and builds global
   `identity_candidates` / `continuity_candidates` maps before any write.
   The advisory `PlanBuffer` (`crates/scan-execution/src/lib.rs:700-731`)
   truncates above 10,000 and returns no summary, but durable publication
   still plans the full set. This proposal’s “page through staging in bounded
   reads” does not show how the global decisions (unambiguous 1:1 rename
   reuse, hardlink-group shared assignment, conflict → fresh record, missing
   locations excluded from lookup) are reached without the global maps.
   Options: (a) two-pass paged planning — first pass builds a durable or
   bounded identity index (identity → occurrence count plus candidate set),
   second pass applies per page against that index; (b) temp-table planning
   inside SQLite — spill the identity index to a durable temp table and do
   set-based assignment in SQL; (c) chunk-local provisional assignment plus a
   cross-chunk merge pass that repairs split groups. Each option must state
   its peak-memory bound and its handling of the adversarial all-same-identity
   and all-distinct-identity extremes. Validation: rename source/target in
   different chunks, hardlink group split across three or more chunks,
   conflicting identities across chunks, plus peak private-bytes measurement
   at 100,000 entries against the 128 MiB budget. Slice 4 may not claim
   bounded planning until these tests pass with memory evidence.
2. **Complete coverage detection without treating partial traversal as
   absence — mechanism incomplete.** The ledger rule (only `complete` counts;
   any `failed`/`discarded` blocks publication; §2.5 gates `missing` on full
   coverage) is the correct safety direction and is retained. What is missing
   is how the expected chunk set is fixed before traversal discovers the tree
   dynamically. If chunks are cut by observation count during traversal, “all
   chunks complete” is tautological unless the total is independently known;
   directories created or deleted mid-scan, nested reparse fences, policy
   exclusions, and unhydrated placeholders further change the denominator.
   Options: (a) bounded metadata-only directory census first to fix
   `chunk_count` and coverage keys, then content enumeration per key;
   (b) hierarchical ledger (directory-level coverage rows rather than a fixed
   chunk count) with an explicit close-out rule; (c) single-generation cursor
   with abandonment — any interruption renders the run non-authoritative and
   re-enumerates from chunk boundaries (simplest, most rework on failure).
   Validation: quota exhaustion in a middle chunk, uncovered-subtree
   missing-file attempt (must not publish), policy-exclusion vs I/O-failure
   distinction, mid-scan directory create/delete, reparse-loop fence — each
   asserting rows plus the root success marker byte-identical and staging
   discarded.
3. **Hardlink, rename, and conflicting-identity handling across pages —
   unresolved.** The §2.5 “semantics unchanged” sentence states the
   requirement but supplies no cross-page mechanism. A rename whose source is
   in chunk 2 and target in chunk 47, a hardlink group spanning chunks, or
   conflicting prior associations in different chunks cannot be resolved by
   sequential per-page apply without either missing the match (minting fresh
   incorrectly) or picking by page order (forbidden by contract). Options: the
   same three as item 1 (global identity pass, temp-table assignment, or
   provisional-plus-merge), with the added constraint that chunk partitioning
   must not assume identity groups are chunk-local unless an
   identity-aware partitioner is itself specified and tested (which needs a
   pre-scan identity census and its own cost). Validation: multi-chunk rename
   / replacement / alias / conflict cases at 100,000-entry scale with Unicode,
   long-path, and alias members, asserting per-location presence, shared vs
   fresh `project_file_id` assignment, and unchanged source bytes.
4. **Lease, generation, and cancellation fencing during final publication —
   partially specified.** Per-chunk fencing (`generation`,
   `configuration_revision`, `lease_token`, deadline, session ID on every
   write; whole-run 30 s / 5 s renewal; disable/remove discards; restart reaps
   to `interrupted`; duplicate delivery fenced by
   `(run_id, chunk_index, lease_token)`) correctly extends the current
   contract and is retained. What is missing is fencing *inside* the single
   final transaction: lease renewal is a separate transaction and cannot run
   mid-apply, so a 100,000-row apply that exceeds the lease cannot be
   detected until commit; a cancellation or disable arriving mid-apply blocks
   on the `Mutex` plus SQLite `Immediate` lock until the publication commits,
   defeating the ≤ 250 ms UI acknowledgement and p95 ≤ 1 s cooperative-stop
   budgets. The §2.4 step-1 “no invalidating watcher event arrived” check
   also needs explicit wiring (which durable flag — `follow_up_requested`,
   coverage hint, or generation bump — and at what read point). Options:
   (a) measure first (10,000-row publication lock duration, then extrapolate
   with phase-split timings) and only then decide; (b) keep single-transaction
   atomicity and document the measured unresponsiveness as an accepted cost
   (requires owner sign-off, not silent); (c) WAL plus reader connections
   (separate review under the SQLite patch policy; not authorized here).
   Splitting the final apply into multiple committing transactions would break
   the atomicity contract and must not be presented as equivalent. Validation:
   cancel mid-publication in both commit orderings at scale, lease-expiry
   mid-snapshot with stale-worker return, disable/remove mid-snapshot with
   fresh re-add identity — each asserting no partial Library rows.
5. **Atomic publication lock duration and Library responsiveness under the
   existing connection and journal architecture — unmeasured.** The
   architecture is one `Database` connection behind a Tauri-host `Mutex`,
   `DELETE` rollback journal, `synchronous = FULL`, `busy_timeout` 2 s,
   `Immediate` transactions (`crates/storage-sqlite/src/lib.rs:73-95,220-230`;
   `DATA_MODEL.md`). Staging batches are correctly short (PR #77 pattern),
   but the final publication does selects plus planning plus N
   inserts/updates plus the missing sweep plus marker plus staging delete in
   one holding transaction. Paging the reads inside that transaction does not
   release the `Mutex` or the SQLite `RESERVED`/`EXCLUSIVE` lock, so Library
   pages, status queries, cancellation writes, and lease renewals all queue
   behind a 100,000-row apply while the `-journal` file (tens of MiB plus
   index churn) commits under `FULL` fsyncs. No lock-duration number exists
   for 10,000 rows, let alone 100,000. Options: (a) instrument now — report
   10,000-row publication wall, lock-hold time, concurrent page/status/cancel
   latency, staged-DB plus journal bytes; (b) dated deferral until that
   evidence plus a responsiveness budget exists; (c) WAL/multi-connection
   redesign as a separate proposal. Validation gate for any qualification run:
   enumerate + stage + final-apply split timings, concurrent read latency
   during apply, cancellation acknowledgement latency during apply, staged-DB
   bytes, journal peak bytes, and cleanup bytes reclaimed.
6. **Crash recovery, disk limits, cleanup, and backup consistency — structure
   correct, values and bounds missing.** Crash-before / crash-after-chunks /
   crash-during-apply rollback, restart reap to `interrupted` with
   `discard_staging_for_run` extended to chunks, one deduped recovery scan,
   TTL plus bounded sweep, backup invalidation before requeue — all correctly
   extend current semantics and are retained. Missing: `coverage_ttl` and
   history-retention values; boundedness of the cleanup sweep itself (a single
   `DELETE` of 100,000 staging plus coverage rows is another long lock);
   disk caps (`MAX_SNAPSHOT_STAGED_RECORDS ≥ 100,000` and a re-derived
   `MAX_SNAPSHOT_STAGED_PATH_BYTES` — the current 4 MiB is insufficient by
   construction at roughly 100 bytes per path); journal-peak disk during the
   final transaction; disk-full mid-chunk and mid-publication behavior;
   backup-taken-mid-snapshot consistency through the SQLite backup API on the
   single connection (blocks or snapshots uncommitted chunks?). Options: set
   caps from measured path-length distribution, batch the cleanup sweep with
   its own fencing, choose TTL in hours with owner approval, and specify
   backup concurrency explicitly. Validation: the §2.7 adversarial set plus
   disk-full injection at chunk staging and at final apply, cleanup-sweep
   bound test (row counts plus lock duration), and backup-mid-snapshot-then-
   recover asserting byte-identical committed data and markers with in-flight
   work `Interrupted` and staging `Discarded`.

## 3. #47/#48 prerequisites carried forward (no new survey)

- #47 (DriveFS): run once in **Mirror files** and once in **Stream files**,
  reading the active mode from Drive for Desktop Preferences. Use only the
  documented disposable leaves `G:\fruitboard-drivefs-<run-id>-enum`,
  `G:\fruitboard-drivefs-<run-id>-burst`, and
  `G:\fruitboard-drivefs-<run-id>-gap`. Required authorizations (not
  present): cloud-synchronized synthetic create/rename/delete and cleanup;
  full-resync authorization for mode switching; pause/disconnect/resume
  authorization for the gap case. No personal project contents may be
  traversed, opened, hashed, or parsed. PR #90 records the blockers; this
  proposal adds no consent and claims no qualification or exclusion.
- #48 (FAT32/cross-volume): disposable genuine FAT32 USB volume or dedicated
  VHD with a drive letter, alongside the existing disposable `C:` NTFS leaf.
  Record source/type, volume serial, filesystem, and allocation unit size;
  use `<LETTER>:\fruitboard-48-fat32-<run-id>` and authorize synthetic
  operations and cleanup on both volumes. Do not substitute the system FAT32
  partition, `G:` DriveFS mount, network share, or no-media device. PRs
  #88/#89 preserve the runbooks and the no-qualifying-target survey; this
  proposal adds no volume and claims no qualification or exclusion.

## 4. Unresolved choices (owner decisions required)

1. F1-A vs F1-B (fixture correction vs coordinated quota increase). This
   proposal recommends F1-A as narrowest but applies neither.
2. F3-A vs F3-B vs F3-C (chunked snapshot vs quota increase vs dated
   deferral). This proposal details F3-A but authorizes none. The §2.9 review
   adds that F3-A additionally requires a demonstrated bounded identity
   algorithm, a defined coverage-partition denominator, and measured
   publication lock-duration evidence before any qualification claim.
3. Chunk sizing (`MAX_CHUNK_OBSERVATIONS`), per-run staged-record cap, and
   per-run staged-path-byte cap for 100,000 entries (require measurement;
   not set here).
4. Coverage partition key shape (directory-shard ranges vs another opaque
   scheme) and coverage TTL/expiry values. Per §2.9 item 2, the denominator
   rule (census, hierarchical ledger, or abandon-and-reenumerate) is part of
   this decision, not just the key encoding.
5. Whether the P2-10 static no-parser guards plus fixture equality satisfy
   P2-10, or a runtime content-read spy is required (orthogonal to scale but
   gates P2-12).
6. F2 path: quiet-host ten-iteration rerun vs accepting #94’s candidate on
   contended evidence vs re-budgeting the 10 s target (requires idle-host
   isolation; #94 alone does not qualify; PR #98’s contended validation does
   not change this).
7. #47/#48: scoped runs with the authorizations above vs explicit owner scope
   exclusions (neither is selected here).
8. History retention values for chunk/coverage/terminal runs, independently
   of diagnostic-log retention.
9. Global identity planning mechanism (§2.9 item 1/3): two-pass index,
   temp-table assignment, or provisional-plus-merge — with peak-memory bound
   and cross-chunk adversarial tests.
10. Final-publication responsiveness (§2.9 item 4/5): accept a measured long
    lock as an explicit cost, defer, or propose WAL/multi-connection
    separately. Splitting the atomic transaction is not an equivalent option.
11. Disk and cleanup bounds (§2.9 item 6): per-run staged caps from measured
    path distribution, batched cleanup-sweep bound, TTL value, disk-full
    behavior, and backup-mid-snapshot concurrency rule.

## 5. Smallest practical next step (recommendation, not authorization)

1. Owner decides F1-A vs F1-B and records the exact sentence; if F1-A, run
   the fresh accepted-protocol ten-iteration run on `custom-9995` before any
   F3 work changes the worker.
2. Run the owner-coordinated quiet-host A/B for #94 vs its before-commit
   (same fixture, 1 warm-up + 10 measured each, all samples reported) to
   settle F2 measurement before any F3 re-budget.
3. If 100,000 entries remain required after (1)–(2), approve **slice 1 only**
   (coverage/staging DDL + migration/retention/cleanup tests) as the next
   bounded implementation step. Do not approve full chunked execution,
   publication, or 100k qualification until slice 1 lands and is reviewed.
   If 100,000 entries are not required for MVP, record an explicit dated
   deferral (option C) instead.

## 6. Provenance

- This proposal was written from merged `origin/main` `0b7612d` plus the
  unmerged heads cited in §0 (PRs #90 `17e5717`, #92 `49a5e649`, #93
  `59faefc`, #94 `588867b`, #95 `bf0aeac`) and their live CI records. It
  edits no benchmark raw JSON, installed run record, platform report, or
  production source.
- Independent review correction (2026-09-09): added the §0 sibling inputs
  (PR #97 `aba1812` with Foundation run `34339792661` and packaging run
  `34339792759` green; PR #98 `15b7f17` contended validation with separately
  dispatched CI), the §2.3/§2.4 paging caveats, the §2.9 unresolved-mechanism
  findings with options and validation requirements, and the §4 items 9–11.
  No migration, protocol implementation, quota, budget, acceptance, gate, or
  platform-scope change is made by this correction.
- Validation for this document is docs/policy checks only
  (`lint:docs`, `privacy:check`, `git diff --check`); no benchmark,
  installed-app, or platform run is claimed. This proposal’s own Foundation
  run `34339001103` and packaging run `34339001084` are green for the
  pre-correction head `3be0140`; the correction re-runs the same docs/policy
  checks before review.
- Handoff: after sibling fix PRs land, refresh the PR #91 reconciliation
  against the resulting merged `main` and sibling fix heads before any
  merge/acceptance review. Do not merge, close the epic, change budgets, or
  enable production scanning from this proposal.

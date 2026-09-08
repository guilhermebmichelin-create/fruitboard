# Benchmark triage — profiling notes (F2 handle / metadata / commit paths)

Date: 2026-09-08. Base: `main` `51f45af`. Branch: `chore/benchmark-triage-20260908`
(worktree `.tools/worktrees/bench-triage-20260908`).

Status: **static analysis only — no source code, budget, quota, fixture, or
workflow was changed.** All counts below are structural (derived from the
pinned source at `51f45af`); they are labelled as derived, not timed. The
benchmark driver used for the re-run (`crates/scan-execution/examples/benchmark.rs`)
emits only whole-scan `scan_ms`; there is **no enumerate-vs-stage-vs-commit
split in the driver output**, so per-phase shares below are hypotheses with
structural numbers, not measured splits. Instrumentation that would time the
split (extra driver phases, ETW/xperf, per-batch timers) was deliberately not
added: this task forbids source edits.

## 0. Which code path the driver actually exercises

- The driver (`benchmark.rs:301-302`) calls `worker.request_manual_scan` then
  `worker.poll`, and `poll` (`crates/scan-execution/src/lib.rs:394-406`) calls
  `self.execute` — the `&mut Database` variant, **not** PR #77's
  `execute_shared` (the `Mutex<Database>` host path used by the desktop
  scan-console host). The staged-batch transaction shape is the same family in
  both (short per-batch staging transactions + fenced atomic publication), but
  any `execute_shared`-specific mutex handoff cost does **not** appear in these
  measurements.
- Production path per run: `start_session` → `request_manual_scan` → `claim`
  (lease + `begin_scan_staging`) → `enumerate_into_run` through the real
  `WindowsFilesystemPort` → per-batch `stage_batch` → pre-publication fence →
  `change_plan` (advisory) → `publish_scan_run` (fenced atomic).

## 1. Staged-batch transaction shape (commit path, PR #62 + #77 family)

Authoritative bounds (all at `51f45af`):

- `crates/storage-sqlite/src/publication.rs:94`:
  `MAX_STAGED_BATCH_RECORDS = 512` per call.
- `crates/storage-sqlite/src/publication.rs:95`:
  `MAX_STAGED_RECORDS = 10_000` total.
- `crates/filesystem-enumeration/src/lib.rs:21-23`:
  `HARD_MAX_BATCH_RECORDS = 512`, `HARD_MAX_BATCH_BYTES = 256 MiB`.
- `crates/scan-execution/src/lib.rs:98-113` (`WorkerConfig::default`):
  `max_observations = MAX_STAGED_RECORDS` (10,000),
  `max_total_path_bytes = MAX_STAGED_PATH_BYTES`.
- `crates/storage-sqlite/src/lib.rs:92`: every writable connection sets
  `synchronous = FULL`.

Derived batch arithmetic for the quota-fitting 10k set (10,000 observations):

- `ceil(10,000 / 512) = 20` staged batches per authoritative run
  (19 × 512 + 1 × 272).
- Per batch, `StagingAdapter::stage_batch` (`lib.rs:911-947`) performs, in
  order: (a) a conditional lease renewal (`renew_scan_lease`, only when
  `now >= next_renewal_ms`, i.e. roughly every 5 s of scan time — so ~2-3
  renewals per 12 s run, not one per batch); (b) a fence re-read
  (`fence_violated`, `lib.rs:960-974`: one `scan_run` row read + one
  `scan_job` row read); (c) DTO conversion per record
  (`to_scan_observation` + `staged_observation`, in-memory); (d) one
  `stage_scan_observations` durable transaction.
- Per run commit-path transaction count (derived): ~20 staging transactions
  plus 1 atomic `publish_scan_run` transaction + ~40 fence row-reads + ~2-3
  lease renewals. Every write transaction runs under `synchronous = FULL`
  (fsync per commit on this host's NVMe volume).
- No `NORMAL`-vs-`FULL` comparison was run (that would be a source change).
  The fsync contribution is therefore an unquantified hypothesis: plausible
  (21 fsync-bearing commits per run) but with zero direct timing evidence in
  this wave — same standing as hypothesis (c) in the decision brief §3.2.

## 2. Handle / metadata path (enumeration, NTFS)

`crates/filesystem-enumeration/src/lib.rs` (Windows section, from `:2026`):

- Per **file**: `read_metadata` (`:2274-2283`) calls `validate_ancestors()`
  first, then one `open_relative` (`NtCreateFile`-family, `FILE_READ_ATTRIBUTES
  | SYNCHRONIZE`) plus one `read_handle_metadata` query. So each of the
  ~10,000 files costs ≥ 1 handle open + 1 metadata query, plus the ancestor
  re-validation described next.
- Per **directory open**: `open_directory` (`:2285-2321`) calls
  `validate_ancestors()`, then one `open_relative` + one metadata read + one
  `query_case_sensitivity`, and pushes a new `ValidationChain` link.
- **Ancestor validation chain** (`:2335-2354`): `validate_ancestors` walks the
  *entire* chain from the current directory to the root and, **per link**,
  performs one `open_relative` + one `read_handle_metadata` + identity
  comparison. It runs on **every** `read_metadata` and **every**
  `open_directory` call. Cost per file op is therefore proportional to its
  directory depth: a file at depth *d* costs ~*d* extra opens + *d* extra
  metadata queries on top of its own open.
- Fixture layout (from the re-run raw JSON counts: 1,000 leaf directories,
  1,025 directories total, 10 empty): most leaves sit at depth 2
  (`roots/shard-NNNN`, `unicode/shard-NNNN`), the long-chain branches at depth
  8 (`long/level-…×6/branch-NNNN`), the nested leaves at depth 5
  (`nested/level-1/level-2/level-3/leaf-NNNN`).
- Derived structural open count for the 10k set (estimate, labelled): ~10,000
  file opens + ~1,015 directory opens + ancestor re-validation ≈
  (10,000 files × mean depth ~2.2) + (1,015 dirs × mean depth ~2.1) ≈
  ~24,000 extra opens, i.e. **~35,000 handle opens and ~35,000 metadata
  queries per full traversal**, each carrying an `NtCreateFile` + query round
  trip through Defender's real-time filter on this host. This is the mechanism
  behind hypothesis (b); it is a derived count, not a timed split.

## 3. Advisory `change_plan` asymmetry (warm-up vs warm iterations)

`crates/scan-execution/src/lib.rs:773-806` (`change_plan`):

- Reference-bound guard (`:801-803`): when
  `previous.len() + buffer.observations.len() > 10,000`, the function returns
  `None` **before** calling `reconcile`.
- Warm-up (empty committed state): `previous` = 0 pages…​0 rows, 0 + 10,000 Direct comparisons against ≤ 10,000 → full `reconcile` over 10,000
  observations runs inside the measured window.
- Warm iterations (10,000 committed rows): `previous` pages the **entire**
  committed library at `MAX_LIBRARY_PAGE_SIZE = 200` → **50 `query_library`
  page reads**, then 10,000 + 10,000 = 20,000 > 10,000 → early `None`, no
  reconcile. So every warm iteration pays 50 paged read transactions and no
  reconcile; the warm-up pays 0 page reads and one full reconcile.
- The driver reports `changes_computed: false` on all 10k warm iterations
  (visible in the raw JSON `lines[].changes_computed`), consistent with the
  early-`None` path. The 50 page reads are indexed short transactions —
  expected small versus the ~12 s window, but they are the *only* systematic
  difference between the warm-up shape and the warm shape besides OS cache
  state, so they are recorded here rather than assumed negligible.

## 4. Top-3 profile hotspots (hypotheses with numbers, not guesses)

| Rank | Hotspot | Structural number (derived, not timed) | What would confirm it |
| --- | --- | --- | --- |
| H1 | Per-file handle-bound metadata + ancestor-chain re-validation | ~35k opens + ~35k metadata queries per 10k traversal (§2); each file op re-walks its full ancestor chain (`:2335-2354`) | Driver/emitted per-phase timers or ETW `NtCreateFile` counts per scan; **not** measured in this wave (no source edits) |
| H2 | Staged-batch commit shape under `synchronous = FULL` | 20 staging txns + 1 publication txn per run, each fsync-bearing (`storage-sqlite/src/lib.rs:92`); +50 `query_library` page reads per warm iteration (§3) | `NORMAL`-vs-`FULL` comparison run or per-batch commit timers; **not** run here (source change, forbidden) |
| H3 | Background I/O + AV contention on a shared multi-agent host | This host: DriveFS sync, Defender real-time, EpicGamesLauncher, T3/agent tooling, and 3+ concurrent `cargo`/`rustc` processes during triage (see rerun-report machine profile); original F2 shape (3 of first 4 iters 23.5–32.9 s, last 6 stable 11.7–12.9 s) fits transient contention | Idle-host re-run per owner sentence F2-rerun (DriveFS paused, AV exclusion, no agent load); **not** achievable in this session — sibling agents share the host |

## 5. Memory sampling method (working-set vs private bytes)

- The harness (`scripts/run-benchmark.mjs:214-307`) samples with `tasklist`
  (`workingSetKb`, locale-separator-stripped KB column) every ~100 ms and
  reports `idleMb` / `peakMb` / `incrementalMb`. **That column is the process
  working set, not private bytes and not incremental private memory.** The
  re-run report therefore labels these numbers "working-set samples" and does
  **not** present them as private-memory qualification — the `<= 128 MiB on
  the 100k set` budget stays unmeasured (F3).
- No private-bytes sampler (e.g. `Get-Process PrivateMemorySize64`,
  performance-counter commit charge) was added: it would be new harness code
  outside the documented methodology, and this task forbids source/workflow
  edits. A private-bytes qualification needs an owner-approved harness change
  first.

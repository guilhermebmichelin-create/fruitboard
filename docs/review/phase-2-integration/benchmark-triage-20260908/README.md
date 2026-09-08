# Benchmark triage 2026-09-08 — consolidated reconciliation (F1/F2/F3)

Status: **research + measurement triage only for P2-11 (#41).**
No budget, quota, fixture definition, acceptance promotion, or
production behavior is amended by this directory. Evidence class:
support for owner decisions F1/F2/F3 — not owner approval, and not
the idle-host isolation that owner sentence F2-rerun requires.

## Reconciliation note

This directory is the single consolidated triage artifact for
2026-09-08. It reconciles two independent sibling drafts written the
same day from the same pinned baseline `51f45af`:

- Base (this directory, branch `chore/benchmark-triage-20260908`,
  worktree `.tools/worktrees/bench-triage-20260908`): the re-run
  evidence set — `rerun-report.md` (new measurements),
  `profiling-notes.md` (static profile), and the two raw JSON files.
- Folded-in draft: `docs/review/phase-2-integration/benchmark-triage-2026-09-08.md`
  on branch `bench/triage-51f45af` (worktree
  `.tools/worktrees/bench-triage-51f45af`). That worktree was treated
  as READ-ONLY for the reconciliation and remains untouched; another
  lane owns its cleanup. Nothing was modified, committed, or deleted
  there.

Findings folded in from the sibling draft because they appear only
there (§1–§5 below):

1. Recomputed-stats verification of the original 2026-09-07 report.
2. The F1 mechanism citation chain and the "no authoritative claims"
   consequence.
3. The idle-host re-run readiness assessment (NOT READY at triage
   time) and the exact idle re-run protocol.
4. Cost-center citations not repeated in `profiling-notes.md`
   (`tests/no-parser.test.mjs:92-96` metadata-API pin,
   `DATA_MODEL.md:49` DELETE journal), the implications analysis of
   the original numbers, and the hypothesis ranking table.
5. Session-B machine/toolchain values not covered by the re-run
   report's §3 profile.

Deliberately NOT duplicated: the verbatim F1/F2/F3 owner-decision
sentences (F1-quota, F1-fixture, F2-budget, F2-host, F2-rerun,
F2-optimize, F3-100k) are byte-identical in both drafts and live in
`rerun-report.md` §7; this file does not repeat them. Machine-profile
rows that overlap the re-run report's §3 table are likewise not
repeated; only session-B-unique values are kept in §5.

Provenance of the folded-in draft (preserved verbatim from its
header):

- Baseline: `origin/main` `51f45af` (PR #84 merged). Verified in
  `C:\Users\artist\fruitboard` via `git fetch origin` +
  `git rev-parse HEAD` / `git rev-parse origin/main` (both `51f45af`).
- Worktree/branch of that draft: `bench/triage-51f45af` (worktree
  `.tools/worktrees/bench-triage-51f45af`), created from `51f45af`.
- Model constraint: that triage was performed under the owner rule
  "ONLY Muse Spark 1.3" (no other model used).
- Sources re-read: `benchmark-2026-09-07.md`,
  `benchmark-2026-09-07-baseline-raw.json`,
  `benchmark-2026-09-07-quota-raw.json`,
  `budget-decision-brief.md` §3–§5,
  `acceptance-prep-2026-09-08.md` P2-11 row + owner question list.
- Triage order per the brief: **D first, then C, then A/B**.
  D confirms F1 blocks authoritative claims; C isolates contention
  with an idle re-run; A/B present clean data and static profiling
  without amending anything.

## Files in this directory

| File | Content |
| --- | --- |
| `rerun-report.md` | New 2026-09-08 re-run measurements at `51f45af` (Run A reproduced, Run B re-measured), findings triage, verbatim owner sentences (§7), limitations, reproducibility |
| `profiling-notes.md` | Static profile of the handle / metadata / commit paths (derived structural counts; no timed splits) |
| `rerun-baseline-raw.json` | Raw Run A data — accepted baseline preset, 10/10 `Failed / ResourceLimit` |
| `rerun-quota-raw.json` | Raw Run B data — quota-fitting 10k set, median 10,978 ms / p95 14,267 ms |
| `README.md` | This consolidated reconciliation (folded-in findings from the sibling draft) |

## 1. Recomputed stats verification (original 2026-09-07 report)

Recomputation was run from the repository root against the committed
raw JSON files (no filtering, no subset selection):

```text
node -e "<read quota-raw iterations[].scanMs, sort, median/max/nearest-rank-p95>"
node -e "<read baseline-raw iterations[] status/outcome>"
```

### Run B — quota-fitting fixture (10,000 observations)

Fixture: `custom-9995`, seed `0`, manifest
`a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a`
(9,995 FLP files + 5 hardlink aliases = 10,000 observations,
`hardlinkSupport: created`).

| Check | Value |
| --- | --- |
| Sorted scan windows (ms) | 11686, 11911, 12294, 12710, 12833, 12918, 12924, 23528, 31304, 32876 |
| Median (n = 10) | 12875.5 ms (~12.9 s) |
| Max | 32876 ms (~32.9 s) |
| Nearest-rank p95 (ceil(0.95 × 10) = 10th value) | 32876 ms (= max by construction) |
| Warm-up first discovery | 11886 ms, Published / Complete / 10,000 locations |
| Report `statistics` block | `medianMs 12875.5, maxMs 32876, nearestRankP95Ms 32876, sampleCount 10` |
| Verdict | **matches the report exactly** |

Derived (not a re-budget, for triage context only):

| Slice | Value |
| --- | --- |
| Stable band, iterations 5–10 sorted (ms) | 11686, 11911, 12294, 12833, 12918, 12924 |
| Stable-band median | 12563.5 ms (25.6% over the 10 s target) |
| Stable-band range | 11686–12924 ms (+16.9% to +29.2% over target) |
| Overall median margin | +28.8% over target |
| Throughput at overall median | ~777 locations/s |
| Throughput at fastest stable (11686 ms) | ~856 locations/s |
| Throughput at slowest spike (32876 ms) | ~304 locations/s |
| `sampleCount` per iteration (memory sampler) | 237, 117, 215, 267, 121, 111, 113, 121, 121, 112 |
| Slow iterations (1, 3, 4) sampleCounts | 237 / 215 / 267 (longer windows, same code path) |
| Stable iterations sampleCounts | 111–121 |

### Run A — accepted baseline preset (finding confirmation)

Fixture: `baseline`, seed `0`, manifest
`8c3d85ec01299995208abfa450378b1b37c704e423593d7a4370ec465254afba`
(10,000 FLP files + 5 hardlink aliases = 10,005 observations,
`hardlinkSupport: created`).

| Check | Value |
| --- | --- |
| All 10 measured iterations | `Failed / ResourceLimit` (10 of 10) |
| Warm-up | `Failed / ResourceLimit`, 10838 ms |
| Sorted time-to-quota-failure (ms) | 10533, 10737, 11071, 11264, 11638, 12331, 12545, 13843, 17391, 21845 |
| Report `statistics` | `null` (no authoritative iterations) |
| Budgets `first-discovery` / `warm-p95` | `measured: null, pass: false` with "no authoritative" notes |
| Verdict | **matches the report exactly** |

### Cancellation and memory (original runs)

| Check | Value |
| --- | --- |
| Quota-run stop latencies (3 samples) | 6, 8, 6 ms (max 8 ms, p95 8 ms) |
| Baseline-run stop latencies (3 samples) | 6, 6, 6 ms (max 6 ms) |
| Cooperative stop budget (p95 <= 1 s) | PASS with wide margin (unchanged) |
| Incremental memory, quota run | 53.2–53.3 MiB every iteration; warm-up 53.0 MiB |
| Incremental memory, baseline run | 52.2–53.4 MiB; warm-up 53.0 MiB |
| Verdict | **matches the report exactly; ~53 MiB is a 10k-set working-set figure only (see F3)** |

## 2. D — F1 blocks authoritative claims (confirmed)

Mechanism (read-only citations, no code change):

- `crates/storage-sqlite/src/publication.rs:95`:
  `MAX_STAGED_RECORDS: i64 = 10_000`.
- `crates/scan-execution/src/lib.rs:107`:
  `WorkerConfig::default` sets
  `max_observations = MAX_STAGED_RECORDS as usize` (10,000).
- Generator output for the accepted `baseline` preset is 10,000
  FLP files **plus** 5 hardlink aliases = **10,005 observations**,
  each alias a separate location (see the raw `fixture.counts`:
  `flpFiles 10000, aliasLocations 5`).
- 10,005 > 10,000, so every Run A traversal hits the enumeration
  resource limit before any staging can publish. The raw data shows
  11 of 11 baseline runs ending `Failed / ResourceLimit` with
  `locationCount: null`.

Consequence: **no authoritative latency, memory, or publication
claim can be made on the accepted fixture.** Run B (quota-fitting
`custom-9995`, exactly 10,000 observations) is a comparison fixture
only; it does not replace the accepted baseline. Any statement of
the form "baseline first discovery passes" must carry the fixture
caveat (Run A not evaluable; Run B warm-up 11886 ms PASS on the
comparison set). This matches the brief §3.3 and the report F1.

No quota or fixture amendment is made here. The owner picks one of
`F1-quota` / `F1-fixture` (see `rerun-report.md` §7).

## 3. C — idle-host re-run protocol and readiness

### 3.1 Readiness: NOT READY on this host at triage time

Observed during the sibling triage session (measurement preconditions
from `benchmark-2026-09-07.md` §Limitations and the brief §5
`F2-rerun` sentence):

| Precondition (F2-rerun sentence) | Observed | Ready |
| --- | --- | --- |
| DriveFS paused | `GoogleDriveFS` running (2 processes, PIDs 8104/10472) | NO |
| AV exclusion for the fixture volume | `MsMpEng` (Defender) running, working set ~783 MB | NO |
| No agent load | Two `opencode` processes active (~584 MB + ~627 MB); sibling worktrees present (`docs/84-postmerge-status`, `research/platform-47-48-51f45af`, journey/wave worktrees) | NO |
| AC + high-performance | `powercfg /getactivescheme`: `Alto desempenho` (High performance) | YES |
| Documented cache state | No cache control performed in this session | PENDING |
| 1 warm-up + 10 measured, same seeds/hashes | Protocol defined, not executed | PENDING |

Because three of the four isolation preconditions fail, **no new
timing numbers were claimed by that session.** Running the benchmark
then would have reproduced the same contended conditions as
2026-09-07 and could interleave with sibling agents' work. This is
reported honestly rather than "fixed" by running anyway.

(Note added during reconciliation: this worktree's own re-run in
`rerun-report.md` is likewise **not** the idle-host isolation — it
isolates concurrent Cargo builds only, not DriveFS/AV/agent load.
F2-rerun conditions remain unmet; see `rerun-report.md` §3.)

### 3.2 Exact idle re-run protocol (ready to execute once idle)

Prerequisites (owner or operator action; none taken here):

1. Pause DriveFS sync (or stop the DriveFS service) for the run
   window; record paused/stopped state and version.
2. Add a Defender exclusion for the fixture temp volume (or record
   why not); record exclusion state.
3. Quiesce agent load: no concurrent benchmark, test, build, or
   journey runs on the host; record `Get-Process` snapshot.
4. Confirm AC power + `Alto desempenho` via
   `powercfg /getactivescheme`; record output.
5. Document OS cache state explicitly (e.g. fresh boot + single
   warm-up, or warm cache; never label the first run "cold" unless
   cache state was controlled).

Commands (from the pinned baseline `51f45af`, pinned toolchain):

```text
node scripts/run-benchmark.mjs --size baseline --seed 0 --out <report-a>.json
node scripts/run-benchmark.mjs --size custom --files 9995 --seed 0 --out <report-b>.json
```

Requirements for the re-run report:

- Same seeds (`0`) and same expected manifest hashes
  (`8c3d85ec…54afba` baseline, `a4760a28…6d08a` quota-fitting);
  regenerate rather than hand-edit; a hash mismatch invalidates
  the run.
- 1 warm-up + 10 measured iterations, fresh process per iteration,
  one committed database (harness default); 3 mid-run
  cancellations; ~100 ms working-set sampling.
- Report median / max / nearest-rank p95 **unfiltered** (all 10
  iterations; do not drop spikes), first run separately, and the
  full budget table with the same caveats.
- Record machine profile, storage kind, power mode, commit,
  toolchain versions, DriveFS/AV/agent-load state, and cache
  state in the write-up.

Expected decision value: if the stable band drops under 10 s on an
idle host, contention (hypothesis a) was decisive; if the stable
band still misses by a similar margin, structural per-file cost
(hypothesis b) dominates and the choice narrows to re-budget vs
optimize (owner sentences `F2-budget` / `F2-host` / `F2-optimize`).

## 4. B — static profile cross-references, implications, and hypothesis ranking

The full static profile of this worktree's own analysis is in
`profiling-notes.md` (driver path, staged-batch transaction shape,
handle/metadata path, `change_plan` asymmetry, hotspots H1–H3). The
cost-center table below is preserved from the sibling draft because
it carries citations that `profiling-notes.md` does not repeat:

### 4.1 Cost centers on the measured path

| # | Stage | Mechanism (file:line) | Why it costs on a 10k-file / ~1,025-dir fixture |
| --- | --- | --- | --- |
| 1 | Per-file handle-bound metadata opens | `crates/filesystem-enumeration/README.md:44-54`: child metadata via `NtCreateFile` relative to the parent handle (`FILE_OPEN_REPARSE_POINT`, attribute access only, never content); `tests/no-parser.test.mjs:92-96` pins `CreateFileW`/`NtCreateFile` as the only metadata APIs | One `NtCreateFile` open per entry plus the directory listing query (`NtQueryDirectoryFile`) per directory; ~10k opens per run even when warm |
| 2 | Ancestor validation chain | `crates/filesystem-enumeration/README.md:76-83`: each child cursor retains a bounded chain of parent handles + expected identities, revalidated before metadata, child-open, and each directory-query operation | Per-directory revalidation work that scales with depth × entries; safety containment, not removable without a design change |
| 3 | Per-batch staging transactions | `crates/scan-execution/src/lib.rs:910-947` (`StagingAdapter::stage_batch`): per-batch fence revalidation + `stage_scan_observations`; `crates/filesystem-enumeration/src/lib.rs:21`: `HARD_MAX_BATCH_RECORDS = 512` | ceil(10000 / 512) = **20 staging transactions** per authoritative 10k run, each revalidating lease/session/revision/cancellation inside storage |
| 4 | Synchronous commit cost | `crates/storage-sqlite/src/lib.rs:92`: `synchronous FULL` on every writable connection; `DATA_MODEL.md:49` confirms DELETE journal + `synchronous=FULL` | Every one of the ~20 staging transactions plus the final `publish_scan_run` pays FULL durability; multiplies any per-commit fsync cost by ~21 |
| 5 | Post-traversal advisory plan | `crates/scan-execution/src/lib.rs:773-806` (`change_plan`): paged `query_library` (200/page) + in-memory `reconcile` over up to 10k observations | Omitted by the reference-bound guard at 10k locations in the measured runs (`changes_computed: false` on iterations 1–10, `true` only on warm-up); negligible in the measured warm-p95 numbers |

The benchmark driver (`crates/scan-execution/examples/benchmark.rs`)
drives the full production path (durable `Database`,
`ScanWorker` with `WorkerConfig::default`, `start_session` /
`request_manual_scan` / `poll` through the real
`WindowsFilesystemPort` to fenced atomic publication) and emits a
single `scan_ms` per run (from `scan_started` to `scan_finished`).
It has **no enumerate-vs-stage-vs-commit timer split**. Adding one
would change `crates/**` behavior and therefore requires the owner
`F2-optimize` sentence first; no such instrumentation is added or
claimed here. The split above is a static, citation-backed
decomposition plus derived batch arithmetic from the measured
totals.

### 4.2 What the original 2026-09-07 numbers imply about the split

- Total work per authoritative run is fixed at 10,000 committed
  locations; the only per-run variance is wall time (11.7–32.9 s),
  so variance lives in I/O + commit latency, not in a different
  code path (`sampleCount` tracks window length: 237/215/267 on
  the three slow iterations vs 111–121 stable).
- The warm-up (11.9 s, first discovery on empty state) sits inside
  the later stable band, so the early spike (iterations 1–4:
  32876, 12710, 23528, 31304) is not a first-run effect; it fits
  transient contention over a freshly generated fixture (DriveFS
  indexing + Defender + agent load documented in the report
  machine profile).
- The stable band floor (~11.7 s, ~856 locations/s) still exceeds
  the 10 s budget by ~17–29%; contention alone cannot explain the
  floor, so a structural per-file + per-batch cost remains after
  contention fades.

(Reconciliation cross-reference: the 2026-09-08 re-run in
`rerun-report.md` §5 sharpens this reading — with zero concurrent
Cargo builds the early 23–33 s spikes did not recur, the warm band
moved to 10.8–11.3 s with one 14.3 s outlier, and the ~11.0 s floor
still exceeds the provisional 10 s target. Contention drives the
variance; a structural floor remains.)

### 4.3 Hypothesis ranking (unchanged order, hardened evidence)

| Rank | Hypothesis | Evidence for | Evidence against / missing |
| --- | --- | --- | --- |
| (a) — first | Background contention | Timing shape (early 23.5–32.9 s collapsing to 11.7–12.9 s); documented load (DriveFS, Defender, agent tooling); fresh fixture indexing plausibly hits early iterations; the sibling triage re-confirmed the same load still present (DriveFS ×2, MsMpEng ~783 MB, opencode ×2) | Not isolated: no idle re-run yet (§3), no per-phase timers |
| (b) — second | Per-file handle + ancestor-validation cost | Stable floor 11.7–12.9 s still misses by ~17–29% after contention fades; ~10k `NtCreateFile` opens + per-directory chain revalidation per run (§4.1 rows 1–2); ~777–856 locations/s ceiling on laptop-class i7-10750H | No instrumented enumerate-vs-rest split (requires `F2-optimize` code change) |
| (c) — third | `synchronous = FULL` per-batch commit | Mechanism confirmed (`storage-sqlite/src/lib.rs:92`); ~20 staging transactions + 1 publication per run (§4.1 rows 3–4) multiply any per-commit cost | Zero direct evidence: no NORMAL-vs-FULL comparison, no commit timing; changing it weakens durability and needs owner approval |

Recommendation (no amendment made): **D first** (F1 contradiction
blocks every authoritative claim), **then C** (idle re-run is the
cheapest isolator for hypothesis a), **then** decide A vs B on
clean data. Do not re-budget on contended numbers.

## 5. A — owner sentences (see `rerun-report.md` §7) and session-B context

The exact owner sentences needed (quoted verbatim from
`budget-decision-brief.md` §5 — F1-quota, F1-fixture, F2-budget,
F2-host, F2-rerun, F2-optimize, F3-100k) are identical in both
drafts and are preserved in `rerun-report.md` §7; each takes effect
only as an owner post, and nothing here amends anything. Pick one of
each F1/F2 alternative pair.

F3 note for the owner: the 100,000-entry set (100,000 FLP files +
aliases) exceeds the 10,000-record quota by an order of magnitude,
so the `<= 128 MiB on the 100k set` budget is unmeasurable today;
~53 MiB is the 10k-set tasklist working-set figure only.

Session-B machine/toolchain values not covered by the re-run
report's §3 profile (no new measurements were taken in that
session; this is context for the readiness assessment, not
benchmark evidence):

| Field | Value |
| --- | --- |
| CPU | Intel(R) Core(TM) i7-10750H @ 2.60 GHz, 12 logical (6P/12T) |
| RAM | 16 GiB (17,041,244,160 bytes) |
| OS | Windows 11, build 10.0.26200, x64 |
| Rust build strings | 1.98.1 via `.tools` (`cargo 1.98.1 (797e8a9bc 2026-08-05)`, `rustc 1.98.1 (48a229cea 2026-09-01)`); `rust-toolchain.toml` channel `1.98.1` |
| Node drift | Shell `v26.4.0` vs pinned `24.20.0` (same drift as brief §2); `tools/toolchain-policy.json` pins Node 24.20.0, Rust 1.98.1 |
| Fixture seeds/hashes referenced | baseline seed `0` / `8c3d85ec01299995208abfa450378b1b37c704e423593d7a4370ec465254afba`; quota-fitting seed `0` files `9995` / `a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a` |

(Power plan, background load, and DriveFS/AV/agent-load details for
that session are in §3.1; commit `51f45af`.)

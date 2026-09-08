# Benchmark triage re-run report — 2026-09-08 (F1/F2/F3 evidence, no decisions)

Status: **research + measurement, evidence draft only, for P2-11 (#41).**
This report re-runs the documented methodology
(`scripts/benchmark-scan.md` + `scripts/run-benchmark.mjs` + driver
`crates/scan-execution/examples/benchmark.rs`) from the pinned baseline to
triage open findings F1/F2/F3. It amends nothing: no budget, quota, fixture,
acceptance ID, workflow, or source change was made here. The verbatim owner
sentences still required are quoted unchanged in §7.

New files only (this task touched nothing else):

- `docs/review/phase-2-integration/benchmark-triage-20260908/rerun-report.md`
  (this file)
- `docs/review/phase-2-integration/benchmark-triage-20260908/profiling-notes.md`
  (handle / metadata / commit-path static profile)
- `docs/review/phase-2-integration/benchmark-triage-20260908/rerun-baseline-raw.json`
- `docs/review/phase-2-integration/benchmark-triage-20260908/rerun-quota-raw.json`

Worktree/branch: `chore/benchmark-triage-20260908` in
`.tools/worktrees/bench-triage-20260908`, created from `main` at `51f45af`
and still at `51f45af` (`git status`: only this new directory, untracked).
The original report, the decision brief, budgets, source, and workflows are
untouched. A sibling triage worktree (`.tools/worktrees/bench-triage-51f45af`,
branch `bench/triage-51f45af`) exists and was not read or modified for this
report; all numbers below are from this worktree's own runs.

## 1. Build + toolchain provenance

- git HEAD: `51f45af8501a06b2025f88f88660760090600e6c` (branch
  `chore/benchmark-triage-20260908`, clean apart from this directory).
  The raw JSONs record `"commit": "51f45af"`.
- Toolchain: `cargo 1.98.1`, `rustc 1.98.1` (pinned via `rust-toolchain.toml`,
  matches `tools/toolchain-policy.json` rust `1.98.1`).
- Node (harness only, not in any scan window): shell `v26.4.0` vs pinned
  `24.20.0` (`.node-version`) — same drift the decision brief §2.1 already
  records; the harness only spawns the driver and parses JSON, it does not
  affect `scan_ms`.
- `uv`: not installed on this host (`where.exe uv.exe` finds nothing;
  policy wants `0.12.9`) — recorded, irrelevant to the Rust benchmark.
- Pinned release build in this worktree (cold `target/`, fresh worktree):
  `cargo build --release -p fruitboard-scan-execution --example benchmark --locked`
  wall time **50,405 ms**, exit success, `target/release/examples/benchmark.exe`
  present. The harness then rebuilt incrementally before each run, timed
  separately and never inside a scan window: `buildMs` 446 (baseline run)
  and 314 (quota run), both recorded in the raw JSON `build` blocks with the
  same pinned command and toolchain string as the original report.
- Cargo serialization: sibling agents share this host and were compiling
  during triage (observed 3–14 concurrent `cargo`/`rustc` processes at
  17:29–17:33 local). This task waited with backoff until **zero**
  `cargo`/`rustc` processes remained before building, and re-checked before
  each harness run (baseline run started with 3 concurrent cargo-family
  processes present; quota run started with 0). No `--bin` override was used;
  no build lock error occurred.

## 2. Exact driver commands

```text
node scripts/run-benchmark.mjs --size baseline --seed 0 --out docs/review/phase-2-integration/benchmark-triage-20260908/rerun-baseline-raw.json
node scripts/run-benchmark.mjs --size custom --files 9995 --seed 0 --out docs/review/phase-2-integration/benchmark-triage-20260908/rerun-quota-raw.json
```

Run from the triage worktree root (so `--out` lands in the new directory).
Protocol per run (unchanged): generate disposable fixture into `os.tmpdir`,
1 warm-up + 10 measured iterations (fresh driver process per iteration, one
shared committed database), 3 mid-run cooperative cancellations on fresh
databases, ~100 ms `tasklist` working-set sampling, fixture temp dir removed
afterwards (both runs print "temporary fixture removed after the run").
No private files, no credentials, no personal paths; seeds and manifest
hashes only.

## 3. Machine profile (same laptop-class host as the original report)

| Field | Value |
| --- | --- |
| CPU model | Intel(R) Core(TM) i7-10750H CPU @ 2.60GHz (laptop-class, 2019) |
| Logical cores | 12 (6 physical / 12 threads) |
| RAM | 17,041,244,160 bytes (16 GiB) |
| OS | Windows 11 Home Single Language, build 10.0.26200.9168, x64 |
| Power plan | High performance (`Alto desempenho`); battery reports 100%; host assumed on AC power (same caveat as the original report) |
| Storage | Local NVMe SSD (same volume as temp fixture), 510.6 GB total, ~85.7 GB free (baseline run) / ~85.5 GB free (quota run); original report saw ~103 GB free — disk filled since, recorded |
| Background load | **Not idle.** Top-CPU snapshot during triage: T3 Code agent host, GoogleDriveFS sync service, EpicGamesLauncher, CrossDeviceService, node, `t3-resource-monitor`, plus sibling fruitboard agents in ~14 sibling worktrees; 3 concurrent cargo-family processes at baseline-run start, 0 at quota-run start. DriveFS was **not** paused, no AV exclusion was added, agent load was **not** stopped |

Isolation verdict: this is a **serialized-build, same-host, same-methodology
re-run** — it is *not* the idle-host isolation the owner sentence F2-rerun
requires (DriveFS paused, AV exclusion, no agent load, documented cache
state). Its value is narrower: identical fixtures (same hashes), pinned
commit one step forward (`2a8bdcf` → `51f45af`, docs-only delta), and one
run with zero concurrent builds. Cache state was not controlled; the first
run is reported separately and is not labelled cold-cache.

## 4. Fixtures (identical to the original report, byte-for-byte by hash)

| Fixture | sizeLabel | Seed | Manifest SHA-256 | Counts |
| --- | --- | --- | --- | --- |
| Accepted baseline preset | `baseline` | `0` | `8c3d85ec01299995208abfa450378b1b37c704e423593d7a4370ec465254afba` (matches original exactly) | 10,000 `.flp` files + 4 other + **5 hardlink aliases** = **10,005 observations**; 1,000 leaves / 1,025 dirs / 10 empty / 5 hl groups; `hardlinkSupport: created` |
| Quota-fitting variant | `custom-9995` | `0` | `a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a` (matches original exactly) | 9,995 `.flp` files + 4 other + 5 aliases = **10,000 observations**; same layout; `created` |

## 5. Results

### Run A — accepted baseline preset (F1 reproduction): REPRODUCED

1 warm-up + 10 measured; **all 11 runs end `Failed / ResourceLimit`**,
`locationCount` null, `changes_computed` false, exit code 1:

| Iteration | Scan window (ms) | Status | Outcome |
| --- | --- | --- | --- |
| warm-up | 14,735 | Failed | ResourceLimit |
| 1 | 10,833 | Failed | ResourceLimit |
| 2 | 10,410 | Failed | ResourceLimit |
| 3 | 11,259 | Failed | ResourceLimit |
| 4 | 16,994 | Failed | ResourceLimit |
| 5 | 10,542 | Failed | ResourceLimit |
| 6 | 10,488 | Failed | ResourceLimit |
| 7 | 10,462 | Failed | ResourceLimit |
| 8 | 11,874 | Failed | ResourceLimit |
| 9 | 17,344 | Failed | ResourceLimit |
| 10 | 10,514 | Failed | ResourceLimit |

F1 reproduced exactly: 10,005 observations vs `MAX_STAGED_RECORDS = 10,000`
(`WorkerConfig::default` mirrors it as `max_observations`), so the run stops
at the enumeration resource limit, staging is discarded, nothing is
published. The harness budget block is `n/a` for both latency budgets with
the note "no authoritative iterations (10 of 10 non-authoritative)". Even the
time-to-quota-failure shows contention spikes (two iterations at ~17 s vs a
~10.4–11.9 s band), consistent with the loaded host.

### Run B — quota-fitting fixture, 10,000 observations (F2 re-measurement)

1 warm-up + 10 measured; **all 11 runs `Published / Complete`**, 10,000
committed locations each, exit code 0:

| Iteration | Scan window (ms) | Status | Outcome | Locations | changes_computed |
| --- | --- | --- | --- | --- | --- |
| warm-up (first discovery) | 10,846 | Published | Complete | 10,000 | true |
| 1 | 11,340 | Published | Complete | 10,000 | false |
| 2 | 11,143 | Published | Complete | 10,000 | false |
| 3 | 10,965 | Published | Complete | 10,000 | false |
| 4 | 10,970 | Published | Complete | 10,000 | false |
| 5 | 10,887 | Published | Complete | 10,000 | false |
| 6 | 10,947 | Published | Complete | 10,000 | false |
| 7 | 11,158 | Published | Complete | 10,000 | false |
| 8 | 10,839 | Published | Complete | 10,000 | false |
| 9 | 10,986 | Published | Complete | 10,000 | false |
| 10 | 14,267 | Published | Complete | 10,000 | false |

Sorted: 10839, 10887, 10947, 10965, 10970, 10986, 11143, 11158, 11340, 14267.
Statistics (n = 10): **median 10,978 ms, max 14,267 ms, nearest-rank p95
14,267 ms** (nearest-rank p95 = max by construction at n = 10).
First discovery 10,846 ms → provisional `<= 30 s` **PASS**.
Warm p95 14,267 ms vs provisional `<= 10 s` → **FAIL (F2 stands)**,
median 10,978 ms (9.8% over budget).

Comparison with the original (same hashes, same host class):

| Metric | 2026-09-07 (commit `2a8bdcf`) | 2026-09-08 re-run (commit `51f45af`) |
| --- | --- | --- |
| First discovery | 11,886 ms | 10,846 ms (−9%) |
| Warm median | 12,875.5 ms | 10,978 ms (−15%) |
| Warm p95 (max) | 32,876 ms | 14,267 ms (−57%) |
| Shape | 3 of first 4 at 23.5–32.9 s, last 6 stable 11.7–12.9 s | 9 of 10 in 10.8–11.3 s, one 14.3 s outlier (iteration 10) |
| Concurrent builds at start | not recorded (heavy DriveFS/AV/agent load documented) | 0 cargo-family processes |

Reading (evidence only, no re-budget): removing concurrent Cargo builds
collapsed the early-run 23–33 s spikes but the floor barely moved — the
stable band sits at ~11.0 s, still ~1.0 s over the provisional 10 s target.
That is consistent with the brief's ranking: contention (a) drives the
*variance*, while the per-file handle/metadata cost (b) sets a structural
*floor* above budget on this laptop. Neither claim is isolated-proof on this
host; the idle-host re-run (F2-rerun) is still outstanding.

### Cancellation

Cooperative stop latency (mid-run token, `cancellation_requested` →
terminal `Cancelled`):

| Run | Stop latency (ms) |
| --- | --- |
| A-1 | 6 |
| A-2 | 6 |
| A-3 | 5 |
| B-1 | 14 |
| B-2 | 19 |
| B-3 | 22 |

Worker stop p95 (nearest-rank over the 3+3 samples: max 22 ms) passes the
`<= 1 s` budget with a very wide margin, as before. The B-run latencies
(14–22 ms) are higher than the original 6–8 ms band; no cause is assigned
(background load was not sampled per-iteration). The 250 ms
UI-acknowledgement budget does not bind the worker (no UI in scope).

### Working memory — working-set samples only, NOT private-memory qualification

`tasklist` working-set sampling, ~100 ms interval (same method):

| Run | Idle (MiB) | Peak (MiB) | Incremental (MiB) |
| --- | --- | --- | --- |
| A warm-up | 6.4 | 59.8 | 53.5 |
| A measured (min..max) | 5.5 | 58.7–58.8 | 53.2–53.3 |
| B warm-up | 5.9 | 58.9 | 53.0 |
| B measured (min..max) | 5.5 (one 7.8) | 58.7–58.8 | 53.2–53.3 (one 50.9, same run as the 7.8 idle) |

Peak incremental working set ≈ **53 MiB** on the 10k set — numerically
identical to the original report. This is a **working-set** figure from a
locale-formatted `tasklist` column; it is **not** incremental private bytes
and is **not** presented as qualification against the `<= 128 MiB on the
100k set` budget (see F3). No private-bytes sampler was added (would be new
harness code; out of scope for this task).

## 6. Findings triage

- **F1 — REPRODUCED, unchanged.** Accepted `baseline` preset yields 10,005
  observations (10,000 files + 5 aliases, hash `8c3d85ec…`) vs
  `MAX_STAGED_RECORDS = 10,000`; 11/11 runs `Failed / ResourceLimit`, zero
  locations published. Requires one of the two verbatim owner sentences
  below (F1-quota or F1-fixture). Nothing was raised or redefined here.
- **F2 — STILL FAILING, variance reduced but floor above budget.**
  Quota-fitting 10k set (hash `a4760a28…`): first discovery 10,846 ms PASS;
  warm median 10,978 ms / p95 14,267 ms vs provisional 10 s → FAIL. The
  23–33 s early spikes from 2026-09-07 did not recur with zero concurrent
  builds, but 10/10 warm iterations still exceed 10 s (lowest 10,839 ms).
  Static profile of where time structurally goes is in `profiling-notes.md`
  (top-3: per-file handle opens + ancestor-chain re-validation ≈ 35k
  opens/queries per traversal; 20 staged `synchronous=FULL` transactions + 1
  publication + 50 advisory page reads per warm iteration; background
  contention). Per-phase timed splits do not exist in the driver output and
  were not instrumented (source edits forbidden). Requires an owner sentence
  (F2-budget, F2-host, F2-rerun, and/or F2-optimize). No re-budget, no
  optimization, no filtering of iterations was performed here.
- **F3 — NOT MEASURABLE, reported as unmeasured.** The 100,000-entry
  qualification set cannot complete under the current quota as a matter of
  code structure: `WorkerConfig::default` sets `max_observations =
  MAX_STAGED_RECORDS` (10,000; `crates/scan-execution/src/lib.rs:107`), so a
  100k traversal ends `ResourceLimit` before staging, exactly as F1
  demonstrates at 10,005. The quota was **not** raised to force it through
  (forbidden by this task), no 100k fixture was generated, and the ~53 MiB
  working-set figure above remains a 10k-set sample, not a 100k
  private-memory qualification. Requires the verbatim F3-100k owner sentence.

## 7. Verbatim owner-decision sentences still required (quoted from budget-decision-brief.md §5 — each amends the plan only when posted by the owner; this report amends nothing)

```text
F1-quota: I approve amending docs/PHASE_2_EXECUTION_PLAN.md to raise the
durable staging quota (MAX_STAGED_RECORDS / WorkerConfig::default
max_observations) from 10,000 to [OWNER FILLS: e.g. 10,500], because the
accepted baseline fixture produces 10,005 observations (10,000 FLP files +
5 hardlink aliases) and every Run A iteration ends ResourceLimit. Record the
new quota, the alias-counting rule, and the re-measurement requirement.
```

```text
F1-fixture (alternative to F1-quota, pick one): I approve amending
docs/PHASE_2_EXECUTION_PLAN.md to define the baseline fixture as exactly
10,000 observations including hardlink aliases (e.g. 9,995 FLP files + 5
aliases, seed 0), instead of 10,000 FLP-named files plus aliases. The
"10,000 FLP-named files" row is superseded by this definition.
```

```text
F2-budget: I approve amending docs/PHASE_2_EXECUTION_PLAN.md warm
reconciliation budget from p95 <= 10 s to [OWNER FILLS: value + host class,
e.g. p95 <= 15 s on laptop-class i7-10750H / <= 10 s on desktop-class],
citing benchmark-2026-09-07 (median 12875.5 ms, p95 32876 ms, n = 10) as
evidence. No code change is authorized by this sentence alone.
```

```text
F2-host (alternative or companion to F2-budget): I approve adding a
desktop-class reference host to docs/PHASE_2_EXECUTION_PLAN.md and
re-measuring the warm-p95 budget there before any re-budget; the
laptop-class result (p95 32876 ms) stands as reported and is not rewritten.
```

```text
F2-rerun: I approve an idle-host re-run per benchmark-2026-09-07 §Limitations
(DriveFS paused, AV exclusion for the fixture volume, no agent load, AC +
high-performance, documented cache state, 1 warm-up + 10 measured) to
isolate background contention before any optimize/re-budget decision. Report
median/max/nearest-rank p95 the same way; do not filter iterations.
```

```text
F2-optimize: I approve profiling and optimizing the scan handle/metadata
path (per-file opens, ancestor validation chain, synchronous=FULL commit
batching) against the quota-fitting 10k set, then re-measuring warm-p95 on
the same host class. Budgets stay as written until the new evidence lands.
```

```text
F3-100k: I acknowledge the 100,000-entry qualification set stays
unmeasurable under the current 10,000-record staging quota, and I approve
[OWNER FILLS: raising the quota / staging the 100k set in bounded chunks /
deferring the 100k qualification with a dated checkpoint] in
docs/PHASE_2_EXECUTION_PLAN.md. The ~53 MiB figure remains a 10k-set
measurement and is not claimed against the 100k budget.
```

## 8. Limitations (carried + new)

- Same laptop-class host; results transfer to no other machine.
- Host was not idle (§3 table): DriveFS syncing, Defender real-time,
  EpicGamesLauncher, T3/agent tooling, ~14 sibling worktrees, concurrent
  builds at Run A start. This re-run isolates *concurrent Cargo builds*
  (Run B started at zero) but not DriveFS/AV/agent load — it does not
  satisfy F2-rerun conditions.
- No cache-state control; first runs reported separately, never "cold-cache".
- `tasklist` working-set sampling at ~100 ms, locale-KB parsing, approximate
  idle correlation — and working set ≠ private bytes (§5 memory caveat).
- Cooperative token path only; durable cancellation API and 250 ms UI
  acknowledgement not exercised (no renderer).
- Advisory `ChangeSummary` omitted by design at 10,000 locations
  (reference-bound guard; `changes_computed: false` on all warm iterations,
  `true` on both warm-ups) — publication unaffected.
- DriveFS (#47) / FAT32 / cross-volume (#48) excluded from every claim. No
  file contents read/hashed/parsed; synthetic bytes only.
- The sibling triage worktree/branch was not used; its draft (if any) is not
  evidence here.

## 9. Reproducibility

From `chore/benchmark-triage-20260908` at `51f45af`, with
`C:\Users\guilh\.cargo\bin` on `PATH` (harness spawns `cargo`):

```text
cargo build --release -p fruitboard-scan-execution --example benchmark --locked
node scripts/run-benchmark.mjs --size baseline --seed 0 --out docs/review/phase-2-integration/benchmark-triage-20260908/rerun-baseline-raw.json
node scripts/run-benchmark.mjs --size custom --files 9995 --seed 0 --out docs/review/phase-2-integration/benchmark-triage-20260908/rerun-quota-raw.json
```

Expected fixture hashes: `8c3d85ec…afba` (baseline), `a4760a28…6d08a`
(custom-9995). Raw JSONs in this directory are the preserved evidence.

## 10. Explicit non-change statement

No budget, quota, fixture definition, acceptance ID, workflow, or source file
was created, edited, or amended by this task. The only writes are the four
new files listed at the top of this report, all under
`docs/review/phase-2-integration/benchmark-triage-20260908/` in the
`triage worktree. No merge to main, no push, no production scanning
activation, no P2 acceptance promotion.

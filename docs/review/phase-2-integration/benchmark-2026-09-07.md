# Phase 2 scanner benchmark — first measured report (2026-09-07)

Status: **first measured evidence for P2-11 (#41)**. This report executes the
protocol in `scripts/benchmark-scan.md` against the integrated (hidden) scan
worker from PR #70 using the benchmark driver in
`crates/scan-execution/examples/benchmark.rs` and the harness in
`scripts/run-benchmark.mjs`. It contains real measurements only; every budget
miss is recorded as a finding with a hypothesis, and no safety check was
relaxed to produce these numbers.

Raw iteration data (evidence, private-safe):
`benchmark-2026-09-07-baseline-raw.json` and
`benchmark-2026-09-07-quota-raw.json` in this directory.

## Machine profile

| Field | Value |
| --- | --- |
| CPU model | Intel(R) Core(TM) i7-10750H CPU @ 2.60 GHz (laptop-class, 2019) |
| Logical cores | 12 (6 physical / 12 threads) |
| RAM | 16 GiB (17,041,244,160 bytes) |
| OS | Windows 11, build 10.0.26200, x64 |
| Power plan | High performance (`Alto desempenho`), host assumed on AC power |
| Storage | Local NVMe SSD (same volume as the temp fixture), 510.6 GB total, ~103 GB free at capture |
| Background load | Active developer host: Google DriveFS sync service, agent tooling (opencode/T3), NVIDIA container services, Windows Defender real-time scanning |

No hostname, user name or absolute path appears in this report or the raw
data; fixture identity is recorded by seed and manifest SHA-256 only.

## Build

- Commit: `2a8bdcf` (branch `chore/41-benchmark-run`).
- Command: `cargo build --release -p fruitboard-scan-execution --example benchmark --locked`.
- Toolchain: pinned via `rust-toolchain.toml` (Rust 1.98.1, cargo 1.98.1).
- Cold build wall time: ~30 s; the harness rebuild before each run was
  incremental (~0.3 s) and is reported separately in the raw files
  (`build.buildMs`), never inside the measured scan windows.
- The driver runs the production path end to end: durable `Database`,
  `ScanWorker::new(WorkerConfig::default())` with the system clock,
  `start_session`, `request_manual_scan`, `poll` (claim + execute) through
  the real `WindowsFilesystemPort` (handle-bound NTFS traversal) up to the
  fenced atomic publication. No shortcuts, no fake ports, no relaxed limits.

## Fixtures

Both fixtures were generated with `scripts/generate-synthetic-tree.mjs`
(recorded seed `0`) into a temp directory outside the repository and removed
after the run. Hardlink support was `created` on both (NTFS).

| Fixture | sizeLabel | On-disk .flp files | .flp observations | Manifest SHA-256 |
| --- | --- | --- | --- | --- |
| Accepted baseline preset | `baseline` | 10,005 (10,000 + 5 hardlink aliases) | 10,005 | `8c3d85ec01299995208abfa450378b1b37c704e423593d7a4370ec465254afba` |
| Quota-fitting variant | `custom-9995` | 10,000 (9,995 + 5 hardlink aliases) | 10,000 | `a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a` |

Methodology deviation (documented, not hidden): the accepted `baseline`
preset produces **10,005 observations**, five more than the durable staging
quota (`MAX_STAGED_RECORDS = 10,000`). Every baseline run therefore ends
non-authoritative with `ResourceLimit` (finding F1 below). To still measure
the latency budgets, a second fixture was generated with exactly 10,000
observations (9,995 FLP files + 5 aliases). Both fixtures are generator
output with recorded seeds and hashes; neither was hand-edited.

## Results

### Run A — accepted baseline preset (finding confirmation)

1 warm-up + 10 measured iterations; every run ended `Failed` /
`ResourceLimit` at exactly the staging-quota boundary. Wall time to the
quota failure is reported honestly; it is **not** a scan-latency pass.

| Iteration | Scan window (ms) | Status | Outcome |
| --- | --- | --- | --- |
| warm-up (first run) | 10,838 | Failed | ResourceLimit |
| 1 | 11,071 | Failed | ResourceLimit |
| 2 | 11,264 | Failed | ResourceLimit |
| 3 | 10,533 | Failed | ResourceLimit |
| 4 | 10,737 | Failed | ResourceLimit |
| 5 | 17,391 | Failed | ResourceLimit |
| 6 | 21,845 | Failed | ResourceLimit |
| 7 | 13,843 | Failed | ResourceLimit |
| 8 | 12,331 | Failed | ResourceLimit |
| 9 | 11,638 | Failed | ResourceLimit |
| 10 | 12,545 | Failed | ResourceLimit |

No authoritative run → the first-discovery and warm-reconciliation budgets
are **not evaluable on the accepted fixture**.

### Run B — quota-fitting fixture (10,000 observations)

1 warm-up + 10 measured iterations; all 10 measured iterations published
authoritatively (10,000 committed locations per run; the advisory change
summary was omitted at this size by the worker's reference-bound guard, by
design).

| Iteration | Scan window (ms) | Status | Outcome | Locations |
| --- | --- | --- | --- | --- |
| warm-up (first discovery, first run) | 11,886 | Published | Complete | 10,000 |
| 1 | 32,876 | Published | Complete | 10,000 |
| 2 | 12,710 | Published | Complete | 10,000 |
| 3 | 23,528 | Published | Complete | 10,000 |
| 4 | 31,304 | Published | Complete | 10,000 |
| 5 | 12,833 | Published | Complete | 10,000 |
| 6 | 11,686 | Published | Complete | 10,000 |
| 7 | 12,294 | Published | Complete | 10,000 |
| 8 | 12,924 | Published | Complete | 10,000 |
| 9 | 12,918 | Published | Complete | 10,000 |
| 10 | 11,911 | Published | Complete | 10,000 |

Statistics (n = 10): **median 12,875.5 ms, max 32,876 ms, nearest-rank p95
32,876 ms** (with n = 10 the nearest-rank p95 is the maximum by
construction). First run reported separately: 11,886 ms (first discovery on
an empty committed state; not labelled cold-cache — OS cache state was not
controlled).

### Cancellation

Cooperative stop latency (request fired mid-run via the typed token; time
from the `cancellation_requested` line to the terminal `Cancelled` outcome):

| Run | Stop latency (ms) |
| --- | --- |
| A-1 | 6 |
| A-2 | 6 |
| A-3 | 6 |
| B-1 | 6 |
| B-2 | 8 |
| B-3 | 6 |

The worker stopped within 8 ms of the mid-run request in every case. The
250 ms UI-acknowledgement budget does not bind the worker (there is no UI in
this measurement); the cooperative-worker stop budget (p95 <= 1 s) passes
with a very wide margin.

### Working memory

Working set sampled every ~100 ms with `tasklist` (locale-formatted column
parsed as KB). Idle baseline = last sample before the measured scan window;
peak = maximum sample inside the window.

| Run | Idle (MiB) | Peak (MiB) | Incremental (MiB) |
| --- | --- | --- | --- |
| A warm-up | 5.8 | 58.9 | 53.0 |
| A measured (min..max) | 5.5 | 58.7–58.9 | 52.2–53.4 |
| B warm-up | 5.9 | 58.9 | 53.0 |
| B measured (min..max) | 5.5 | 58.7–58.8 | 53.1–53.3 |

Peak incremental memory was ~53 MiB on the 10,000-location set. **Measured
on the baseline set, not the accepted 100,000-entry qualification set**
(see F3).

## Budget comparison

| Budget | Target | Measured | Verdict |
| --- | --- | --- | --- |
| Baseline first discovery | <= 30 s | 11,886 ms (Run B warm-up; Run A not evaluable) | **PASS** |
| Unchanged warm reconciliation p95 | <= 10 s | 32,876 ms (median 12,875 ms) | **FAIL** (finding F2) |
| Cooperative worker stop p95 | <= 1 s | 8 ms (max of 6 samples) | **PASS** |
| Incremental private memory | <= 128 MiB | ~53 MiB (baseline set; see F3) | **PASS** (set caveat) |
| Batch/staging <= 512 records / <= 256 MiB | Enforced | 512 records per batch is the enumerator's hard bound; quota enforcement demonstrated by F1 | Not directly observable through the worker's public API; no violation observed |

Queue/leases, retry budgets, progress/list pacing and the IPC Library query
are covered by deterministic tests and are outside this wave's measurements.

## Findings

**F1 — The accepted baseline fixture cannot complete an authoritative scan
under the durable staging quota.** The `baseline` preset generates 10,000
FLP files plus 5 hardlink aliases = 10,005 observations, while
`MAX_STAGED_RECORDS = 10,000`. Every run stops at the enumeration resource
limit (`WorkerConfig::default` mirrors the quota: `max_observations =
MAX_STAGED_RECORDS`), staging is discarded, and no location is published.
The enforcement behaved exactly as designed (quota failure retained prior
results; committed rows untouched). Hypothesis: the accepted fixture count
(10,000) and the accepted staging quota (10,000 records) did not account for
hardlink aliases being separate locations. Owner decision needed: either
raise the staging quota above 10,005 or define the baseline fixture as
exactly 10,000 observations including aliases. This finding also invalidates
the budget row "Baseline fixture 10,000 FLP-named files" as literally
implemented.

**F2 — Warm reconciliation p95 (32.9 s; median 12.9 s) exceeds the 10 s
provisional budget on this host.** First discovery passed (11.9 s). The
first four measured iterations were much slower (23.5–32.9 s) while the last
six were stable (~11.7–12.9 s), matching heavy background activity
(DriveFS indexing of the freshly generated fixture, Defender real-time
scanning, agent tooling) during the first part of the run. Hypotheses, in
order of suspicion: (a) background I/O/AV contention on the fixture volume;
(b) per-file handle-bound metadata opens (`NtCreateFile` per entry plus the
ancestor validation chain per directory) dominating a 10k-file traversal on
laptop-class hardware; (c) `synchronous = FULL` commit cost per staged batch.
The miss is reported honestly; it is not "fixed" by loosening a check or by
selecting a subset of iterations. Owner review is required before this
budget can be considered met on laptop-class hardware.

**F3 — The 100,000-entry qualification set cannot be scanned at all under
the current staging quota.** 100,000 FLP files + aliases far exceed
`MAX_STAGED_RECORDS = 10,000`, so the accepted working-memory budget
(<= 128 MiB on the 100,000-entry set) is unmeasurable today. The 128 MiB
budget passes on the 10,000-location set (~53 MiB incremental) with that
caveat.

## Limitations

- Laptop-class hardware (i7-10750H, 2019); results are not desktop/server
  class and are not transferable to other hosts.
- The host was not idle: Google DriveFS (DriveFS modes remain excluded under
  #47), Windows Defender, and the agent tooling itself ran during the
  measurements; iteration-level variance reflects that.
- No warm-cache control beyond the stated single warm-up; the first run is
  reported separately and is not labelled cold-cache.
- Memory sampling granularity is ~100 ms and the column is locale-formatted
  KB (parsed with separator stripping); idle-baseline correlation is
  approximate.
- Cancellation here is the worker's cooperative token path (mid-run
  request); the durable cancellation API and the 250 ms UI acknowledgement
  budget are not exercised (no renderer in scope).
- The advisory `ChangeSummary` is omitted by design at 10,000 locations
  (reference-bound guard); publication is unaffected.
- DriveFS (#47) and FAT32/cross-volume identity (#48) are excluded from any
  claim. No file contents were read, hashed, hydrated or parsed; the
  fixtures contain synthetic bytes only.

## Reproducibility

```text
node scripts/run-benchmark.mjs --size baseline --seed 0 --out <report-a>.json
node scripts/run-benchmark.mjs --size custom --files 9995 --seed 0 --out <report-b>.json
```

Both commands rebuild the pinned release driver (timed separately), generate
the fixture into a temp directory, run 1 warm-up + 10 measured iterations
with a fresh process per iteration against one committed database, measure 3
mid-run cancellations, sample working set every 100 ms, and print the budget
table. Fixture temp directories are removed on exit.

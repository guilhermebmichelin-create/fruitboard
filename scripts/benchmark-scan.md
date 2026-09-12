# Scanner benchmark methodology (P2-11 prep, #41)

Status: **integrated harness; first measurements recorded.** This document defines how the
accepted provisional budgets from
`docs/PHASE_2_EXECUTION_PLAN.md` ("Proposed targets accepted as provisional
budgets") are measured by `scripts/run-benchmark.mjs` and the native
`crates/scan-execution/examples/benchmark.rs` driver. The
[first measured report](../docs/review/phase-2-integration/benchmark-2026-09-07.md)
records results and limitations; this methodology does not amend budgets or
qualify installed-app behavior.

## Scope

Present today:

- `scripts/generate-synthetic-tree.mjs`, a deterministic, seeded generator for
  the accepted baseline fixture (10,000 FLP-named synthetic files across 1,000
  leaf directories) and the qualification set (100,000 FLP-named files across
  10,000 leaf directories), including Unicode names, long paths near the
  Windows MAX_PATH limit, Windows-only hardlink alias cases, empty
  directories and nested directories.
- `scripts/run-benchmark.mjs`, which validates or generates a fixture,
  recomputes its manifest SHA-256, captures the environment, builds the
  release driver, and executes the scan and cancellation protocol below.
- The native driver composes the real Windows filesystem port, scan worker,
  and durable database. Working-set samples describe the driver process;
  they are not a measurement of incremental private memory in the desktop app.

Not established by this harness:

- rendered UI cancellation acknowledgement, renderer progress/list pacing,
  installed-app behavior, or 100,000-entry memory qualification. The accepted
  baseline generates 10,005 locations including aliases, exceeding the
  current 10,000-record quota. The first report's quota-fitting comparison
  fixture does not replace the accepted baseline. Findings F1-F3 remain open.

## Reference machine profile

Every benchmark run captures and publishes a machine profile so results are
reproducible and attributable. `scripts/run-benchmark.mjs` records the
following into the environment report:

| Field                                             | Source                                 | Operator duty                                                              |
| ------------------------------------------------- | -------------------------------------- | -------------------------------------------------------------------------- |
| CPU model and core count                          | Node `os.cpus()`                       | verify against the physical host                                           |
| Total RAM                                         | Node `os.totalmem()`                   | verify against the physical host                                           |
| OS platform, release, architecture                | Node `os` module                       | record the OS build in notes                                               |
| Storage capacity/free bytes of the fixture volume | `fs.statfs`                            | record the storage kind (for example NVMe SSD) and its connection in notes |
| Power mode                                        | `powercfg /getactivescheme` on Windows | confirm the host is on AC power for every iteration                        |

The report intentionally contains no hostnames, user names or absolute
filesystem paths. The operator completes the storage-kind and power notes
manually in the evidence write-up; nothing is auto-detected that Node cannot
observe reliably.

## Pinned release build

Measurements are only valid against a pinned release build:

- build with the workspace-pinned Rust toolchain (1.98.1) and locked
  dependencies (`cargo build --release --locked` or the packaging command in
  `DEVELOPMENT.md`);
- record the exact commit, build command and toolchain in the report notes
  before measuring;
- never measure development builds, ad hoc patches, or unrecorded states;
- re-run the environment capture whenever the build or host changes.

## Fixture

- Generate with `scripts/generate-synthetic-tree.mjs` using a recorded seed;
  the manifest SHA-256 printed by the generator identifies the fixture.
- Baseline fixture: 10,000 FLP-named files across 1,000 leaf directories.
- Qualification set: 100,000 FLP-named files across 10,000 leaf directories.
- Fixtures contain synthetic bytes only; no private projects, no parser, no
  real FLP content.
- Record the fixture location, seed and manifest hash in the report notes.
  Regenerate rather than hand-editing; a fixture whose hash does not match its
  recorded seed is invalid evidence.

## Iteration protocol

For each budget being measured, on an otherwise idle host:

1. Capture the environment report (`scripts/run-benchmark.mjs`) and fill in
   the operator notes (storage kind, power mode, OS build, build commit).
2. Perform exactly one warm-up iteration of the measured operation. The
   warm-up is excluded from the statistics below.
3. Perform exactly 10 measured iterations of the same operation against the
   same fixture. Do not run other work concurrently, keep the host on AC
   power, and restart the application between iterations per the evidence
   write-up so each iteration starts from the committed state.
4. Record per-iteration wall-clock durations for the whole measured
   operation. For scan latency that is enumerate + stage + final apply, as
   defined by the accepted budgets. Never skip safety checks to make an
   iteration faster; a miss is investigated, not excused.
5. Record failures, cancellations and anomalies alongside the durations. A
   failed iteration is reported as a failure, not discarded silently.

For a terminal `Partial` scan, `scan_finished` also carries the bounded
`partial_class` value when the retained coverage diagnostics identify exactly
one class. Multiple retained classes emit `partial_multiple`; absent or
truncated diagnostics emit `partial_unknown`. The protocol never carries
paths, file content, native error messages, or raw stderr.

The harness performs these steps with one fresh driver process per iteration
and the same committed database for warm-up and measured scans. Cancellation
measurements use separate databases. This is driver-process restart evidence;
it does not exercise closing and reopening the desktop app.

## Statistics and reporting

From the 10 measured durations report:

- the **median**;
- the **maximum**;
- the **nearest-rank p95**: sort the 10 durations ascending and take the
  value at rank `ceil(0.95 * 10) = 10`. With 10 samples the nearest-rank p95
  therefore equals the maximum; both are reported explicitly so nobody
  mistakes a lower order statistic for a p95.

The first-run (warm-up) duration is reported separately. Do not label it a
cold-cache measurement unless the OS cache state was actually controlled for
that run; otherwise it is only "first run after process start".

Every published result states: host profile, OS build, storage kind, power
mode, build commit, fixture seed and manifest hash, iteration count, and the
three statistics plus the first run. If a target is rejected by measurement,
amend the budgets table in the execution plan with evidence and owner review
before broadening support.

## Provisional budgets (quoted, not measured)

The following budgets are quoted from
`docs/PHASE_2_EXECUTION_PLAN.md` ("Proposed targets accepted as provisional
budgets") and are owner-accepted starting points, not measured promises and
not results:

| Area              | Proposed target                                                                              | Failure behavior / measurement                                                                               |
| ----------------- | -------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| Baseline fixture  | 10,000 FLP-named synthetic files across 1,000 directories; qualification set 100,000 entries | No private projects; no parser; include Unicode, long paths and aliases in correctness sets                  |
| NTFS scan latency | Baseline first discovery <=30 s; unchanged warm reconciliation p95 <=10 s                    | Measure enumerate + stage + final apply; investigate misses, never skip safety checks                        |
| Cancellation      | UI acknowledgement <=250 ms; cooperative worker stop p95 <=1 s between responsive I/O calls  | Blocking OS calls may exceed this; invalidate publication immediately, report observed stop latency honestly |
| Working memory    | Scanner incremental private memory <=128 MiB on the 100,000-entry set                        | Measure against idle app; bounded streaming, no full-tree in-memory accumulation                             |
| Batch/staging     | <=512 records per batch; <=256 MiB staging per run                                           | Enforced quota failure retains prior results; benchmark disk use and cleanup                                 |
| Queue/leases      | One worker, one follow-up/root; 30 s lease, renewal every 5 s                                | Fake-clock expiry tests; lease validation before all writes                                                  |
| Retry budget      | Three automatic retries at 1/2/4 s, bounded jitter <=20%; then explicit retry                | Persist attempts; no tight loops on disconnected roots                                                       |
| Progress/list     | <=4 progress updates/s; <=200 records/page; baseline query p95 <=200 ms                      | Test throttling and page order; measure IPC query separately from rendering                                  |
| Root budget       | Initially qualify <=100 configured roots                                                     | Validate budget before claiming larger scale; never silently ignore excess roots                             |

CI correctness tests use deterministic clocks and invariants, never
hardware-sensitive absolute timing assertions, per the accepted plan.

## Harness behavior

`node scripts/run-benchmark.mjs --manifest <fixture>/manifest.json
[--out <report.json>]`:

- validates the fixture manifest (relative POSIX paths, ascending order,
  counts consistent with entries, hardlink groups complete when support is
  `created`, case-insensitive `.flp` naming) and recomputes its SHA-256;
- builds with `cargo build --release -p fruitboard-scan-execution --example
benchmark --locked`, timing the build separately;
- runs one warm-up and ten measured scans by default, plus three cancellation
  measurements, recording failures alongside successful results;
- samples process working set every 100 ms by default and reports scan and
  cancellation statistics with the environment and provisional budget table;
- refuses to run when the integration marker
  `crates/scan-execution/Cargo.toml` is absent; when that marker exists but the
  complete Cargo workspace is unavailable, falls back to an explicitly
  unmeasured environment report.

Use `--size baseline --seed 0` instead of `--manifest` to generate a disposable
fixture. `--size custom --files 9995 --seed 0` reproduces the quota-fitting
comparison when five hardlink aliases are created; verify the manifest counts.
Generated fixtures are removed unless `--keep-fixture` is supplied. `--bin`
skips the build and therefore requires independently recording the binary's
release-build provenance. Protocol overrides are exploratory runs unless they
satisfy the accepted iteration and environment requirements above.

## Evidence home and open gates

Benchmark evidence belongs alongside the #41 aggregator index in
`docs/review/phase-2-integration/`. DriveFS and cross-volume/FAT32 identity
claims remain gated on open follow-ups #47 and #48: no benchmark or
correctness claim covers those modes until their evidence lands or the owner
explicitly excludes them from scope.

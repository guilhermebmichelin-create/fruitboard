# Phase 2 benchmark performance follow-up

Status: evidence-only investigation, captured 2026-09-08. This follow-up does
not change production code, shared benchmark presets, quotas, budgets, or
acceptance criteria. The diagnostic example is example-only and wraps the
existing filesystem-port boundary.

## Result at a glance

- **F1 reproduced:** the accepted baseline has 10,000 FLP files plus five
  hardlink alias locations, so enumeration presents 10,005 observations.
  WorkerConfig::default and SQLite staging both cap a run at 10,000 records.
- **F2 has a measured cost center, not an idle-host verdict:** the latest
  official warm p95/max is 14,267 ms; the new profile measured 10.754 s of
  filesystem-port calls in a 12.157 s warm diagnostic (88.46%). Three
  independent fresh-process diagnostics were 9.924–10.057 s on the same
  contended host.
- **F3 is not currently qualifiable:** worker, staging, and reconciliation
  buffers all carry 10,000-record bounds. A safe 100,000-entry qualification
  needs an owner-approved bounded snapshot/chunk design or an end-to-end
  re-budget; raising one constant is insufficient.

The official raw records remain authoritative:
[baseline raw](../benchmark-triage-20260908/rerun-baseline-raw.json),
[quota-fitting raw](../benchmark-triage-20260908/rerun-quota-raw.json), and
[triage report](../benchmark-triage-20260908/rerun-report.md). New raw
filesystem-port records are in
[profile-fs-calls-raw.jsonl](profile-fs-calls-raw.jsonl), with
[provenance](provenance.json).

## Current decision state

The methodology and provisional budgets are unchanged:
[benchmark methodology](../../../../scripts/benchmark-scan.md) and
[Phase 2 execution plan](../../../../docs/PHASE_2_EXECUTION_PLAN.md).

I checked merged PR [#86](https://github.com/guilhermebmichelin-create/fruitboard/pull/86),
open draft reconciliation PR [#91](https://github.com/guilhermebmichelin-create/fruitboard/pull/91),
and open draft installed-app journey PR [#92](https://github.com/guilhermebmichelin-create/fruitboard/pull/92).
No owner decision sentence resolving F1, F2, or F3 is present in the checked
issue/PR comments or reviews. This report recommends decisions but does not
make them.

## F1 — 10,005 observations versus a 10,000-record staging limit

The accepted baseline manifest is seed 0, hash
8c3d85ec01299995208abfa450378b1b37c704e423593d7a4370ec465254afba:

| Manifest component | Count | Included in observations? |
| --- | ---: | --- |
| FLP-named files | 10,000 | Yes |
| Hardlink alias locations | 5 | Yes |
| Other files | 4 | No |
| Total observations presented to the worker | **10,005** | **Yes** |

The reproduction in
benchmark-triage-20260908/rerun-baseline-raw.json has one warm-up and all
ten measured iterations ending Failed / ResourceLimit, with no authoritative
publication. The comparison fixture has 9,995 FLP files plus the same five
aliases and produces exactly 10,000 observations; its manifest hash is
a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a.

The enforcement chain is explicit:

1. crates/scan-execution/src/lib.rs:103-110 sets the worker's default
   EnumerationLimits.max_observations to MAX_STAGED_RECORDS, 10,000.
2. crates/filesystem-enumeration/src/lib.rs:1421-1423 terminates traversal
   when that observation count would be exceeded.
3. crates/storage-sqlite/src/publication.rs:94-96 and :1513-1520 enforce the
   same 10,000-record staging fence independently of the enumerator.

This is a fixture/accounting mismatch, not an off-by-one in the staging
transaction: valid hardlink aliases count as locations/observations even when
they share identity.

Owner decision required:

- Make the baseline an exact 10,000-observation fixture (custom-9995,
  preserving aliases), then rerun the accepted protocol; or
- Keep the accepted 10,000-FLP fixture and raise every relevant quota and test
  budget above 10,005, then rerun and record that aliases count toward quota.

The first is the narrowest fixture correction. Neither is applied here because
each changes acceptance input or a shared quota.

## F2 — measured scan-path bottlenecks

### Official benchmark

The official quota-fitting rerun at triage commit 51f45af used one warm-up and
ten measured iterations. Warm scan times were:

~~~text
11,340, 11,143, 10,965, 10,970, 10,887, 10,947, 11,158, 10,839, 10,986, 14,267 ms
~~~

With nearest-rank p95 for n=10, p95 is the maximum: **14,267 ms**, 4,267 ms
(42.67%) above the provisional 10,000 ms target. Median is **10,978 ms**,
978 ms (9.78%) above target. The raw report remains the source for these
statistics.

### New diagnostic profile

crates/scan-execution/examples/profile-fs-calls.rs wraps
FilesystemPort/DirectoryCursor and times the exact calls made by ScanWorker.
It changes no worker, filesystem, storage, or benchmark code. The residual is
scan_ms minus filesystem_call_sum_ms; it includes staging, reconciliation
reads, publication, and worker/instrumentation overhead and is not assigned to
one storage phase.

The profile used merged commit
0b7612db3570e6235d4d2c86a678dd9004264f30, the custom-9995 fixture, and the
pinned Rust toolchain. The installed-app journey completed first; no
Fruitboard desktop, cargo, or rustc process was active at the checkpoint.
DriveFS and six T3 processes remained active. Every result here is a
**contended shared-host diagnostic, not an idle-host measurement or acceptance
result**.

Same-process pair:

| Run | Scan | Filesystem calls | Residual | Filesystem share |
| --- | ---: | ---: | ---: | ---: |
| first (not called cold) | 10,577 ms | 9,231 ms | 1,346 ms | 87.27% |
| warm | 12,157 ms | 10,754 ms | 1,402 ms | 88.46% |

Three independent fresh-process runs against the same database were 10,057 ms,
9,993 ms, and 9,924 ms, with filesystem shares 87.15–87.19%. This small sample
is variance evidence, not a p95 rerun.

The 12,157 ms warm sample decomposes as follows:

| Port boundary | Calls | Time | Filesystem share | Production work included |
| --- | ---: | ---: | ---: | --- |
| next_entry | 12,056 | 4,952.690 ms | 46.05% | ancestor validation and NtQueryDirectoryFile |
| read_metadata | 11,030 | 5,342.511 ms | 49.68% | ancestor validation, relative open, metadata and identity queries |
| open_directory | 1,025 | 456.877 ms | 4.25% | ancestor validation, child open, metadata, case-sensitivity query |
| root inspect/open | 3 | 2.560 ms | 0.02% | root qualification/open |

next_entry plus read_metadata account for 10,295.201 ms, 84.69% of the full
scan and 95.73% of timed filesystem-port time. Counts are stable across all
five records; wall time varies.

The source explains the structural mechanism without claiming individual
native syscall counts: Windows next_entry, read_metadata, and open_directory
each call validate_ancestors before their operation
(crates/filesystem-enumeration/src/lib.rs:2229-2287), and validation walks the
parent chain and reopens/rechecks each link (:2335-2354). The profile times the
public boundary, so it measures each complete operation.

### Measured findings versus hypotheses

Measured:

- Filesystem-port time is 86.53–88.46% of diagnostic wall time.
- next_entry and read_metadata are the dominant measured subpaths.
- The residual is approximately 1.27–1.40 seconds, but is not phase-isolated.
- Fresh-process warm scans can land near 10 seconds, while the official sample
  contains a 14.267-second spike. This demonstrates variance, not its cause.

Hypotheses requiring an idle-host or phase-isolated rerun:

- DriveFS, antivirus, T3, or other host activity contributed to the official
  spike.
- Repeated ancestor validation is the main structural optimization opportunity.
- Staging, about 50 library pages at 200 rows/page, and SQLite synchronous=FULL
  publication contribute to the residual. The current profile does not rank
  those residual components.

Recommended next action if the owner chooses F2 optimization: add approved phase
timers around enumeration, staging, change-plan reads, and publication, then
profile a quiesced host. The first code candidate is a correctness-preserving
cache or stable traversal context for ancestor validation, retaining
revalidation/fencing semantics. A second is reducing duplicate metadata queries
where the Windows safety contract permits it. Storage batching or SQLite
durability changes need separate measurement and must not be silently weakened.
The next official decision should wait for the required idle-host ten-iteration
rerun.

## F3 — 100,000 entries under bounded resources

The current end-to-end path cannot qualify 100,000 entries:

| Bound | Current implementation | Consequence |
| --- | --- | --- |
| Worker observations | 10,000 (WorkerConfig::default) | Enumeration stops before 100,000 |
| Staged records | 10,000 (MAX_STAGED_RECORDS) | Storage rejects over-quota staging |
| Staged path bytes | 4 MiB (MAX_STAGED_PATH_BYTES) | Larger record quota may still fail on path bytes |
| One stage batch | 512 (MAX_STAGED_BATCH_RECORDS) | 100,000 records require 196 batches |
| Advisory plan buffer | 10,000 (PlanBuffer::CAPACITY) | Changing only storage cannot scale reconciliation |
| Library page | 200 (MAX_LIBRARY_PAGE_SIZE) | 100,000 rows require about 500 pages |
| Existing memory evidence | 53 MiB working-set increment at 10,000 | Not private-memory qualification at 100,000 |

Concrete options:

| Option | Required design work | Resource/acceptance gate |
| --- | --- | --- |
| Raise end-to-end bound | Change worker, staging, path-byte, plan-buffer, and publication/reconciliation limits together | Measure private bytes, staged DB bytes, wall time, cleanup, and complete 100,000-record publication |
| Bounded chunked snapshot | Enumerate chunks, retain root-generation/coverage ledger, publish one authoritative snapshot after all chunks succeed | Bound chunk memory and durable staging; prove cancellation, retry, missing-file, and crash recovery |
| Staged two-pass qualification | Build a durable bounded manifest/coverage set, then reconcile and publish from bounded pages | Define ownership, expiry, disk quota, and atomicity; this is a new scan contract |
| Defer 100,000 | Keep the current 10,000 contract with an owner-approved checkpoint | Do not describe the provisional 100,000 budget as qualified |

The recommended bounded-resource direction is the chunked snapshot option: it
limits private memory without weakening authoritative publication. It requires
a storage/protocol design; calling current publication once per chunk would
create multiple root generations and could mark records from other chunks
missing. “Raise the constants” is suitable only if the owner accepts a new
private-memory, disk, and latency budget and updates every coupled fence.

For scale planning only, 100,000 records at 512 per batch is 196 batches and at
200 per page is 500 pages. These are arithmetic planning values, not
measurements or acceptance changes.

## Reproduction and evidence commands

Official fixture commands:

~~~text
node scripts/run-benchmark.mjs --size baseline --seed 0 --out <report-a>.json
node scripts/run-benchmark.mjs --size custom --files 9995 --seed 0 --out <report-b>.json
~~~

Persistent diagnostic fixture using pinned Node:

~~~text
<repo>\\.tools\\node-v24.20.0-win-x64\\node.exe --input-type=module -e "import { buildPlan, writePlan } from './scripts/generate-synthetic-tree.mjs'; const plan = buildPlan({ sizeLabel: 'custom-9995', seed: '0', fileCount: 9995, leafDirectoryCount: 1000 }); await writePlan(plan, process.argv[1]);" -- <fixture>
~~~

Build/run using pinned Rust:

~~~text
$env:CARGO_HOME = (Join-Path (Get-Location) '.tools\\cargo-home')
$env:RUSTUP_HOME = (Join-Path (Get-Location) '.tools\\rustup-home')
& (Join-Path $env:CARGO_HOME 'bin\\cargo.exe') build --release -p fruitboard-scan-execution --example profile-fs-calls --locked
& .\\target\\release\\examples\\profile-fs-calls.exe --root <fixture> --db <profile-db> --runs 2 --out <profile-fs-calls-raw.json>
& .\\target\\release\\examples\\profile-fs-calls.exe --root <fixture> --db <profile-db> --runs 1 --warm --out <profile-warm-1.json>
~~~

Before any timed run, record the installed-app checkpoint and host processes.
The captured profile used no concurrent Fruitboard build or desktop process,
but retained DriveFS/T3 activity and is marked contended in
[provenance.json](provenance.json). Do not use these records to promote the
10-second target or to make an idle-host claim.

## Checks

The evidence branch ran:

- cargo fmt --all -- --check
- cargo check -p fruitboard-scan-execution --example profile-fs-calls --locked
- cargo build --release -p fruitboard-scan-execution --example profile-fs-calls --locked
- pnpm.cmd lint:docs
- pnpm.cmd privacy:check
- git diff --check

No production implementation, shared fixture preset, quota, budget, or
acceptance criterion is changed by this follow-up.

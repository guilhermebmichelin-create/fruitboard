# P2-11 warm-reconciliation diagnostic investigation

Date: 2026-09-13. This is a bounded diagnostic follow-up on the remaining
warm-reconciliation performance failure. It does not qualify performance,
reinterpret the historical `Partial`, change a target or limit, or implement
a production scanning optimization.

## Disposition

The measured costs make repeated ancestor validation the strongest structural
explanation for the approximately 9-second warm scan. The native profiler
recorded 26,163 ancestor-validation calls and 60,973 reopened validation links
per 10,000-observation traversal; the calls consumed about 6.31-6.38 seconds
of a 7.83-7.94 second direct enumeration. Per-entry handle and metadata work
is the next measured filesystem cost.

The phase-timed worker run also measured material staging and publication
costs, but those buckets include several operations and do not isolate SQLite.
The existing profiler's residual likewise includes staging, reconciliation,
publication, fencing, conversion, and worker overhead. No evidence in this
session explains the historical `Partial`, and no later successful run should
be relabeled as an explanation of that event.

## Source and fixture boundary

| Item | Value |
| --- | --- |
| reviewed `origin/main` baseline | `83da093672b5e2154097c897af533821b04f2352` |
| source used by the existing profilers | `83da093672b5e2154097c897af533821b04f2352` |
| earlier qualification measurement | `69f27f64f26aa657182a9260cc8e78f28a5838fb` (reported as `69f27f6`) |
| fixture | `custom-9995`, seed `0`, exactly 10,000 observations |
| fixture manifest SHA-256 | `a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a` |
| fixture counts | 9,995 FLP files, 4 other files, 5 alias locations, 1,025 directories, 1,000 leaves, 10 empty directories |

`origin/main` was fetched before the clean worktree was created. The two
commits after `69f27f6` and before the reviewed baseline are documentation
commits (`f811cf3` and `83da093`); their diff is confined to documentation
and agent workflow files, with no production enumeration, storage,
reconciliation, or publication change. Therefore the existing profiler
measurements are against the reviewed merged code, not a changed production
implementation.

This spike adds only a feature-gated diagnostic phase profile in
`scan-execution` and exposes it through the existing filesystem-call profiler.
With the feature disabled, the production path remains unchanged. The
phase-timed binary is therefore `83da093` plus the diagnostic-only changes in
this spike; it is not qualification evidence.

The previously completed qualification run remains exactly as reported:
source `69f27f6`, warm p95 `10,173 ms`, median `10,014 ms`, and 10/10
authoritative scans. Those values remain a non-passing result for the stated
warm target and do not explain the historical `Partial`.

## Bounded diagnostic session

The run counts were declared before each workload launch. The session plan was
three runs of the existing end-to-end profiler, three runs of the existing
native-operation profiler, then a two-run phase-timed follow-up after the
mixed residual required phase attribution. Total: eight profiler attempts.
Every attempt was retained; there were no retries or discarded outliers. No
10-run qualification was repeated.

Before the phase-timed launch, the process and lock audit found no Cargo,
rustc, benchmark, scan, or storage-smoke workload. The exclusive window was
therefore exclusive for the diagnostic workload, but not a strict quiet-host
qualification window: DriveFS and Defender services remained present, and
short samples ranged from 0-23% CPU and 13-100% disk idle. This limitation is
why the results are diagnostic only and do not identify background services as
the cause.

Commands, with local paths replaced by placeholders, were:

```powershell
$env:CARGO_TARGET_DIR = "<reusable validation cache>"
$env:CARGO_HOME = "<pinned cargo home>"
$env:RUSTUP_HOME = "<pinned rustup home>"

cargo build --release -p fruitboard-scan-execution `
  --example profile-fs-calls --locked
cargo build --release -p fruitboard-filesystem-enumeration `
  --features diagnostics --example profile-native-ops --locked

profile-fs-calls.exe --root <custom-9995 fixture> --db <profile db> `
  --runs 3 --out <end-to-end raw JSONL>
profile-native-ops.exe --root <custom-9995 fixture> `
  --runs 3 --out <native raw JSONL>

cargo build --release -p fruitboard-scan-execution `
  --features diagnostics --example profile-fs-calls --locked
profile-fs-calls.exe --root <custom-9995 fixture> --db <phase profile db> `
  --runs 2 --out <phase-timed raw JSONL>
```

The phase profile records only bounded counters and durations. It does not
emit paths, metadata, SQL, file contents, or host identifiers.

## Reused profiler results

The existing `profile-fs-calls` wrapper measured the public filesystem calls
around the real worker path. Its residual is a subtraction, not a storage
timer.

| run | scan | filesystem-call sum | residual | authoritative result |
| ---: | ---: | ---: | ---: | --- |
| first | 8,915 ms | 7,773 ms | 1,142 ms | Published / Complete / 10,000 |
| warm 1 | 8,862 ms | 7,672 ms | 1,190 ms | Published / Complete / 10,000 |
| warm 2 | 8,840 ms | 7,647 ms | 1,193 ms | Published / Complete / 10,000 |

Call counts were stable: 12,056 `next_entry`, 11,030 `read_metadata`, and
1,025 `open_directory` calls per run.

The existing `profile-native-ops` profiler isolates the enumeration crate
with a no-op recording sink. It recorded:

| run | enumeration | native-call sum | residual | ancestor calls / links |
| ---: | ---: | ---: | ---: | ---: |
| first | 7,832 ms | 7,608 ms | 224 ms | 26,163 / 60,973 |
| warm 1 | 7,890 ms | 7,663 ms | 227 ms | 26,163 / 60,973 |
| warm 2 | 7,936 ms | 7,707 ms | 228 ms | 26,163 / 60,973 |

The native sum contains the profiler's timed native buckets. The ancestor
bucket alone was 6,306 ms, 6,351 ms, and 6,382 ms respectively. The remaining
per-run native costs were approximately 1,022-1,040 ms for 11,030 entry
opens, 71-72 ms for entry metadata, 93-95 ms for 1,025 directory opens,
6 ms for directory metadata, and 108-111 ms for 14,108 directory queries.

## Phase-timed worker results

The added profile measures the unchanged worker path at six useful boundaries.
`enumeration` contains nested staging calls; `change_plan` contains nested
library-page reads and reconciliation. Those nested buckets must not be
summed with their containing bucket.

| phase | first run | warm run | calls on warm run |
| --- | ---: | ---: | ---: |
| total scan | 9,127 ms | 9,036 ms | 1 |
| enumeration (including staging) | 8,457 ms | 8,365 ms | 1 |
| staging batches | 451 ms | 441 ms | 20 |
| change plan | 9 ms | 52 ms | 1 |
| library pages | 0.2 ms | 49 ms | 50 |
| reconciliation | 8 ms | 0 ms | 0 |
| publication | 651 ms | 606 ms | 1 |

The warm `change_plan` result is intentional current behavior: it reads 50
pages of the 10,000-row committed library, then the existing bound sees
10,000 previous plus 10,000 observed records and returns before calling
reconciliation. The first run has an empty previous library and therefore
does call pure-Rust reconciliation. This is not evidence that reconciliation
is a warm bottleneck.

Subtracting the nested staging bucket from warm enumeration leaves about
7,924 ms for traversal and its adapter work. Comparing that with the 7,832 ms
filesystem-call sum leaves roughly 92 ms of traversal/adapter work outside the
timed public filesystem calls. The remaining worker setup, fences, page
handling, and result plumbing are small relative to the filesystem path but
are included in the end-to-end residual. Staging and publication together are
about 1,047 ms warm; staging includes DTO conversion, renewal/fence reads,
storage validation, inserts, and transaction commit, while publication
includes revalidation, reads, planning, row application, marker/job/stage
updates, cleanup, and the final durable transaction. Neither is a pure SQLite
measurement.

## Ranked hypotheses

| rank | hypothesis | evidence | status |
| ---: | --- | --- | --- |
| 1 | Full ancestor-chain revalidation is the dominant structural cost. | 26,163 validations reopen 60,973 links; the bucket is about 6.31-6.38 s of 7.83-7.94 s direct enumeration. `validate_ancestors` runs before every entry metadata read and child-directory open, and `next_entry` also validates the chain. | strongly supported by timing and call counts |
| 2 | Per-entry handle-bound metadata is the next filesystem cost. | 11,030 entry opens cost about 1.02-1.04 s, with about 71-72 ms of entry metadata queries; directory opens add about 93-95 ms. | supported, but smaller than rank 1 |
| 3 | Staging transaction shape contributes a material mixed residual. | 20 batches cost 441 ms warm. The phase includes conversion, fences/renewals, validation, one insert loop per batch, and `synchronous = FULL` commit. | measured phase, not isolated SQLite attribution |
| 4 | Atomic publication contributes a material mixed residual. | The complete publication call costs 606 ms warm and 651 ms first. It includes all fenced read/plan/apply/cleanup work and the durable commit. | measured phase, internal cost split unknown |
| 5 | Warm advisory change-plan reads are measurable but not dominant. | 50 indexed library pages cost 49 ms warm; reconciliation is skipped by the existing bound. | measured, not the failure driver |
| 6 | Host I/O or filter contention may explain variance between runs. | DriveFS and Defender were present and the window was not strict-quiet; this session has no controlled comparison and stable call counts. | possible external factor, not established |

## Focused optimization proposal

Prototype one traversal-scoped ancestor-validation fast path in the Windows
filesystem port: retain the already-open directory handle chain and validate
each handle's kind, reparse state, and qualified identity with handle-bound
metadata queries, rather than reopening every ancestor by name for every
cursor operation. Keep the current fail-closed path as the fallback whenever
the handle-bound checks cannot prove the same safety property. This targets
the only bucket that accounts for roughly four-fifths of direct enumeration;
it is a proposal for a separately gated experiment, not an implementation in
this spike.

The correctness risk is substantial. The current name-relative reopen checks
that every path component still resolves to the identity captured when the
cursor was opened and prevents a reparse or replacement from escaping the
root. A handle-only fast path must demonstrate equivalent protection against
directory replacement, reparse insertion, root replacement, rename races,
case-mode changes, access failures, and stale handles. It must also preserve
hardlink aliases, locator spelling, cancellation, and the distinction between
authoritative and partial outcomes.

Required tests before considering the proposal for production include the
existing root/child/ancestor replacement and junction tests, plus Windows
adversarial tests that replace or reparse each depth in a live traversal,
verify root identity at the end, exercise access-denied and disappeared
entries, preserve hardlink aliases, and compare the published snapshot with
the current implementation. The candidate must first run as a diagnostic A/B
on the same fixture and a strict quiet host; it must not be accepted from a
single faster run.

If the candidate fails to prove equivalent validation semantics, the next
specific experiment is to retain the current enumeration path and add
storage-internal diagnostic buckets for staging validation/insert/commit and
publication read/plan/apply/commit. That would determine whether the roughly
1.05-second storage phases contain a separately actionable cost without
mistaking the mixed residual for SQLite time.

## Evidence and delivery

Raw JSONL, build logs, process-window audits, session metadata, and retained
binary hashes are stored outside Git in the private validation-evidence
capture for this investigation. The committed report is sanitized and does
not include fixture paths, database contents, or host identifiers.

The reusable Rust validation cache was owned solely for this round and was
always selected explicitly through `CARGO_TARGET_DIR` as
`<reusable-validation-cache>`. The private evidence location is
`<warm-reconciliation-20260913-evidence>`. At the final handoff, the build
volume had 63.44 GiB free, the validation cache measured 6.83 GiB, and a
3-GiB rebuild allowance would leave 60.44 GiB above the 30-GiB reserve. No
second Rust cache or evidence target was created.

The full `pnpm check` command reached only its first toolchain gate and failed
because the worktree-local pinned Python environment points at a missing
temporary interpreter. Its independent remaining gates were run separately:
embedded SQLite, privacy, formatting, workspace lint, typecheck, workspace
tests, and the release build passed. The final diagnostics test and clippy
passes also passed with `--features diagnostics --all-targets`. The temporary
toolchain junctions used to inspect the clean worktree were removed after the
checks. The reusable Rust cache, evidence, retained binaries, fixture, and
ignored Node dependency install remain available; removing the cache would
require a full rebuild, so cleanup should be deliberate and path-verified.

Key retained SHA-256 values:

- diagnostic phase binary: `8c6fd21e7c2912c41e2c7d0b08cd80a76a2889343b42bc65f8013cb6584b3042`
- native profiler binary: `b0170afca5257f37e5db121868c14e22a879bc34d0526f561a5c4cb0fabbb082`
- end-to-end profiler raw JSONL: `55dc728604f448cedc3e8761c07ace9e863004ade1dc51c5f5b4535fa05e274a`
- native profiler raw JSONL: `6c421ddb9224c6acbbdf7aacdec73230ae14f3a865588819f287b81c8d12cf78`
- phase-timed raw JSONL: `cdc4917c1de5ec2749341a7dcc4a8535bb33af2604d4bf421806cbddab8b1c2a`

The validation cache is the reusable cache designated for this round and has
one owner. All Rust commands used the pinned Rust 1.98.1 toolchain and
`--locked`; no retained evidence target was used for compilation. No target,
limit, security setting, platform scope, or production scanning behavior was
changed.

# PR #94 performance validation (2026-09-09)

Status: independent validation evidence, contended shared-host only.
This report validates PR #94 without introducing another speculative
optimization. It does not merge, close the epic, change quotas, fixture
definitions, budgets, SQLite durability, or ancestor-validation guarantees.
`custom-9995` is treated as the existing comparison fixture, not as a newly
approved replacement for F1's accepted baseline.

Stack: PR #94 head `588867bca154e798f189f6c99de8a2668005d141` (base
`59faefc2806a725368a59e7b6fc9be7f863f4fec`, PR #93) with before
`1d7c29828d4560c959f4fc140b8544ce06c05b80` and candidate
`b35c01a1ab75381e8d1d562fafcafa4764ba0b6f`. This validation branch adds only
an 8-line diagnostic-scaffolding removal (see below) plus this evidence.

## 1. Audit of the existing measurement

All statistics below were recomputed from committed raw files with pinned
Node 24.20.0. Reported values match exactly.

### 1.1 Whole-scan ten-iteration protocol (uninstrumented `benchmark` path)

Both sides: `custom-9995`, manifest
`a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a`
(9,995 FLP + 5 aliases + 4 other = 10,000 observations), release `--locked`,
`benchmark` example without the `diagnostics` feature, 1 warm-up + 10
fresh-process measured iterations + 3 cancellations + 100 ms working-set
sampling. Authoritative publication 10/10 Complete on both sides.

| Metric | Before (`benchmark-before-raw.json`) | Candidate (`benchmark-after-raw.json`) |
| --- | --- | --- |
| Warm-up | 11,915 ms | 11,197 ms |
| Measured | 12019, 12021, 11766, 11986, 11639, 11609, 11627, 12188, 11647, 11705 | 11315, 13338, 14453, 17682, 11348, 11377, 11392, 11559, 11475, 16732 |
| Sorted | 11609, 11627, 11639, 11647, 11705, 11766, 11986, 12019, 12021, 12188 | 11315, 11348, 11377, 11392, 11475, 11559, 13338, 14453, 16732, 17682 |
| Median | 11,735.5 ms | 11,517 ms (-1.9%) |
| Max / nearest-rank p95 (n=10) | 12,188 ms | 17,682 ms (worsened, outlier-driven) |
| Cancellation stop latencies | 5, 6, 5 ms; median 5, max/p95 6 | 6, 6, 7 ms; median 6, max/p95 7 |
| Working-set increment | 53.0-53.1 MiB | 52.9-53.1 MiB |
| Build commit in raw | `1d7c298` | `b35c01a` |

No iteration was filtered. The candidate median movement is directional only;
p95 is a failure against the unchanged provisional `<= 10 s` budget. No
end-to-end p95 win is established.

### 1.2 Native diagnostic (instrumented direct enumeration, no storage)

Tool `profile-native-ops-v1` (`diagnostics` feature), 3 same-process runs per
side, direct `enumerate()` only. Must not be compared directly with
whole-scan numbers.

| Metric (median of 3) | Before | Candidate | Change |
| --- | --- | --- | --- |
| Enumeration wall | 8,942 ms | 8,728 ms | -2.4% |
| Timed native-call sum | 8,699 ms | 8,483 ms | -2.5% |
| Ancestor validation | 7,224.8 ms | 7,043.2 ms | -2.5% |
| Entry metadata | 106.161 ms | 77.649 ms | -26.9% |
| Calls/observations | 26,163 validations; 60,973 links; 14,108 queries; 11,030 opens; 11,030 metadata; 10,000 obs / Complete | identical | unchanged |

Recomputed sums match `native_call_sum_ms` exactly
(e.g. before run0 8,687.009 ms vs 8,687 ms; after run0 8,483.018 vs 8,483).
Reported ancestor median `7,038 ms` differs from recomputed `7,043.2 ms` by
~5 ms; entry-metadata and enumeration medians match. No material impact.

### 1.3 Identical fixtures, toolchains, profiles, feature flags

- Fixture: identical SHA, counts, `created` hardlink support on both sides.
- Toolchain: pinned Rust 1.98.1 MSVC, Node 24.20.0, locked deps on both sides.
- Build profile: release `--locked` both sides. Before build 22,908 ms cold,
  candidate 1,110 ms incremental in the original capture; profile identical.
- Feature flags: whole-scan driver uses default production port
  (`WindowsFilesystemPort::new`, diagnostics `None`) — uninstrumented.
  Native diagnostic uses `new_with_diagnostics` under `diagnostics` —
  instrumented. Separation is correct.
- Provenance nit: `optimization-20260909-provenance.json` records candidate
  as short `b35c01a`; full SHA is `b35c01a1ab75381e8d1d562fafcafa4764ba0b6f`.

### 1.4 Nested timers: no double counting

`validate_ancestors()` records one `AncestorValidation` span per public call;
`DirectoryQuery`, `EntryOpen`, `EntryMetadata`, `DirectoryOpen`,
`DirectoryMetadata`, `DirectoryCaseSensitivity`, `Root*` record only their
own syscall(s) outside validation. Inner ancestor opens/metadata stay inside
the ancestor span and are not re-recorded. `filesystem_nanos()` sums disjoint
spans; verified against raw sums. PR #93 `profile-fs-calls-v1` times the
public boundary inclusive of validation and must not be summed with the split
native spans.

### 1.5 Instrumented vs uninstrumented separation

- Instrumented: `profile-native-ops` direct traversal, same-process, no
  staging/publication/reconciliation. Answers "where in native calls".
- Uninstrumented: `benchmark` full scan (enumerate + stage + final apply),
  fresh-process per iteration, with working-set/cancellation. Answers
  "whole-scan wall time". The -26.9% applies only to the ~106 ms metadata
  slice (~1.2% of enumeration, ~0.3% of whole scan, ~28 ms absolute).

### 1.6 Why one fewer query does not establish a whole-scan p95 win

`read_handle_metadata` dropped only the unused `FILE_ATTRIBUTE_TAG_INFORMATION`
query; `FILE_BASIC_INFORMATION.FileAttributes` already carries the reparse and
recall/offline flags and the tag value was never consumed. Expected whole-scan
saving is ~tens of ms, not hundreds. Observed whole-scan median deltas
(-218 ms committed; -1,485 ms in the new contended window below) exceed that
envelope and track host variance instead. Committed p95 worsened 12,188 to
17,682 ms on outliers. A single-query removal therefore supports a narrow
correctness-preserving cleanup, not a p95 or 10-second qualification claim.

## 2. Isolated worktrees and prebuilds

- Isolated detached worktree at `1d7c298` with isolated
  `target-perf94-before` (before binary).
- Isolated detached worktree at `b35c01a` with isolated `target-perf94-after`
  (candidate binary).
- This validation worktree on branch `perf/94-validation-20260909` from
  `588867b` plus the correction below, with isolated `target-perf94-validation`.
- No reset or switch of another agent's checkout. The main checkout was left
  on `docs/41-performance-followup-20260908`.
- Prebuilds with pinned cargo 1.98.1, release, `--locked`, benchmark example:
  before 28.80 s, after 26.06 s, both isolated
  `target-*/release/examples/benchmark.exe` present. No concurrent cargo at
  build preflight for the first build; second build followed sequentially.

## 3. Timed-run window and host conditions (contended, not quiet)

Quiet-host requires pausing DriveFS, a fixture-volume antivirus exclusion,
stopping other agent/build activity, AC/high-performance power. Power was
High performance (`Alto desempenho`), CPU i7-10750H 12 logical, Windows
10.0.26200 x64 — matching prior provenance. The following quiet conditions
were unmet and are unauthorized to change here:

- Google DriveFS 2 processes active (78 MB + 19 MB class).
- Windows Defender `MsMpEng` + `MpDefenderCoreService` active; no
  fixture-volume exclusion applied.
- Epic launcher/helpers, 6x T3, 4-5x opencode agent processes active.
- Performance-counter detail unavailable without perf-log privilege;
  `Win32_Processor.LoadPercentage` sampled 2% when idle. Process presence
  alone is not presented as contention proof; observed wall-time variance
  below is the contention evidence.

Sequence: initial preflight had 0 cargo/rustc/desktop. A first before-run
was then contended by 3x cargo + 1x rustc (1.1 GB) started by sibling agents
mid-window (see outlier file). After they exited, before2/after2 were run
back-to-back with 0 cargo/rustc/desktop at each preflight but the background
activity above remaining. All runs are therefore contended shared-host
evidence, explicitly not idle-host qualification.

## 4. New A/B protocol runs (all samples retained, contended)

Same persistent fixture regenerated with pinned Node, SHA verified
`a4760a28...1196d08a`, same volume class. Harness
`scripts/run-benchmark.mjs` with `--bin` isolated binaries. `prebuilt:true`
in raw is mapped here: before2 = `1d7c298`, after2 = `b35c01a`.

| Metric | Before2 (`benchmark-validate-before2.json`) | Candidate2 (`benchmark-validate-after2.json`) |
| --- | --- | --- |
| Warm-up | 14,965 ms Published Complete 10,000 | 14,090 ms Published Complete 10,000 |
| Measured | 17671, 15109, 13114, 12747, 11661, 12574, 16203, 18092, 20098, 13641 | 12521, 13959, 12836, 12943, 12990, 13075, 12811, 12827, 12988, 12738 |
| Median | 14,375 ms | 12,889.5 ms |
| Max / p95 | 20,098 ms | 13,959 ms |
| Cancellation stop | 6, 5, 5 ms; median 5 max 6 | 6, 6, 7 ms; median 6 max 7 |
| Working-set inc | 53.0-53.3 MiB | 53.0-53.3 MiB |
| Authoritative | 10/10 Complete Published 10,000 | 10/10 Complete Published 10,000 |

Both fail the provisional 10 s p95. The candidate median/max movement in this
window is directional only and inherits the contended classification; it is
not a qualification. Variance proof: the earlier same-binary before run under
active sibling builds gave warm-up 35,823 ms, median 18,664 ms, max 61,868 ms,
cancellation max 12,382 ms (`benchmark-validate-contended-outlier.json`,
10/10 authoritative). That file is retained to show contention impact, not as
a comparison point.

`custom-9995` remains the existing comparison fixture. F1's accepted baseline
(10,000 FLP + 5 aliases = 10,005 observations vs 10,000-record fence) and its
owner decision are unchanged by this validation.

## 5. Narrow diagnostic correction in this branch

`1d7c298` introduced unconditional `pending_entries: VecDeque<OsString>` plus
a per-`next_entry` pop branch that is never populated. It allocates per cursor
and executes on every production `next_entry` even without `diagnostics`,
so the before/candidate whole-scan comparison measures a scaffolded baseline
rather than clean production. This branch removes only that dead scaffolding
(8 deletions in `crates/filesystem-enumeration/src/lib.rs`: import, field,
branch, three inits). No behavior, quota, identity, cancellation, or
durability change. Implementation change is diagnostic-only cleanup;
 coordination with Agent 1 (PR #94 owner) is requested via review — this
separate validation branch avoids overwriting `perf/93-enumeration-optimization`.

Checks on pinned toolchain, isolated target `target-perf94-validation`:

- `cargo fmt --all -- --check` pass
- `cargo test -p fruitboard-filesystem-enumeration --locked`: 44 passed, 2 ignored
- `cargo clippy -p fruitboard-filesystem-enumeration --all-targets --locked -- -D warnings` pass
- `cargo clippy -p fruitboard-filesystem-enumeration --features diagnostics --example profile-native-ops --locked -- -D warnings` pass
- `git diff --check` pass
- Full `pnpm check` not claimed (shared desktop build owned by another agent).

## 6. Recommendation: revise then keep as cleanup, no performance claim

- Keep the candidate's functional change: removing the redundant attribute-tag
  query is correct (attributes already in basic info, tag unused, junction
  regression `ntfs_fixture_metadata_marks_a_junction_as_reparse` passes,
  ancestor validation/identity/cancellation unchanged).
- Revise by accepting the 8-line dead-code removal in this branch (or
  equivalent in PR #94) so production no longer pays for an empty queue.
- Do not present the candidate as a whole-scan p95 improvement or 10-second
  qualification. Committed p95 worsened on outliers; new contended runs also
  fail 10 s. The next performance claim requires an owner-coordinated
  quiet-host A/B (DriveFS paused, AV exclusion, no sibling builds, AC/high
  performance) with the same 1 warm-up + 10 measured + 3 cancellation protocol
  on both commits.
- No change to quotas, fixture definitions, budgets, SQLite durability,
  ancestor-validation guarantees, or production scanning visibility.

## 7. Provenance

See `validation-20260909-provenance.json`. Raw files in this directory are
the authoritative sources; this markdown reports but does not replace them.

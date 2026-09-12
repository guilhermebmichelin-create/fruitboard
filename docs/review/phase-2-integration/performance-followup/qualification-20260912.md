# Performance qualification preparation - 2026-09-12

Status: **normal-configuration measurements captured; the coordinated
quiet-host gate failed; no quiet-host qualification claim.** This report was
prepared from reviewed
`origin/main` `3ebac5f7a76c3425620ceba59e6078b32fb6cd85` in isolated branch
`perf/41-qualification-20260912`. It does not change quotas, budgets, fixture
targets, security controls, cloud settings, production scanning, or the
bounded-scan protocol.

## F1 - fixture cardinality is reproduced

The generator was run with pinned Node `24.20.0`, seed `0`, and hardlink
support created. The manifest was validated and its canonical SHA-256 was
recomputed. The worker observes every `.flp` location, including hardlink
aliases; the four non-FLP files are not observations.

| Fixture role | FLP files | Alias locations | Total observations | Manifest SHA-256 | Status |
| --- | ---: | ---: | ---: | --- | --- |
| Accepted `baseline` safety case | 10,000 | 5 | **10,005** | `8c3d85ec01299995208abfa450378b1b37c704e423593d7a4370ec465254afba` | Overflow retained |
| Proposed quota-fitting alternative `custom-9995` | 9,995 | 5 | **10,000** | `a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a` | Alternative only |

Both manifests validate with `hardlinkSupport: created`, 1,000 leaf
directories, 1,025 directories, 10 empty directories, and 5 hardlink
groups. The manifest entry totals are 10,009 and 10,004 respectively because
they also include the four excluded non-FLP files.

The accepted overflow case remains safety evidence: the existing baseline
raw runs show `Failed / ResourceLimit` with no authoritative publication when
10,005 observations meet the enforced 10,000-record fence. The alternative
does not redefine the accepted target or rewrite that evidence. It is the
only fixture selected for the controlled #94 A/B because it fits the current
quota without a quota change.

## F2 - controlled A/B is prepared, but the quiet window is not open

The A/B isolates #94 using the exact shared-parent pair used by the #98
validation:

| Side | Commit | Relationship | Benchmark binary SHA-256 |
| --- | --- | --- | --- |
| Before | `1d7c29828d4560c959f4fc140b8544ce06c05b80` | PR #93 diagnostic parent | `9339afe117b6008a7589b3b33bcd65557ee29addd3c500700ae8f9e4f7622006` |
| Candidate | `b35c01a1ab75381e8d1d562fafcafa4764ba0b6f` | #94 duplicate metadata query removal | `c5f4898735efaffeb498210d03efab400b16fce299477f76a24840a549774e4a` |

Both sides use the same #93 diagnostic scaffold and default, uninstrumented
benchmark feature set. The #98 dead-scaffolding cleanup is intentionally not
part of this isolated comparison. The current reviewed `origin/main` remains
the qualification branch base; these historical detached worktrees are only
for the controlled A/B binaries.

Prebuilt artifacts use separate target directories and the pinned workspace
toolchain:

```text
cargo build --release -p fruitboard-scan-execution --example benchmark --locked
cargo build --release -p fruitboard-scan-execution --example profile-fs-calls --locked
```

Recorded toolchain values are `rustc 1.98.1 (48a229cea 2026-09-01)` and
`cargo 1.98.1 (797e8a9bc 2026-08-05)`. The benchmark binaries were built from
the exact commits above, with locked dependencies and release optimizations.
The same two commits also have prebuilt `profile-fs-calls` binaries for a
post-failure phase profile. The before profile was run after its normal timing
result; the current-main profile was run after its separate normal timing
result. Neither profile is a quiet-host or acceptance measurement.

The required measurement sequence, once Agent 1 coordinates an eligible
window, is one warm-up plus ten fresh-process measured iterations per side,
before then candidate, with the same persistent `custom-9995` fixture and
manifest, 100 ms working-set sampling, and three fresh cancellation samples.
Every sample is retained. Each side will report warm-up, run-order and sorted
scan times, median, maximum, nearest-rank p95, authority/status/location
counts, cancellation latency statistics, and memory samples.

The preflight captured at `2026-09-12T07:17:23-03:00` was not eligible:

| Gate | Observation | Result |
| --- | --- | --- |
| Power | High-performance plan; battery reported 100% | Pass for this snapshot |
| Build/driver processes | No `cargo`, `rustc`, benchmark, or Fruitboard process observed | Pass for this snapshot |
| DriveFS | Two `GoogleDriveFS` processes active | **Fail** |
| Antivirus | `MsMpEng` and `MpDefenderCoreService` active; exclusion state could not be inspected without administrator access | **Fail / unverified** |
| Other agent activity | Node/Codex and `opencode` activity present | **Fail** |
| CPU/disk idle proof | CPU snapshot was 3%; required 60-second CPU/disk sampling was not complete and the disk idle counter was unavailable | **Fail / unverified** |

No DriveFS/cloud setting, Defender/security control, or sibling process was
paused or changed. Therefore no scan timing from this branch is presented as
quiet-host evidence. A contended run would not answer F2 and is not a
substitute for the required window.

## Agent 3 first quiet-host window - aborted before measurement

The first-window preflight was captured at `2026-09-12T08:09:38-03:00` on
this host. The window was fail-closed before any build, benchmark, fixture
access, or timed scan:

| Gate | Observation | Result |
| --- | --- | --- |
| Power | Active scheme is High performance (`Alto desempenho`) | Pass for this snapshot |
| Build/driver processes | `cargo=0`, `rustc=0`, `tauri/Fruitboard=0`, `pnpm/npm=0` | Pass for this snapshot |
| DriveFS/cloud | `GoogleDriveFS=2` | **Fail; cloud activity not paused** |
| Antivirus control | `MsMpEng=1`, `MpDefenderCoreService=1`; fixture-volume exclusion was not verified | **Fail / unverified** |
| Sibling host activity | `T3 Code=5`, `codex=8`, `node=6` | **Fail; host not quiet** |
| CPU/disk idle proof | No qualifying 60-second CPU and fixture-volume disk-idle sample was captured | **Fail / unverified** |

DriveFS, Defender, sibling activity, and host controls were not changed. The
measurement sequence was therefore not started, and no partial timing,
cancellation sample, or raw measurement file was created in this window.

### Audit of existing results

The historical raw files were audited rather than rerun. They are not a
pre-existing quiet-host final result: `benchmark-before-raw.json` and
`benchmark-after-raw.json` are classified in their reports as contended, with
nearest-rank p95 values of 12,188 ms and 17,682 ms; the later validation pair
is likewise contended at 20,098 ms and 13,959 ms. Those historical reports and
raw files remain unchanged. They do not qualify the unchanged 10-second
target, and they do not satisfy the required first-window preflight.

## Normal-configuration measurements - not qualification evidence

The strict quiet-host recipe was unavailable, but the host could execute the
prepared binaries. To preserve actual evidence without mislabeling it, the
following runs are explicitly **normal configuration / contended shared-host
measurements**. They are not a quiet-host pass, an acceptance result, or a
re-budget. The same persistent `custom-9995` fixture and canonical manifest
were used for every side; each run used one warm-up, ten measured fresh-driver
iterations, three fresh cancellation databases, and 100 ms working-set
sampling. No installed tests or intentionally started sibling build was part
of these commands.

Agent 1's coordinated host sample was `2026-09-12 08:15:22.111` through
`08:16:25.247 -03:00` (63.136 s, 10 samples):

| Gate / activity | Observation | Interpretation |
| --- | --- | --- |
| Power | High-performance plan; AC online; battery 100% | Pass |
| CPU | 1%-6%, average 3.2% | Low aggregate CPU; pass for this gate |
| Fixture volume (`C:`) | 95.87%-99.83% idle | Low volume activity in this sample; pass for this gate |
| Build and driver presence | `cargo=0`, `rustc=0`, benchmark=0, Fruitboard/Tauri=0 throughout the sample | No heavy build/driver at that checkpoint |
| DriveFS/cloud | 2 `GoogleDriveFS.exe`, 9 related cloud processes, measurable I/O | **Requirement unmet; process presence was accompanied by activity** |
| Antivirus | `MsMpEng=1`, `MpDefenderCoreService=1`; real-time/on-access protection enabled; fixture exclusion not verifiable without administrator access | **Requirement unmet / unverified** |
| Agent/background workload | 16 Codex-related processes, 2 opencode-related, 6 T3, and 2 unrelated Node services; CPU/I/O activity was observed from Codex, opencode, T3, and Defender | **No-agent-load requirement unmet; actual workload, not presence alone** |

The scan runs also showed variable normal-host activity. Spot checks during
the current-main run ranged from 28% CPU and 99% `C:` idle to 21% CPU and 13%
`C:` idle (85% disk time, approximately 15.3 MB/s), and later 42% CPU and 82%
`C:` idle (approximately 65.9 MB/s). These are system-level observations,
not attribution of a particular scan outlier to one process. A first candidate
attempt was stopped when a sibling cargo/rustc wave appeared; its incomplete
output was not retained. The completed candidate and current-main runs had no
cargo/rustc/installed-driver process observed at their start, listed spot
checks, or completion, but the runner did not retain a continuous host trace,
so this is not a proof of absence for every millisecond.

### Exact provenance and raw artifacts

| Side | Source / production meaning | Benchmark binary SHA-256 | Profile binary SHA-256 |
| --- | --- | --- | --- |
| Before | `1d7c29828d4560c959f4fc140b8544ce06c05b80` (#93 diagnostic parent) | `9339afe117b6008a7589b3b33bcd65557ee29addd3c500700ae8f9e4f7622006` | `e8983c3f6140f802d3832f1e2af9a39f0cb79f86cb972b9d6fbafee531d7a387` |
| Candidate | `b35c01a1ab75381e8d1d562fafcafa4764ba0b6f` (#94) | `c5f4898735efaffeb498210d03efab400b16fce299477f76a24840a549774e4a` | `9d8a6fabf10a82241387a3df1f89529e7b31bfe0ba6bc928a12be67115678355` |
| Current merged | Production tree of `origin/main` `3ebac5f7a76c3425620ceba59e6078b32fb6cd85`; qualification branch `2561d3f` is documentation-only on top | `34347c0e79face9e61d7adf8bd0663318d1e4f6509f6d081331075ee01fc1025` | `f08afcdeaa5732d4629cba24c430d459872065b723bdef9608706f47bfac1d9f` |

All release artifacts use locked dependencies and the workspace-pinned
Windows toolchain: `rustc 1.98.1 (48a229cea 2026-09-01)`, `cargo 1.98.1
(797e8a9bc 2026-08-05)`, and Node `v24.20.0`. The current-main rebuild was
performed with `cargo build --release -p fruitboard-scan-execution --example
benchmark --example profile-fs-calls --locked` in its separate target
directory. The normal benchmark command was:

```text
<repo>\.tools\node-v24.20.0-win-x64\node.exe scripts\run-benchmark.mjs --manifest <fixture>\custom-9995\manifest.json --bin <verified-benchmark.exe> --iterations 10 --warmup 1 --cancel-iterations 3 --cancel-after-ms 250 --settle-ms 300 --memory-interval-ms 100 --out <raw-report.json>
```

The prepared profile command was:

```text
<verified-profile-fs-calls.exe> --root <fixture>\custom-9995 --db <profile-db> --runs 2 --out <profile-raw.jsonl>
```

The raw reports are sanitized by the harness (no absolute paths, usernames,
or hostnames):

| Raw artifact | SHA-256 |
| --- | --- |
| [`benchmark-normal-before94-raw.json`](benchmark-normal-before94-raw.json) | `14eeeff2aa044b8f7529abae141a6a3851eb7699f3724e150dedf80e1528c905` |
| [`benchmark-normal-candidate94-raw.json`](benchmark-normal-candidate94-raw.json) | `4e2fac9ead9ed59de104b6a5eb2150df4dd5ca166aa37c52d99126301564556d` |
| [`benchmark-normal-current-main-raw.json`](benchmark-normal-current-main-raw.json) | `0174f9081dea7d46c7caf984f31bc17becb87a3f4d51b9234bdcd461d1f2c6fd` |
| [`profile-normal-before94-raw.jsonl`](profile-normal-before94-raw.jsonl) | `d27010c8d1a864addfbdff9b70c989b3fdd0ecf3dd0068cf62acb91f60a150bd` |
| [`profile-normal-candidate94-raw.jsonl`](profile-normal-candidate94-raw.jsonl) | `42c132dd1937929fb4b8126ab6e592cf61c61027717a5d5816573aa9d91d4f2d` |
| [`profile-normal-current-main-raw.jsonl`](profile-normal-current-main-raw.jsonl) | `3186382520a56ae938455535c0af2e9eda47298ba80c6b15a8a5f4a8221cf170` |

### Raw result summary

The linked raw JSON retains every line and every iteration. Scan times below
are in run order; `F` marks a non-authoritative record that is still retained.
The harness computes statistics over authoritative measured iterations and
does not silently discard failures.

| Side | Warm-up | Ten measured scan times (ms) | Authority/status | Median (ms) | Max / nearest-rank p95 (ms) | Cancellation stop latencies (ms) | Measured incremental working set |
| --- | ---: | --- | --- | ---: | ---: | --- | ---: |
| Before `1d7c298` | 11,184 | 10,474, 10,582, 16,301, 12,213, 19,935, 44,778, 19,663, 11,049, 36,704, 13,457 | 10/10 Published / authoritative | 14,879 | 44,778 / 44,778 | 17, 6, 16 (median 16, p95 17) | 53.3 MiB peak measured increment |
| Candidate `b35c01a` | 9,962 | 10,264, 10,130, 9,973, 10,252, 10,125, 10,115, 10,046, 10,010, 9,928, 10,391 | 10/10 Published / authoritative | 10,120 | 10,391 / 10,391 | 7, 7, 7 (median/p95 7) | 53.1 MiB peak measured increment |
| Current merged `origin/main` | 13,968 | 17,918, 19,088, 14,982, 18,644, 25,601, **11,137 F**, 11,248, 12,046, 12,053, 11,686 | 9/10 Published / authoritative; 1/10 Failed / Partial / non-authoritative | 14,982 (n=9) | 25,601 / 25,601 (n=9) | 54, 9, 6 (median 9, p95 54) | 53.2 MiB peak measured increment |

The `custom-9995` candidate is an alternative fixture only: 9,995 FLP files
plus five aliases produce 10,000 observations. It does not replace the
accepted 10,000-FLP / 10,005-observation overflow safety case, change quotas,
or claim the 100,000-entry target.

### Interpretation and measured cost

The normal-config before/candidate result is directionally favorable to the
candidate (median 14,879 ms to 10,120 ms and p95 44,778 ms to 10,391 ms), but
it is not an isolated optimization result: the sides ran at different times,
with uncontrolled DriveFS, Defender, agent workload, and disk activity. The
before-side outliers are therefore not assigned to a specific process, and no
promotion or optimization decision follows from this A/B. Even the candidate
normal p95 is 391 ms above the unchanged 10,000 ms target.

The current merged implementation was measured separately, not inferred from
the historical A/B. It has a normal-config authoritative p95 of 25,601 ms
over nine successful measured iterations and one retained non-authoritative
partial failure. It remains over target and unqualified. The one failed
iteration is an execution outcome, not a filtered timing sample.

The prepared profiles were run on the same fixture and normal host class. The
candidate profile also had no cargo/rustc process at launch or completion; no
continuous host trace was retained for it:

| Profile | Warm scan | Filesystem-port time | Residual | Dominant measured calls |
| --- | ---: | ---: | ---: | --- |
| Before `1d7c298` | 9,985 ms | 8,670 ms | 1,315 ms | `next_entry` 4,016.015 ms + `read_metadata` 4,348.683 ms = 8,364.699 ms (83.8% of wall; 96.5% of filesystem time) |
| Candidate `b35c01a` | 9,235 ms | 8,022 ms | 1,212 ms | `next_entry` 3,718.374 ms + `read_metadata` 4,005.431 ms = 7,723.806 ms (83.6% of wall; 96.3% of filesystem time) |
| Current merged | 9,484 ms | 8,272 ms | 1,211 ms | `next_entry` 3,826.490 ms + `read_metadata` 4,162.643 ms = 7,989.133 ms (84.2% of wall; 96.6% of filesystem time) |

The profile measures public filesystem-port boundaries, not individual native
syscalls; residual includes staging, reconciliation reads, publication, and
worker overhead. The measured evidence supports a focused, safety-preserving
traversal-context/ancestor-validation investigation. Any candidate must keep
handle-relative opens, identity checks, reparse fences, cancellation, and
authoritative publication rules, with explicit revalidation at safety
boundaries. Do not cache away a safety check, weaken SQLite durability, or
implement the unapproved chunked-snapshot protocol.

The `Get-Counter` paths used by the earlier preparation were unavailable on
this Windows image (`object not found` / `counter not found`); the host gate
used `Win32_PerfFormattedData_*` CIM counters instead. Working-set values are
driver-process working set, not incremental private bytes in the desktop app.
OS cache state was not controlled, so no run is called cold.

## Focused improvement gate and recommendations

The normal measurements fail the unchanged 10-second warm-p95 target, but the
before/candidate difference is confounded by host timing and background load.
The three fresh profiles nevertheless reproduce a stable measured cost center:
`next_entry` plus `read_metadata` consume about 84% of scan wall time and over
96% of timed filesystem-port time. This supports a focused investigation of a
correctness-preserving traversal context or ancestor-validation optimization,
not a broad rewrite. Re-measure any candidate on the same host with paired
order/control runs and the strict quiet gate.

F1 recommendation: keep the accepted 10,000-FLP baseline and its 10,005-
observation `ResourceLimit` run as overflow safety evidence. Keep
`custom-9995` explicitly alternative-only; do not redefine targets, quotas, or
the accepted fixture.

F2 recommendation: do not promote either normal-config A/B result or the
current-main result. Obtain a dedicated/eligible host window satisfying the
owner recipe (DriveFS paused, fixture-volume AV exclusion verified, no agent
load or sibling heavy build, AC/high-performance, and documented cache state),
then repeat the exact one-warm-up/ten-measured protocol. If that environment
cannot be supplied, the exact unmet prerequisite is an owner-approved host or
an explicit decision to relax the recipe; no acceptance claim follows here.

F3 recommendation: explicitly defer 100,000-entry qualification with a dated
owner checkpoint. If the owner still requires it, approve #96 slice 1's
coverage/staging schema, migration, retention/cleanup DDL, and tests before
separate design and measurement of bounded planning and atomic publication.

No source change is implemented or approved by these measurements. Any
optimization must retain handle-relative opens, identity checks, reparse
fences, cancellation, and authoritative-outcome rules, with explicit
revalidation at safety boundaries. Do not cache away a safety check, weaken
SQLite durability, raise quotas, or implement the unapproved chunked-snapshot
protocol.

## F3 and bounded-scan proposal #96

The current implementation still enforces 10,000 worker observations,
10,000 staged records, 4 MiB staged path bytes, 512 records per batch, and a
10,000-entry advisory plan buffer. A 100,000-entry qualification is therefore
unsupported and remains unmeasured. No quota or budget was raised and no
100,000-entry scan was attempted.

Recommendation: explicitly defer 100,000-entry qualification until the owner
confirms it is required and accepts a dated checkpoint. If it remains
required, approve only #96 slice 1 first (coverage/staging schema,
migration, retention and cleanup DDL plus tests), followed by separate design
decisions and review for:

- bounded global identity/reconciliation planning across chunks;
- a non-tautological coverage denominator and close-out rule;
- cross-chunk rename, hardlink, replacement, and conflict assignment;
- fencing and responsiveness during the single atomic publication;
- measured lock, journal, disk, cleanup, crash, restart, and backup behavior;
- explicit path-byte, staging, coverage-TTL, and history-retention bounds.

The new chunk protocol is not implemented here. Paged reads alone are not
evidence of bounded planning, and a publication per chunk would violate the
existing atomic snapshot contract.

## Decisions ready for owner review

| Finding | Decision status from this work |
| --- | --- |
| F1 | Reproduced: accepted baseline is 10,005 observations versus the 10,000 fence. Keep it as overflow safety evidence; evaluate `custom-9995` only as a labeled alternative. No fixture decision is applied here. |
| F2 | Normal-config before/candidate/current-main raw measurements and phase profiles were captured, but the coordinated quiet-host gate failed. The 10-second target remains unchanged and unqualified; no optimization or acceptance decision is made. |
| F3 | Unsupported under current bounds and unmeasured. Recommend explicit dated deferral unless the owner accepts #96's staged design work; do not raise quotas or implement the new protocol here. |

No merge, production scanning activation, or qualification claim is made by
this report.

# Performance qualification preparation - 2026-09-12

Status: **preparation evidence only; no quiet-host measurement or
qualification claim.** This report was prepared from reviewed
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
post-failure phase profile; no profile has been timed in this preparation
window.

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

## Focused improvement gate

The existing #93 diagnostic evidence remains the profile basis if the quiet
A/B still fails the unchanged nearest-rank p95 target: filesystem-port work
was measured at 86.53%-88.46% of diagnostic wall time, and `next_entry` plus
`read_metadata` accounted for 84.69% of the 12,157 ms warm diagnostic. The
native candidate reduced the entry-metadata slice from 106.161 ms to 77.649 ms
but did not establish an end-to-end p95 win. The focused hypothesis is a
correctness-preserving traversal-context optimization for repeated ancestor
validation, with explicit revalidation at safety boundaries; it must retain
handle-relative opens, identity checks, reparse fences, cancellation, and
authoritative-outcome rules.

No source change is justified by the current contended evidence. If the quiet
A/B fails, first run the prebuilt phase profile on the same fixture and host
window, then measure any narrowly scoped candidate with the same A/B protocol.
Do not cache away a safety check or weaken SQLite durability.

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
| F2 | No quiet-host pass yet. The controlled A/B is prebuilt and gated on Agent 1's coordinated quiet window. The 10-second target remains unchanged and unqualified. |
| F3 | Unsupported under current bounds and unmeasured. Recommend explicit dated deferral unless the owner accepts #96's staged design work; do not raise quotas or implement the new protocol here. |

No merge, production scanning activation, or qualification claim is made by
this report.

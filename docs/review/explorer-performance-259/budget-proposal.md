# Phase 3 native resource budget proposal

Status: **Proposal for owner review; not accepted qualification targets.**
This table gives a concrete decision to review after the quiet #259 measurements.
Merging a query optimization alone does not accept this table or the Phase 3
checkpoint. Record an explicit owner decision in the checkpoint before describing
any result as passing approved resource budgets.

## Timed boundaries and candidate targets

| Surface                  | Constructed matrix                                                                 | Proposed warm p95 | Additional proposed process-lifetime memory target                                                  |
| ------------------------ | ---------------------------------------------------------------------------------- | ----------------: | --------------------------------------------------------------------------------------------------- |
| Plugin Explorer          | 100/1,000/2,000 current entries, one Sampler reference each; 2,001 limited entries |            200 ms | Report separately; no Explorer memory ceiling proposed from this narrow matrix                      |
| Native analysis worker   | Approved sample, 4,601,596 and 25,000,000 bytes                                    |          1,000 ms | Parent and parser separately: each at most 128 MiB peak working set and 128 MiB peak private commit |
| Native analysis worker   | 64 MiB input bound                                                                 |          3,000 ms | Same separate 128 MiB targets                                                                       |
| Supervised parse request | All four constructed file sizes                                                    |          2,000 ms | Parser target above; keep the existing 10-second hard request deadline                              |

Explorer includes the actual native command and outer JSON envelope
serialization, including its internal response-budget serialization. It excludes
webview IPC, rendering and fixture setup. The proposed 200 ms target deliberately
uses the existing saved-query comparison as the candidate; this does not
retroactively make that baseline an accepted Explorer command budget.

The native worker includes held-source observations/hashes, supervised
health/descriptor/parse validation, the probe's child-memory observation and
SQLite publication. It excludes job discovery/claim, scanner enumeration,
desktop lease-renewal monitor, IPC/rendering and setup. The supervised parse
timer excludes preceding health/descriptor requests and child-memory observation.
The fixed benchmark clock does not qualify lease renewal or shutdown races.

Memory targets apply to the two native **probe** processes, not the complete
desktop/WebView2 application. Working set and private commit are different
counters; process-lifetime peaks include setup. Parent/child maxima are not a
simultaneous combined RAM measurement. The proposed 128 MiB gives each process
headroom above this 64 MiB-input run's approximately 71/68 MiB resident peaks;
it is not a proof of maximum memory for diverse real plugin state.

## Why these candidates

The quiet optimized Explorer 2,000-entry p95 is 104.06 ms against a 332.56 ms
quiet baseline. A 200 ms candidate keeps margin without hiding the previously
slow result. The native worker's 25 MB and 64 MiB p95 observations are about
404 ms and 1,164 ms, respectively; 1/3-second candidates leave room above those
observations. The request candidate is distinct from the unchanged hard deadline.
These are reviewable proposals, not universal performance promises.

Each case records a first request, one excluded warm-up and ten warm samples;
nearest-rank p95 for ten samples is the maximum. It does not reliably estimate a
production tail. Preserve all samples and machine/power/noise observations.
The owner app must be closed; other agent/build/test workloads must be quiescent.
The OS cache is not flushed, so no cold-disk claim is made. Use independent
retained release binaries with source/hash provenance and the storage preflight.

## Acceptance work still open

- Owner approval or revision of the proposed boundaries/targets.
- Broader representative compatibility and plugin-payload/reference coverage;
  opaque padding and repeated Sampler references only characterize this matrix.
- Whole installed-app startup, UI/IPC latency and desktop memory qualification.
- Existing scanner performance failures and all previously recorded gaps; no
  scanner target or acceptance is changed by this proposal.
- Phase 3 checkpoint review with the compatibility matrix, error/recovery
  evidence and retained limitations. Later phases remain gated.

Issue [#259](https://github.com/guilhermebmichelin-create/fruitboard/issues/259)
remains open for that budget/qualification review. The implementation and quiet
observations are recorded in [the performance report](README.md).

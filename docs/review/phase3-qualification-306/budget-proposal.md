# Current FLP intelligence resource decisions

Status: **Proposals only; no budget or phase acceptance.**
Date: 2026-10-11. Baseline: merged PR305,
`0e86b8efb2ccdeeed37caec37ca1ac175c2a2583`.

The protocol and these candidates were recorded before collecting this review's
results. Later observations cannot silently change targets, omit slow samples,
or convert a failed quiet-host guard into qualification. This supplements the
[native proposal](../explorer-performance-259/budget-proposal.md); it preserves
that proposal and all historical scanner failures.

## Native candidates carried forward

| Boundary                                           | Matrix                                               | Candidate warm p95 |
| -------------------------------------------------- | ---------------------------------------------------- | -----------------: |
| Explorer command plus outer envelope serialization | 100/1,000/2,000 complete entries; 2,001 limited      |             200 ms |
| Native worker through SQLite publication           | Approved sample, 4,601,596 and 25,000,000 bytes      |           1,000 ms |
| Native worker through SQLite publication           | Constructed 64 MiB bound                             |           3,000 ms |
| Supervised parse request                           | All four sizes; preceding health/descriptor excluded |           2,000 ms |

Worker parent and parser each retain proposed **128 MiB** process-lifetime peak
working set and peak private commit limits. Their peaks are separate, include
setup, and must not be added into a simultaneous RAM measurement. The hard
ten-second parser deadline remains unchanged. Fixed-clock worker probes do not
qualify desktop lease renewal or shutdown races.

## Whole installed-app candidates

| Boundary                                                                                                 | Matrix                                                                |                                   Candidate |
| -------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- | ------------------------------------------: |
| Process creation to observed populated selected Library, including debugger discovery and root selection | Populated profile, 100/1,000/2,000 repeated approved sample files     |                           Warm p95 1,500 ms |
| Renderer invoke through returned Explorer envelope                                                       | Same fully analyzed roots; native work, JSON and WebView IPC included |                             Warm p95 300 ms |
| Explorer refresh activation through ready DOM plus next animation frame                                  | Same roots; renderer validation and rendering included                |                             Warm p95 500 ms |
| Mixer details refresh through ready DOM plus next animation frame                                        | Approved named mixer fixture, sixteen ordinary saved records          |                             Warm p95 100 ms |
| Concurrent whole-app steady process sum                                                                  | Selected 2,000-entry root, Explorer open; ten-second window           | Working set 512 MiB; private commit 384 MiB |
| Concurrent whole-app sum during genuine supervised analysis                                              | Constructed 64 MiB input; preparation excluded                        | Working set 768 MiB; private commit 512 MiB |

These whole-app candidates add headroom above the historical seven-entry
observations: startup 718.59 ms, steady working set/private commit
375.88/220.70 MiB, and the 25 MB analysis window 414.60/248.08 MiB. They are
reviewable starting points, not extrapolated guarantees of large-library or
real-project performance. No whole-app limits were previously accepted.

The process sum includes the owned desktop, parser when present and descendant
WebView2 processes at each sample. Working set sums can count shared pages more
than once; private commit is not physical RAM. Requested memory sampling is
100 ms, with process-tree refresh every second. Actual intervals, membership
and process start identities are retained; brief processes and peaks can be
missed. Instrumentation is outside the application sum and its overhead is
reported. Other populated roots remain in the profile and are disclosed.

## Evidence rules and decisions

Each latency series has a first request, one excluded warm-up and ten measured
warm samples. Nearest-rank p95 is their maximum; this small single-host sample
does not estimate a production tail. OS caches are not flushed. Populated launch
is not fresh-profile or cold-disk startup. DOM plus an animation frame is not
hardware display paint. Installation, copies, scan, initial analysis, screenshots
and accessibility checks are outside timers.

The native precondition requires no active application/build/test overlap and
CPU load at most 20 percent. Installed windows also record initial processor
load against that limit after a five-second settling interval. A window above
that limit can collect characterization, but cannot pass the quiet-host guard.
Every measurement window retains process CPU
deltas and a per-process 0.5 CPU-second unrelated application/build/test guard.
Only explicitly identified instrumentation and owned measured processes are
excluded. A failed guard remains characterization, even if timings meet a
candidate. CPU snapshots cannot certify terminated or inaccessible workloads.
Installed measurements hold the shared host lock; all artifacts are independent
retained copies, with source/configuration/hash provenance.

The owner can accept or revise these **specific boundaries and candidates**
after reviewing results. Approval must be recorded with date, scope and any
unmet cases. It does not accept broader FLP compatibility, scanner performance,
the Phase 3 exit, production activation, licensing or distribution. Those
remain separate decisions in the checkpoint and gap ledger.

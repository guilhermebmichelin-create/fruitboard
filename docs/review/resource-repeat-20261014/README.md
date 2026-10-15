# Phase 3 resource repeat, 2026-10-14

Status: **Worker guarded observations available; Explorer and installed resource
qualification incomplete. No budget or Phase 3 acceptance.**

Related issues: [#306](https://github.com/guilhermebmichelin-create/fruitboard/issues/306)
and [#259](https://github.com/guilhermebmichelin-create/fruitboard/issues/259).
This supplements the [original packet](../phase3-qualification-306/README.md)
and [merged guard preparation](../resource-qualification-runner-306/README.md).
Original samples, failed guards and checkpoint decisions remain intact.

## Result

The independently reviewed October 11 worker matrix passed the unchanged
initial-load and background-CPU guards. All four cases were below their proposed
worker, supervised-request and separate parent/parser memory limits. This is
useful guarded evidence, not approval of the proposals or the whole application.

The October 14 native Explorer repeat returned complete results for
100/1,000/2,000 entries and a bounded refusal at 2,001. Its recorded timings were
below the proposed 200 ms, but background activity failed the guard. This session
answered owner questions during the attempt; T3 and agent CPU were counted.
It does not establish whether a fully idle T3 session could pass.

Installed preparation processed the nineteen approved files and all sample
copies in the 100- and 1,000-entry roots through actual scan, supervised analysis
and immutable publication. The attempt then stopped: the desktop did not exit
within the harness's ten-second graceful-close wait after the 1,000-entry case.
The published preparation record preceded the close failure. The cause is under
investigation; neither a larger timeout nor forced cleanup establishes a pass.
No new installed latency, startup or memory window was collected.

A separate debugging repeat completed the same 19/100/1,000 preparation and
closed the exact installed processes in 87.3/69.0/60.1 ms. It did not reproduce
the timeout or establish a product fix. A later native Explorer attempt with
all agents idle also failed the unchanged guard; its 2,000-entry p95 was
276.87 ms. Both follow-ups are retained below alongside the original failure.

The [sanitized results](results.json) retain all first, excluded warm-up and ten
measured native samples, comparisons, noise failures and exact source/artifact
identities. Raw paths, process identities, database contents and logs remain in
private evidence outside Git and compiler caches.

## Native observations

| Worker input bytes     | Worker warm p95 | Supervised parse warm p95 |
| ---------------------- | --------------: | ------------------------: |
| 46,703 approved sample |         9.75 ms |                   1.90 ms |
| 4,601,596 constructed  |        97.74 ms |                  38.44 ms |
| 25,000,000 constructed |       445.56 ms |                 195.91 ms |
| 67,108,864 constructed |     1,163.68 ms |                 537.50 ms |

The worker window began at 7% processor load and had no observed unrelated
process above 0.5 CPU-seconds. Its ten-second hard parser deadline is unchanged.
At 64 MiB, parent peak working set/private commit was 71.58/66.53 MiB; parser
peaks were 68.17/64.86 MiB. These are separate process-lifetime peaks, include
setup and cannot be added into simultaneous whole-app memory.

| Explorer input entries | Command plus outer serialization warm p95 | State                         |
| ---------------------- | ----------------------------------------: | ----------------------------- |
| 100                    |                                  11.23 ms | Complete                      |
| 1,000                  |                                 119.20 ms | Complete                      |
| 2,000                  |                                 195.00 ms | Complete                      |
| 2,001                  |                                   2.83 ms | Limited; no partial aggregate |

Explorer began at 16% processor load. The background guard failed for three T3
processes at 6.375, 6.703125 and 1.53125 CPU-seconds, the agent CLI at 0.640625,
the desktop shell at 0.890625 and a graphics helper at 0.53125. Each independently
exceeded the unchanged 0.5 limit. No name-based exemption was introduced.

An earlier repeat with T3 closed also failed: a background audio helper used
0.625 CPU-seconds. Its original observations remain retained separately. The
helper was subsequently quit normally; the next attempt preserved T3 at the
owner's instruction. These runs are not controlled before/after evidence of
an application speedup or a qualified latency defect.

### Idle active-session follow-up

After all three subagents finished and installed cleanup released the host, a
separate attempt stopped before samples because initial CPU load was 23%, above
the unchanged 20% guard. Chrome was closed normally before that attempt; T3 stayed
open. The stopped attempt and receipts remain intact.

The next attempt retained samples even if its precondition failed. Root paused
for 120 seconds, with no chat, agent, build or test activity during the recorded
native window. Initial load was 24%; three T3 processes consumed 2.828125,
9.265625 and 0.96875 CPU-seconds, each above 0.5. The shared assessor classified
it as characterization. Only the collector's exact process/start identity was
excluded; T3 received no exemption.

| Explorer input entries | Native command plus serialization warm p95 | State                         |
| ---------------------- | -----------------------------------------: | ----------------------------- |
| 100                    |                                    9.51 ms | Complete                      |
| 1,000                  |                                  101.06 ms | Complete                      |
| 2,000                  |                                  276.87 ms | Complete                      |
| 2,001                  |                                    3.95 ms | Limited; no partial aggregate |

All correctness assertions passed. The 2,000-entry result exceeds the proposed
200 ms comparison, but the failed guard prevents a qualification or regression
claim. This demonstrates failure during one idle active-session attempt, not
that every fully inactive session or another host would fail. Do not repeat
qualification while this session or other host work exceeds the guards.

## Installed attempt and harness corrections

The package used the existing owned review identity with a fresh profile; it
did not use the production profile or seed a database. Four approved mixer
cases retained sixteen ordinary saved records. Scale preparation required
current metadata and complete counts, not just file enumeration. Its completed
cohort before the close failure was 1,119 entries: nineteen approved files plus
100 and 1,000 independent byte-identical approved samples. The planned
3,119-entry startup/steady-memory cohort was not reached.

Three private harness issues were corrected and separately validated before
the attempt:

- A Hashtable byte inventory could not be measured by `Measure-Object Bytes`.
  The corrected sum was checked against the retained inventory and exercised
  during an actual populated-profile archive with matching hashes.
- The packaged desktop was compared to unpackaged compiler output. Expected
  installed identities now pin the original installer and independently retained
  installed executable copies. Reinstallation reproduced all three installed
  hashes; the separately retained unpackaged desktop is not substituted.
- An asynchronously redirected PowerShell process could lose an observable
  exit code. Retaining its process handle immediately preserved the exit status
  in runtime checks and an actual installed archive/setup/cleanup run.

The successful small installed checks do not qualify graceful shutdown at larger
scale. The later 1,000-entry close failure remains visible. The original
source-equivalent 2,000-entry correctness and 2,001-entry refusal evidence remains
historical; this attempt did not repeat it.

Cleanup removed only the owned review installation and registration, retained
the database unchanged through uninstall, and released the installed host lock.
The nineteen approved source hashes and protected profile inventories matched.
The independent scheduled controller removed its own task and preserved T3.
Cleanup success is not a successful timed run or proof of graceful app shutdown.

### Shutdown reproduction and owned recovery

The separate debugging sequence used the same independently retained installed
package and unchanged application source, with a fresh owned review profile.
Before each close it verified terminal scan/analysis state and the exact process
ID, image path and UTC creation time. The 19/100/1,000 processes accepted one
graceful close and exited in 87.3/69.0/60.1 ms; CDP disconnected normally. The
1,000-entry root had one completed scan and all 1,000 analysis jobs complete.
These observations do not identify the cause of the original timeout, and no
product fix is supported or implemented.

The debugging runner itself ended with exit code 1 after successful app exit:
private cleanup read a timeout-only optional field from a successful-close
receipt under PowerShell StrictMode. The original error remains preserved.
Recovery validated optional-field reads against both receipt shapes, then
uninstalled only the owned review installation. It sent no second close or
forced termination. The separately recorded recovery verified the current
database, 671 protected-profile files, nineteen source hashes and 283 archived
prior-profile files, removed the owned registration and released its host lock.
Root independently rechecked the current database and the original failed
attempt's database in that archive. No performance collector ran in this repeat.

## Source, limits and remaining work

The source is merged PR308, `8e9d90d`; its reviewed head was `890d149`.
The compiled application graph matches merged PR307 `4fadc55` and retained
PR305 artifacts built at `0e86b8e`. The shared guard, every prepared script,
independent retained binary and original installed artifact identity was checked.
The fifteen passing PR308 checks cover the identical merged source tree and
unchanged application. A new PR still requires its normal updated-head checks.

The [existing proposals](../phase3-qualification-306/budget-proposal.md) and
guard thresholds are unchanged. Every observed non-owned process is considered,
including system/service, T3 and agent work. Exclusions require exact recorded
process/start identities for the measurement or instrumentation. Boundary
snapshots still miss terminated or inaccessible processes. Ten warm samples
have nearest-rank p95 equal to their maximum; OS caches were not flushed.
Repeated approved samples test scale, not representative FLP diversity.

Next, collect pending installed windows only after the host satisfies both
resource and storage preconditions. Retain exact-process close diagnostics; if
the timeout recurs, record elapsed time around ordered native shutdown phases
before proposing a behavioral fix. The current repeat does not support guessing
which host caused the earlier timeout. A measurement needs an actual quiet
period; T3 can remain open and its CPU still counts. All agent/build/test work
must stop before qualification. Keep failed windows and samples if that host
cannot satisfy the guard; do not weaken it.

The [next-slice scope packet](../phase3-next-slices-20261014/README.md) separately
defines G3/G5/G6 work. It does not close those gaps. Resource qualification,
explicit budget decisions, full Phase 3 exit and production/distribution remain
pending. Historical scanner failures and the G8 ledger are carried forward.

The private handoff records the single reusable validation cache, owner
Codex/root, exact source/evidence paths and retention duties. No local rebuild
or new compiler cache was needed for this report. Preflight reserved 30 GiB
after 1.50 GiB expected output; completion recorded 37.93 GiB free. Fixtures,
databases, profiles, raw samples, logs, binaries and cancelled/failed attempts
remain outside disposable caches. No compiler cleanup or rebuild cost was added.

The debugging handoff later recorded 33.65 GiB free. The last native follow-up
preflight passed with 31.34 GiB and a 0.25 GiB output estimate; completion recorded
30.63 GiB. A subsequent host check fell below the 30 GiB reserve while Windows
servicing was active; the cause of the capacity change was not established.
New build/fixture windows stopped. Resume them only after a fresh passing
preflight; preserve all evidence and do not terminate system servicing to obtain
a benchmark. Lightweight report work creates no compiler cache.

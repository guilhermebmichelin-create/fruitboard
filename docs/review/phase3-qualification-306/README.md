# Current Phase 3 resource and checkpoint review

Status: **Mixed quiet observations and characterization; qualification incomplete.**
Date: 2026-10-11. Issue
[#306](https://github.com/guilhermebmichelin-create/fruitboard/issues/306),
following [#259](https://github.com/guilhermebmichelin-create/fruitboard/issues/259).
Application baseline: merged PR305,
`0e86b8efb2ccdeeed37caec37ca1ac175c2a2583`.

## Owner summary

The installed development app returned complete Explorer results for 100,
1,000 and 2,000 fully analyzed projects. At 2,001 projects it refused aggregation
without showing partial totals. The settled Explorer and mixer refresh windows
passed the declared quiet-host guards and met the proposed limits.

Startup, memory and native-worker results remain characterization. The final
native Explorer window passed its quiet guards but measured **203.61 ms** at
2,000 entries against the proposed 200 ms. That is a current measured candidate
shortfall, following two retained noisy windows. Targets were not raised and
failed attempts were retained. Broader Phase 3 gaps remain open.

Review the [budget proposal](budget-proposal.md) and
[checkpoint decisions](checkpoint.md) separately. Merging this evidence does not
accept budgets or Phase 3. [Results](results.json) retain first requests, excluded
warm-ups, all ten warm samples, guards and sanitized provenance.

## Installed app: settled refresh windows

| Selected root entries | Explorer IPC warm p95 | Explorer refresh warm p95 | Quiet guard | Result                                 |
| --------------------- | --------------------: | ------------------------: | ----------- | -------------------------------------- |
| 100                   |              24.00 ms |                  25.30 ms | Passed      | Complete counts; meets proposed limits |
| 1,000                 |             177.30 ms |                 178.60 ms | Passed      | Complete counts; meets proposed limits |
| 2,000                 |             297.60 ms |                 299.00 ms | Passed      | Complete counts; meets proposed limits |

IPC includes native work, JSON and WebView transport through the returned
envelope. UI refresh includes validation and rendering through ready DOM plus
the next animation frame. Candidates remain 300 ms and 500 ms respectively;
the 2,000-entry IPC observation leaves only 2.40 ms below its candidate.
Mixer details refresh was **15.00 ms** warm p95 against the proposed 100 ms,
with a passed quiet guard and an unchanged immutable details snapshot.

Initial scan/analysis and copies were outside these timers. All roots were
terminal before each settled refresh window, and scan status/publication
boundaries were equal before and after it. The earlier series is retained as
preliminary setup observations because scan quietness was not proven. Its
2,000-entry IPC p95 was 348.60 ms and failed the quiet guard; it is not substituted
for the settled result or erased.

## Installed startup and memory: characterization

| Selected root entries | Populated launch warm p95 | Quiet guard |
| --------------------- | ------------------------: | ----------- |
| 100                   |               1,402.76 ms | Failed      |
| 1,000                 |               1,188.10 ms | Failed      |
| 2,000                 |               1,327.91 ms | Failed      |

Launch includes process creation, debugger discovery/attachment, explicit root
selection and observation of the selected Library's first project. Root choice
is not persistent. Each profile was populated before measurement; caches were
not flushed. This is neither fresh-profile nor cold-disk startup. The observations
are below the proposed 1,500 ms but do not qualify that limit.

| Concurrent whole-app window                         | Sampled peak working set | Sampled peak private commit | Quiet guard |
| --------------------------------------------------- | -----------------------: | --------------------------: | ----------- |
| Selected 2,000-entry Explorer open, ten seconds     |               436.11 MiB |                  243.38 MiB | Failed      |
| Actual supervised 64 MiB reanalysis and publication |               574.64 MiB |                  358.11 MiB | Failed      |

The profile held 3,119 committed entries in four roots before the large-input
case, which added one entry. The app, parser when observed and descendant
WebView2 processes are summed at each sample. These values are below the
proposed steady 512/384 MiB and analysis 768/512 MiB, respectively, but do not
qualify them. Requested sampling was 100 ms and tree refresh one second;
actual intervals and observed membership are retained in results. Peaks and
brief processes can be missed. Working-set sums can double-count shared pages;
private commit is not physical RAM.

## Current native results: guarded Explorer shortfall and worker characterization

| Explorer entries | First noisy matrix warm p95 | Repeated noisy matrix warm p95 | Final guarded matrix warm p95 | Result                                      |
| ---------------- | --------------------------: | -----------------------------: | ----------------------------: | ------------------------------------------- |
| 100              |                    11.82 ms |                       11.28 ms |                      11.19 ms | Ready; complete                             |
| 1,000            |                   121.18 ms |                      109.71 ms |                     102.46 ms | Ready; complete                             |
| 2,000            |                   207.14 ms |                      256.00 ms |                     203.61 ms | Ready; guarded result above proposed 200 ms |
| 2,001            |                     3.66 ms |                        6.55 ms |                       3.71 ms | Limited; no partial aggregation             |

The first two full matrices failed their background CPU guards. The final matrix
passed both initial CPU and background guards after an additional settling
interval. Its 2,000-entry p95 missed the candidate by 3.61 ms (1.81%). Profile
that exact boundary before a separate performance implementation PR.
Additional attempts were
refused before collecting samples when initial processor load exceeded 20%.
Names containing “quiet” in private attempt labels describe the intended run,
not its outcome. The historical PR261 quiet 104.06 ms observation belongs to
its older source; it is not current-head qualification or a controlled
before/after comparison with the expanded current application.

| Worker input bytes     | Worker warm p95 | Supervised parse warm p95 |
| ---------------------- | --------------: | ------------------------: |
| 46,703 approved sample |        12.60 ms |                   2.38 ms |
| 4,601,596 constructed  |       105.46 ms |                  45.96 ms |
| 25,000,000 constructed |       492.90 ms |                 239.41 ms |
| 67,108,864 constructed |     1,408.74 ms |                 736.74 ms |

The worker window explicitly collected characterization at initial 27% CPU,
above the unchanged 20% guard, and also failed the background CPU guard.
At 64 MiB, parent lifetime peaks were 71.56/66.53 MiB working set/private commit;
parser peaks were 68.26/64.97 MiB. All recorded worker/request/individual-process
values were below their proposed limits. These are separate lifetime peaks,
not simultaneous whole-app memory. Fixed-clock probes exclude desktop lease
monitoring and shutdown races; the hard ten-second parser deadline is unchanged.

## Correctness, provenance and limits

The installed run used actual scan, supervised parsing, immutable publication
and renderer commands under a fresh review identity; its database was not
seeded. All nineteen approved corpus copies reached terminal analysis states:
sixteen had current metadata and three retained expected negative states.
All four approved mixer cases returned sixteen ordinary saved records.
The scale cases used independent byte-identical copies of the approved sample;
they test count/transport/rendering scale, not diverse representative FLPs.

The 2,001-entry refusal ran last, outside all timers. Full committed Library
pagination verified 2,001 entries, and Explorer returned no counts, groups or
entries. Full analysis is unnecessary for a refusal based on committed entry
count; pending owned work was stopped with the app before uninstall.

Every latency series records the first request, one excluded warm-up and ten
warm measurements. Nearest-rank p95 equals the largest of those ten values.
The declared guards remain initial CPU at most 20% and at most 0.5 CPU-seconds
per unrelated application/build/test process. Coding-interface activity was
not silently excluded. Boundary CPU snapshots omit terminated or inaccessible
processes, so a passed guard has that limitation.

Optimized native probes and an analysis-enabled unsigned NSIS package were
built from the baseline source with pinned tools. The existing parser binary
was independently copied after source/locked-graph equivalence verification.
Artifact hashes, source/tree and baseline CI references are in results; exact
commands, configuration, raw logs, process inventories, fixtures and database
remain in private evidence outside Git and compiler caches. Sampler/pagination
setup failures were corrected in the private harness; application code is
unchanged. Failed attempts remain available and are described in results.

All nineteen approved source hashes and personal profile inventories remained
unchanged. Only the owned review installation/registration was removed; its
database remained byte-identical through uninstall and the shared host lock
was released. The single reused cache remains owned by Codex `/root`; no new
permanent compiler cache or manual compiler cleanup was introduced. Capacity
at uninstall was 33.22 GiB free; the later final native matrix used additional
retained database space. The private handoff records current capacity and the
final storage preflight, with the 30 GiB reserve maintained.

## Validation and next work

Application/corpus/lockfiles are unchanged from PR305's tested head `9c6e190`;
its fifteen checks passed and its merged source is this application baseline.
Current optimized probes, installed correctness and measurements supplement
that CI. Documentation/privacy/script/policy checks validate this packet;
normal CI on the final PR head is still required. A redundant full local
`pnpm check` was not run for documentation-only changes.

Keep #259 and #306 open for the remaining qualification. Profile the native
2,000-entry query's measured candidate shortfall and propose a focused fix.
Repeat worker, startup and memory in a coordinated quiet window, retaining
the same boundaries and targets. A final worker repeat was refused before
measurement because initial CPU again exceeded 20%; existing fixture copies
and all earlier closed probe databases remain retained for reuse. Broader genuine
compatibility and remaining G3/G5/G6 work still need their own scope/evidence.
Historical scanner failures and all later-phase/release gates remain unchanged.

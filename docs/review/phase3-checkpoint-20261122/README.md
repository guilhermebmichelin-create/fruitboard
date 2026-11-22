# Phase 3 checkpoint proposal

Date: 2026-11-22. Review the opt-in Windows Library with local NTFS roots and
the exact saved-build and field limits in the
[field reliability matrix](../flp-intelligence-readiness/field-reliability.md).
Phase 3 exit remains pending.

## Installed startup

The runner reused the hash-verified unsigned analysis-enabled installer
from [28fc508](https://github.com/guilhermebmichelin-create/fruitboard/commit/28fc508),
also used for [the installed memory measurements](../installed-memory-306/README.md).
This package predates the Home and navigation copy changes in PR431.
It loaded an independent copy of the populated review database from the
[October 19 run](../resource-remainder-20261019/README.md).
The original database and all 3,121 source fixture hashes were preserved.

The endpoint was the expected first project heading in the populated
Library after the runner selected the intended root. Timing began before
process creation and included debugger discovery. A persistent helper
was ready before each series. Each size had one initial launch, one
excluded warm-up, and ten measured warm launches. The operating system
caches were retained.

| Projects in selected root |  Warm p95 | Proposed target | Full-window noise guard |
| ------------------------- | --------: | --------------: | ----------------------- |
| 100                       | 719.16 ms |        1,500 ms | Failed                  |
| 1,000                     | 737.78 ms |        1,500 ms | Failed                  |
| 2,000                     | 729.88 ms |        1,500 ms | Failed                  |

The background precheck passed for each series. Over each full window
Windows Explorer consumed 1.53–1.86 CPU-seconds, above the unchanged
0.5 CPU-second per-process guard. WebView2 processes without a verified
owned identity also exceeded the guard. These timings are characterization.
All 36 launches reached the endpoint, exited after one graceful close
request within ten seconds, and produced valid eight-stage shutdown traces.
The review package was uninstalled and the dedicated test profile was restored.

The next startup run needs an exclusive Windows session that passes the
full-window guard across all three sizes. Record exact package provenance and
use the same root-selection endpoint. Preserve this attempt and the earlier
failures. Avoid further repeats in a session with the same measured noise.

## Native Explorer disposition

[PR #430](https://github.com/guilhermebmichelin-create/fruitboard/pull/430)
batched current metadata reads while preserving freshness, root scoping, and
response bounds. In the guarded follow-up recorded in the
[Phase 3 status](../development-status-20261121/README.md), the complete
100-, 1,000- and 2,000-entry results measured 10.31, 88.93 and 161.96 ms warm p95.
The 2,000-entry value came from a separate repeat after an earlier guard
failure. At 2,001 it returned an explicit limit with no partial aggregate.

Recommend adopting the **200 ms warm p95** target for the native Explorer
command plus outer envelope serialization, and resolving
[#259](https://github.com/guilhermebmichelin-create/fruitboard/issues/259)
after owner review. The comparison covers the recorded fixture matrix and
boundary. Installed IPC, rendering, startup and scanner throughput retain
their separate criteria.

## Scope and decisions

- **Proposed Phase 3 scope.** Keep the opt-in local Windows Library.
  Acceptable fields and unsupported states follow the maintained matrix
  for exact saved builds 24.1.0.4225, 25.1.3.4922 and 26.1.0.5530.
  Use the current saved facts, bounded note and automation summaries,
  ordinary mixer records, within-root sample checks, and Plugin Explorer.
- **Proposed deferrals.** Keep [#399](https://github.com/guilhermebmichelin-create/fruitboard/issues/399)
  for additional exact-build support once the registered installers and
  independently verified genuine saves are available. Track broader sample
  resolution separately because it adds filesystem access and interpretation
  rules. Split richer arrangements and duration, MIDI, automation targets
  and curves, and mixer effects into field-specific work backed by genuine
  fixtures.
- **Remaining evidence.** [#306](https://github.com/guilhermebmichelin-create/fruitboard/issues/306)
  still needs guarded installed startup and the outstanding installed timing
  and native parser-worker windows against the declared
  [resource proposals](../phase3-qualification-306/budget-proposal.md).
  Passing installed memory observations and the 2,001-entry correctness
  check retain their source boundaries. Historical scanner performance,
  DriveFS, FAT32 and cross-volume identity remain separate.
- **Owner decisions.** Review the native target, approve or revise the first
  supported scope and each deferral, and review the remaining measurements
  before accepting the Phase 3 exit. Keep production activation, licensing,
  signing and distribution as separate decisions. Phase 4 grouping and
  version history follow explicit exit acceptance.

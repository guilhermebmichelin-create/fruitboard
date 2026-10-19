# Current-source installed resource review, 2026-10-18

**Result: installed correctness and shutdown checks passed, and four guarded latency windows were collected. Resource budgets, complete resource qualification, and the Phase 3 exit remain unapproved.** Issues [#306](https://github.com/guilhermebmichelin-create/fruitboard/issues/306) and [#259](https://github.com/guilhermebmichelin-create/fruitboard/issues/259) remain open.

The fresh installed run completed the 21-file approved cohort, checked 100-, 1,000- and 2,000-entry roots, and recorded four latency windows whose unchanged all-process guards passed. A later 100-entry startup window met the proposed timing candidate but failed the same CPU guard. The run stopped there; it did not collect the remaining startup, whole-app memory, 64 MiB analysis-memory, or 2,001-entry refusal cases. The complete sanitized measurements and hash receipts are in [results.json](results.json).

## Source and installed package

The application baseline is merged PR331 at commit d90b660, tree cb13f50. The fresh analysis-jobs package was built and tested at commit 839e5fb, tree 8e8d292. Its compiled application graph, dependencies, lockfiles and approved corpus are equivalent to d90b660; the only four differences are documentation files for the separate #332 relative-path design. All 15 hosted checks passed on 839e5fb. Those checks cover unchanged application code, not installed performance, and the documentation PR still needs its own updated-head CI.

The unsigned NSIS installer hash is f993f02d358a27c8ab0894b3d149ac44460000a3285ba72eb2d2e599018bfc66. The post-Tauri desktop executable is 133553dd5c9072b7390f67fa4f032dc70ff386b0a3290c8da7251cea71ccf16a; it is not the installed executable identity. The installed desktop hash, independently observed in two installs, is 05f518bcbd4d0600c4745f199d24ad9f36a58148b56099a51679ce77731f5bd1. The parser and smoke sidecar hashes, package configuration hash, build receipt hash, sizes and toolchain versions are recorded in the results. A bare prebundle Cargo executable was not captured, so no identity is inferred from one.

## Correctness and scale

All 21 approved fixtures passed installed terminal-analysis and Explorer-state assertions: 18 entries were complete/current and three typed parser failures were failed/stale. The partial parser result for FIX-RB-UNKNOWN.flp was published as complete analysis and current Explorer data. The failed-fixture mapping follows the source contract: a typed failure is saved as negative history, so Explorer reports stale; no_analysis means no saved history. See the [parser corpus tests](../../../crates/flp-parser/tests/corpus.rs), [analysis tests](../../../crates/storage-sqlite/src/analysis/tests.rs), [Explorer storage tests](../../../crates/storage-sqlite/src/plugin_explorer/tests.rs), and [registered corpus contract](../../../fixtures/parser-corpus/manifest.md).

The three real installed scale roots contained 100, 1,000 and 2,000 entries. Their current states, counts and groups matched the expected results. Together with the 21-file base cohort, the profile held 3,121 entries. Each scale root repeats one approved sample file, so this checks list size, not broader project diversity. The 2,001-entry refusal was not collected.

This is the current 21-file corpus, including the approved F22 and F23 cases. It is not the older 19-file cohort from the October 14 repeat; the old report remains unchanged. The installed run used independent byte copies and generated no new fixture. Source and staged fixture hashes remained unchanged.

## Guarded latency observations

The pre-recorded installed candidates are 300 ms for renderer IPC, 500 ms for Explorer refresh through ready DOM plus the next animation frame, 100 ms for mixer-details refresh, and 1,500 ms for populated-profile startup. The four windows below passed the unchanged quiet-host and all-process CPU guard. Their nearest-rank p95 values are below those candidates:

| Installed boundary                                        | Entries or records | Warm p95 | Candidate | Initial CPU load | Guard  |
| --------------------------------------------------------- | -----------------: | -------: | --------: | ---------------: | ------ |
| Renderer IPC, including native work, JSON and WebView IPC |                100 |  13.8 ms |    300 ms |               4% | Passed |
| Explorer refresh through ready DOM and next frame         |                100 |  22.3 ms |    500 ms |               4% | Passed |
| Renderer IPC                                              |              1,000 | 114.9 ms |    300 ms |               2% | Passed |
| Explorer refresh through ready DOM and next frame         |              1,000 | 107.0 ms |    500 ms |               2% | Passed |
| Renderer IPC                                              |              2,000 | 197.1 ms |    300 ms |               5% | Passed |
| Explorer refresh through ready DOM and next frame         |              2,000 | 202.5 ms |    500 ms |               5% | Passed |
| Mixer details refresh, sixteen ordinary saved records     |                 16 |  12.9 ms |    100 ms |               4% | Passed |

Populated-profile startup for 100 entries measured 1,105.6 ms p95 against the 1,500 ms candidate, but it is **characterization only**. Although initial processor load was 5%, the all-process guard failed: Explorer used 2.03125 CPU-seconds, two Chrome processes used 1.140625 and 1.203125 CPU-seconds, and Office Click-to-Run used 14.421875 CPU-seconds. Each exceeded the unchanged 0.5 CPU-second limit for an unrelated process. T3/FL Studio remained open and counted; no process-name exception was used. Later qualification windows were stopped.

Each latency series includes its first request, one excluded warm-up and ten warm samples; the JSON preserves every value at recorded precision. Nearest-rank p95 is the maximum of these ten values. The run used one host and did not flush operating-system caches. Startup used a populated profile and includes debugger discovery; it is not a cold-start result. DOM plus a frame is not hardware display paint.

The sampler also recorded incidental working-set and private-commit observations during latency windows. Those samples are **not** the proposed steady-state or 64 MiB memory qualification: requested intervals were 100 ms, process-tree refresh was 1,000 ms, observed intervals varied, and short-lived peaks can be missed. Working-set sums may count shared pages more than once, and private commit is not physical RAM. No memory pass is claimed.

## Shutdown, cleanup and prior attempts

All 21 close receipts show one graceful close request, exit code 0, and a validated eight-stage native trace within the unchanged 10,000 ms deadline. There were no retries. The receipt elapsed-time p95 was 721.0451 ms (maximum 1,401.0116 ms); native trace shutdown p95 was 28 ms (maximum 41 ms). Sanitized receipt and trace hashes are listed in results.json; raw traces and process identities remain private.

The owned review installation and registration were removed and its host lock released. No owned desktop, parser or WebView process remained. The 671 protected profile files and all approved source/staged fixture hashes matched. The final database was independently retained by hash and was unchanged by uninstall. The unexplained October 14 timeout while closing after the 1,000-entry root was not reproduced here; this evidence does not identify a cause or support a product-fix claim.

Two earlier private-runner failures remain distinct. An unsupported PowerShell no-clobber parameter stopped one launch before the runner or app started. A first installed attempt then used an incorrect expectation that every failed analysis should appear as no_analysis. It observed stale for FIX-RB-LIMIT.flp and stopped; source tests establish stale as the right state for the three typed failures. That attempt recorded no successful exit or shutdown trace, and no recovery close was sent. Neither failure is evidence of a product defect. Both attempts and their raw evidence are retained privately.

## Storage and decisions

This documentation packet required no local build or cache write. Its final four-path storage preflight allowed 0.05 GiB of output and maintained the 30 GiB free-space reserve. The single reusable validation cache has one owner; the primary build output remains preserved. A previous exact-path retirement removed 5.76 GiB of verified obsolete compiler intermediates after checking 1,875 protected hashes; the rebuild cost of those retired intermediates is unmeasured. Installed binaries are independently retained, and raw fixtures, profiles, databases, logs and traces remain outside Git and disposable compiler caches.

These observations do not approve the proposed budgets, establish native worker or Explorer qualification, close #259 or #306, accept the historical Scanner p95 of 10,173 ms against its unchanged 10,000 ms target, satisfy the Phase 3 checkpoint, or authorize Phase 4. The [October 14 repeat](../resource-repeat-20261014/README.md), [original #306 review](../phase3-qualification-306/README.md), [budget proposal](../phase3-qualification-306/budget-proposal.md), and [checkpoint](../phase3-qualification-306/checkpoint.md) preserve prior results and decisions.

# Installed Windows memory observations

Date: 2026-11-22. Related issue
[#306](https://github.com/guilhermebmichelin-create/fruitboard/issues/306).
Both observed peaks were below their proposed limits and both windows passed
the all-process noise guard. Installed startup and broader resource
qualification remain open.

## Source and measurement boundaries

The retained unsigned Windows review installer was built at
[28fc508](https://github.com/guilhermebmichelin-create/fruitboard/commit/28fc508)
with the `analysis-jobs` feature. Its SHA-256 was
`c048a36b41bb31f1cb71a6491a0ae6c0686df76d7a01674b802fe5a155f6f091`.
Both runs verified the installer and installed desktop and parser hashes.
[PR #431](https://github.com/guilhermebmichelin-create/fruitboard/pull/431)
has since changed Home and Library navigation copy. The observations below
are pinned to the retained package.

The 64 MiB window started just before explicitly requesting a second analysis
of the retained constructed fixture. It ended once Project Details reported a
new, completed snapshot. The new saved version and channel count matched the
previous result. The parser was present in all nine in-window memory samples.
The fixture hash matched before and after the run.

For steady memory, the runner independently copied the populated database
retained by the [October 19 review](../resource-remainder-20261019/README.md).
The app confirmed all expected entries and current saved references for the
1,000- and 2,000-entry roots before opening Plugin Explorer on the 2,000-entry
root. The window then held the populated view open for ten seconds. All 63
in-window samples included WebView2. The original database and 3,121 source
fixture hashes matched before and after replay.

A fresh setup earlier in the session timed out waiting for all 1,000
analyses. The retained database showed 1,000 discovered locations, 681
complete snapshots and 126 queued jobs. No memory window ran in that attempt.
The replay exercised saved-result display and steady memory with a fully
populated profile. Fresh population throughput remains outside this result.

## Results

| Window                                        |  Duration | Samples | Working set peak / proposed limit | Private commit peak / proposed limit | Noise guard |
| --------------------------------------------- | --------: | ------: | --------------------------------: | -----------------------------------: | ----------- |
| 64 MiB explicit analysis through new snapshot |  1,367 ms |       9 |                  431.91 / 768 MiB |                     247.07 / 512 MiB | Passed      |
| 2,000-entry Explorer steady state             | 10,009 ms |      63 |                  417.95 / 512 MiB |                     234.01 / 384 MiB | Passed      |

The processor started at 1% and 3%, respectively. The existing guard
counted all observed unrelated process CPU deltas against 0.5 CPU-seconds
per process. Exclusions were tied to process ID and start time. Each sample
sums the installed desktop and observed descendants. Requested sample and
tree-refresh intervals were 100 ms and 1,000 ms. Actual in-window gaps were
139–298 ms during analysis and 132–296 ms in steady state, so brief peaks can
be missed. Summed working sets can count shared pages multiple times.

Both sessions used the installed host lock, closed gracefully with valid
eight-stage shutdown traces, uninstalled their test app and restored the
existing 100-project review profile with its original database hash.
Evidence remains outside Git and compiler caches. No native rebuild or cache
write was needed.

Next, use a populated review profile to measure startup at 100, 1,000 and
2,000 entries through visible root selection and a populated Library.
Complete the remaining installed timings and review the separate
[proposed budgets](../phase3-qualification-306/budget-proposal.md) before the
Phase 3 checkpoint.

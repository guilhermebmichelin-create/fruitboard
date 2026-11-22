# Installed Windows Explorer and mixer latency

Date: 2026-11-22. Related issue
[#306](https://github.com/guilhermebmichelin-create/fruitboard/issues/306).
All four installed latency windows passed the existing CPU guard and met their
proposed limits. Guarded startup remains open.

## Source and boundaries

The runs reused the hash-verified unsigned Windows analysis-jobs installer
from [28fc508](https://github.com/guilhermebmichelin-create/fruitboard/commit/28fc508)
with the same binaries as the [installed memory run](../installed-memory-306/README.md).
This package predates PR431's Home and navigation copy changes.
Each run used an independently copied populated review database from the
[October 19 run](../resource-remainder-20261019/README.md).

The runner selected each intended root after scans settled. It verified that
all 100, 1,000 and 2,000 entries had current saved metadata, and that the group,
match and reference counts were complete. Renderer IPC measures the WebView
invoke through the returned envelope, including the native operation and JSON.
Refresh measures button activation through the ready DOM and the next animation
frame. The initially populated view and fixture preparation precede timing.
Each series retained its first measured request, one excluded warm-up and ten
warm requests. The first request is from an already populated app.

## Results

| Surface                                          |  Warm p95 | Proposed target | Noise guard |
| ------------------------------------------------ | --------: | --------------: | ----------- |
| Explorer IPC, 100 entries                        |  15.90 ms |          300 ms | Passed      |
| Explorer IPC, 1,000 entries                      | 215.80 ms |          300 ms | Passed      |
| Explorer IPC, 2,000 entries                      | 286.80 ms |          300 ms | Passed      |
| Explorer refresh, 100 entries                    | 161.20 ms |          500 ms | Passed      |
| Explorer refresh, 1,000 entries                  | 183.10 ms |          500 ms | Passed      |
| Explorer refresh, 2,000 entries                  | 329.40 ms |          500 ms | Passed      |
| Mixer details refresh, 16 ordinary saved records |  15.10 ms |          100 ms | Passed      |

Each Explorer size shared one guarded window for its paired IPC and refresh
series. The first run finished all Explorer sizes, then stopped before the
mixer timer because the driver searched the first Library page only. The mixer
retry advanced through Library pages before starting its separate guarded
window and verified that the saved mixer result stayed the same.

Both sessions held the installed host lock, closed gracefully with valid
eight-stage traces, uninstalled their review app, and restored the existing
test profile byte-identically. All 3,121 source fixture hashes matched before
and after both sessions. Raw timings, CPU receipts and fixture inventories
remain outside Git and the compiler cache. No rebuild or Cargo cache write
was needed.

The 2,000-entry IPC row had 13.20 ms of headroom on this host and corpus.
Review the [proposed budgets](../phase3-qualification-306/budget-proposal.md)
with their recorded boundaries. These windows cover WebView IPC and refresh
in the running app. Installed startup, production activation and Phase 3 exit
retain their separate decisions.

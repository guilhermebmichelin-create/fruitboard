# Native parser-worker resource results

Date: 2026-11-22. Related issue
[#306](https://github.com/guilhermebmichelin-create/fruitboard/issues/306).
All four native worker windows passed the per-process CPU guard and observed
latency and memory were below the proposed native targets.

## Provenance and boundary

Built the release worker from merged
[`f03dce1`](https://github.com/guilhermebmichelin-create/fruitboard/commit/f03dce155f7fab06aee0892361a1d8c50cde050f)
using the pinned Rust 1.98.1 MSVC toolchain and
`cargo build -p fruitboard-analysis-execution --example resource_probe --release --locked`.
The independently copied worker SHA-256 was
`283b8955fc1090ddb6af7e7dc5f24b43c93d613ae55ab81f63ff7a3464a26e24`.
The retained release parser SHA-256 was
`8c70539cc447bd508c84f8a7fd17286b796f13e50b8a203f562da651e22e385d`.
Its source, root Cargo manifest, lockfile and toolchain inputs matched its
recorded `73132da` build.

Each size had its own initial-load check and CPU window, one first request,
one excluded warm-up, and ten measured warm requests. The worker measurement
ends at SQLite publication. The supervised parse measurement excludes the
earlier health and capabilities requests. The worker checks the published
content hash, and the harness verified all 48 fixture copies after execution.
The enlarged inputs are constructed workloads.

## Results

| Input bytes | Worker warm p95 | Supervised parse warm p95 | Worker peak working set / commit | Parser peak working set / commit |
| ----------: | --------------: | ------------------------: | -------------------------------: | -------------------------------: |
|      46,703 |         9.37 ms |                   1.76 ms |                  8.36 / 2.61 MiB |                  4.56 / 1.07 MiB |
|   4,601,596 |        83.79 ms |                  37.25 ms |                 12.28 / 7.02 MiB |                  8.75 / 5.23 MiB |
|  25,000,000 |       427.92 ms |                 216.93 ms |                31.65 / 26.42 MiB |                28.20 / 24.72 MiB |
|  67,108,864 |     1,090.56 ms |                 521.72 ms |                71.90 / 66.84 MiB |                68.36 / 64.99 MiB |

The [proposals](../phase3-qualification-306/budget-proposal.md) are 1,000 ms
worker warm p95 through 25 MB, 3,000 ms at the 64 MiB bound, 2,000 ms for
supervised parsing, and 128 MiB each for peak working set and private commit
of each process. These remain owner decisions. The memory values are
per-process lifetime peaks and cannot be added to infer concurrent app memory.
The guard includes all observed unrelated process CPU deltas using PID and
start time. Its boundary snapshots can miss terminated or inaccessible tasks.

The worker uses a fixed clock for its lease. Installed app startup, WebView2
rendering, lease renewal, shutdown and broader saved-build compatibility retain
separate evidence. See the
[installed memory review](../installed-memory-306/README.md)
and [Phase 3 follow-through](../development-status-20261121/README.md).

The sole reusable compiler cache was used sequentially and passed the
storage preflight again after Cargo-created aliases were replaced with
verified byte-identical independent files. The executable, fixtures,
databases, samples and guard receipts remain outside Git and the cache.

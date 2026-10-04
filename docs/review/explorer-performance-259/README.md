# Plugin Explorer query preparation and quiet resource measurements

Issue [#259](https://github.com/guilhermebmichelin-create/fruitboard/issues/259)
follows the [#258 characterization](../metadata-resources-258/README.md).
SQLite now reuses prepared metadata queries for input capture, current-snapshot
selection, bounded snapshot loading and history pagination. SQL predicates,
parameters, validation, transaction boundaries and size limits are unchanged.
Every call binds its selected source and executes against current database
state; no metadata result or freshness decision is cached.

The native Explorer command at 2,000 entries improved from 332.56 ms to 104.06 ms
warm p95, approximately **68.7% less time**, with complete counts. Both sides ran
on the same machine with Fruitboard closed and no overlapping local builds/tests.
All raw samples and component profiles are in [results.json](results.json).
These are quiet observations against explicitly proposed targets, not acceptance
of resource budgets, general compatibility or the Phase 3 checkpoint.
See the [budget proposal](budget-proposal.md) for the owner's review decision.

## Explorer before and after

| Entries | Before first | Before warm p95 | After first | After warm p95 | Result                     |
| ------- | -----------: | --------------: | ----------: | -------------: | -------------------------- |
| 100     |     16.11 ms |        16.06 ms |     5.88 ms |        4.69 ms | ready; complete counts     |
| 1,000   |    160.52 ms |       154.20 ms |    48.74 ms |       48.68 ms | ready; complete counts     |
| 2,000   |    344.91 ms |       332.56 ms |    96.94 ms |      104.06 ms | ready; complete counts     |
| 2,001   |      2.97 ms |         2.90 ms |     2.99 ms |        2.71 ms | limited; no partial totals |

Subsequent component profiles identified database reading as the main cost.
These separate series run after the full-command matrix and are not substituted
for the full-command samples. Component medians are not additive p95 estimates.

| 2,000-entry component                               | Before warm median | After warm median |
| --------------------------------------------------- | -----------------: | ----------------: |
| Storage read and freshness checks                   |          276.02 ms |          50.41 ms |
| Projection and internal output-budget serialization |           41.85 ms |          39.28 ms |

Outer envelope serialization is recorded separately in the raw profiles.
The full response still serializes to 932,564 bytes at 2,000 entries; bounds,
grouping/counts and authority are not weakened to reach the comparison.
Lifetime working-set peaks were approximately 20.68/20.70 MiB before/after
at that size. This optimization targets repeated SQL preparation, not broad
query batching, payload memoization or new indexes/migrations.

## Native analysis before and after

Four file sizes ran in fresh independently prepared before/after copies, with
before then after for each increasing size. Every run used the real supervised
parser and worker, checked saved version/channel facts and native source digest,
and completed publication. No worker speed-up is claimed from the small changes
below; file reading/hashing/parsing dominate this boundary and timings vary.

| Constructed file              | Before warm p95 | After warm p95 | After parent peak working set | After parser peak working set |
| ----------------------------- | --------------: | -------------: | ----------------------------: | ----------------------------: |
| Approved sample, 46,703 bytes |         9.15 ms |        9.06 ms |                      7.96 MiB |                      4.29 MiB |
| 4,601,596 bytes               |        82.41 ms |       83.81 ms |                     11.82 MiB |                      8.64 MiB |
| 25,000,000 bytes              |       407.28 ms |      404.18 ms |                     31.28 MiB |                     28.11 MiB |
| 64 MiB input bound            |     1,119.24 ms |    1,163.82 ms |                     71.45 MiB |                     68.13 MiB |

Parent/child working-set/private-commit snapshots and lifetime peaks, first
requests and excluded warm-ups remain separate in the raw report. They include
process setup, are not per-request increments and cannot be summed to infer
simultaneous peak desktop RAM. See the previous report's Windows counter
definitions. The existing 64 MiB cap and 10-second supervised deadline remain.

## Source and measurement controls

- Base: merged PR #260, `ee76e47611d3e15d62514b14e99abb95221d59df`.
  The before Explorer binary uses that production code plus the same test-only
  component instrumentation as after. The before worker reuses the independently
  retained #258 binary: native execution/storage/parser production sources are
  identical between its base `249a72a` and merged #260; example/helper hashes and
  artifact hashes are recorded. The after binaries include the query change.
- Windows 11 build 26200; Intel Core i7-10750H, 12 logical processors,
  approximately 15.87 GiB RAM; High performance power plan; local NTFS.
- Owner app and resident parser closed normally before timing. Preflight CPU
  load was at most 20%; the recorded windows completed with no local build/test
  overlap or owner app restart. Host/process records remain private.
- Builds, fixture preparation and measurement windows were sequential under
  one cache owner. Independent before/after binaries remained outside the
  writable compiler cache; no hardlinks were used for retained executables.
- Each case: separate release process, first request, excluded warm-up,
  ten measured requests in original order. The median averages the middle pair;
  nearest-rank p95 equals the maximum for ten samples. No sample is discarded.
- Approved sample and opaque-state padding are constructed size probes, not
  diverse real plugin workloads. Explorer uses validated synthetic saved facts,
  one Sampler reference per entry, with no filesystem/parser work in its timer.
- OS/file caches were not flushed. Fixtures were copied and hashed before
  timing, so first process request is not a cold-disk claim. Native worker
  discovery/claim, desktop lease monitor, scanner, IPC/rendering remain excluded.

## Repeat and checks

Use [#258's explicit fixture/cache/probe instructions](../metadata-resources-258/README.md#repeat-safely)
and [DEVELOPMENT.md](../../../DEVELOPMENT.md#local-build-and-disk-space-rules).
Reuse the one named cache sequentially, run storage preflight against expected
additional output while preserving 30 GiB free, retain byte-copied binaries with
source/build/hash provenance, and finish preparation before the quiet window.
Never swap, seed or restore an owner's personal profile for these probes.

Optional test-only component instrumentation:

```powershell
$env:FRUITBOARD_RESOURCE_EXPLORER_PROFILE = '1'
# Select fresh directory, entry count and retained parser as described in #258.
& "$evidence\explorer-probe.exe" --exact foundation::plugin_explorer::resource_probe::explorer_resource_probe --ignored --nocapture --test-threads=1
Remove-Item Env:\FRUITBOARD_RESOURCE_EXPLORER_PROFILE
```

Leaving that variable absent preserves the existing full-command probe behavior.
The profile records subsequent storage, projection/internal budget serialization
and outer-envelope serialization series. Setup, decoding/assertions and memory
observation stay outside their timers. The actual full-command timer is unchanged.

A regression alternates two files/roots after warm reads, checks selected
snapshot/history IDs, disables/re-enables one root without reviving its old
result, then publishes a fresh snapshot and rechecks both histories. Existing
root/alias/stale-source, input/reference/group/output bound, corruption,
recovery/timeout/cancellation suites remain required. Normal tests have no
machine-dependent timing assertions. The PR supplies pinned default/native
checks, real-parser integration and updated-head CI evidence.

No dependencies, schema, IPC vocabulary, UI, parser cap or production activation
are added. Private fixtures/databases/logs/host records/binaries stay outside Git;
only scalar reports and relative source/artifact hashes are published. The
handoff records cache ownership, live capacity, reusable outputs and evidence
that must remain. No owner data or prior evidence was deleted. Issue #259 stays
open for budget acceptance and the remaining qualification review.

# Native metadata and Plugin Explorer resource characterization

Issue [#258](https://github.com/guilhermebmichelin-create/fruitboard/issues/258)
adds opt-in release probes and records their raw scalar results in
[results.json](results.json). Production code, parser limits, dependencies,
database schema and rendering are unchanged. No personal FLPs or databases are
used. These measurements do **not** accept the Phase 3 checkpoint.

## Results and interpretation

Every native worker run published the expected version, channel count and
independently checked source digest. Each Explorer run returned either complete
counts or a limited result without partial entries, groups or reference totals.

| Constructed file                    | First worker request | Warm median |    Warm p95 | Parent peak working set | Parser peak working set |
| ----------------------------------- | -------------------: | ----------: | ----------: | ----------------------: | ----------------------: |
| Approved sample, 46,703 bytes       |             24.78 ms |     7.27 ms |     8.81 ms |                7.54 MiB |                4.29 MiB |
| Reported-size case, 4,601,596 bytes |             99.96 ms |    81.73 ms |    91.93 ms |               11.80 MiB |                8.64 MiB |
| 25,000,000 bytes                    |            441.74 ms |   396.24 ms |   405.33 ms |               31.32 MiB |               28.13 MiB |
| 64 MiB input bound                  |          1,187.35 ms | 1,054.54 ms | 1,147.44 ms |               71.57 MiB |               68.28 MiB |

| Explorer entries  | Result                     | First command | Warm median |  Warm p95 | Peak working set |
| ----------------- | -------------------------- | ------------: | ----------: | --------: | ---------------: |
| 100               | ready, complete counts     |      16.18 ms |    15.53 ms |  20.75 ms |        14.63 MiB |
| 1,000             | ready, complete counts     |     166.24 ms |   153.20 ms | 161.53 ms |        16.24 MiB |
| 2,000 input bound | ready, complete counts     |     327.82 ms |   314.56 ms | 328.89 ms |        20.72 MiB |
| 2,001             | limited, no partial totals |       3.30 ms |     2.39 ms |   2.50 ms |        12.16 MiB |

The 2,000-entry command exceeds the existing 200 ms saved-query comparison.
That baseline is not a newly approved Explorer command/serialization budget.
No parser throughput or memory target is approved here, and the ten-second
supervised request deadline is not a total-worker timing target. Peak memory
does not establish a maximum for arbitrary real plugin state.

The host was quiet at preflight (2% reported CPU load; no other build/test
workloads). The already-open owner app and its resident parser remained open.
The full-window owner-app CPU guard **failed**: desktop activity exceeded the
predeclared 0.5 CPU-second allowance; the resident parser had no CPU activity
at the first observation after the guard. Preserve every raw result as noisy
characterization, **not an idle-window qualification pass**. No result was
dropped or relabeled as passing. The earlier incomplete attempt and harness
diagnostics remain in private evidence.

Follow-up [#259](https://github.com/guilhermebmichelin-create/fruitboard/issues/259)
tracks explicit resource budgets, profiling limit-sized Explorer reads and a
quiet-host rerun before checkpoint acceptance. Earlier scanner qualification
failures, compatibility limits and installed UI qualification remain separate.

## Measurement boundary

- Windows 11 build 26200; Intel Core i7-10750H, 12 logical processors,
  approximately 15.87 GiB RAM, High performance power plan, local NTFS.
- Each case uses a separate release process, one recorded first request, one
  excluded warm-up and ten measured requests in original order. Median is the
  middle-pair average. Nearest-rank p95 for ten samples equals the maximum;
  this small sample does not estimate the tail of real production traffic.
- Native analysis times the real `AnalysisWorker::execute`: held-source
  observations/hashes, supervised descriptor/parse validation and SQLite
  publication. It also includes the probe's child-memory observation. Job
  discovery/claim/setup, scanner enumeration, desktop renewal monitor,
  IPC and rendering are outside the timer. A fixed test clock keeps leases
  valid; this probe does not measure renewal or shutdown races.
- The supervised-parse and child-reported parse durations remain separate in
  the raw report. The reusable child is queried while still owned by its
  supervisor and is shut down after the worker case.
- Explorer times the actual native command and JSON response serialization.
  Response decoding/assertions and memory observation are outside that timer.
  It reads a fresh synthetic database through the usual storage and validation
  APIs, with one known Sampler reference per entry. It does not enumerate actual
  files, cross webview IPC, render groups or cover diverse real plugin payloads.
- Source copies were created and hashed before measurement. File-system and
  operating-system caches were not flushed; first request is not a cold-disk
  claim. Warm native requests use twelve distinct prepared source copies.
- Windows working-set and private-commit snapshots and process-lifetime peaks
  are recorded for parent and child separately. Peaks include setup/seeding;
  they are not per-request incremental memory or simultaneously sampled combined
  RAM. Commit is not resident RAM. Counter meanings follow Microsoft's
  [PROCESS_MEMORY_COUNTERS_EX](https://learn.microsoft.com/en-us/windows/win32/api/psapi/ns-psapi-process_memory_counters_ex)
  and [GetProcessMemoryInfo](https://learn.microsoft.com/en-us/windows/win32/api/psapi/nf-psapi-getprocessmemoryinfo).

## Repeat safely

Read [DEVELOPMENT.md](../../../DEVELOPMENT.md#local-build-and-disk-space-rules).
Use one named owner of the existing validation cache. Maintain 30 GiB free
after expected output on every involved volume. Probes and fixture/database
preparation run sequentially; measurements run alone after all builds and
preparation finish. Stop if host activity prevents a quiet window. Existing
owner profiles must never become test databases or be swapped/restored.

The following example uses hypothetical **absolute**, separately owned paths.
Select compatible existing source/cache paths first; never create one cache per
probe. Charge at least the expected build output plus approximately 1.09 GiB
for 48 independent fixtures. Reports, fixtures, databases and independent
executables belong in evidence, outside disposable compiler caches.

```powershell
$env:CARGO_TARGET_DIR = 'C:\Build\FruitboardValidation'
$evidence = 'C:\Evidence\Metadata258'
node scripts/preflight-build-storage.mjs --source $PWD.Path --build $env:CARGO_TARGET_DIR --cache $env:CARGO_TARGET_DIR --evidence $evidence --estimated-output-gib 6
cargo build -p fruitboard-analysis-execution --example resource_probe --release --locked
cargo test -p fruitboard-desktop --release --features analysis-jobs --locked --no-run --message-format=json
```

Record the source boundary and SHA-256 hashes of probe source, approved sample
and binaries. Independently copy the example executable, the desktop library
test executable selected from Cargo's JSON compiler-artifact output, and a
verified matching release parser into `$evidence`; never hardlink them. The
parser used here is the unchanged 0.1.1 independently retained from PR #257;
its hash is in the report. Do not run the ordinary desktop app test executable
instead of the library test executable. Complete all cache writes before
measurement. If a matching parser is unavailable, build it in the same owned
cache before retaining the copy.

```powershell
node scripts/prepare-native-resource-fixtures.mjs --output "$evidence\fixtures"
# For each case: sample, reported, 25mb, 64mib. Selected files must be synthetic.
& "$evidence\worker-probe.exe" "$evidence\fixtures\25mb" "$evidence\fruitboard-flp-parser.exe"
# For each entry count: 100, 1000, 2000, 2001. Select a fresh directory each time.
$env:FRUITBOARD_RESOURCE_EXPLORER_DIRECTORY = "$evidence\explorer-1000"
$env:FRUITBOARD_RESOURCE_EXPLORER_ENTRIES = '1000'
$env:FRUITBOARD_RESOURCE_PARSER = "$evidence\fruitboard-flp-parser.exe"
& "$evidence\explorer-probe.exe" --exact foundation::plugin_explorer::resource_probe::explorer_resource_probe --ignored --nocapture --test-threads=1
```

Worker output is one scalar JSON report on stdout; failures emit a fixed message
and nonzero exit. Explorer writes `result.json` inside its selected fresh
directory. Preserve stderr and exit codes, all samples and host activity checks;
do not discard slower runs. Existing worker `probe-db` or Explorer directories
are refused. A failed/partial run is retained; choose fresh prepared evidence
for a rerun. Rehash the fixture inventory and approved corpus afterwards.

The fixture helper reads only `FIX-FL2026-SAMPLE.flp`, streams opaque state
padding in 64 KiB chunks, creates independent copies and records their hashes.
It refuses existing output and independently checks the storage reserve. It
does not replace the full path/cache preflight or host coordination. Constructed
padding preserves this sample's recognized facts, not realistic plugin variety.

## Verification and handoff

The resource run asserts source digests and saved facts for all 48 worker jobs,
and complete-or-limited Explorer behavior for all 48 command responses.
Ordinary tests cover the summary math without asserting machine-dependent
timings. Existing native feature suites cover timeout/crash/cancellation,
stale-source fencing, retry/recovery and independent Explorer bounds; these
probes do not replace those tests. See the PR for exact changed-head checks.

No new installer or visual qualification is required for this test-only change.
No migrations, new privileges or production enablement are introduced. Fixture
bytes, local databases, logs, host/process records and independently retained
binaries stay private. Only scalar/synthetic reports and relative source hashes
are published. The task handoff records the single cache owner, absolute paths,
remaining capacity and retained evidence. No cleanup of owner data or caches is
authorized by these probes.

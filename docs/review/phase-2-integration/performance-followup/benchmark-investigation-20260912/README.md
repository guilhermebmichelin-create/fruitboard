# PR #108 benchmark investigation

Date: 2026-09-12. This is a correctness and diagnostics follow-up only. It
does not qualify performance, accept Phase 2, optimize traversal, change
limits, weaken safety checks, or implement chunked snapshots.

## Disposition

The current-main iteration-6 failure remains unexplained. The preserved run
proves a terminal non-authoritative `Partial` execution, but it contains no
coverage-failure kind, native operation, phase, or host event that identifies
the cause. The nine later successful iterations and the later reproductions
below do not explain that original iteration.

No evidence supports attributing it to Defender, DriveFS, contention, or a
product defect. The host was not an exclusive or quiet performance host, so
the performance window is failed closed. No security or cloud setting was
changed.

## Preserved baseline

The original raw artifact is preserved verbatim in the qualification capture
commit `fa1b4c9` at
`docs/review/phase-2-integration/performance-followup/benchmark-normal-current-main-raw.json`.
The preserved checked-out capture file's SHA-256 is
`0174f9081de7d46c7caf984f31bc17bebcb87a3f4d51b9234bdcd461d1f2c6fd`; the
same committed content has Git blob `2f104fe8d37fb27655aa8d36b8a52752c3affbf9`
and UTF-8 content hash
`05f560e88dafe022d7b90881477b21ded1362b4ee3fa50c40bb1e49fbc35f124`.

The relevant identity is:

| Item | Evidence |
| --- | --- |
| production tree | `3ebac5f7a76c3425620ceba59e6078b32fb6cd85` |
| fixture | `custom-9995`, seed `0`, manifest canonical SHA-256 `a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a` |
| fixture counts | 9,995 FLP files, 4 other files, 5 alias locations, 1,025 directories, 1,000 leaves, 10 empty directories, 5 hardlink groups |
| verified production binary | SHA-256 `34347c0e79face9e61d7adf8bd0663318d1e4f6509f6d081331075ee01fc1025` |
| original measurement | one warm-up plus ten measured iterations |

Iteration 6 records `exitCode=1`, `scanMs=11137`, `status=Failed`,
`outcome=Partial`, `authoritative=false`, `errorCode=null`,
`locationCount=null`, `changesComputed=false`. Its only terminal protocol
line has no failure message, and the raw report has no notes. The other nine
measured iterations published `Complete` with 10,000 locations.

The artifact therefore establishes the observed terminal state and the fact
that no publication was reported. It cannot establish which enumeration
branch ran, whether a native call returned a particular `PortError`, or what
the host was doing at that instant.

## Investigation findings

### Harness

`scripts/run-benchmark.mjs` starts a fresh driver process for each iteration
against one shared committed database. It retains a child failure in the raw
iteration record, but returns harness exit code 0 after writing a protocol
report. The existing statistics code uses only authoritative iterations for
performance statistics; the failed record is still retained and is not
filtered from the evidence. For the original set, that means performance
statistics have `n=9`, not ten.

The external manifest is validated before the run. The persisted fixture was
validated without regeneration and its canonical hash and counts match the
baseline. The generator uses deterministic layout/marker bytes; its creation
timestamps are host metadata and are not part of the manifest hash.

The diagnostic change now whitelists driver protocol fields and fixed codes.
Unparseable output, arbitrary error messages, native stderr, paths, and file
content are not written to a report. `partialClass` is extracted from the
fixed `partial_class` protocol field only.

### Every `Partial` enumeration path

The inventory below is from `crates/filesystem-enumeration/src/lib.rs`. All
listed paths call `add_failure(..., Outcome::Partial)` directly or through a
fixed `PortError` mapping; no path is inferred from a native message.

| Enumeration location | Trigger | Bounded class |
| --- | --- | --- |
| root qualification, opened-root validation, final-root validation | root identity is unavailable | `partial_root_identity_unavailable` |
| directory cursor | `NotFound` | `partial_directory_not_found` |
| directory cursor | timestamp out of range | `partial_timestamp_out_of_range` |
| directory cursor | reparse/change detected | `partial_directory_changed` |
| directory cursor | unsupported/other read failure | `partial_directory_read` |
| child directory open | `NotFound` | `partial_directory_not_found` |
| child directory open | timestamp out of range | `partial_timestamp_out_of_range` |
| child directory open | reparse/change detected | `partial_directory_changed` |
| child directory open | unsupported/other read failure | `partial_directory_read` |
| opened child validation | reparse, identity mismatch | `partial_directory_changed` |
| opened child validation | child identity unavailable | `partial_directory_identity_unavailable` |
| entry metadata | `NotFound` | `partial_entry_disappeared` |
| entry metadata | timestamp out of range | `partial_timestamp_out_of_range` |
| entry metadata | reparse/change detected | `partial_directory_changed` |
| entry metadata | unsupported/other read failure | `partial_metadata_read` |
| directory metadata | entry changes to a reparse point | `partial_directory_changed` |
| FLP file boundary checks | timestamp out of range | `partial_timestamp_out_of_range` |
| FLP file boundary checks | byte size exceeds the durable range | `partial_byte_size_out_of_range` |
| locator-key bound | locator key exceeds the staging bound | `partial_locator_key_too_long` |

The following are deliberately not `Partial`: access denial resolves to
`Denied`; root not-found/change and root timestamp failures resolve to
`RootUnavailable`; resource limits resolve to `ResourceLimit` without a
coverage failure; invalid names and duplicate locators resolve to `Invalid`;
sink failure resolves to `SinkFailed`; cancellation resolves to `Cancelled`;
unsupported root qualification resolves to `UnsupportedFilesystem`.

The worker reports a class only for `Outcome::Partial`. Exactly one retained
failure maps to its class. More than one maps to `partial_multiple`; no
retained failure, or any omitted failure diagnostic, maps to
`partial_unknown`. This avoids guessing from the first retained item when the
bounded diagnostic list is incomplete.

## Bounded instrumentation

The isolated branch adds `ScanExecution.partial_class`, a closed enum with no
path or native-message payload, and emits it as `partial_class` on
`scan_finished`. Both `execute` and `execute_shared`, including both resolve
paths, carry it through. Storage files and storage APIs were not changed;
Agent 3 retains ownership of any future durable error-code integration.

The deterministic regression now verifies:

- a disappearing child directory yields `Failed` / `Partial` /
  `partial_directory_not_found`;
- no publication occurs and staging is discarded;
- committed rows, the root last-success publication marker, and their
  serialized snapshot remain byte-identical;
- restoring the directory permits a later `Published` / `Complete` scan.

## Reproduction commands

Use a verified persistent fixture and binary; do not regenerate or mutate the
fixture during the run. The following is the reproducible correctness command
(replace angle-bracket placeholders locally):

```powershell
$repoRoot = (Get-Location).Path
$nodePath = Join-Path $repoRoot ".tools\node-v24.20.0-win-x64\node.exe"
$fixtureManifest = "<verified-custom-9995>\manifest.json"
$verifiedBinary = "<verified-benchmark.exe>"
$outRoot = Join-Path $repoRoot "docs\review\phase-2-integration\performance-followup\benchmark-investigation-20260912\attempts"
New-Item -ItemType Directory -Force -LiteralPath $outRoot | Out-Null

& $nodePath (Join-Path $repoRoot "scripts\run-benchmark.mjs") `
  --manifest $fixtureManifest `
  --bin $verifiedBinary `
  --warmup 1 `
  --iterations 10 `
  --cancel-iterations 0 `
  --settle-ms 300 `
  --memory-interval-ms 100 `
  --keep-fixture `
  --out (Join-Path $outRoot "attempt.json")

Get-Content -LiteralPath (Join-Path $outRoot "attempt.json") -Raw |
  ConvertFrom-Json |
  Select-Object warmUp, iterations, notes
```

The harness exit code is not the scan disposition. Inspect each
`warmUp`/`iterations[*]` record, especially `exitCode`, `status`, `outcome`,
`authoritative`, `partialClass`, `errorCode`, and `locationCount`.

Focused deterministic checks:

```powershell
$cargo = "<verified-cargo.exe>"
& $cargo test -p fruitboard-scan-execution --locked `
  p2_03_partial_disappearance_preserves_rows_and_snapshot_byte_identical `
  -- --nocapture
& $cargo test -p fruitboard-scan-execution --locked `
  partial_class_is_bounded_and_does_not_guess_after_truncation `
  -- --nocapture
& $cargo test -p fruitboard-scan-execution --locked
& $nodePath --test tests\benchmark-scaffold.test.mjs
& $cargo fmt --all -- --check
```

Agent 1’s coordination review failed closed for a quiet performance window:
the point-in-time preflight did not establish exclusivity, DriveFS and
Defender processes were present, and the required sustained host gate was not
met. A benchmark lock, if used by another owner, does not control those host
processes. Correctness reproduction may proceed under that limitation; no
performance conclusion may be drawn.

## Sanitized attempts

The retained reports are in `attempts/`.

| Report | Binary | Result | Host activity recorded |
| --- | --- | --- | --- |
| `current-main-original-attempt-01.json` | preserved production binary, `34347c…1025` | warm-up and 3/3 measured `Published` / `Complete`; no Partial reproduced | 53 samples; cargo/build activity appeared, CPU/disk varied, and DriveFS/Defender/Node/Codex were present |
| `instrumented-attempt-01.json` | isolated diagnostic binary, `7fa96d7fc4811510385d83333fd5cf1ffe71ec98b59684200f56c4ff8e50560c` | warm-up and 10/10 measured `Published` / `Complete`; every `partialClass` is null | 93 samples; cargo `0–2`, rustc `0–11`, benchmark `0–1`, DriveFS `2`, each observed Defender service `1`, Node `6–19`, Codex `3–4`, CPU `0–100`; not quiet |

Report SHA-256 values:

- `current-main-original-attempt-01.json`: `f7a4f159dfb499c84d950c326c7c0c0acbd9846c1747c0ed449a2540310ce80b`
- `instrumented-attempt-01.json`: `ef44e3eabba6105a82b5819b33a4e480e2a6196b8bfac79496268891d0a60657`

The activity summaries are aggregate observations only. They do not identify
any process as the cause; the disk counter was not treated as a normalized
contention measurement.

## Final interpretation for PR #108

PR #108 should be read as: “9 of 10 measured iterations published; iteration
6 was a genuine but unclassified non-authoritative Partial scan. The raw
artifact cannot explain it. Follow-up instrumentation and deterministic
correctness checks establish safe handling for known disappearance cases, but
the original trigger was not reproduced.”

This branch is a focused diagnostic draft only. It does not merge, claim
Phase-2/performance acceptance, or authorize a traversal optimization. A
future quiet run may use `partial_class` to identify a new occurrence; a later
pass by itself remains non-explanatory.

# Next scanner performance qualification runbook (2026-09-13)

Status: **prepared, not executed**. This is one bounded P2-11 run for the
current merged scanner implementation on the unchanged local-NTFS target. It
does not amend a budget, limit, fixture definition, platform scope, or
acceptance criterion. It does not qualify the installed desktop UI, DriveFS,
FAT32/cross-volume identity, or the 100,000-entry memory target.

Basis: `scripts/benchmark-scan.md`, `scripts/run-benchmark.mjs`, PR #108's
qualification capture, PR #112's diagnostic/sanitization capture, the
published #109 decision table, and the Phase 2 execution/scan contracts.
The small reusable validator in
`scripts/validate-qualification-report.mjs` is the executable report contract
used below; it does not start a benchmark or change host state.

Report validation is a report-integrity gate only. It does not prove host
eligibility, source provenance, or full Phase 2 acceptance. The separate host
preflight, exact-source/build evidence, platform-scope, and Phase 2 acceptance
requirements remain binding and must be satisfied independently before any
performance disposition is considered.

## Boundary and answer

The recommended execution profile is:

- fixture: proposed `custom-9995`, seed `0`, exactly 10,000 scanner
  observations, after the owner records that F1 choice;
- source: the exact SHA of `origin/main` after the selected source stack is
  merged into `main`, built as the release `benchmark` driver with locked
  dependencies;
- operation: the existing `scripts/run-benchmark.mjs` path through the real
  Windows filesystem port, scan worker, staging, and atomic publication;
- protocol: one first-run warm-up, 10 measured fresh-driver iterations against
  one committed database, and 3 fresh cancellation measurements;
- answer: whether the current merged implementation meets the unchanged
  first-discovery, warm-reconciliation, and cooperative-stop budgets on an
  eligible host.

The current #112 source-stack head is historical review provenance only. It is
not merged and is not a measurement boundary. At execution time, record the
exact fetched `origin/main` SHA after the owner merges the selected source
stack; do not substitute a PR head or installed-evidence SHA.

The historical #108 before/candidate A/B is not part of this run. It was
normal-configuration, contended evidence against older source boundaries and
does not answer the current merged implementation question. Add A/B only if
the owner explicitly selects an A/B protocol to answer a named unresolved
optimization question; then run the current side as well and keep the A/B as a
separate diagnostic experiment. No extra run is needed for the current-only
qualification answer.

## Protocol and host-profile selection

The owner must select the host profile before timing. The executable block in
section 4 reads `FRUITBOARD_HOST_PROFILE` and accepts exactly one of:

| Layer                   | `repository-minimum`                                                                                                                                                                                                                        | `strict-quiet-host`                                                                                                                                                                                                                                                   |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository requirements | Required: clean final merged source, pinned release build, verified fixture, local NTFS, AC power, no concurrent build/test/benchmark/installed run, agent quiescence confirmation, and the methodology's one-plus-ten-plus-three protocol. | The same requirements.                                                                                                                                                                                                                                                |
| Owner-selected controls | No Defender exclusion, High performance scheme, DriveFS absence, or numerical idle thresholds are required by this profile.                                                                                                                 | The owner additionally selects High performance, DriveFS paused/absent, a non-volume Defender exclusion that covers the exact disposable fixture path, and the prior report's 60-sample idle thresholds. The owner supplies these controls; the runbook changes none. |
| Observations            | Process presence, Defender status/exclusions, DriveFS presence, power text, and CPU/disk samples are recorded. An unavailable observation is `unknown`, not a pass.                                                                         | The selected controls are fail-closed: an unavailable or unconfirmed control is not a pass.                                                                                                                                                                           |

The runbook has no implicit default: missing or unknown
`FRUITBOARD_HOST_PROFILE` aborts before timing. The owner/operator must also
set `FRUITBOARD_AGENT_QUIESCENCE_ATTESTATION=confirmed` only after Agent 1 and
all sibling agents have stopped build, test, benchmark, and installed-run work
for the dedicated window. This attestation is not inferred from a process
list. If the strict profile is selected, set
`FRUITBOARD_DRIVEFS_QUIESCENCE_ATTESTATION=confirmed` only after the owner has
verified that DriveFS is paused or absent. Missing attestations remain
`unknown` and fail the selected gate.

The repository requirements come from `scripts/benchmark-scan.md` and the
Phase 2 contracts. The strict profile is an owner-selected operational
control, not a new repository budget. Process counts and Defender data are
observations unless the selected profile makes a specific control binding.
The prior report's numerical recipe is CPU average `<10%` with no spike above
`25%`, and fixture-volume disk idle average `>90%` with no dip below `70%`;
record the raw samples either way.

These are recommendations from previous reports, not additional acceptance
gates:

- `custom-9995` is the no-code F1 alignment recommendation. Retaining the
  10,005-observation fixture instead requires a separately approved,
  end-to-end limit change; raising one constant is not a valid fix.
- Cache control, `profile-fs-calls`, traversal optimization, a historical A/B,
  and a 100,000-entry run are not needed to answer the current-only question.
  The warm-up must simply not be called `cold-cache` unless cache state was
  controlled.

The 100,000-entry working-memory budget remains unmeasured under the current
10,000-record bounds. A successful 10,000-observation run records driver
working-set data but cannot promote F3 or claim full Phase 2 performance
acceptance.

## Fixture decision and verified accounting

The existing raw artifacts agree on the canonical hashes and counts:

| Role                   | FLP files | Alias locations | Other files | Scanner observations | Canonical manifest SHA-256                                         | Disposition                          |
| ---------------------- | --------: | --------------: | ----------: | -------------------: | ------------------------------------------------------------------ | ------------------------------------ |
| Accepted `baseline`    |    10,000 |               5 |           4 |           **10,005** | `8c3d85ec01299995208abfa450378b1b37c704e423593d7a4370ec465254afba` | Preserve as overflow safety evidence |
| Proposed `custom-9995` |     9,995 |               5 |           4 |           **10,000** | `a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a` | Recommended; owner selection pending |

The generator counts primary `kind: "flp"` entries separately from alias
locations. The scanner observes both primary and alias locations; `kind:
"other"` entries are not observations. Thus the manifest entry totals are
10,009 for `baseline` and 10,004 for `custom-9995`, while the scanner totals
are 10,005 and 10,000 respectively. The four existing raw reports
(`benchmark-2026-09-07-*` and `benchmark-triage-20260908/rerun-*`) reproduce
these values and hashes.

Do not overwrite the original baseline raw files or the retained #108
current-main `Failed`/`Partial` artifact. The verbatim #108 capture is
preserved by its qualification-capture history and is described in
`performance-followup/benchmark-investigation-20260912/README.md`; its
file-byte SHA-256 is
`0174F9081DEA7D46C7CAF984F31BC17BECB87A3F4D51B9234BDCD461D1F2C6FD`.
It is not assumed to be present in this checkout. A new fixture must be
generated in a new empty directory or an owner-selected persistent fixture
must validate to the exact hash; never hand-edit a manifest.

## Readiness snapshot (2026-09-13)

No qualification benchmark, release build, installer, host preflight, or
60-second measurement was started in this sibling window. A disposable
synthetic fixture was generated and removed only for the runbook validation
recorded in [the validation record](qualification-runbook-validation-20260913.md);
that check is not performance evidence.

| Check            | Read-only result                                                                                                                                                                                                                                                                                                                                                                | Qualification state                                                                                                 |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Source           | This runbook was reviewed against the current #112 source stack; the exact execution SHA remains the fetched `origin/main` after merge. The current merged baseline does not yet contain the #112 sanitized harness/partial diagnostic fields.                                                                                                                                  | **Blocked until source stack is merged**                                                                            |
| Pinned tools     | `.tools` Node `v24.20.0`, cargo/rustc `1.98.1`, rustfmt, and `x86_64-pc-windows-msvc` are available; pinned uv `0.12.9` is available. pnpm is `11.25.0`; the bundled Corepack reports `0.35.0` versus policy `0.36.0`, but neither Corepack/pnpm nor uv is used by the benchmark command.                                                                                       | Benchmark-required Node/Rust pins observable; package-manager drift is recorded, not made into a new benchmark gate |
| Disk/volume      | `C:` is local `NTFS`, `510,511,804,416` bytes total and `2,645,377,024` bytes free at capture. The contracts specify recording capacity/free space but no minimum free-space acceptance threshold.                                                                                                                                                                              | Record again on the selected fixture volume                                                                         |
| Existing fixture | No repository or `target` manifest exists. Historical raw reports are present and agree on the hashes above.                                                                                                                                                                                                                                                                    | **Prepare and verify a new/persistent fixture after owner selection**                                               |
| Existing binary  | `target/release/examples/benchmark.exe` exists but has no final-source provenance; current file SHA-256 is `3f8ac1f51f4e26ae8c6fe4bc982674c6eaf9e1dc8244e3b7d99df268ad1b6aec`. Do not use it.                                                                                                                                                                                   | **Build and hash from final merged SHA**                                                                            |
| Test lock        | The active Foundation Smoke lock path is absent; no active owner is present. Retained `owner.lock` files under historical `.tools/evidence` are not active-run ownership and must not be deleted.                                                                                                                                                                               | Pass for this snapshot; recheck immediately before the run                                                          |
| Host observation | `powercfg` and CPU/disk CIM counters are available. The captured snapshot observed AC/High performance, DriveFS `2`, Defender engine/core `1/1`, Node `5`, Codex `6`, and T3 `6`; cargo/rustc/benchmark/Fruitboard were `0`. These are observations, not proof of contention or quiescence. Defender status is readable, but no fixture-path exclusion is selected or verified. | **Profile selection and dedicated-window attestations still required**                                              |

The host is therefore ineligible now. Stop here until the owner supplies the
dedicated window and the selected quiet-host controls. Do not pause DriveFS,
change Defender, terminate processes, delete locks, or touch real project
roots as part of this task.

## Exact execution sequence

Replace only the run-root choice if the owner selects another local NTFS
volume. Run after the owner decision, after the source stack is merged, and
from a clean checkout. All evidence below is kept outside the repository.
The commands in this section are the `custom-9995` sequence; if the owner
selects the 10,005 alternative, stop and obtain its separately approved
coupled-limit protocol before using any execution command below.

### 1. Resolve and record the merged source

```powershell
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path -LiteralPath (Get-Location)).Path
Set-Location -LiteralPath $repoRoot
$runId = "qualification-" + (Get-Date -Format "yyyyMMdd-HHmmss")
$runRoot = Join-Path $env:TEMP ("fruitboard-" + $runId)
if (Test-Path -LiteralPath $runRoot) {
  throw "Choose a new run root; refusing to reuse evidence."
}
$reportsRoot = Join-Path $runRoot "reports"
$fixtureRoot = Join-Path $runRoot "fixture"
New-Item -ItemType Directory -Path $reportsRoot -Force | Out-Null

$dirty = @(git status --porcelain)
if ($dirty.Count -ne 0) {
  throw "Use a clean dedicated checkout; do not discard sibling edits."
}
git fetch origin main --prune
$finalMergedSha = (git rev-parse --verify origin/main).Trim()
$headSha = (git rev-parse --verify HEAD).Trim()
if ($headSha -ne $finalMergedSha) {
  throw "Checkout HEAD must equal fetched origin/main before building."
}
$benchmarkSource = Get-Content -LiteralPath (Join-Path $repoRoot "scripts\run-benchmark.mjs") -Raw
$driverSource = Get-Content -LiteralPath (Join-Path $repoRoot "crates\scan-execution\examples\benchmark.rs") -Raw
if ($benchmarkSource -notmatch "sanitizeDriverLine" -or
    $benchmarkSource -notmatch "partialClass" -or
    $benchmarkSource -notmatch "stderrObserved" -or
    $driverSource -notmatch "partial_class") {
  throw "final source does not contain the #112 sanitized diagnostic path"
}

$sourceRecord = [ordered]@{
  recordedAtUtc = [DateTime]::UtcNow.ToString("o")
  mergedMainSha = $finalMergedSha
  headSha = $headSha
  commit = ((git show -s --format="%H %s" $finalMergedSha).Trim())
  prospectiveBoundary = "current #112 source-stack handoff; final boundary is merged origin/main"
}
$sourceRecord | ConvertTo-Json -Depth 4 |
  Set-Content -LiteralPath (Join-Path $reportsRoot "source-boundary.json") -Encoding utf8
```

The recorded SHA is the source boundary. If it is still the pre-stack baseline,
stop; do not build a PR head as a substitute. The final source must expose the
issue #112 sanitized report path and the bounded `partial_class` driver field.
Record the final merged SHA supplied by the source-stack owner; no historical
integration candidate or acceptance-documentation commit is required to remain
an ancestor.

### 2. Build and hash the exact release driver

```powershell
$nodeHome = Join-Path $repoRoot ".tools\node-v24.20.0-win-x64"
$nodePath = Join-Path $nodeHome "node.exe"
$cargoHome = Join-Path $repoRoot ".tools\cargo-home"
$rustupHome = Join-Path $repoRoot ".tools\rustup-home"
$cargoPath = Join-Path $cargoHome "bin\cargo.exe"
$rustcPath = Join-Path $cargoHome "bin\rustc.exe"
$targetRoot = Join-Path $runRoot "target"

$env:CARGO_HOME = $cargoHome
$env:RUSTUP_HOME = $rustupHome
$env:RUSTUP_TOOLCHAIN = "1.98.1-x86_64-pc-windows-msvc"
$env:CARGO_TARGET_DIR = $targetRoot

if ((& $nodePath --version).Trim() -ne "v24.20.0") { throw "wrong Node pin" }
if (-not ((& $cargoPath --version).Trim() -match "^cargo 1\.98\.1 ")) { throw "wrong cargo pin" }
if (-not ((& $rustcPath --version).Trim() -match "^rustc 1\.98\.1 ")) { throw "wrong rustc pin" }

& $cargoPath build --release `
  -p fruitboard-scan-execution `
  --example benchmark `
  --locked
if ($LASTEXITCODE -ne 0) { throw "release driver build failed" }

$binaryPath = Join-Path $targetRoot "release\examples\benchmark.exe"
if (-not (Test-Path -LiteralPath $binaryPath -PathType Leaf)) {
  throw "release benchmark binary is missing"
}
$binaryHash = (Get-FileHash -LiteralPath $binaryPath -Algorithm SHA256).Hash.ToLowerInvariant()
[ordered]@{
  sourceSha = $finalMergedSha
  buildCommand = "cargo build --release -p fruitboard-scan-execution --example benchmark --locked"
  node = (& $nodePath --version).Trim()
  cargo = (& $cargoPath --version).Trim()
  rustc = (& $rustcPath --version).Trim()
  binarySha256 = $binaryHash
} | ConvertTo-Json -Depth 4 |
  Set-Content -LiteralPath (Join-Path $reportsRoot "release-build.json") -Encoding utf8
```

Expected: a successful release build and a new binary hash. An existing target
binary without this provenance is not acceptable. The build happens before
the idle proof so compilation activity cannot contaminate the proof.

### 3. Generate and verify the selected fixture

```powershell
if (Test-Path -LiteralPath $fixtureRoot) {
  throw "Fixture path must be new and empty; refusing to mutate an existing tree."
}

$buildFixture = @'
import { buildPlan, writePlan } from "./scripts/generate-synthetic-tree.mjs";
const destination = process.argv[1];
const plan = buildPlan({
  sizeLabel: "custom-9995",
  seed: "0",
  fileCount: 9995,
  leafDirectoryCount: 1000,
});
const result = await writePlan(plan, destination);
console.log(JSON.stringify({
  sizeLabel: plan.sizeLabel,
  seed: plan.seed,
  hardlinkSupport: plan.hardlinkSupport,
  counts: plan.counts,
  manifestSha256: result.hash,
}));
'@
& $nodePath --input-type=module -e $buildFixture -- $fixtureRoot
if ($LASTEXITCODE -ne 0) { throw "fixture generation failed" }

$manifestPath = Join-Path $fixtureRoot "manifest.json"
$verifyFixture = @'
import { readFileSync } from "node:fs";
import { computeManifestHash, validateSyntheticManifest } from "./scripts/generate-synthetic-tree.mjs";
const [manifestPath, expectedHash] = process.argv.slice(1);
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
const validation = validateSyntheticManifest(manifest, { allowPending: false });
if (!validation.ok) throw new Error(validation.errors.join("; "));
const counts = manifest.counts;
const observations = counts.flpFiles + counts.aliasLocations;
const actualHash = computeManifestHash(manifest);
if (manifest.sizeLabel !== "custom-9995" || manifest.seed !== "0") throw new Error("fixture identity mismatch");
if (manifest.hardlinkSupport !== "created") throw new Error("hardlinks were not created");
if (observations !== 10000 || counts.flpFiles !== 9995 || counts.aliasLocations !== 5 || counts.otherFiles !== 4) throw new Error("fixture counts mismatch");
if (actualHash !== expectedHash) throw new Error(`manifest hash mismatch: ${actualHash}`);
console.log(JSON.stringify({ sizeLabel: manifest.sizeLabel, seed: manifest.seed, observations, counts, manifestSha256: actualHash }));
'@
& $nodePath --input-type=module -e $verifyFixture -- $manifestPath `
  a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a
if ($LASTEXITCODE -ne 0) { throw "fixture verification failed" }

$manifestFileHash = (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash.ToLowerInvariant()
[ordered]@{
  fixture = "custom-9995"
  seed = "0"
  observations = 10000
  canonicalManifestSha256 = "a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a"
  manifestFileSha256 = $manifestFileHash
  hardlinkSupport = "created"
} | ConvertTo-Json -Depth 4 |
  Set-Content -LiteralPath (Join-Path $reportsRoot "fixture-verification.json") -Encoding utf8
```

Expected generator/verification output includes `hardlinkSupport: "created"`,
9,995 FLP files, 5 aliases, 4 other files, and canonical hash
`a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a`. A hash
or count mismatch is a preflight failure; do not hand-edit or rerun the
benchmark against it.

### 4. Check the selected host profile and active test lock

Run this after all build/fixture preparation and immediately before timing. It
is read-only. The 60 samples are a strict-profile control when selected, and
otherwise remain an observation; they are never a scan measurement. The
operator must coordinate the dedicated window before this block:

```powershell
$selectedHostProfile = [string]$env:FRUITBOARD_HOST_PROFILE
if ($selectedHostProfile -notin @("repository-minimum", "strict-quiet-host")) {
  throw "set FRUITBOARD_HOST_PROFILE to the owner's recorded profile; unknown is not a pass"
}
$strictQuietHost = $selectedHostProfile -eq "strict-quiet-host"
$agentQuiescence = if ($env:FRUITBOARD_AGENT_QUIESCENCE_ATTESTATION -eq "confirmed") { "confirmed" } else { "unknown" }
$workloadQuiescence = if ($env:FRUITBOARD_WORKLOAD_QUIESCENCE_ATTESTATION -eq "confirmed") { "confirmed" } else { "unknown" }
$driveFsAttestation = if ($env:FRUITBOARD_DRIVEFS_QUIESCENCE_ATTESTATION -eq "confirmed") { "confirmed" } else { "unknown" }

$fixtureResolved = (Resolve-Path -LiteralPath $fixtureRoot).Path
$fixtureDrive = [IO.Path]::GetPathRoot($fixtureResolved).TrimEnd('\')
$volume = $null
try {
  $volume = Get-CimInstance Win32_LogicalDisk -Filter "DeviceID='$fixtureDrive'" -ErrorAction Stop |
    Select-Object -First 1
} catch {
  $volume = $null
}
$localNtfsState = if ($null -eq $volume) { "unknown" } elseif ($volume.FileSystem -eq "NTFS" -and $volume.DriveType -eq 3) { "pass" } else { "fail" }

$batteryState = "unknown"
try {
  $battery = @(Get-CimInstance Win32_Battery -ErrorAction Stop)
  if ($battery.Count -eq 0) {
    $batteryState = "not-applicable"
  } else {
    $batteryStatus = @($battery | Select-Object -ExpandProperty BatteryStatus)
    $batteryState = if (@($batteryStatus | Where-Object { $_ -in @(2, 6, 7, 8, 9, 11) }).Count -gt 0) { "pass" } else { "fail" }
  }
} catch {
  $batteryState = "unknown"
}
$powerText = (& powercfg /getactivescheme 2>&1) -join " "
$highPerformance = $powerText -match "(?i)high performance|alto desempenho"

# Presence is recorded. It is not contention evidence for Node, Codex, T3 or
# DriveFS. Only observed prohibited work is a repository gate.
$presenceNames = @("cargo", "rustc", "benchmark", "fruitboard", "GoogleDriveFS", "MsMpEng", "MpDefenderCoreService", "opencode", "codex", "t3", "node")
$processPresence = [ordered]@{}
foreach ($name in $presenceNames) {
  $processPresence[$name] = 0
}
$processInspectionState = "unknown"
$activityFindings = @()
$prohibitedActivity = @()
$unknownActivity = @()
try {
  $processes = @(Get-CimInstance Win32_Process -ErrorAction Stop)
  $processInspectionState = "known"
  foreach ($name in $presenceNames) {
    $processPresence[$name] = @($processes | Where-Object {
      [IO.Path]::GetFileNameWithoutExtension([string]$_.Name) -like "*$name*"
    }).Count
  }
  foreach ($process in $processes) {
    $processName = [IO.Path]::GetFileNameWithoutExtension([string]$process.Name)
    $relevant = @($presenceNames | Where-Object { $processName -like "*$_*" }).Count -gt 0
    if (-not $relevant) { continue }
    $commandLineKnown = -not [string]::IsNullOrWhiteSpace([string]$process.CommandLine)
    $activity = "presence-only"
    if ($processName -match "(?i)^(cargo|rustc|benchmark)$") {
      $activity = "prohibited-build-or-benchmark"
    } elseif (-not $commandLineKnown -and $processName -match "(?i)^(node|opencode|codex|t3|fruitboard|GoogleDriveFS)$") {
      $activity = "unknown"
      $unknownActivity += [ordered]@{ name = $processName; pid = [int]$process.ProcessId; reason = "command line unavailable" }
    } elseif ($commandLineKnown -and ([string]$process.CommandLine -match "(?i)(cargo\s+(build|test|check|clippy|fmt)|rustc\s|pnpm(?:\.cmd)?\s+.*(test|build)|npm(?:\.cmd)?\s+(test|run\s+build)|node(?:\.exe)?\s+.*(test|vitest|run-benchmark|run-installed)|benchmark(?:\.exe)?\s+--|run-benchmark|run-installed-ntfs|fruitboard.*(benchmark|scan-console|foundation-smoke))")) {
      $activity = "prohibited-build-test-benchmark-or-installed-run"
    }
    if ($activity -like "prohibited-*") {
      $prohibitedActivity += [ordered]@{ name = $processName; pid = [int]$process.ProcessId; activity = $activity }
    }
    $activityFindings += [ordered]@{
      name = $processName
      pid = [int]$process.ProcessId
      commandLineState = if ($commandLineKnown) { "observed-and-classified" } else { "unknown" }
      activity = $activity
    }
  }
} catch {
  $processInspectionState = "unknown"
}
$prohibitedActivityState = if ($processInspectionState -eq "unknown") { "unknown" } elseif ($prohibitedActivity.Count -gt 0) { "prohibited" } elseif ($unknownActivity.Count -gt 0) { "unknown" } else { "clear" }

$smokeLockPath = Join-Path $env:LOCALAPPDATA "com.fruitboard.desktop.foundation-smoke.lock.json"
$storageLockPath = Join-Path $repoRoot "storage\owner.lock"
$lockPaths = @($smokeLockPath, $storageLockPath)
$lockRecords = @(
  foreach ($lockPath in $lockPaths) {
    [ordered]@{ path = $lockPath; present = (Test-Path -LiteralPath $lockPath -PathType Leaf) }
  }
)
$lockState = "clear"
$lockOwnerAlive = $false
$lockReadable = $true
if ($lockRecords[0].present) {
  try {
    $lock = Get-Content -LiteralPath $smokeLockPath -Raw | ConvertFrom-Json
    if ($null -eq $lock.pid -or $lock.pid -is [bool] -or $lock.pid -isnot [int]) { throw "lock pid is not an integer" }
    $lockOwnerAlive = $null -ne (Get-Process -Id $lock.pid -ErrorAction SilentlyContinue)
    $lockState = if ($lockOwnerAlive) { "active" } else { "unknown" }
  } catch {
    $lockReadable = $false
    $lockState = "unknown"
  }
}
if ($lockRecords[1].present) {
  $lockState = if ($lockState -eq "active") { "active" } else { "unknown" }
}

function Test-DisposablePathCoverage {
  param([string]$ExcludedPath, [string]$TargetPath, [string]$VolumeRoot)
  try {
    $excludedFull = [IO.Path]::GetFullPath($ExcludedPath).TrimEnd('\')
    $targetFull = [IO.Path]::GetFullPath($TargetPath).TrimEnd('\')
    # Re-add the separator because a drive-relative string such as C: is not
    # the same path as the volume root C:\ when .NET resolves it.
    $volumeFull = [IO.Path]::GetFullPath(($VolumeRoot.TrimEnd('\') + '\')).TrimEnd('\')
    if ($excludedFull.Equals($volumeFull, [StringComparison]::OrdinalIgnoreCase)) { return $false }
    return $targetFull.Equals($excludedFull, [StringComparison]::OrdinalIgnoreCase) -or
      $targetFull.StartsWith($excludedFull + '\', [StringComparison]::OrdinalIgnoreCase)
  } catch {
    return $false
  }
}

$mpStatus = $null
$defenderState = if ($strictQuietHost) { "unknown" } else { "not-selected" }
$defenderReadable = $null
$fixturePathExclusions = @()
try {
  $mpStatus = Get-MpComputerStatus -ErrorAction Stop
  $mpPreference = Get-MpPreference -ErrorAction Stop
  $defenderReadable = $true
  foreach ($excluded in @($mpPreference.ExclusionPath)) {
    if (-not [string]::IsNullOrWhiteSpace([string]$excluded) -and
        (Test-DisposablePathCoverage ([string]$excluded) $fixtureResolved $fixtureDrive)) {
      $fixturePathExclusions += [string]$excluded
    }
  }
  if ($strictQuietHost) {
    $defenderState = if ($fixturePathExclusions.Count -gt 0) { "pass" } else { "fail" }
  } else {
    $defenderState = "observed"
  }
} catch {
  $defenderReadable = $false
  if (-not $strictQuietHost) { $defenderState = "unknown" }
}

$idleSamples = @()
for ($index = 0; $index -lt 60; $index++) {
  $cpuReadings = @(Get-CimInstance Win32_Processor -ErrorAction SilentlyContinue | Select-Object -ExpandProperty LoadPercentage)
  $cpu = if ($cpuReadings.Count -gt 0) { [double](($cpuReadings | Measure-Object -Average).Average) } else { $null }
  $disk = @(Get-CimInstance Win32_PerfFormattedData_PerfDisk_LogicalDisk -Filter "Name='$fixtureDrive'" -ErrorAction SilentlyContinue | Select-Object -First 1)
  $idle = if ($disk.Count -eq 1) { [double]$disk[0].PercentIdleTime } else { $null }
  $idleSamples += [ordered]@{ atUtc = [DateTime]::UtcNow.ToString("o"); cpuPercent = $cpu; diskIdlePercent = $idle }
  if ($index -lt 59) { Start-Sleep -Seconds 1 }
}

$cpuValues = @($idleSamples | Where-Object { $null -ne $_.cpuPercent } | ForEach-Object { [double]$_.cpuPercent })
$diskValues = @($idleSamples | Where-Object { $null -ne $_.diskIdlePercent } | ForEach-Object { [double]$_.diskIdlePercent })
$cpuStats = if ($cpuValues.Count -gt 0) { $cpuValues | Measure-Object -Average -Minimum -Maximum } else { $null }
$diskStats = if ($diskValues.Count -gt 0) { $diskValues | Measure-Object -Average -Minimum -Maximum } else { $null }
$cpuIdleState = if ($cpuValues.Count -ne 60) { "unknown" } elseif ($cpuStats.Average -lt 10 -and $cpuStats.Maximum -le 25) { "pass" } else { "fail" }
$diskIdleState = if ($diskValues.Count -ne 60) { "unknown" } elseif ($diskStats.Average -gt 90 -and $diskStats.Minimum -ge 70) { "pass" } else { "fail" }

$driveFsState = if (-not $strictQuietHost) { "not-selected" } elseif ($processPresence.GoogleDriveFS -eq 0) { "pass" } elseif ($driveFsAttestation -eq "confirmed") { "pass" } else { "unknown" }
$repositoryGatesPass = $localNtfsState -eq "pass" -and
  $batteryState -in @("pass", "not-applicable") -and
  $lockState -eq "clear" -and
  $processInspectionState -eq "known" -and
  $prohibitedActivityState -eq "clear" -and
  $agentQuiescence -eq "confirmed" -and
  $workloadQuiescence -eq "confirmed"
$selectedControlsPass = if ($strictQuietHost) {
  $highPerformance -and $driveFsState -eq "pass" -and $defenderState -eq "pass" -and
    $cpuIdleState -eq "pass" -and $diskIdleState -eq "pass"
} else {
  $true
}
$preflightPass = $repositoryGatesPass -and $selectedControlsPass

$preflight = [ordered]@{
  capturedAtUtc = [DateTime]::UtcNow.ToString("o")
  platform = [Environment]::OSVersion.VersionString
  selectedHostProfile = $selectedHostProfile
  repositoryGates = [ordered]@{
    localNtfs = $localNtfsState
    acPower = $batteryState
    activeTestLock = $lockState
    prohibitedActivity = $prohibitedActivityState
    agentQuiescence = $agentQuiescence
    workloadQuiescence = $workloadQuiescence
  }
  selectedControls = [ordered]@{
    strictQuietHost = $strictQuietHost
    highPerformance = if ($strictQuietHost) { if ($highPerformance) { "pass" } else { "fail" } } else { "not-selected" }
    driveFsQuiescence = $driveFsState
    defenderFixturePathCoverage = $defenderState
    idleThresholds = if ($strictQuietHost) { [ordered]@{ cpu = $cpuIdleState; disk = $diskIdleState } } else { "not-selected" }
  }
  fixtureVolume = [ordered]@{
    drive = $fixtureDrive
    fileSystem = if ($null -ne $volume) { $volume.FileSystem } else { $null }
    freeBytes = if ($null -ne $volume) { [uint64]$volume.FreeSpace } else { $null }
    totalBytes = if ($null -ne $volume) { [uint64]$volume.Size } else { $null }
  }
  power = [ordered]@{ highPerformance = $highPerformance; acPower = $batteryState; rawSchemeObserved = $powerText }
  cache = [ordered]@{ controlled = $false; note = "OS cache state not controlled; warm-up is first run after process start" }
  processPresence = $processPresence
  processInspectionState = $processInspectionState
  activityFindings = $activityFindings
  unknownActivity = $unknownActivity
  testLock = [ordered]@{ state = $lockState; locks = $lockRecords; smokeReadable = $lockReadable; smokeOwnerAlive = $lockOwnerAlive }
  defender = [ordered]@{ selected = $strictQuietHost; state = $defenderState; readable = $defenderReadable; realTimeProtection = if ($null -ne $mpStatus) { [bool]$mpStatus.RealTimeProtectionEnabled } else { $null }; matchingFixturePathExclusions = $fixturePathExclusions; volumeRootExclusionAccepted = $false }
  idle = [ordered]@{ sampleCount = 60; cpu = $cpuStats; disk = $diskStats; cpuState = $cpuIdleState; diskState = $diskIdleState; samples = $idleSamples }
  pass = $preflightPass
}
$preflight | ConvertTo-Json -Depth 10 |
  Set-Content -LiteralPath (Join-Path $reportsRoot "host-preflight.json") -Encoding utf8
if (-not $preflightPass) {
  throw "host preflight failed; preserve host-preflight.json and do not start the benchmark"
}
```

The repository-minimum profile passes only with known local-NTFS/AC state, no
active test lock, no observed prohibited activity, confirmed agent/workload
quiescence, and a known process inspection. Node/Codex/T3/DriveFS presence is
retained as observation; it is not by itself contention. A known, classified
non-prohibited command line remains presence-only; an inaccessible command
line remains `unknown` and fails the repository gate. The strict profile
additionally requires its selected High performance, DriveFS, exact fixture
path exclusion, and 60-sample idle controls. A whole-volume exclusion never
satisfies the Defender control. Any selected control that is missing or
unknown fails closed. The block changes no Defender, DriveFS, process, or lock
state and never terminates or archives anything.

### 5. Execute the prescribed current-only run

```powershell
$reportPath = Join-Path $reportsRoot "benchmark-current-raw.json"
$consoleLog = Join-Path $reportsRoot "benchmark-current-console.log"
$benchmarkScript = Join-Path $repoRoot "scripts\run-benchmark.mjs"

& $nodePath $benchmarkScript `
  --manifest $manifestPath `
  --bin $binaryPath `
  --iterations 10 `
  --cancel-iterations 3 `
  --cancel-after-ms 250 `
  --settle-ms 300 `
  --memory-interval-ms 100 `
  --keep-fixture `
  --out $reportPath 2>&1 | Tee-Object -FilePath $consoleLog
$harnessExit = $LASTEXITCODE
```

The #112 Node harness accepts `--warmup`, but its execution count is the
hard-coded `WARM_UP_RUNS = 1`; the option is not a protocol override and is
therefore omitted. The other options above are actual #112 options: `--manifest`,
`--bin`, `--iterations`, `--cancel-iterations`, `--cancel-after-ms`,
`--settle-ms`, `--memory-interval-ms`, `--keep-fixture`, and `--out`. The
separate build step intentionally does not use `--cargo`; the exact pinned
binary is supplied with `--bin`. The native driver receives only `--root`,
`--db`, `--settle-ms`, and (for cancellation) `--cancel-after-ms`.

Expected console lines identify `custom-9995`, seed `0`, the canonical
manifest hash, one warm-up, 10 measured iterations, statistics with `n=10`,
and cancellation statistics with `n=3`. The harness exit code is not the scan
disposition; inspect the JSON even when the harness writes it successfully.

For review, the complete #112 Node option surface is: `--manifest`, `--size`,
`--files`, `--seed`, `--iterations`, `--warmup`, `--cancel-iterations`,
`--cancel-after-ms`, `--settle-ms`, `--memory-interval-ms`, `--cargo`, `--bin`,
`--keep-fixture`, `--out`, and `--help`/`-h`. This run uses the manifest path and
the options shown above; it deliberately does not use the generator-mode
options (`--size`, `--files`, `--seed`), build override (`--cargo`), or help.

The #112 report contract is narrower than the driver protocol. At the top
level the actual report fields are `schema`, `role`, `capturedAtIso`, `machine`,
`cpu`, `memoryTotalBytes`, `powerMode`, `fixtureVolume`, `fixture`, `build`,
`measurement`, `warmUp`, `iterations`, `statistics`, `cancellation`,
`cancellationStatistics`, `memory`, `budgets`, and `notes`. Each attempt record
uses the actual camel-case fields `label`, `exitCode`, `totalMs`, `scanMs`,
`status`, `authoritative`, `outcome`, `locationCount`, `generation`,
`changesComputed`, `errorCode`, `failureCode`, `failureMessage`,
`partialClass`, `stderrObserved`, `memory`, and sanitized `lines`; cancellation
records additionally use `cancellationRequestedMs` and `stopLatencyMs`.
The native driver's `scan_ms`, `location_count`, and `partial_class` are
sanitized into those report fields; the validator does not invent alternate
names. The validator derives `medianMs`, `maxMs`, `nearestRankP95Ms`, and
`sampleCount` from the validated authoritative measured records and compares
all four values with the reported aggregates. It uses the harness's exact
median and nearest-rank calculations, with no timing rounding. Cancellation
statistics are derived separately from validated terminal cancellation
records' `stopLatencyMs`; cancellation `scanMs` is not a substitute for
cooperative-stop latency. Every warm-up, measured, and cancellation attempt
must also have the driver's successful `exitCode` of `0`. The actual
`measurement` fields also include `warmUpRuns`,
`measuredIterations`, `statistics`, `firstRunReportedSeparately`,
`sampleIntervalMs`, `cancelAfterMs`, and `settleMs`. Memory MiB values may be
fractional; counts remain integers.

### 6. Validate, retain, and hash every result

```powershell
$summaryPath = Join-Path $reportsRoot "qualification-summary.json"
$validatorPath = Join-Path $repoRoot "scripts\validate-qualification-report.mjs"
$validatorNode = Join-Path $repoRoot ".tools\node-v24.20.0-win-x64\node.exe"

if (-not (Test-Path -LiteralPath $validatorNode -PathType Leaf)) {
  throw "pinned Node is required to run the report validator"
}

# The validator is read-only with respect to the host. It writes only the
# requested summary and returns non-zero for every non-qualifying case.
& $validatorNode $validatorPath `
  --report $reportPath `
  --summary $summaryPath `
  --expected-fixture "custom-9995" `
  --expected-seed "0" `
  --expected-manifest "a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a" `
  --expected-locations 10000 `
  --expected-iterations 10 `
  --expected-cancellations 3 `
  --harness-exit $harnessExit
$validatorExit = $LASTEXITCODE

$hashTargets = @($binaryPath, $manifestPath, $reportPath, $consoleLog,
  (Join-Path $reportsRoot "source-boundary.json"),
  (Join-Path $reportsRoot "release-build.json"),
  (Join-Path $reportsRoot "fixture-verification.json"),
  (Join-Path $reportsRoot "host-preflight.json"),
  $summaryPath)
$hashes = $hashTargets | ForEach-Object {
  $item = Get-Item -LiteralPath $_
  [ordered]@{ artifact = $item.Name; bytes = $item.Length; sha256 = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
}
$hashes | ConvertTo-Json -Depth 5 |
  Set-Content -LiteralPath (Join-Path $reportsRoot "artifact-hashes.json") -Encoding utf8
if ($validatorExit -ne 0) {
  throw "non-qualifying; preserve the complete report and see qualification-summary.json"
}
```

The report must retain all 10 measured records, the warm-up, and all 3
cancellation records, including failures. The summary must make
`disposition`, `successfulAuthoritative`, `authoritativeSampleCount`,
`failedOrNonAuthoritative`, and `attemptFailures` explicit. The validator
derives scan statistics only from validated authoritative `Published`/
`Complete` measured records with `exitCode: 0` and valid `scanMs`; it
derives cancellation statistics only from validated terminal `Cancelled`
records with `exitCode: 0` and valid `stopLatencyMs`. A reported aggregate
that differs from those derived values is inconsistent and makes the run
non-qualifying. Budget checks use the derived measurements, not an unverified
summary. A failed attempt, missing bounded/fixed diagnostic, missing
cancellation metric, or missing memory metric remains non-qualifying even if
a successful subset's p95 is under 10,000 ms; failed records and their
dispositions remain in the summary.

## Pass/fail disposition

The run is qualified only if all of the following hold:

- the selected preflight passes before timing;
- warm-up is `Published` / `Complete` / authoritative with 10,000 locations
  and driver `exitCode: 0`;
- all 10 measured records are `Published` / `Complete` / authoritative with
  10,000 locations, driver `exitCode: 0`, and valid timings (`n=10`);
- the warm-up scan time is `<=30,000 ms`;
- measured nearest-rank warm p95 is `<=10,000 ms` (with `n=10`, this is the
  maximum; report median and maximum too), and all reported scan aggregates
  match the derived values;
- all 3 cancellation records are terminal `Cancelled` with valid stop latency
  and driver `exitCode: 0`, with cancellation p95 `<=1,000 ms` derived from
  `stopLatencyMs` rather than scan duration;
- the required sanitized diagnostics and driver working-set metrics exist;
- the report, provenance, raw console log, manifest, binary, and hashes are
  retained.

The UI acknowledgement target `<=250 ms` is not measured by this driver and
must not be inferred from worker stop latency. The 10,000-entry driver memory
measurement is recorded with its working-set caveat; it is not a pass for the
100,000-entry private-memory budget.

The validator treats a `Partial` result as retained diagnostic evidence, not a
successful sample. A bounded `partialClass` makes the record compatible with
the #112 sanitized schema, but the run is still non-qualifying when that
record prevents the required ten authoritative measured samples. Missing or
malformed numeric fields, inconsistent aggregates, missing iterations, missing
metrics, or a failed cancellation record are likewise non-qualifying. No
timing value is rounded before aggregate comparison or a target comparison.
This validator result still does not establish host eligibility, source
provenance, or the remaining Phase 2 acceptance gates.

Disposition rules are fail-closed:

- failed preflight: abort before timing; report `non-qualifying`;
- fewer than 10 measured records: non-qualifying;
- any non-authoritative, non-`Published`/`Complete`, or incomplete-location
  measured record: non-qualifying;
- a `Partial` or other failed record without its bounded/fixed diagnostic:
  non-qualifying;
- a target miss: report the measured miss and retain all attempts;
- a successful subset never converts the full run into a pass.

Do not repeat a sample to obtain a pass, discard an outlier, change limits or
budgets, use the 10,005 fixture as if it were quota-fitting, or relabel the
historical unexplained #108 `Failed`/`Partial` as fixed. The original
10,005-observation `ResourceLimit` evidence remains intact and separate from
this proposed 10,000-observation qualification.

## Minimal owner decisions

1. Record the recommended `custom-9995`, seed `0`, and canonical manifest hash
   above, or explicitly choose the alternative. The alternative requires its
   own approved protocol and is not silently substituted.
2. Select the current-only qualification protocol in this runbook, or name the
   specific unresolved question that justifies adding historical A/B runs.
3. Provide a dedicated host window and set the selected profile plus the
   quiescence attestations. If `strict-quiet-host` is selected, the owner
   supplies the DriveFS pause/absence, a non-volume Defender exclusion that
   covers the exact disposable fixture path, High performance state, and
   60-second idle proof. This task changes none of those settings and
   terminates no unrelated process.

After those choices and the source-stack merge, the command sequence above is
ready to run. No further benchmark investigation or broad acceptance-packet
rewrite is required for this bounded qualification attempt.

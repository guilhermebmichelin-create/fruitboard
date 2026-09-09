# Installed-app Windows journey checklist

Status: **Executed on an unsigned installed Windows package; Phase 2
acceptance remains pending.** The run used current `main` at
`0b7612db3570e6235d4d2c86a678dd9004264f30`, verified equal to `origin/main`
before the run, with `packaging-smoke,scan-console` enabled. The current
[Foundation CI run](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34293250231)
completed successfully. That result is build and contract evidence; the
installed-app observations are recorded in the table and linked run record
below.

This checklist keeps expected behavior separate from operator observations. Do
not copy expected text into the evidence table. Leave a row empty when a step
was not run or could not be observed.

## Evidence boundary

Keep these evidence classes separate in the handoff:

- merged commit and remote CI provenance;
- hidden worker and deterministic lifecycle tests;
- fake-adapter rendering and native typed adapter tests;
- installed-app observations recorded by this checklist;
- F1-F3 benchmark findings and the owner's budget decisions;
- owner acceptance for P2-01 through P2-12.

The installed journey does not promote an acceptance ID by itself. It also does
not qualify DriveFS (#47), cross-volume or FAT32 identity (#48), network shares,
ACL revocation during a watch, or real OS buffer-overflow timing.

## Pinned build and package commands

Run from the repository root on Windows PowerShell. In this shared worktree,
derive the sibling `.tools` directory from the checkout instead of recording a
machine-specific username or absolute path:

```powershell
$repositoryRoot = (Get-Location).Path
$toolCandidates = @(
  (Join-Path $repositoryRoot ".tools"),
  (Join-Path $repositoryRoot "..\.tools"),
  (Join-Path $repositoryRoot "..\..")
)
$reviewTools = $toolCandidates |
  Where-Object {
    (Test-Path -LiteralPath (Join-Path $_ "cargo-home\bin\cargo.exe") -PathType Leaf) -and
    (Test-Path -LiteralPath (Join-Path $_ "node-v24.20.0-win-x64\node.exe") -PathType Leaf) -and
    (Test-Path -LiteralPath (Join-Path $_ "uv-0.12.9\uv.exe") -PathType Leaf)
  } |
  Select-Object -First 1
if ($null -eq $reviewTools) {
  throw "The shared .tools directory was not found relative to the checkout."
}
$reviewTools = (Resolve-Path -LiteralPath $reviewTools).Path
$env:CARGO_HOME = Join-Path $reviewTools "cargo-home"
$env:RUSTUP_HOME = Join-Path $reviewTools "rustup-home"
$env:Path = (Join-Path $reviewTools "node-v24.20.0-win-x64") + ";" +
  (Join-Path $env:CARGO_HOME "bin") + ";" +
  (Join-Path $reviewTools "uv-0.12.9") + ";" + $env:Path

node --version       # v24.20.0
pnpm.cmd --version    # 11.25.0
uv.exe --version      # 0.12.9
rustc --version       # rustc 1.98.1 (...)
git rev-parse HEAD    # record the combined merged commit
uv.exe sync --frozen --python 3.11.16
pnpm.cmd install --frozen-lockfile --ignore-scripts
pnpm.cmd check        # default feature-off/release and repository gates
cargo test -p fruitboard-desktop --features scan-console --locked
cargo clippy -p fruitboard-desktop --features scan-console --all-targets --locked -- -D warnings
```

The installed journey package uses the repository's existing isolated
Foundation Smoke flavor. Build its inert sidecar first, then package the same
Tauri app with both the existing packaging feature and `scan-console` enabled:

```powershell
node scripts/prepare-foundation-sidecar.mjs
pnpm.cmd --filter @fruitboard/desktop exec tauri build --ci --no-sign `
  --config src-tauri/tauri.package.conf.json `
  --features packaging-smoke,scan-console --bundles nsis
```

The Tauri CLI accepts comma-separated features. The resulting unsigned NSIS
installer is under `target\release\bundle\nsis`; use the exact installer path
printed by that completed build when installing. The existing
`pnpm.cmd package:windows:smoke` command enables only `packaging-smoke`; it is
the Phase 1 packaging smoke and is not the feature-enabled scanner package.
Do not select an arbitrary newest artifact when stale bundles are present.

## Disposable fixture and database setup

The generator's baseline is deliberately larger than one selected leaf. Use
the deterministic baseline to provide the seed and manifest identity, then add
the single generated leaf as the scan root. The leaf has ten FLP-named files,
so this journey does not silently turn the unresolved F1 quota finding into a
baseline qualification claim. The full generated baseline still contains
10,005 locations on Windows; F1 remains open in the
[budget decision brief](budget-decision-brief.md).

```powershell
$journeyId = Get-Date -Format "yyyyMMdd-HHmmss"
$journeyRoot = Join-Path $env:TEMP "fruitboard-installed-journey-$journeyId"
$fixtureRoot = Join-Path $journeyRoot "fixture"
New-Item -ItemType Directory -Path $journeyRoot | Out-Null
node scripts/generate-synthetic-tree.mjs --size baseline --seed 20260908 --out $fixtureRoot
$scanRoot = Join-Path $fixtureRoot "roots\shard-0000-sunset-beat"
if (-not (Test-Path -LiteralPath $scanRoot -PathType Container)) {
  throw "The deterministic baseline leaf was not generated."
}
Get-ChildItem -LiteralPath $scanRoot -File | Select-Object Name,Length
```

Record the generator's printed `manifest sha-256`, seed `20260908`, the
relative scan root `roots/shard-0000-sunset-beat`, and the generated manifest
file. The leaf has ten FLP-named files and exercises the four-record Library
page size. The fixture contains only synthetic marker bytes; do not add private
FLP files or source content.

If a longer cancellation window is needed, generate a separate fixture and
record its own seed and manifest hash. Keep it outside the first fixture so a
registered Library root and its retained rows are never an ancestor of the
cancellation root:

```powershell
$cancelFixtureRoot = Join-Path $journeyRoot "cancellation-fixture"
node scripts/generate-synthetic-tree.mjs --size baseline --seed 20260909 --out $cancelFixtureRoot
$cancelRoot = Join-Path $cancelFixtureRoot "roots"
if (-not (Test-Path -LiteralPath $cancelRoot -PathType Container)) {
  throw "The separate cancellation fixture was not generated."
}
```

The package config uses identifier `com.fruitboard.desktop.foundation-smoke`,
so Tauri stores this disposable app's database at:

```powershell
$appDataRoot = Join-Path $env:LOCALAPPDATA "com.fruitboard.desktop.foundation-smoke"
$databasePath = Join-Path $appDataRoot "storage\fruitboard.db"
```

Close every previous Foundation Smoke process before setup. Start only when
`$appDataRoot` does not exist; if it exists, preserve it for review or move it
to a run-specific archive after confirming the app is closed. Do not delete a
user's normal `com.fruitboard.desktop` data directory. Install the NSIS package
into a directory beneath `$journeyRoot`, using the same silent current-user
pattern as the existing Windows packaging smoke:

```powershell
$installRoot = Join-Path $journeyRoot "Installed Fruitboard"
$installerPath = Read-Host "Exact NSIS installer path printed by the completed build"
if ([string]::IsNullOrWhiteSpace($installerPath)) {
  throw "An exact NSIS installer path is required."
}
$installerPath = (Resolve-Path -LiteralPath $installerPath).Path
$installerPathIsFile = Test-Path -LiteralPath $installerPath -PathType Leaf
if (-not $installerPathIsFile) {
  throw "The supplied installer path is not a file."
}
$bundleDirectory = (Resolve-Path -LiteralPath "target\release\bundle\nsis").Path.TrimEnd("\") + "\"
if (-not $installerPath.StartsWith($bundleDirectory, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw "The installer must be inside target\release\bundle\nsis."
}
Get-FileHash -LiteralPath $installerPath -Algorithm SHA256
$installProcess = Start-Process -FilePath $installerPath `
  -ArgumentList @("/S", "/D=$installRoot") -Wait -PassThru -WindowStyle Hidden
if ($installProcess.ExitCode -ne 0) { throw "The NSIS install failed." }
if (-not (Test-Path -LiteralPath (Join-Path $installRoot "fruitboard-desktop.exe") -PathType Leaf)) {
  throw "The installed executable was not created."
}
```

Launch the installed `fruitboard-desktop.exe`, add only `$scanRoot`, and close
the app before recording a stable database SHA-256 after the first durable root
mutation. Keep the fixture and database until the evidence review is complete.
At cleanup, close the app, use the installed `uninstall.exe`, and archive or
remove only the run-specific `$journeyRoot` and the dedicated Foundation Smoke
data directory after the owner has accepted the record.

```powershell
Get-FileHash -LiteralPath $databasePath -Algorithm SHA256
```

## Expected observations

These are expectations to check against the app. They are not evidence.

| Step                         | Operator action                                                                                                                                                                      | Expected observation                                                                                                                                                                                                                                          |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1. Provenance                | Record merged commit, installer hash, tool versions, Windows build, fixture seed/hash, selected relative root, and database path/hash.                                               | The record identifies one combined merged commit and one disposable fixture/database pair.                                                                                                                                                                    |
| 2. Root selection/cancel     | Open Preferences, choose the generated leaf, enter a recognizable display name, then cancel a second picker interaction.                                                             | The selected root is shown with its display name; picker cancellation leaves existing settings unchanged and starts no scan.                                                                                                                                  |
| 3. Enabled settings          | Leave the root enabled and close/reopen the app before a user-started scan.                                                                                                          | The root and enabled state survive restart. Startup may enqueue recovery work for an enabled root, so record whether queued/running work appears and do not label it user-started.                                                                            |
| 4. Scan now                  | Start a scan and observe queued, running, and terminal states.                                                                                                                       | Status is durable and honest, counts and Cancel are available while work runs, and no fabricated percentage is displayed when total work is unknown. If a transient state finishes too quickly, record it as unobserved rather than inferring it.             |
| 5. Library paging            | Open Library and inspect the first page, next page, and return to the first page.                                                                                                    | The ten-file leaf exercises the four-record page size. Rows show filename, owning root, root-relative path, byte size, modified time, and present/missing state; no inferred FLP metadata or grouping is shown.                                               |
| 6. Restart survival          | Close and relaunch after a committed scan; if work is interrupted, relaunch during that work.                                                                                        | The committed list and root settings remain. Startup may enqueue a recovery scan for an enabled root, so record whether recovery appears; do not describe it as a user-started scan.                                                                          |
| 7. Add/rename/modify         | Add one synthetic `.flp`, rename one existing file, and change one file's metadata; run the documented manual or watcher follow-up.                                                  | Added and renamed locations converge, modified metadata changes without a content claim, and one authoritative reconciliation determines presence.                                                                                                            |
| 8. Remove/restore            | Remove one tracked file, reconcile, then restore it at the same location and reconcile again.                                                                                        | The removed location remains visible as missing after an authoritative run and returns to present after restoration.                                                                                                                                          |
| 9. Watcher follow-up         | With the combined supervisor available, burst changes and observe the follow-up/reconciliation.                                                                                      | Bursts coalesce to at most one queued follow-up per root; coverage loss requests full reconciliation, stale generations are ignored, and hints alone never mark a file missing.                                                                               |
| 10. Cancel/unavailable/retry | Use `$cancelRoot` if the selected leaf finishes before Cancel can be observed; cancel a running scan, then make the root unavailable, request a scan, restore access, and use Retry. | Cancellation is persisted when observed; if the transient acknowledgement cannot be observed, record it as pending. Unavailable or failed work retains the last committed Library view and labels the safe failure; Retry can reconcile after access returns. |
| 11. Cleanup                  | Close the app and uninstall after preserving the run record.                                                                                                                         | The evidence record remains reviewable and only the dedicated disposable package/data paths are cleaned up.                                                                                                                                                   |

## Installed-app evidence table

Fill this table only from the packaged, installed Windows application at the
combined supervisor commit. A screenshot, fake adapter, hidden worker log, or
unit test belongs to its own evidence class and does not fill an installed-app
row.

| Step                         | Observed result                                                                                                                                                                                                                                                                | Evidence link or artifact                                                             | Operator/status                                    |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------- | -------------------------------------------------- |
| 1. Provenance                | Current `main`/`origin/main` SHA, toolchain, Windows build, package features, installer/executable hashes, fixture seeds/hashes, selected relative roots, and isolated database hashes are recorded.                                                                           | [Run record](installed-journey/run-20260908.md#provenance-and-build)                  | Installed native; complete                         |
| 2. Root selection/cancel     | Real Windows `Selecionar pasta` picker: cancellation left Preferences unchanged; selecting the generated leaf added it, and the display name was saved.                                                                                                                        | [Run record](installed-journey/run-20260908.md#root-selection-and-settings)           | Installed native; complete                         |
| 3. Enabled settings          | `Installed Journey Root` remained enabled, was renamed, and startup view `Library` persisted across a clean close/relaunch. Startup recovery was observed separately and was not labeled user-started.                                                                         | [Run record](installed-journey/run-20260908.md#root-selection-and-settings)           | Installed native; complete                         |
| 4. Scan now                  | Native `Scan now` reached `Running` with counters, `Total work unknown`, no percentage, and `Cancel`; a durable queued state was not captured. Cancellation reached `Cancelled` with the safe no-publication message.                                                          | [Run record](installed-journey/run-20260908.md#native-scan-cancel-and-retry)          | Running/cancelled observed; queued unverified      |
| 5. Library paging            | Four-record pages were exercised across pages 1–3 and back. Rows showed filename, owning root, root-relative path, bytes, modified time, and Present/Missing state; no FLP metadata was inferred.                                                                              | [Run record](installed-journey/run-20260908.md#library-paging)                        | Installed native; complete                         |
| 6. Restart survival          | Committed rows and root settings survived clean restart. After an immediate process termination during native work, the DB retained an interrupted run (`error_code=restart`) and the same job completed at attempt 2 after relaunch.                                          | [Run record](installed-journey/run-20260908.md#restart-and-interrupted-work-recovery) | Installed native; complete                         |
| 7. Add/rename/modify         | Watcher follow-up published an added synthetic file, retained the old renamed path as Missing while showing the new path Present, and updated modified metadata.                                                                                                               | [Run record](installed-journey/run-20260908.md#watcher-follow-ups-and-convergence)    | Installed native; complete                         |
| 8. Remove/restore            | Removing a tracked synthetic file made its committed row Missing; recreating the same path made it Present after the next watcher follow-up.                                                                                                                                   | [Run record](installed-journey/run-20260908.md#watcher-follow-ups-and-convergence)    | Installed native; complete                         |
| 9. Watcher follow-up         | External add/rename/mtime/remove/restore operations triggered native follow-ups, reset pagination after snapshot changes, and retained prior committed rows during incomplete work. Burst coalescing bounds were not independently counted.                                    | [Run record](installed-journey/run-20260908.md#watcher-follow-ups-and-convergence)    | Follow-up observed; bound unverified               |
| 10. Cancel/unavailable/retry | Cancel and unavailable-root retention were observed. Prior rows stayed visible. Retry after cancellation returned `The scan state changed. Refresh the status and try again.`; restored unavailable work also did not converge after its automatic retry budget was exhausted. | [Run record](installed-journey/run-20260908.md#native-scan-cancel-and-retry)          | Defects/gaps recorded; successful Retry unverified |
| 11. Cleanup                  | Graceful close completed; the installed NSIS package was uninstalled with exit code 0. The run-specific synthetic fixtures, dedicated DB, and recoverable pre-existing Foundation Smoke archive remain for evidence review; normal user data was not touched.                  | [Run record](installed-journey/run-20260908.md#cleanup-and-boundary)                  | Package removed; review artifacts retained         |

## Gate and handoff

The operator reports the exact combined commit, the feature-enabled CI URL,
toolchain/build commands, installer and database hashes, fixture seed and
manifest hash, observed limitations, and this completed table to the owner.
P2-03 through P2-08 remain unaccepted until their integrated evidence is
reviewed. F1 (quota versus 10,005 baseline observations), F2 (warm p95 versus
the provisional 10 s target), and F3 (the unmeasured 100,000-entry set) remain
owner decisions; this journey does not amend a budget. Production scanning
stays hidden pending the acceptance review.

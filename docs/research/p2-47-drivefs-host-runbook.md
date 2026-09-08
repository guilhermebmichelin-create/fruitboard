# P2 manual host runbook — #47 DriveFS watcher and placeholder behavior

- Status: **READY — NOT EXECUTED.** No DriveFS claim is made by this file.
- Parent: #33. Follows #42 (partial result in
  `docs/research/p0-d-drivefs-watcher.md`). Until this runbook is executed
  or the owner posts an explicit scope-exclusion sentence, DriveFS behavior
  stays labeled **unverified** and no Drive claim may be made for #36/#37.
- Dependents: P2-09 (watcher bursts/overflow/event loss) stays **Pending**;
  this file does not promote P2-07, P2-09, or P2-12, and does not amend
  `docs/PHASE_2_EXECUTION_PLAN.md`.
- Research rules: state the question, environment, inputs, observed result,
  limitations, and resulting decision; disposable synthetic trees with
  recorded seed and manifest hash; never add private files, credentials,
  personal paths, account identifiers, or raw diagnostic dumps; an ignored
  fixture or an unavailable environment is **not a pass**.

## Question (#47 DriveFS)

Can the scanner rely on watcher events and placeholder metadata on
DriveFS-backed roots — mirrored versus streamed enumeration, placeholder
versus hydrated metadata, hydration side effects of metadata-only reads,
and burst/rename/disconnect watcher fidelity?

## Preconditions (operator checklist — all must be recorded before step 1)

- [ ] Windows 11 host with Drive for Desktop installed, signed in, and
  actively syncing (tray status + `GoogleDriveFS.exe` running).
- [ ] Drive for Desktop version recorded (see environment block commands).
- [ ] Mount identity recorded: drive letter or mount path **plus** the
  volume GUID from `mountvol`. Do not traverse personal Drive content;
  only the disposable `fruitboard-drivefs-<run-id>` leaves below are touched.
- [ ] Active content-caching mode read from the Drive for Desktop settings
  UI (**Mirror files** vs **Stream files**) and recorded per run. Local
  client state databases are not parsed; the UI is authoritative.
- [ ] Owner consent recorded for: (a) creating disposable synced churn on
  the live Drive (all leaves removed after the run), (b) switching the
  caching mode + full resync between the mirrored and streamed runs,
  (c) pausing sync / disconnecting mid-watch for step 5 with a verified
  resume.
- [ ] Pinned baseline: `git rev-parse HEAD` equals the commit in the
  environment block; `.tools` toolchain versions verified with
  `node scripts/verify-toolchains.mjs`.

## Host state observed 2026-09-08 (preconditions, not results)

Recorded read-only on the operator host at baseline `51f45af` (PR #84).
The DriveFS run itself is still blocked on the unchecked boxes above.

- Windows 11, build `10.0.26200`, 64-bit.
- Drive for Desktop `125.0.0.0`
  (`$env:ProgramFiles\Google\Drive File Stream\<version>\GoogleDriveFS.exe`),
  process running, account signed in, actively syncing.
- Authenticated virtual mount present (drive letter + `mountvol` GUID
  recorded by the operator at run time; exposes the signed-in user root). WMI reports the virtual filesystem as FAT32 — this is the
  DriveFS virtual-filesystem report, **not** a real FAT32 implementation,
  and it does not qualify as the #48 FAT32 volume.
- Both stream-state and mirror-state client databases exist with recent
  write activity, so the **active mode cannot be inferred from outside** —
  step 0 (UI-reported mode per run) is mandatory.
- `fsutil file queryfileid` works non-elevated on NTFS; `fsutil fsinfo
  volumeinfo/ntfsinfo/sectorinfo` is access-denied non-elevated and
  path-not-found on the virtual DriveFS mount. Elevation and inference
  limits are noted wherever `fsutil` is used below.
- Local harness smoke (same host, same commit, `C:` NTFS disposable trees
  under `%TEMP%`, auto-removed): `research-fs-probe.ps1 -Mode Identity`
  10/10 PASS; `-Mode Watcher` 50 created + 1 renamed + 1 deleted, then 10
  append-phase Changed with 0 errors, PASS. This validates that the
  procedures below execute — it is **not** #47 evidence.

## Environment block (fill per run; serials are run identifiers, not personal data)

```text
Windows build:
Drive for Desktop version:
Mount (letter/path + mountvol GUID):
Active mode this run (Mirror files | Stream files, from Drive UI):
Baseline commit (git rev-parse HEAD):
node / rustc / uv versions (.tools pins):
Probe command + Rust harness invocation (if any):
Run ID:
Seed(s) + leaf manifest SHA-256:
```

Version commands:

```powershell
git rev-parse HEAD
node scripts/verify-toolchains.mjs
(Get-Item "$env:ProgramFiles\Google\Drive File Stream\*\GoogleDriveFS.exe").VersionInfo.ProductVersion
mountvol
```

## Disposable fixtures (per mode — run everything twice unless the owner excludes one mode)

Do **not** run the DriveFS steps against `%TEMP%` (NTFS) — that measures
the wrong filesystem. All leaves live on the Drive mount and are removed
after the run.

- Enumeration leaf: `<Drive>:\fruitboard-drivefs-<run-id>-enum` with 10
  FLP-named synthetic files (`enum-01.flp` … `enum-10.flp`). Content is the
  synthetic marker only (`FRUITBOARD SYNTHETIC FIXTURE. NOT AN FL STUDIO
  PROJECT. NO PRIVATE DATA.`); never hash, parse, or open contents during
  enumeration steps.
- Burst leaf: `<Drive>:\fruitboard-drivefs-<run-id>-burst` for the watcher
  load (50 creates, 1 rename, 1 delete, 10 spaced appends — the P0-D load).
- Disconnect leaf: `<Drive>:\fruitboard-drivefs-<run-id>-gap`, small leaf
  whose Drive root is paused/disconnected mid-watch by the operator.

Leaf creation + manifest (enumeration leaf example; record the printed SHA):

```powershell
$runId = "20260908-01"
$leaf = "<Drive>:\fruitboard-drivefs-$runId-enum"
New-Item -ItemType Directory -Path $leaf | Out-Null
$marker = "FRUITBOARD SYNTHETIC FIXTURE. NOT AN FL STUDIO PROJECT. NO PRIVATE DATA.`n"
1..10 | ForEach-Object {
  $name = "enum-{0:D2}.flp" -f $_
  [IO.File]::WriteAllText((Join-Path $leaf $name), ("$marker" + "seed=$runId slot=$name`n"))
}
$manifest = Get-ChildItem -LiteralPath $leaf -File | Sort-Object Name | ForEach-Object {
  "{0}  {1}" -f (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $_.Name
}
$manifestHash = [BitConverter]::ToString(
  [Security.Cryptography.SHA256]::Create().ComputeHash(
    [Text.Encoding]::UTF8.GetBytes(($manifest -join "`n") + "`n"))).Replace("-", "").ToLowerInvariant()
Write-Output "MANIFEST seed=$runId files=10 sha256=$manifestHash"
$manifest | Write-Output
```

Cleanup after each mode (verify empty afterwards):

```powershell
Remove-Item -LiteralPath $leaf -Recurse -Force
```

## Operations

### 1. Enumerate mirrored vs streamed leaves (no content reads)

Per file record: presence, size/mtime availability, placeholder vs
hydrated state via the `Offline` attribute flag. Never read contents.

```powershell
Get-ChildItem -LiteralPath $leaf -File | Sort-Object Name | ForEach-Object {
  $offline = [bool]($_.Attributes -band [IO.FileAttributes]::Offline)
  "{0} present=Y size={1} mtime={2:o} offline={3}" -f $_.Name, $_.Length, $_.LastWriteTime, $offline
}
```

### 2. Metadata-only hydration side-effect check (no hashing/parsing)

On one placeholder (`offline=True`) file per mode: snapshot
`Attributes`/`Length`/`LastWriteTime`, perform metadata-only reads (item
properties + open/close a read handle without reading), then re-snapshot
and record whether hydration was triggered (Offline cleared, size/state
difference, or sync activity). Deliberate hydration is out of scope.

```powershell
$target = Join-Path $leaf "enum-01.flp"
$before = Get-Item -LiteralPath $target
"BEFORE offline={0} size={1} mtime={2:o}" -f [bool]($before.Attributes -band [IO.FileAttributes]::Offline), $before.Length, $before.LastWriteTime
$handle = [IO.File]::Open($target, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
$handle.Close()
$after = Get-Item -LiteralPath $target
"AFTER offline={0} size={1} mtime={2:o}" -f [bool]($after.Attributes -band [IO.FileAttributes]::Offline), $after.Length, $after.LastWriteTime
```

### 3. Burst/rename/delete/append load per mode

Self-contained watcher procedure on the Drive burst leaf (same load shape
as `scripts/research-fs-probe.ps1 -Mode Watcher`, which only targets
`%TEMP%` and therefore cannot stand in for DriveFS). Record the OBSERVED
lines verbatim.

```powershell
$driveBurst = "<Drive>:\fruitboard-drivefs-$runId-burst"
New-Item -ItemType Directory -Path $driveBurst | Out-Null
$watcher = New-Object IO.FileSystemWatcher $driveBurst
$watcher.IncludeSubdirectories = $true
$watcher.EnableRaisingEvents = $true
$subscriptions = @(
  Register-ObjectEvent $watcher Created -SourceIdentifier "DriveCreated"
  Register-ObjectEvent $watcher Changed -SourceIdentifier "DriveChanged"
  Register-ObjectEvent $watcher Renamed -SourceIdentifier "DriveRenamed"
  Register-ObjectEvent $watcher Deleted -SourceIdentifier "DriveDeleted"
  Register-ObjectEvent $watcher Error -SourceIdentifier "DriveError"
)
try {
  1..50 | ForEach-Object {
    [IO.File]::WriteAllBytes((Join-Path $driveBurst ("burst$_.bin")), (New-Object byte[] 256))
  }
  Start-Sleep -Seconds 3
  Rename-Item -LiteralPath (Join-Path $driveBurst "burst1.bin") -NewName "burst1-renamed.bin"
  Remove-Item -LiteralPath (Join-Path $driveBurst "burst2.bin")
  Start-Sleep -Seconds 2
  $createdPhase = @(Get-Event -SourceIdentifier "DriveCreated" -ErrorAction SilentlyContinue).Count
  $renamedPhase = @(Get-Event -SourceIdentifier "DriveRenamed" -ErrorAction SilentlyContinue).Count
  $deletedPhase = @(Get-Event -SourceIdentifier "DriveDeleted" -ErrorAction SilentlyContinue).Count
  Write-Output "OBSERVED creation-phase created=$createdPhase renamed=$renamedPhase deleted=$deletedPhase"
  Remove-Event -SourceIdentifier "Drive*" -ErrorAction SilentlyContinue
  1..10 | ForEach-Object {
    [IO.File]::AppendAllText((Join-Path $driveBurst "burst3.bin"), "x")
    Start-Sleep -Milliseconds 100
  }
  Start-Sleep -Seconds 3
} finally {
  foreach ($subscription in $subscriptions) {
    Unregister-Event -SubscriptionId $subscription.Id -ErrorAction SilentlyContinue
  }
  $watcher.Dispose()
}
$appendChanged = @(Get-Event -SourceIdentifier "DriveChanged" -ErrorAction SilentlyContinue).Count
$appendErrors = @(Get-Event -SourceIdentifier "DriveError" -ErrorAction SilentlyContinue).Count
Remove-Event -SourceIdentifier "Drive*" -ErrorAction SilentlyContinue
Write-Output "OBSERVED append-phase changed=$appendChanged errors=$appendErrors"
Remove-Item -LiteralPath $driveBurst -Recurse -Force
```

### 4. Genuine overflow attempt (not-reproduced is a result, not a pass)

The P0-D 500-file / 4 KiB single-threaded attempt did not reproduce
overflow locally. Attempt a faster load per mode and record whether an
Error (overflow) event was actually observed, with the exact generator
parameters. Suggested starting point (tune and record what ran):

```powershell
$watcher.InternalBufferSize = 4096  # set before EnableRaisingEvents; record the value used
# Generator: N parallel jobs x M files of K bytes; record N/M/K and timing.
```

Record `overflow observed Y/N` plus generator parameters, or
`not reproduced` with parameters. Either is a finding; neither alone is
a pass.

### 5. Disconnect/reconnect hint + reconciliation outcome

With pause/disconnect consent recorded: start a watch on the gap leaf,
pause sync (or disconnect) mid-watch, create 3 synthetic files during the
gap, resume/reconnect, and record: hints delivered across the gap, hints
lost, or coverage-loss followed by full reconciliation; the
reconciliation outcome (authoritative or not — unseen files are never
marked missing without a successfully completed authoritative scan).

## Observations to record (per mode)

### Enumeration table (one row per file, per mode)

| Mode | File | Present | Size avail | Mtime avail | Offline (placeholder) | Hydrated |
| ---- | ---- | ------- | ---------- | ----------- | --------------------- | -------- |
| Mirrored | enum-01.flp … | | | | | |
| Streamed | enum-01.flp … | | | | | |

### Hydration side-effect table

| Mode | Target | Triggering op | Offline before | Offline after | Hydration observed |
| ---- | ------ | ------------- | -------------- | ------------- | ------------------ |
| Mirrored | | Get-Item + open/close handle | | | Y/N |
| Streamed | | Get-Item + open/close handle | | | Y/N |

### Watcher table (per mode)

| Mode | Created (expect 50) | Renamed (expect 1) | Deleted (expect 1) | Append Changed (≥1) | Error events | Coalescing observed |
| ---- | ------------------- | ------------------ | ------------------ | ------------------- | ------------ | ------------------- |
| Mirrored | | | | | | |
| Streamed | | | | | | |

### Overflow + disconnect table (per mode)

| Mode | Overflow observed | Generator params | Gap hints delivered/lost | Coverage-loss | Reconciliation outcome |
| ---- | ----------------- | ---------------- | ------------------------ | ------------- | ---------------------- |
| Mirrored | Y/N (or not reproduced + params) | | | | |
| Streamed | Y/N (or not reproduced + params) | | | | |

## Limitations (must be stated in the findings)

- Single host and single DriveFS version; results are not transferable to
  other versions, modes, or machines.
- OS-tool probes versus the Rust `notify`/handle-bound watcher are
  inference unless the Rust harness itself ran on the Drive root — label
  which ran (record the exact `cargo test` invocation when it runs).
- Sleep/resume and multi-machine sync races are out of scope unless
  explicitly exercised.

## Decision output

Commit written findings to `docs/research/` with a support/fallback matrix
(which Drive modes are supported, which fall back to
manual/full-reconciliation-only, which are excluded):

| Mode | Supported | Fallback (full-reconciliation-only) | Excluded |
| ---- | --------- | ----------------------------------- | -------- |
| Mirrored | | | |
| Streamed | | | |

…or return the explicit owner scope-exclusion sentence (owner-only; takes
effect only as an owner post — this file excludes nothing):

> "I approve excluding DriveFS-backed roots (mirrored and streamed) from
> the Phase 2 scanner scope: the scanner treats DriveFS mounts as
> unsupported roots (refuses or falls back to
> manual/full-reconciliation-only per the #47 support matrix) until #47
> host evidence lands and P2-09 Drive claims are explicitly re-opened."

## Reproduction

Run steps 0–5 above on Windows with an authenticated Drive for Desktop
mount, once per caching mode, at the pinned commit with verified
`.tools` toolchains. The local-only smoke
(`powershell.exe -NoProfile -ExecutionPolicy Bypass -File
scripts/research-fs-probe.ps1 -Mode Watcher`) validates the load shape on
NTFS `%TEMP%` trees; it is harness validation, not DriveFS evidence.

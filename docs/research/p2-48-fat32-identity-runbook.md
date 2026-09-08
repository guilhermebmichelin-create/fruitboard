# P2 manual host runbook — #48 cross-volume and FAT32 file identity

- Status: **READY — BLOCKED.** No second writable real FAT32 volume exists
  on the operator host (see host state). No cross-volume/FAT32 claim is
  made by this file.
- Parent: #33. Follows #43 (partial result in
  `docs/research/p0-e-file-identity.md`). Until this runbook is executed
  or the owner posts an explicit scope-exclusion sentence, non-NTFS
  identity stays labeled **unverified** and the #36 reconciler identity
  policy is **NTFS-local only**.
- Dependents: P2-07 (hardlink aliases and uncertain identity) stays
  **Partial pending #48**; this file does not promote P2-07, P2-09, or
  P2-12, and does not amend `docs/PHASE_2_EXECUTION_PLAN.md`.
- Research rules: state the question, environment, inputs, observed result,
  limitations, and resulting decision; disposable synthetic files with
  recorded seed and manifest hash; never add private files, credentials,
  personal paths, account identifiers, or raw diagnostic dumps; OS-tool
  results mapped to Rust file-identity APIs are labeled inference; an
  ignored fixture or an unavailable environment is **not a pass**.

## Question (#48 identity)

Where is (volume serial, file ID) trustworthy for continuing a file
locator outside local NTFS — same-file moves across volumes, FAT32
file-ID semantics, and placeholder hydration effects on identity?

## Preconditions (operator checklist — all must be recorded before step 1)

- [ ] Windows 11 host with two writable volumes: one NTFS and one real
  FAT32 (USB stick, attached VHD, or dedicated partition — record which,
  plus cluster size). The DriveFS virtual mount reporting FAT32 does
  **not** qualify (see host state).
- [ ] `fsutil` availability recorded (`fsutil file queryfileid` works
  non-elevated on NTFS; `fsutil fsinfo` subcommands need elevation —
  record whether the run was elevated).
- [ ] Both volume serials recorded (serials are environment identifiers
  for the run, not personal data).
- [ ] Pinned baseline: `git rev-parse HEAD` equals the commit in the
  environment block; `.tools` toolchain versions verified with
  `node scripts/verify-toolchains.mjs`.
- [ ] One synthetic file per operation row; the NTFS hardlink-alias pair
  plus the FAT32 hardlink attempt (FAT32 has no hardlinks — record the
  exact failure mode).

## Host state observed 2026-09-08 (blocked — not results)

Recorded read-only on the operator host at baseline `51f45af` (PR #84):

- `C:` NTFS (serial `184002EB`), writable. Local P0-E replay harness
  passes here (see smoke note).
- Virtual DriveFS mount present, WMI-reported as FAT32 (serial
  `19831116`), authenticated and syncing — this measures **DriveFS
  semantics**, not FAT32 semantics, and is **not** the #48 FAT32 volume.
  It may be used only for step 5 (optional hydration identity check).
- `D:` removable slot with no media (size 0) — a future USB FAT32 stick
  would appear here or under a new letter; not available now.
- No attached VHD and no second writable real FAT32 partition found
  (`Get-Volume` shows only `C:` NTFS plus sub-GB system volumes).
- **Missing precondition: a second writable real FAT32 volume.** The
  FAT32 replay (steps 1–4) cannot run on this host as-is. Provisioning is
  an operator-owned action (see below); until then this runbook stays
  READY and #48 stays unverified.
- Local harness smoke (same host, same commit, `C:` NTFS disposable tree
  under `%TEMP%`, auto-removed): `research-fs-probe.ps1 -Mode Identity`
  10/10 PASS, re-confirming the P0-E local-NTFS table. This is harness
  validation, not #48 evidence.

### Provisioning the missing FAT32 volume (operator-owned, data-loss warning)

Any option is acceptable; record which was used plus type/serial/cluster.
Formatting destroys data on the target — the operator confirms the
target device explicitly.

```powershell
# Option A: USB stick already formatted FAT32 — record letter, then:
Get-Volume -DriveLetter <LETTER> | Format-List DriveLetter, FileSystemLabel, FileSystem, AllocationUnitSize
# Option B: dedicated VHD (elevated, Hyper-V cmdlets):
New-VHD -Path <path>\fruitboard-fat32.vhdx -SizeBytes 2GB -Dynamic
Mount-VHD -Path <path>\fruitboard-fat32.vhdx
Initialize-Disk -Number <N> -PartitionStyle MBR
New-Partition -DiskNumber <N> -UseMaximumSize -DriveLetter <LETTER> | Format-Volume -FileSystem FAT32 -NewFileSystemLabel FRUITBOARD
Dismount-VHD -Path <path>\fruitboard-fat32.vhdx  # after the run; detach when done
```

Volume identity commands (record both serials; elevation noted):

```powershell
Get-CimInstance Win32_LogicalDisk -Filter "DeviceID='C:'" | Select-Object DeviceID, FileSystem, VolumeSerialNumber
Get-CimInstance Win32_LogicalDisk -Filter "DeviceID='<LETTER>:'" | Select-Object DeviceID, FileSystem, VolumeSerialNumber
fsutil fsinfo volumeinfo <LETTER>:\   # needs elevation; record if unavailable and use Get-Volume AllocationUnitSize instead
```

## Environment block (fill per run; serials are run identifiers, not personal data)

```text
Windows build:
NTFS volume (letter, type, serial, cluster size):
FAT32 volume (letter, provisioned via USB/VHD/partition, type, serial, cluster size):
fsutil available Y/N, elevated Y/N:
Baseline commit (git rev-parse HEAD):
node / rustc / uv versions (.tools pins):
Run ID + seed + manifest SHA-256:
```

## Disposable fixtures

One synthetic file per operation row under run-specific directories (all
removed after the run):

```powershell
$runId = "20260908-01"
$ntfsLeaf = "C:\fruitboard-48-ntfs-$runId"   # operator may relocate under %TEMP%
$fatLeaf = "<LETTER>:\fruitboard-48-fat32-$runId"
```

Record the seed, file/alias counts, and manifest SHA-256 the same way as
the #47 runbook leaf procedure (sorted `name + sha256` lines hashed into
one manifest SHA). Contents are the synthetic marker only.

## Operations

### 1. Replay the P0-E operation table on FAT32 (row by row)

For each row, record `fsutil file queryfileid` plus the volume serial
before and after; classify identity Stable vs New ID. Same shape as the
P0-E NTFS table.

```powershell
function Get-RowIdentity($LiteralPath) {
  $id = (fsutil file queryfileid "$LiteralPath" 2>&1 | Select-Object -First 1)
  Write-Output "path-free row: fileid=$id"
}
```

Rows: rename in directory; move within the volume; overwrite in place
(truncate + write); copy to a new path; replacement save via temp +
move-over; delete + recreate under the same name. Record serial
stability across all rows.

### 2. Cross-volume moves NTFS ↔ FAT32 (both directions)

Move one synthetic file NTFS → FAT32 and one FAT32 → NTFS; record
identity before and after. A cross-volume move is a copy + delete at the
filesystem level, so New ID is the expected honest result — record what
was actually observed and classify honestly (same file vs copy + delete).

### 3. FAT32 hardlink-attempt failure mode

Replay the NTFS hardlink-alias survival case on NTFS (pair survives —
P2-07 baseline), then attempt `New-Item -ItemType HardLink` on FAT32 and
record the exact failure mode (error text). Reconciler consequence to
confirm: aliases cannot exist there; each path is its own file.

```powershell
New-Item -ItemType HardLink -Path (Join-Path $fatLeaf "alias.flp") -Target (Join-Path $fatLeaf "primary.flp")
```

### 4. FAT32 timestamp granularity check (confirm or refute — do not assume)

The known FAT limitation is 2-second timestamp granularity. Set an odd
second and read back:

```powershell
$probe = Join-Path $fatLeaf "granularity.flp"
[IO.File]::WriteAllBytes($probe, (New-Object byte[] 64))
(Get-Item -LiteralPath $probe).LastWriteTime = Get-Date "2026-09-08 12:34:55"
(Get-Item -LiteralPath $probe).LastWriteTime.ToString("o")
```

Record observed behavior (rounded to an even second vs preserved).

### 5. Optional hydration identity check (only if a DriveFS session is available)

Re-check identity stability across a placeholder hydration transition on
the DriveFS mount (same before/after `queryfileid` pattern); otherwise
record hydration effects as **not run**. Never conflate this row with the
FAT32 table.

## Observations to record (tables)

### FAT32 operation × identity table (same shape as P0-E)

| Operation | Identity (Stable / New ID) | Serial stable |
| --------- | -------------------------- | ------------- |
| Rename within a directory | | |
| Move within the volume | | |
| Overwrite in place (truncate + write) | | |
| Copy to a new path | | |
| Replacement save (new temp + move over) | | |
| Delete + recreate under the same name | | |
| Hardlink attempt (failure mode, not an ID) | | |

### Cross-volume move table

| Direction | Identity before | Identity after | Honest classification (same file vs copy + delete) |
| --------- | --------------- | -------------- | -------------------------------------------------- |
| NTFS → FAT32 | | | |
| FAT32 → NTFS | | | |

### Granularity + hardlink notes

```text
FAT32 timestamp behavior observed:
FAT32 hardlink failure mode (exact error):
Hydration identity check (ran / not run + outcome):
```

## Limitations (must be stated in the findings)

- Host- and filesystem-implementation-specific; FAT32 behavior here does
  not transfer to exFAT, ReFS, or network filesystems.
- OS-tool (`fsutil`) results mapped to Rust identity APIs are inference
  unless the Rust enumerator itself ran on the FAT32 volume — label which
  ran (record the exact `cargo test` invocation when it runs).
- Network shares, ACL revocation mid-watch, and real overflow timing are
  separate unverified items, not covered by this run.

## Decision output

Commit written findings to `docs/research/`, extending the P2-03/P2-06
fallback posture with the P0-E policy (where identity may continue a
locator, where the reconciler must fall back to path-continuity-only
with explicit uncertainty, and the FAT32/cross-volume exclusion list):

| Locator situation | May continue via (serial, file ID) | Must fall back to path-continuity-only + uncertainty |
| ----------------- | ---------------------------------- | ---------------------------------------------------- |
| Same-volume NTFS rename/move | | |
| FAT32 same-volume ops (per findings) | | |
| Cross-volume moves | | |
| Replacement save / delete + recreate | | |

…or return the explicit owner scope-exclusion sentence (owner-only; takes
effect only as an owner post — this file excludes nothing):

> "I approve limiting the reconciler file-identity policy to local NTFS
> only: FAT32, exFAT, ReFS, network filesystems, and cross-volume moves
> stay unverified, identity continuity outside local NTFS falls back to
> path-continuity-only with explicit uncertainty, and P2-07 non-NTFS
> claims are explicitly re-opened only when #48 evidence lands."

## Reproduction

Run steps 1–5 above on Windows with one NTFS and one real FAT32 volume
at the pinned commit with verified `.tools` toolchains. The local-only
smoke (`powershell.exe -NoProfile -ExecutionPolicy Bypass -File
scripts/research-fs-probe.ps1 -Mode Identity`) replays the P0-E NTFS
table on `%TEMP%`; it is harness validation, not FAT32/cross-volume
evidence.

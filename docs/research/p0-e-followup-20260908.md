# P0-E follow-up: cross-volume and FAT32 identity — volume survey 2026-09-08

- Status: **remains unverified — no suitable FAT32/cross-volume target on this host**
- Date: 2026-09-08
- Baseline: `main` at `51f45af` (PR #84 merged, CI green); worktree branch
  `spike/p0-e-followup-20260908` created from `main` at that commit.
- Related: #48 (spike follow-up), #36/#40 (reconciler identity policy), P2-07
  (not promoted by this file). DriveFS (#47) is out of scope here.
- Predecessor: `docs/research/p0-e-file-identity.md` (partial — local NTFS
  only) and the #48 manual evidence plan in
  `docs/review/phase-2-integration/acceptance-prep-2026-09-08.md`.

## Question

Where is (volume serial, file ID) trustworthy for continuing a file locator
outside local NTFS — same-file moves across volumes, FAT32 file-ID
semantics, and placeholder hydration effects on identity? (#48)

## Environment

- Host: Windows 11, build 26200 (10.0.26200), 64-bit.
- Toolchain: system `node v26.4.0`; pinned `.tools` node `v24.20.0`;
  `rustc 1.98.1` / `cargo 1.98.1`; `.tools` `uv 0.12.9`;
  `fsutil.exe` present (system copy). Shell ran non-elevated.
- Git HEAD: `51f45af`.
- Probe command (volume survey only): `fsutil fsinfo volumeinfo` per test
  volume (drive-letter form), with `Win32_Volume` / `Win32_LogicalDisk` /
  `DriveInfo` / `Get-Volume` as equivalents, plus a DriveFS process check
  and a `Test-Path` existence check on the Drive letter (no directory
  listing, no writes outside the system temp volume).
- No `cargo test` invocation was run: with no qualifying target volume
  there is no Rust harness run to report.

## Volumes found

| Volume | Label / type | Filesystem | Test-role verdict |
| --- | --- | --- | --- |
| C: | `Windows-SSD`, fixed boot volume, serial `184002EB` | NTFS | Only writable real block volume; holds `%TEMP%`. Not a cross-volume pair by itself. |
| System partition (no drive letter) | `SYSTEM_DRV`, system volume | FAT32 (256 MiB) | Not a test target: system-owned, unmounted, no path to host a fixture. |
| Hidden partition (no drive letter) | — | NTFS (~1.1 GiB) | Not a test target: unmounted, no path to host a fixture. |
| D: | Removable slot | None (no media, not ready) | Missing: no medium inserted, no filesystem to probe. |
| G: | `Google Drive`, serial `19831116` | Reports FAT32; is the DriveFS virtual filesystem (DriveFS process active, round 200 GiB virtual size, hidden from `Get-Volume`, remote-storage support flag) | Excluded: DriveFS belongs to #47 and is out of scope for #48. Not used as FAT32 evidence. No writability probe was performed on it. |
| Network volumes | — | — | None present. |

Notes on the survey method, not identity evidence: `fsutil fsinfo volumeinfo`
on C: was refused without elevation (access denied); on G: it returned the
FAT32/Google-Drive description with the remote-storage flag paraphrased
above; on D: the device reported not ready. The table above rests on the
OS volume equivalents plus the DriveFS process identity. No personal Drive
content was listed and no file was written to G:.

## Disposable synthetic inputs

None generated. No tree was created under `%TEMP%\fruitboard-p0e-<run-id>`,
so there is no seed and no manifest SHA-256 to record, and nothing to
remove. Reason: the #48 plan requires a writable NTFS volume plus a
writable FAT32 volume, and that precondition is not met on this host — per
the plan, without it the run cannot happen and stays unverified.

## Observed result

- No writable FAT32/exFAT test volume exists on this host.
- No second writable real volume exists, so cross-volume move identity
  (NTFS to FAT32 and back) was not exercised.
- FAT32 operation table (rename / move / overwrite / copy / replacement
  save / delete-plus-recreate): not run.
- FAT32 hardlink-attempt failure mode: not run (FAT32 has no hardlinks —
  the failure mode remains unrecorded on a real volume).
- FAT32 timestamp granularity spot-check: not run.
- Placeholder-hydration identity effect: not run (DriveFS out of scope).
- The identity probe script (`scripts/research-fs-probe.ps1 -Mode Identity`)
  was deliberately not run: executing it on NTFS would only reproduce the
  closed P0-E local-NTFS findings and must not be presented as #48
  evidence. No NTFS identity observation is claimed by this file.

## Limitations

- Single host, single session, non-elevated token; results describe this
  host only.
- G: is excluded by scope (#47), not by a writability finding; nothing in
  this file characterizes DriveFS behavior.
- System partitions without drive letters were classified as unusable by
  inspection (system-owned, unmounted), not by a write attempt.
- OS-tool-to-Rust identity-API mapping is absent here, not even inference:
  no OS-tool identity probe ran because no target volume exists.
- Network shares, ACL revocation mid-watch, and real overflow timing remain
  separate unverified items, untouched by this survey.

## Resulting decision

Remains unverified — requires a Windows 11 host with two writable real
volumes (one NTFS plus one genuine FAT32 volume, e.g. USB stick, attached
VHD, or dedicated partition, with cluster size recorded), or an explicit
owner scope exclusion for FAT32 and cross-volume identity. Until either
lands, non-NTFS identity stays labeled unverified, the #36 reconciler
identity policy stays NTFS-local only, and P2-07 is not promoted. No
qualification is claimed by this file.

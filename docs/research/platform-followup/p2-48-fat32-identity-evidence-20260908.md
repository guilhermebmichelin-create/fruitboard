# #48 FAT32 and cross-volume identity evidence - 2026-09-08

- Status: **blocked - no safe writable real FAT32 target is available.**
- Run timestamp: `2026-09-08T21:49:01-03:00`.
- Baseline: `0b7612db3570e6235d4d2c86a678dd9004264f30` (`main` at the start of this evidence run).
- Related issue: [#48](https://github.com/guilhermebmichelin-create/fruitboard/issues/48).
- This file is new evidence under `docs/research/platform-followup/`; it does not edit or replace the #48 runbook.

## Scope and question

Issue #48 asks for same-file move behavior across volumes, FAT32 file-ID
semantics, hardlink and timestamp behavior, and the identity effect of DriveFS
placeholder hydration. The issue body has no follow-up comments at this run.

The prior volume survey explicitly found no suitable FAT32 or cross-volume
target. This run performed only a fresh read-only inventory to check whether
that prerequisite had changed. It did **not** repeat the local NTFS identity
probe, and no NTFS result below is new platform qualification.

## Read-only host inventory

The inventory did not enumerate any directory, inspect project contents, write a
fixture, format a volume, or alter DriveFS configuration.

| Volume/device | Read-only observation | #48 verdict |
| --- | --- | --- |
| `C:` | Fixed NVMe-backed NTFS, serial `184002EB`; this is the current writable work volume | NTFS side exists, but alone it cannot provide a cross-volume case |
| `D:` / USB device | Removable device reports `RAW`, `No Media`, size `0`; no filesystem or volume serial | Not usable |
| Unmounted system partition | FAT32, `SYSTEM_DRV`, 256 MiB, `SystemVolume=True`, no drive letter | Not a safe test target |
| Unmounted hidden partition | NTFS, approximately 1.1 GiB, no drive letter | Not a safe test target |
| `G:` | Label `Google Drive`, WMI reports FAT32 and serial `19831116`, but `GoogleDriveFS.exe` `125.0.0.0` is running and `mountvol` identifies a DriveFS virtual mount | Not real FAT32 evidence; reserved for #47 only |
| Attached disks | Disk 0 is the boot NVMe; Disk 1 is the no-media USB device. No second mounted real filesystem or attached VHD was visible in the read-only disk inventory | No qualifying second volume |

The shell was non-elevated. `fsutil.exe` is present at the system location,
but no `fsutil file queryfileid` operation was run because there was no
qualifying target. The DriveFS-looking `G:` mount was not used as a FAT32
substitute and no writability probe was performed on it.

## Cases run

No disposable synthetic fixture was created, so there is no run seed, file
count, or manifest hash for this blocked run.

| Case | Result | What can be established |
| --- | --- | --- |
| FAT32 rename, same-volume move, and in-place overwrite | **Not run** | FAT32 identity stability is unverified |
| FAT32 copy and replacement save | **Not run** | FAT32 new-ID behavior is unverified |
| FAT32 delete/recreate under the same name | **Not run** | FAT32 name reuse behavior is unverified |
| NTFS to FAT32 and FAT32 to NTFS | **Not run** | Cross-volume identity/classification is unverified |
| FAT32 hardlink attempt | **Not run** | No real FAT32 failure mode was captured |
| FAT32 timestamp granularity | **Not run** | No FAT32 timestamp observation was captured |
| DriveFS hydration identity | **Not run** | Placeholder/hydration identity remains a separate #47 prerequisite |

### Existing NTFS baseline, not rerun

The only operation evidence available remains the earlier local-NTFS table in
[`p0-e-file-identity.md`](../p0-e-file-identity.md). It records rename, same-
volume move, and in-place overwrite as stable IDs; copy, replacement save, and
delete/recreate as new IDs; and hardlink aliases as sharing an ID. This report
does not claim those observations were re-executed.

The per-path limit from that NTFS baseline remains important: a shared file ID
does not collapse locations. Each normalized path keeps its own presence
record; an alias can survive removal of its sibling. Conversely, a stable name
does not prove the same file after replacement or delete/recreate, and content
similarity does not prove identity. Those are existing NTFS-local constraints,
not FAT32 or cross-volume qualification.

## Exact setup needed to unblock #48

An owner/operator must provide a genuinely disposable FAT32 volume before the
run can start:

1. Supply an already-formatted FAT32 USB device, or explicitly provision and
   attach a dedicated disposable VHD. Do not use the system FAT32 partition,
   the DriveFS virtual mount, a network share, or a device containing valuable
   data. The owner must perform and confirm any partitioning or formatting;
   this run does not format drives.
2. Give the volume a drive letter and record its source/type, volume serial,
   filesystem, and allocation unit size. It must allow create, rename, move,
   copy, replacement, delete/recreate, timestamp, and hardlink-attempt
   operations under exactly one disposable leaf such as
   `<LETTER>:\fruitboard-48-fat32-<run-id>`.
3. Confirm the existing `C:` NTFS volume may host the matching disposable
   cross-volume leaf. The operator must authorize the synthetic create/delete
   operations and cleanup on both leaves; no personal project contents are
   inspected.
4. Run at the pinned baseline with the repository-local toolchain environment
   verified, and record elevation status for `fsutil fsinfo`/identity commands.
5. Keep any optional DriveFS hydration check separate from the FAT32 table and
   complete the #47 UI-mode and cloud-consent prerequisites first.

Until a target meeting those conditions exists, #48 remains unverified and the
NTFS-local identity policy must not be promoted to FAT32, cross-volume, or
DriveFS behavior by this report.

## Limitations

- No FAT32 or cross-volume operation ran on this host, so no identity,
  timestamp, hardlink, or per-path result is claimed for those cases.
- The DriveFS virtual filesystem reports FAT32 in WMI, but that is not a real
  FAT32 implementation and is excluded from this evidence.
- The prior NTFS observations are OS-tool evidence and are not re-run here;
  mapping such results to Rust identity APIs remains an inference unless the
  Rust enumerator is run on the target filesystem.
- exFAT, ReFS, network filesystems, ACL revocation, and real overflow timing
  remain separate unverified cases.
- No platform exclusion is selected by this report.

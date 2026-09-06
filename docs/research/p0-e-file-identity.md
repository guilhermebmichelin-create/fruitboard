# P0-E: Windows file identity across NTFS and Drive roots

- Status: **Partial — local NTFS characterized; cross-volume and Drive unverified**
- Date: 2026-09-06
- Environment: Windows 11 (build 26100), NTFS fixed volume, fsutil
  (`queryfileid`, i.e. 128-bit file IDs) plus volume serial. Probes used OS
  tools against a disposable synthetic tree under the system temp directory,
  removed afterwards. No personal roots traversed.
- Related: #43, ADR-004, ARCHITECTURE.md fingerprint tiers.

## Question

Where is (volume serial, file ID) trustworthy for continuing a file locator,
and where must the reconciler fall back to other evidence?

## Local NTFS observations

The volume serial was stable across all operations. File-ID behavior per
operation on one synthetic file:

| Operation                              | Identity |
| -------------------------------------- | -------- |
| Rename within a directory              | Stable   |
| Move within the volume                 | Stable   |
| Overwrite in place (truncate + write)  | Stable   |
| Copy to a new path                     | New ID   |
| Replacement save (new temp + move over)| New ID   |
| Delete + recreate under the same name  | New ID   |
| Hardlink (second path to same file)    | Same ID  |

## Limitations

- OS-tool probes, not Rust `std::fs`; mapping to Rust file-identity APIs is
  inference, labeled as such.
- Cross-volume moves unverified: no second writable volume exists here (the
  Drive-labeled volume is not writable by the test user).
- FAT32 and DriveFS ID semantics unverified; placeholder hydration effects
  untested. Size/mtime granularity not re-measured.

## Decisions for #36

1. (Volume serial, file ID) continuity may continue a locator across rename
   and same-volume move. It is evidence, not proof.
2. An ID change must not be read as delete-plus-create without
   generation-scoped reconciliation: replacement saves and delete-recreate
   cycles change the ID under a stable name.
3. A stable name must not be read as the same file: delete-recreate reuses the
   name with a new ID.
4. Hardlink aliases share one ID but remain distinct paths: preserve one
   presence record per path and deduplicate at the file-identity level, never
   by collapsing locations. When one alias disappears, the surviving path
   stays available under the same identity. This matches DATA_MODEL.md, where
   `file_location` is unique per normalized path while the
   `(device_id, volume_id, filesystem_file_id)` uniqueness links aliases to
   one underlying file. Required regression before #36/#38/#40: two aliases
   exist, one disappears, the other remains available.
5. Copies are new files, even with identical content; content hashing stays a
   move/duplicate candidate signal, never identity.
6. Absence requires a successfully completed authoritative scan of an available
   root. Offline, denied, cancelled, or partial enumeration never marks unseen
   files missing.

## Reproduction

Run `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
scripts/research-fs-probe.ps1 -Mode Identity` on Windows. The probe builds a
disposable synthetic tree under the system temp directory, asserts each row of
the table above plus alias survival and serial stability, prints path-free
PASS lines, and removes the tree. Any FAIL line is a finding, not an
environment quirk: rerun with `-Mode Identity` isolated before concluding.

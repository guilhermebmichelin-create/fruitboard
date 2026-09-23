# Drive virtual root: Windows Stream files host run, 2026-09-23

## Question and decision

Can the experimental `driveVirtual` mode manually scan a real streamed Google
Drive for desktop mount without marking an unseen prior file missing? On this
one host, the installed app **passed after a narrow Windows port fix**. The
original PR head failed enumeration. At this host-run checkpoint, known offline
placeholders, mirrored comparison, disconnect/reconnect, overflow, and local
NTFS regression remained untested. The mode remains experimental and manual-only.

## Environment and scope

- Windows 11 `10.0.26200.0`; Drive for desktop `125.0.0.0`, running.
- Authenticated `G:` virtual mount, volume GUID
  `\\?\Volume{2fcba7b7-b0a5-11f1-84a2-4c796e699681}\`; volume report:
  fixed drive, `Google Drive` label, FAT32-like filesystem, flags `0x106`.
  This virtual report does not establish real FAT32 behavior.
- Active mode: **Stream files**, reported by the owner from Drive Preferences.
  The mode was not independently captured from the settings UI.
- Original PR source: `fc620e3eb92283eeab8bb8a765061d801921aa96`.
  Patched source is the later commit containing this note. Toolchains used:
  Node 24.20.0, pnpm 11.25.0, Rust 1.98.1, Python 3.11.16.
- The owner approved one uniquely named, under-1-MiB synthetic folder and
  cleanup despite `G:` having 14.29 GiB free, below the normal 30 GiB
  fixture reserve. The owner chose current mode only; no mode switch or sync
  pause was authorized. The run touched only its synthetic leaf.
- Run ID and seed: `20260923-0717-167`; ten marker-only `enum-01.flp` through
  `enum-10.flp` files, 1,130 bytes total. Manifest SHA-256:
  `5ffc18fce39c3ea3b1f7477e8899d76f08d79f915f62bfa662a36a60613db6e3`.
  A nested watcher leaf held 50 256-byte files plus ten one-byte appends.
  The complete fixture stayed under 1 MiB and was removed after the run.

## Inputs and observed results

| Probe | Observed result |
| --- | --- |
| OS enumeration and metadata | All ten synthetic files appeared with size and modification time. All ten had `Offline=false`; therefore this run did not test an offline placeholder. Opening and closing a read handle on `enum-01.flp` without reading bytes left its flag, size, and modification time unchanged. |
| Original installed app | The explicit `driveVirtual` root registered, but manual scan failed with `unsupported`, zero observed files, and no committed library rows. The Preferences desktop and narrow states and narrow failure state were captured. |
| Windows handle probe | Opening the leaf and reading basic and standard handle information succeeded. `GetFileInformationByHandleEx(FileCaseSensitiveInfo)` failed with `ERROR_INVALID_PARAMETER` (87). The volume did not advertise `FILE_CASE_SENSITIVE_SEARCH`. This supports the explanation for the original scan failure. |
| Patched installed app, first manual scan | Completed and committed ten manifest-matching Present rows. The patch treats unsupported per-directory case-sensitivity information as case-insensitive only for a `DriveVirtual` candidate whose volume does not advertise case-sensitive search. `localNtfs` handling remains unchanged. |
| Patched installed app, second manual scan | After `enum-10.flp` was renamed to a non-FLP name, a second explicit scan completed. All ten prior library rows remained Present. The archived test database shows nine rows seen in run 2 and `enum-10.flp` last seen in run 1; no Missing row was written. The original filename and manifest were then restored. |
| Watcher burst | A 4,096-byte-buffer OS `FileSystemWatcher` observed 50 Created, 1 Renamed, 1 Deleted, 0 Changed after ten spaced appends, and 0 errors. The app did not start an automatic scan after the burst or rename; both recorded jobs were manual. This OS probe does not establish Rust `notify` fidelity or overflow behavior. |
| Visual check | Installed app screenshots show the experimental root in Preferences at desktop and 400-pixel narrow window sizes, plus completed/committed library states and the retained `enum-10.flp` row in the narrow library view. |

The original unsigned installer SHA-256 was
`f15857ed6c6ea68b941a0841f6d881d0e430b634ef85050be1ed8179eee3e64a`;
the patched unsigned installer SHA-256 was
`26ee05bd59a5278cba924ef91961d10e3acd1e4ae0326fb1128614ab427a7246`.
Independent byte copies of both installers, both built executables, screenshots,
the synthetic manifest, probe outputs, and archived synthetic app databases are
retained outside Git and outside Cargo caches under the local run evidence
directory `pr167-drivefs-stream-20260923`. No private Drive files, client logs,
account data, or actual FL Studio projects were used as fixtures.

## Support and remaining validation

| Mode or case | Current decision | Reason |
| --- | --- | --- |
| Stream files, freshly created hydrated files | Experimental manual scan only | One patched installed-app run observed all ten files and safely retained an unseen prior row. This does not prove completeness on other mounts or versions. |
| Stream files, known offline placeholders | Unverified | Every fixture file reported `Offline=false`; metadata-only placeholder hydration could not be tested. |
| Mirror files on NTFS | Unverified comparison | The owner limited the run to the current Stream mode. No mode switch or resync occurred. |
| Disconnect/reconnect and watcher overflow | Unverified | Sync pause and full matrix were outside the approved scope. No overflow attempt was made. |
| Automatic DriveFS watcher support | Excluded | The experimental application mode remains manual-only. OS watcher event delivery was incomplete for appends in this run. |

The disposable Drive folder and installed test packages were removed. The
preexisting Foundation Smoke database was restored with its original hash;
normal application data was not changed. This finding does not close #47 or
promote the DriveFS matrix in the Phase 2 plan.

## Owner disposition, 2026-09-23

After the [installed local NTFS regression](drive-virtual-local-ntfs-regression-20260923.md)
passed and all ten checks passed on the integrated PR head, the owner approved
marking PR #167 ready for review as a limited experimental, manual-only spike.
Issue #47 remains open. This review scope does not establish general DriveFS
support or authorize a mode switch, sync pause, or automatic DriveFS watching.
Offline placeholders, mirrored comparison, disconnect/reconnect, and overflow
remain unverified on the host.

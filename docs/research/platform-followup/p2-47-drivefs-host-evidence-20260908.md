# #47 DriveFS host follow-up evidence - 2026-09-08

- Status: **blocked - no DriveFS mode was qualified.**
- Run timestamp: `2026-09-08T21:49:01-03:00`.
- Baseline: `0b7612db3570e6235d4d2c86a678dd9004264f30` (`main` at the start of this evidence run).
- Related issue: [#47](https://github.com/guilhermebmichelin-create/fruitboard/issues/47).
- This file is new evidence under `docs/research/platform-followup/`; it does not edit or replace the #47 runbook.

## Scope and question

Issue #47 asks whether watcher events, placeholder metadata, metadata-only reads,
and disconnect/reconnect behavior can be qualified for DriveFS roots in both
Drive for Desktop caching modes: **Mirror files** and **Stream files**.

The issue body has no follow-up comments at this run. The governing runbook
requires the active mode to be read from the Drive for Desktop Preferences UI,
and requires owner consent before creating disposable synced files, changing
mode, or pausing/disconnecting synchronization.

## Read-only host evidence

The survey did not list any Drive contents and did not write to the DriveFS
mount.

| Observation | Result |
| --- | --- |
| Operating system | Windows 11 Home Single Language, build `10.0.26200`, 64-bit |
| DriveFS process | `GoogleDriveFS.exe` present; observed product version `125.0.0.0` |
| Mount | `G:`; volume label `Google Drive`; `mountvol G:\ /L` reported `\\?\Volume{f1648289-a335-11f1-849b-4c796e699681}\` |
| Volume report | WMI reports filesystem `FAT32`, serial `19831116`; this is the DriveFS virtual filesystem report, not real FAT32 evidence for #48 |
| Active caching mode | **Not established.** No mode was inferred from process, mount, or client state; the runbook requires the Preferences UI |
| Privilege/tooling | Non-elevated shell; `fsutil.exe` is present. No `fsutil` file-ID probe was run |
| Pinned tools | Repo-local toolchain verification passed: Node `24.20.0`, pnpm `11.25.0`, Corepack `0.36.0`, Rust `1.98.1`, uv `0.12.9`, Python `3.11.16` |

The ordinary shell is not configured for the pinned tools (`node` reported
`26.4.0`, while `rustc` and `uv` were not on `PATH`). The verification above
was run through the repository's local `.tools` binaries with the corresponding
Rust and uv environment variables set. This is a tooling setup detail, not a
DriveFS observation.

## Cases run

No DriveFS fixture was created. The missing mode and consent prerequisites make
the safe test location unavailable for this run.

| Case | Mirror files | Stream files | What is established |
| --- | --- | --- | --- |
| Synthetic enumeration and placeholder metadata | Not run | Not run | Nothing about placeholder presence, size/mtime availability, or hydration state |
| Metadata-only `Get-Item` plus open/close | Not run | Not run | No hydration side effect can be established |
| Create/rename/delete/append watcher load | Not run | Not run | No DriveFS watcher behavior, counts, or coalescing can be established |
| Fast overflow attempt | Not run | Not run | No DriveFS overflow result |
| Pause/disconnect, gap writes, resume | Not run | Not run | No gap hint coverage or reconciliation outcome |
| Rust watcher/enumerator harness on DriveFS | Not run | Not run | No OS-to-Rust inference was produced |

In particular, no `Offline` attributes were inspected, no DriveFS file was
opened, and no placeholder was intentionally hydrated. The local NTFS watcher
smoke from the earlier research is not repeated or presented as DriveFS
evidence here.

## Current qualification matrix

| DriveFS mode | Qualification result | Reason |
| --- | --- | --- |
| Mirror files | **Unverified** | The mode was not captured from the Preferences UI and no consented DriveFS fixture run occurred |
| Stream files | **Unverified** | The mode was not captured from the Preferences UI and no consented DriveFS fixture run occurred |

This is a blocker report, not a support decision or a platform exclusion.

## Exact setup needed to unblock #47

An owner/operator must provide all of the following before the run can start:

1. Open Drive for Desktop Preferences and record whether the current run is
   **Mirror files** or **Stream files**. Run the complete procedure once per
   mode; switching modes requires the owner to authorize the resulting full
   resync and to record when it is complete.
2. Confirm an explicitly disposable, synced test location on the DriveFS mount.
   The runbook leaves are exactly `G:\fruitboard-drivefs-<run-id>-enum`,
   `G:\fruitboard-drivefs-<run-id>-burst`, and
   `G:\fruitboard-drivefs-<run-id>-gap`; they contain only the documented
   synthetic marker and are removed after each mode.
3. Give explicit consent for those cloud-synchronized create/rename/delete
   operations and cleanup. No personal project contents may be traversed,
   opened, hashed, or parsed.
4. Give explicit consent for the pause/disconnect and verified resume required
   by the gap case. Without that consent, the disconnect row remains not run.
5. At run time, record the DriveFS version, mount letter/GUID, UI-reported
   mode, baseline commit, run ID, seed, manifest hash, and whether the Rust
   harness was run on the DriveFS root.

Until those prerequisites are supplied, DriveFS watcher, placeholder,
hydration, and disconnect behavior remain unverified for both modes.

## Limitations

- This is a single Windows host and DriveFS version survey; it does not
  characterize any mode or transfer behavior to another host/version.
- Process and volume metadata do not establish the active caching mode.
- No DriveFS operation was run, so there is no watcher result and no evidence
  about placeholders or hydration side effects.
- No Rust harness ran on the DriveFS mount; any future OS-tool observation must
  be labeled inference unless the Rust harness also runs there.
- No platform exclusion is selected by this report.

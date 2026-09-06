# P0-D: DriveFS watcher and placeholder behavior

- Status: **Partial — local NTFS watcher characterized; DriveFS unverified**
- Date: 2026-09-06
- Environment: Windows 11 (build 26100), NTFS fixed volume, PowerShell 5.1
  with .NET FileSystemWatcher (ReadDirectoryChangesW equivalent), fsutil.
  Probes used OS tools against disposable synthetic trees under the system
  temp directory; all trees removed after each run. No personal roots traversed.
- Related: #42, ARCHITECTURE.md scanner pipeline, ADR-004.

## Question

Can the scanner rely on watcher events and placeholder metadata on
DriveFS-backed roots, and what does the local watcher actually deliver?

## DriveFS availability

No DriveFS driver, service process, or mount exists in this environment. A
locally attached volume labeled "Google Drive" (FAT32, fixed, empty root) is
not writable by the test user and exposes no placeholder, hydration, or sync
state. It cannot stand in for DriveFS semantics.

All DriveFS cases are therefore **unverified**: mirrored vs streamed
enumeration, placeholder vs hydrated metadata, hydration side effects of reads,
burst/rename/disconnect behavior on Drive roots, and watcher fidelity there.
Claiming Drive support requires a follow-up run on an authenticated Drive for
Desktop host. A local-only scanner scope needs an explicit owner decision.

## Local NTFS watcher observations

Single-threaded scripted load against a synthetic tree, default buffer:

- 50 file creations produced 50 Created plus 50 Changed events (one per
  write-close); no loss, no merging.
- A rename arrived as one atomic Renamed event carrying old and new names.
- A delete arrived as one Deleted event.
- After draining creation-phase events, 10 spaced appends to one file produced
  further Changed events on this host: appends are observed as their own phase
  rather than hidden inside creation traffic. Exact per-write counts are not
  guaranteed across machines, so debouncing per path remains the scanner's job.

Overflow attempt: 500 file creations with a 4 KiB buffer still delivered all
500 Created events with no Error event — one generating thread cannot outpace
delivery here. Overflow was **not reproduced**; the Error-event path is still
required because the underlying API documents buffer overflow explicitly, but
it needs a faster generator or real burst load to observe.

## Limitations

- OS-tool probes, not the Rust `notify` crate; mapping to `notify` behavior is
  inference, labeled as such.
- Single-volume NTFS only; network, FAT32, and virtual filesystems untested.
- Disconnect/reconnect and sleep/resume not exercised.

## Decisions for #37

1. Watcher events are hints that schedule reconciliation, never truth; only a
   successfully completed authoritative scan supplies absence evidence.
2. Coalesce and debounce per path in the scanner; expect one event per flush.
3. Handle the overflow/error event by forcing full reconciliation of the
   affected root; offline, denied, or partial enumeration must not mark unseen
   files missing.
4. DriveFS roots stay unqualified until the blocked environment run completes.

## Reproduction

Run `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
scripts/research-fs-probe.ps1 -Mode Watcher` on Windows. The probe builds a
disposable synthetic tree under the system temp directory, replays the burst,
rename, delete, and append operations, prints the observed event counts, and
removes the tree. Observed counts are measurements from this host, not API
guarantees: a different machine or load may deliver different coalescing, so
issue #37 must debounce per path regardless.

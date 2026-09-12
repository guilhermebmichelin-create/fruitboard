# Installed-journey evidence

This directory contains the supporting record for the installed Windows run
entered in the [installed-app checklist](../installed-app-journey-checklist.md).
The run record is [run-20260908.md](run-20260908.md).

The additive #107 queued-state and restart record is
[run-20260912-queued-restart.md](run-20260912-queued-restart.md). Its
historical review driver is [queued-state-restart.mjs](queued-state-restart.mjs)
and its recorded driver hash is intentionally kept separate from the revised
locked NTFS driver [installed-ntfs-cases.mjs](installed-ntfs-cases.mjs), which
is launched only through `scripts/run-installed-ntfs-cases.ps1`.
The historical run records and the preserved S5 disposition remain unchanged.

The evidence classes are intentionally separate:

- `run-20260908.md` records observations from the unsigned, installed
  Foundation Smoke package using the native Tauri adapter and watcher.
- `cdp-session.mjs` is a small, review-only WebView2 CDP driver used to capture
  installed DOM state and send UI actions. The Windows folder picker was
  operated through the native Windows UI Automation surface; the driver does
  not replace the native adapter.
- Automated feature-on Rust tests, default repository gates, and client
  fake-adapter tests are listed as automated evidence only. They do not fill
  the installed-app rows.

- The 2026-09-12 remaining-case run records only local NTFS observations. It
  does not qualify FAT32, DriveFS, network shares, or Phase 2 acceptance.

No database, fixture contents, screenshots, personal projects, or machine
identity are committed. The run-specific synthetic files and isolated
Foundation Smoke database remain under `%TEMP%` and `%LOCALAPPDATA%` for the
owner's review.

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

## Supported locked-run entry point

For a new locked installed local-NTFS evidence run, invoke the PowerShell
wrapper from the repository root. It owns the exclusive Foundation Smoke
lock, the run-specific install directory, shared-data archival, driver
environment, database inspection, uninstall, and owner-only lock release:

```powershell
$journeyRoot = Join-Path $env:TEMP "fruitboard-ntfs-cases-<run-id>"
.\scripts\run-installed-ntfs-cases.ps1 -JourneyRoot $journeyRoot
```

`-InstallerPath`, `-NodePath`, `-PythonPath`, `-UvPath`, and
`-RustBinDirectory` may supply already-pinned inputs when the recorded build
was produced separately. Direct invocation of `installed-ntfs-cases.mjs` is
not a supported entry point because it does not own the wrapper's setup and
cleanup contract. `queued-state-restart.mjs` remains historical evidence for
the `#107` run and must not be silently replaced or rerun under its recorded
hash.

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

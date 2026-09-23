# Local NTFS regression after Drive virtual integration, 2026-09-23

## Question and decision

Does the installed app still scan and watch a disposable local NTFS root after
adding the experimental `driveVirtual` mode, including disable/re-enable and
an app restart? **Yes on this Windows host.** Ten installed NTFS safety cases
passed. A focused addition to the root lifecycle case observed new completed
scan jobs and the expected files without a manual Scan now after re-enable and
again after restart. This qualifies the local NTFS regression case for PR #167;
it does not qualify DriveFS mirrored folders or offline placeholders.

## Environment, inputs, and source boundary

- Windows `10.0.26200.0`; disposable fixture on the `C:` NTFS volume.
- Installed unsigned Foundation Smoke package with `packaging-smoke` and
  `scan-console` features, built from integrated source commit
  `470f82a155bd5d3d029fe271e561ab4066b1ef40` after merging the then
  current `main`. Installer SHA-256:
  `8a15f8207c1efcbe07200b61147c107751392345efd9267edc094474fba4664d`.
  The later test-driver and documentation commits do not change packaged app
  code.
- Pinned Node 24.20.0, pnpm 11.25.0, Rust 1.98.1, uv 0.12.9, Python
  3.11.16. The focused installed driver SHA-256 was
  `8f7a53bc31a71ad225c9866fd53ae7c9a7e040aa032e2630091c95e8e4d2087e`.
- The locked `run-installed-ntfs-cases.ps1` wrapper used the recorded installer
  and a new disposable `%TEMP%` run directory. Its test identity and native
  database were isolated from normal Fruitboard data. The prior Foundation
  Smoke database was archived before the run and restored afterward with its
  original SHA-256
  `9ffc6f3685ab9c5da22a90703bf6ec490c9f7f4e2802a9f75cd7ed0fdecfd8e2`.
- The lifecycle fixture began with one marker-only synthetic FLP named file.
  Three more marker-only files exercised disabled, re-enabled, and restarted
  watcher states. Final sorted filename/content manifest SHA-256:
  `e4e332e92b737b747befadca49777232a9642a43aca3a99273f0972b293aceab`.
  No actual FL Studio projects or personal files were scanned.

## Observed result

| Operation | Installed observation |
| --- | --- |
| Baseline and safety cases | All 10 case results passed: baseline commits, cancellation and retry ordering, restart recovery, denied traversal, resource limit, hardlink aliases, retry exhaustion, and root lifecycle. |
| Disabled root | A Scan now request was rejected. A new fixture file created while disabled did not start a new job or appear in the committed page. |
| Re-enabled root | After a new file was created, completed job `01a0cf48-2486-7372-b27a-65b4b3be0778` replaced the prior job and published three Present rows without a Scan now request. |
| App restart | After the installed app was stopped and restarted, another new file produced completed job `01a0cf48-32d2-733e-b89e-b887089a1805` and four Present rows without a Scan now request. |
| Cleanup | The installed package was uninstalled, no test app process or Foundation Smoke host lock remained, and the prior test database was restored byte-for-byte. |

The test driver and its full transcript, run provenance, final synthetic manifest,
installer and executable copies, and read-only database snapshots are retained
outside Git and outside compiler caches under local evidence directory
`pr167-drivefs-stream-20260923/ntfs-regression-20260923`. The final test
database SHA-256 is
`c6cb20bcb36feb088cd417ea17dfef07224672c5c4e9562980249ca3ef4a0f9d`.

## Limitations

The installed result demonstrates automatic local NTFS reconciliation after
the configuration and process transitions. The transcript does not identify
each underlying OS watcher event, so it does not measure event fidelity or
latency. This run did not change Google Drive mode, pause sync, or test a
mirrored Drive folder. It is functional regression evidence, not performance
qualification.

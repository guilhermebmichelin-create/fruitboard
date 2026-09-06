# Issue #34 installed picker verification and handoff

Reviewed PR #53 at `4a5d04afa68175ae3f66e5c90a765c842009b319` on 2026-09-06.
The remaining installed-app selection/cancellation check passed on the local
interactive Windows desktop. See [the path-free summary](installed-picker.json).
Application source was not changed for this run.

## Review outcome

- The native picker is an asynchronous command whose blocking dialog runs on
  a dedicated blocking task; malformed requests are rejected before opening UI.
- The Windows drive-root overlap reproduction is covered in both insertion
  orders, with trailing-separator and UNC cases.
- Successful mutations followed by list-refresh failure now report a stale
  list honestly; client tests cover add and remove cases.
- All seven PR #53 checks passed. Its
  [push-to-main Foundation CI](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34026765963)
  passed at the reviewed commit. This local run also passed 56 Node policy tests,
  45 client tests, toolchain verification, and the locked production NSIS build.

No new Windows blocker was found in this focused review. The next feature is
Issue #35: onboarding and root settings.

## Installed application observations

The run used pinned Node 24.20.0, pnpm 11.25.0, and Rust 1.98.1. Dependencies
were installed with `pnpm.cmd install --frozen-lockfile --ignore-scripts`, then
`pnpm.cmd package:windows:smoke` built the actual application and NSIS installer.
The 2,642,244-byte installer was verified as `NotSigned` and installed under a
path containing spaces and Unicode. It used the existing separate smoke identity.

Windows UI Automation invoked the application's Preferences link and Add folder
button. The Windows folder dialog appeared in the application process. Its
native Cancel and Select Folder controls were exercised through their window
handles; no fake platform adapter, direct root-mutating IPC, or database writes
were used to simulate selection. Read-only SQLite queries verified outcomes.
The child environment enabled `--force-renderer-accessibility` for inspection;
it did not enable remote debugging or change the packaged configuration.

| Interaction | Observed result |
| --- | --- |
| Cancel picker with no roots | Dialog closes, Add folder re-enables, no error, zero rows; database SHA-256 unchanged |
| Select disposable folder with spaces/Unicode | Folder added message; exactly one root with expected canonical path/name in UI and SQLite |
| Close app and restart installed executable | Same root appears in Preferences; database hash unchanged |
| Open picker and cancel with one root | Existing root preserved; database hash unchanged |
| Remove, then Keep | Root remains; database hash unchanged |
| Remove, then Confirm remove | Folder removed message and empty state; zero root rows |
| Check source marker through all steps | File remains byte-identical |
| Close and uninstall | Application directory removed; schema-v2 database retained byte-identically; no app/sidecar processes left |

Both selected-folder verification and cancellation therefore crossed the real
renderer-to-native boundary in an installed Windows application. This is one
local Windows observation, not a claim about every machine or filesystem.

## Data preservation and reproduction

Before installation, the existing dedicated smoke state was moved from
`%LOCALAPPDATA%\com.fruitboard.desktop.foundation-smoke` to
`.tools/evidence/issue-34-picker-20260906/retained-before-picker`. Its database
hash matched before and after the move. Nothing was deleted from that retained
state, and ordinary Fruitboard application data was untouched.

The current smoke database remains under the dedicated identity with zero scan
roots, schema version 2, and size 28,672 bytes. The disposable selected directory
and marker remain under `.tools/evidence/issue-34-picker-20260906/`, along with
local automation helpers. These private artifacts are ignored, not committed.

To repeat: use the pinned prerequisites and build command above, preserve any
existing dedicated smoke state by a validated move to a new destination, install
the unsigned package, and follow the interaction table using a fresh disposable
folder. Check the selected path, row counts, source marker hashes, and restart
behavior. Close the app and uninstall afterward, preserving its data. Never use
personal projects as disposable fixtures or delete previous evidence to bypass
the preconditions.

## Next agent actions

1. Submit these evidence documents in a focused documentation PR. They close
   the interactive evidence gap from #53; no application fix is required by
   this run. Do not include the pre-existing untracked HANDOFF.md accidentally.
2. Implement #35: accessible onboarding and root settings. Define the user flow,
   keyboard/focus behavior, and empty/loading/unavailable/error states first.
   Add typed native repository operations for display-name and enabled-state
   changes, preserving root IDs and selected paths. Validate requests and persist
   settings across restart; do not make controls that only update client state.
3. Keep availability distinct from scan status. A stored `available` value is
   not proof of current reachability or a completed scan. Explain that added
   folders have not been scanned; do not show live progress or ready results
   until scanner execution exists. Refresh the stale Home/Preferences copy that
   still describes only health/startup-preference capabilities.
4. Test settings persistence, malformed requests, failure/retry with draft
   preservation, source-file safety, accessible control names, focus after
   removal, narrow layout, and migration/backup behavior where schema changes.
   Run pinned checks and applicable CI on the final PR commit. Capture actual
   keyboard/visual evidence for the new UI; label fake-port evidence separately.
5. Continue #47/#48 when the needed environments are available. Before shipping
   integrated #36/#38/#40 behavior, agree scan generations, cancellation, queue
   recovery, atomic application, and per-path presence. Only a complete,
   authoritative scan can establish absence. The accepted Rust-parser evaluation
   remains after the filesystem-only Scanner MVP.

# Issue #18 Windows packaging review

This is bounded P0-G evidence for the Phase 1 foundation. It is not a public
release, a signing claim, a production parser, or an audio player.

## Reproduction

On Windows with the pinned Node and Rust toolchains plus the documented Tauri
prerequisites:

```powershell
pnpm.cmd install --frozen-lockfile --ignore-scripts
pnpm.cmd smoke:windows:foundation
```

The command builds the inert Rust probe from `Cargo.lock`, copies it to Tauri's
generated-binary directory, builds an explicitly unsigned current-user NSIS
package with `--ci --no-sign`, and executes the complete smoke. Generated
installers, raw process evidence, and private paths remain under ignored local
directories. Only the path-free summary in
[`windows-smoke.json`](windows-smoke.json) is committed.

The dedicated `Windows Packaging Smoke` workflow repeats package, signature,
install, native launch/exit, sidecar, data-preservation, reinstall, and uninstall
checks on a fresh `windows-latest` worker when packaging inputs change, monthly,
or on manual dispatch. GitHub's service-hosted Windows session did not expose an
interactive WebView2 debugging target even while the application process stayed
alive, so the workflow explicitly marks the window/audio portion unprobed; the
interactive observations below come from the local Windows run. The workflow
uses read-only repository permission, receives no secrets, and uploads no
artifact. The unsigned installer exists only during the bounded run.

## Local observation

Observed on 2026-09-05 with Windows AMD64, Microsoft Defender antivirus and
real-time protection enabled, and WebView2 reporting Edge 152.0.0.0. This warm
local build reused compiler state from an earlier package attempt.

| Measurement                         |                   Observed |
| ----------------------------------- | -------------------------: |
| NSIS installer                      | 2,588,735 bytes (2.47 MiB) |
| Installed directory                 | 8,694,050 bytes (8.29 MiB) |
| Application executable              | 8,468,992 bytes (8.08 MiB) |
| Inert sidecar executable            |   156,160 bytes (0.15 MiB) |
| Warm package build                  |                  24,974 ms |
| Cold launch to inspectable document |                     747 ms |
| Warm launch to inspectable document |                     508 ms |
| SQLite database                     |      16,384 bytes (16 KiB) |

Both installer and application signatures were verified as `NotSigned`. The
installer was copied to a path containing spaces and Unicode, installed to a
different path containing spaces and Unicode, launched twice, and closed
through the main window. Defender produced no observed install or launch block.
WebView2 emitted a benign `Chrome_WidgetWin_0` unregister diagnostic with Windows
error 1412 during graceful close; the processes still closed normally.

## Sidecar lifecycle

The package enables `tauri-plugin-shell` only for the `packaging-smoke` Cargo
feature. Rust calls the configured Tauri sidecar directly; no shell or process
permission is granted to the renderer. The executable has no dependencies and
implements only three fixed modes.

| Scenario           | Result                                                      |
| ------------------ | ----------------------------------------------------------- |
| Start/respond      | `ping` produced the fixed `pong:path-accepted` response     |
| Controlled failure | Exit code 17 and fixed stderr were contained                |
| Timeout            | The wait process remained alive past the 250 ms deadline    |
| Termination        | The timed-out child was killed and reaped                   |
| Path handling      | Executable and argument paths with spaces/Unicode succeeded |

The child never echoes its supplied path. The harness records only booleans,
fixed codes, durations, and sizes.

## Data preservation

The package uses the separate
`com.fruitboard.desktop.foundation-smoke` identity so testing cannot collide
with ordinary Fruitboard state.

1. The first installed launch created the SQLite database and saved Library as
   the startup view.
2. Silent uninstall removed the application directory but left the database
   present and byte-identical.
3. Reinstall and native database reopen restored Library.
4. A second uninstall again left the database present and byte-identical.

The final synthetic database is deliberately retained for local review. It is
not committed or uploaded. A future production uninstaller must preserve the
same application-data boundary unless a separately confirmed data-removal flow
is designed.

## Audio capability probe

The harness exposes a loopback-only Chrome DevTools Protocol port to the test
process through the documented development-only WebView2 environment variable.
It evaluates `HTMLMediaElement.canPlayType` against bounded candidate formats;
no media file is opened, generated, committed, or played. WAV, MP3 and FLAC are
the initial security allowlist and fail the smoke if WebView2 advertises no
support; AAC and Ogg are informational candidates.

| Candidate  | WebView2 result |
| ---------- | --------------- |
| WAV PCM    | `probably`      |
| MP3        | `probably`      |
| FLAC       | `probably`      |
| AAC in MP4 | `probably`      |
| Ogg Vorbis | `probably`      |

This proves API-level format signaling only. Real decode, malformed-input,
seeking, device, and playback behavior remains owned by the later audio phase.
The remote-debugging flag exists only in the smoke child environment and is not
part of the packaged configuration.

## Distribution and update limits

- The NSIS installer uses current-user mode and Tauri's WebView2
  `downloadBootstrapper`; a machine without WebView2 needs network access.
- `allowDowngrades` is false. There is no updater plugin, update endpoint,
  manifest, public release, or rollback claim. Until the signed release design,
  an approved replacement build is installed manually with the same production
  identity after backup/recovery checks.
- A public Windows artifact must sign the application, sidecars, installer, and
  update metadata from a protected release environment; timestamping, certificate
  custody/rotation, reputation, revocation, and verification remain release
  gates. No certificate or signing secret belongs in this repository or PR CI.
- The inert Rust binary does not validate Python/PyFLP packaging, licensing,
  memory limits, Job Objects, or a production parser protocol. P0-B/P0-C remain
  closed.
- macOS packaging and WebKit behavior remain unverified.

## P0-G decision

No observation invalidates ADR-001. Tauri produced a small installable NSIS
foundation, preserved Rust-owned data through uninstall/reinstall, retained the
narrow renderer capability, supervised the inert external binary, and exposed
the required WebView media API. Tauri remains accepted; the limitations above
become explicit later-phase and release gates rather than an ADR amendment.

## Primary references

- [Tauri Windows installers](https://v2.tauri.app/distribute/windows-installer/)
- [Tauri external binaries](https://v2.tauri.app/develop/sidecar/)
- [Tauri Windows signing](https://v2.tauri.app/distribute/sign/windows/)
- [Microsoft WebView2 debugging](https://learn.microsoft.com/en-us/microsoft-edge/webview2/how-to/debug-visual-studio-code)

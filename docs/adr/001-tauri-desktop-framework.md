# ADR-001: Tauri 2 desktop framework

- Status: Accepted
- Date: 2026-09-04
- Approved: 2026-09-04 by the product owner

## Context

Fruitboard needs a Windows desktop application with long-lived filesystem
watching, SQLite, external parser supervision, native file/folder actions, and a
web-based UI that can later support macOS. It must minimize the authority of
untrusted FLP metadata and remain responsive during background work.

Electron is mature and supplies a uniform bundled Chromium/Node runtime. Tauri
2 uses the operating-system webview and a Rust host with explicit capabilities.
Both can run a Python parser process and both require IPC validation, packaging,
signing, and updates.

## Decision

Use Tauri 2 with React/TypeScript/Vite. The Rust host owns privileged operations
and exposes narrow use-case commands. The UI is bundled locally; no remote page
receives Tauri capability access.

Run a Phase 1 Windows packaging spike before declaring the framework
irreversible. Reconsider if WebView2/audio/accessibility compatibility, sidecar
lifecycle/signing, installer behavior, or team/toolchain cost is a demonstrated
blocker.

## Alternatives

- Electron: consistent Chromium and Node ecosystem, but a larger runtime and
  broader JS/native surface than needed for this Rust-suitable core.
- Native Windows UI: excellent platform integration but little PWA/client reuse
  and a weaker direct macOS path.
- Browser-only PWA: cannot continuously scan arbitrary desktop folders or open
  FL Studio reliably.

## Consequences

- The team must maintain Rust, Node, WebView2, Python-sidecar, and native build
  tooling.
- Platform webview differences require Windows and later macOS tests.
- Native services can remain responsive and least-privileged behind Tauri
  capabilities.
- Desktop installer size should avoid a bundled Chromium cost, though the
  Python runtime may still be material.
- Electron remains a fallback, not a parallel implementation.

## Phase 1 implementation note

Issue #11 pins Rust 1.98.1 with the MSVC Windows target, `clippy`, and `rustfmt`
in `rust-toolchain.toml`. Issue #12 implements the first Tauri 2 shell with one
local window and one inert `get_app_health` command. Issue #16 adds only exact
read/write startup-view permissions for the first persisted use case. The
capability does not grant filesystem, shell, process, SQL, opener, recovery, or
remote-origin access; the client bundle is local and protected by a restrictive
CSP.

Issue #18 completes P0-G with a separate unsigned NSIS smoke identity. The
2.47 MiB installer produced an 8.29 MiB current-user installation; cold/warm
inspectable startup measured 747/508 ms on the local evidence host. Install,
graceful close, uninstall, reinstall and database restoration passed. Rust-only
Tauri plugin access exercised inert sidecar response, controlled failure,
timeout and termination without granting renderer shell/process authority.
WebView2 advertised the bounded candidate media formats. The observed online
WebView bootstrap requirement, unsigned trust posture, benign close diagnostic,
and missing macOS/real-playback qualification are explicit gates, but none
invalidates the Tauri decision. No ADR amendment is required.

## References

- [Tauri overview](https://v2.tauri.app/start/)
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/)
- [Electron process model](https://www.electronjs.org/docs/latest/tutorial/process-model)
- [Electron security](https://www.electronjs.org/docs/latest/tutorial/security)

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
in `rust-toolchain.toml`. It introduces no Tauri/native package; Issue #12 must
validate the remaining Visual C++ Build Tools and WebView2 prerequisites when
the first desktop shell is compiled.

## References

- [Tauri overview](https://v2.tauri.app/start/)
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/)
- [Electron process model](https://www.electronjs.org/docs/latest/tutorial/process-model)
- [Electron security](https://www.electronjs.org/docs/latest/tutorial/security)

# Phase 1 review: Application foundation

Status: **Accepted by the owner on 2026-09-06; Phase 2 may proceed**

Phase 1 establishes a reproducible, accessible, least-privilege Windows
application foundation. It deliberately does not implement the scanner, FLP
parser, project library, Kanban workflow, audio player, synchronization, or PWA.

## Acceptance map

| Issue | Merged PR / evidence                                                                                                          | Accepted outcome                                                         |
| ----- | ----------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| #11   | [PR #20](https://github.com/guilhermebmichelin-create/fruitboard/pull/20)                                                     | Exact Node, pnpm, Rust, Python/uv and SQLite policies; workspace locks   |
| #12   | [PR #21](https://github.com/guilhermebmichelin-create/fruitboard/pull/21)                                                     | Local Tauri shell, React client, typed platform port, minimal capability |
| #13   | [PR #22](https://github.com/guilhermebmichelin-create/fruitboard/pull/22)                                                     | Shared tokens, routing, responsive navigation and accessible states      |
| #14   | [PR #23](https://github.com/guilhermebmichelin-create/fruitboard/pull/23)                                                     | Versioned command envelope, panic containment, redacted bounded logs     |
| #15   | [PR #24](https://github.com/guilhermebmichelin-create/fruitboard/pull/24)                                                     | Rust-owned SQLite, atomic migrations, backup/recovery and WAL gate       |
| #16   | [PR #25](https://github.com/guilhermebmichelin-create/fruitboard/pull/25)                                                     | Startup preference through React, typed IPC, Rust and SQLite restart     |
| #17   | [PR #26](https://github.com/guilhermebmichelin-create/fruitboard/pull/26)                                                     | Six stable CI gates, privacy/dependency checks and governance baseline   |
| #18   | [PR #29](https://github.com/guilhermebmichelin-create/fruitboard/pull/29) and [packaging evidence](review/issue-18/README.md) | Unsigned NSIS smoke, inert sidecar, media probe and data preservation    |

PR #29 is merged (squash commit `8251f43`, 2026-09-05) and issue #18 is
closed. Parent epic #10 was closed on explicit owner acceptance on 2026-09-06;
merging alone did not accept the phase.

## Verification evidence

- PR #26's [push-to-main Foundation CI](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/33982318322)
  passed all six jobs at merge commit `2cf9697`, including the complete pinned
  Windows gate.
- PR #29 passed all seven checks on its final SHA `7f26ae1` (six Foundation CI
  jobs plus `windows-packaging-smoke`), then squash-merged as `8251f43`. The
  [push-to-main Foundation CI](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/33990241467)
  passed all six jobs at `8251f43`, including the complete pinned Windows gate
  in 16m45s.
- Post-merge review follow-up
  [PR #30](https://github.com/guilhermebmichelin-create/fruitboard/pull/30)
  hardens the launch probe (app-origin target plus rendered-shell marker), keeps
  WebView and application readiness distinct, widens the packaging workflow to
  `crates/**` and `packages/**`, and corrects this checkpoint's merge state.
- Post-merge review follow-up
  [PR #31](https://github.com/guilhermebmichelin-create/fruitboard/pull/31)
  validates origins as parsed protocol/host/port (rejecting suffixes,
  credentials, and unexpected ports), re-validates the snapshot URL on every
  poll, bounds discovery network I/O with cancellation, and widens the
  packaging trigger to `scripts/**`.
- The refreshed [interactive schema-v2 evidence](review/issue-18/windows-smoke.json)
  was captured on 2026-09-05 using application and probe code at `b67f26f`,
  with two evidence-only harness fields retaining the observed URLs. Both
  launches rendered the shell at `http://tauri.localhost/#/` and closed normally.
  The run built a 2.47 MiB unsigned NSIS installer and an 8.29 MiB installation.
  Cold WebView/shell intervals were 1,346/1,699 ms; warm intervals were
  1,729/963 ms. These are separate probe intervals, not total startup latency;
  the prior schema-v1 747/508 ms figures are superseded and not comparable.
  See the [measurement and data-retention notes](review/issue-18/README.md).
  The dedicated packaging workflow repeats the non-interactive package,
  native lifecycle, and data-safety subset on fresh hosted Windows workers
  without secrets or artifact upload; the service-hosted session does not claim
  interactive WebView/audio evidence.
- The installed Tauri sidecar started, responded, returned a controlled failure,
  exceeded a bounded timeout, and terminated cleanly from paths and arguments
  containing spaces/Unicode.
- The launch probe accepts only a debuggable target served from the packaged
  app origin (`https://tauri.localhost`, `http://tauri.localhost`, or
  `tauri://localhost`); `about:blank`, missing URLs, devtools, and foreign
  origins fail closed. Origins are parsed and compared as protocol, hostname,
  and port — host suffixes, credentials, unexpected ports, and malformed URLs
  fail closed — and the evaluated snapshot URL is re-validated on every poll,
  so post-discovery navigation cannot bypass the check. Discovery requests are
  cancelled at the remaining deadline. It then requires a rendered, usable
  shell marker — a `· Fruitboard` document title, the primary navigation with
  links, and a page heading, with no loading or error state — before trusting
  any audio capability signal. WebView target appearance and shell rendering
  are measured as distinct bounded phases, and the policy is covered by unit
  regressions in `tests/webview-probe.test.mjs`.
- The `Windows Packaging Smoke` workflow triggers on every packaged input,
  including `crates/**` (the storage crate owns the preserved database) and
  `packages/**` (the shared UI package owns the rendered shell), and
  `scripts/**` (the probe helper and its runners), so storage, styling, or
  probe changes cannot bypass packaging and data-preservation verification.
- Install, uninstall, reinstall, native database reopen, and second uninstall
  preserved a byte-identical 16 KiB database and the saved Library startup view.
- WebView2 returned `probably` for the bounded WAV PCM, MP3, FLAC, AAC and Ogg
  Vorbis capability queries. This is not playback/decode qualification.

## Accessibility and product evidence

- [Issue #13 review](review/issue-13/README.md) contains desktop/narrow visual
  evidence and documents keyboard, focus, contrast, reduced-motion and
  forced-colors controls.
- [Issue #16 review](review/issue-16/README.md) contains desktop/narrow keyboard
  recordings for the persisted preference workflow.
- Component axe coverage includes five shell states plus preference and startup
  gate states. JSDOM color-contrast exclusion is compensated by executable token
  contrast policy tests.
- Issue #18 adds no user-facing control or visual state, so no new screenshot is
  claimed.

## Security and privacy checkpoint

- The local window retains only health and startup-preference permissions. The
  package smoke invokes its fixed external binary from Rust and gives no generic
  shell/process permission to JavaScript.
- CSP, prototype freezing, command request/response validation, safe route/render
  fallbacks, panic sanitization, diagnostic redaction and bounded retention are
  executable policies.
- SQLite paths, connections, SQL and recovery authority remain native. WAL stays
  disabled despite the verified 3.53.2 embedded runtime.
- Repository privacy rejects credentials, personal home paths, FLPs, presets,
  audio, databases, logs and key material without echoing private filenames.
- pnpm and RustSec dependency checks run in CI. The 17-entry RustSec baseline is
  explicit and any new warning fails.
- The smoke package is explicitly unsigned, separately identified and absent
  from public release channels. Neither CI workflow receives signing secrets.

## Documentation and decisions

Phase 1 implementation notes are present in `ARCHITECTURE.md`, `DATA_MODEL.md`,
`DEVELOPMENT.md`, `SECURITY.md`, and ADR-001/002/003/006. P0-G supports the
existing Tauri decision, so ADR-001 does not require amendment. The conditional
parser decision and GPL gate remain unchanged.

## Open risks and deliberate deferrals

- Windows signing, SmartScreen reputation, protected release credentials,
  signed update manifests, SBOMs, public release and rollback qualification;
- production Python/PyFLP packaging, parser compatibility and licensing gates,
  process memory/job restrictions, scanner/watcher behavior and real FLP files;
- real media decode/playback, malformed-media testing and audio-device behavior;
- a macOS build/package lane and WebKit qualification;
- enforced required checks while the repository remains private on GitHub Free;
- the reviewed Tauri Linux/Unicode RustSec transitive baseline;
- all Phase 2+ product behavior, OAuth/sync and PWA functionality.

No unresolved result currently invalidates Tauri or the Phase 1 architecture.
The close-time WebView diagnostic and online-bootstrapper dependency are
recorded in the #18 evidence rather than hidden.

## Post-acceptance sequence (executed on 2026-09-06 acceptance)

1. Phase 2 epic
   [#33](https://github.com/guilhermebmichelin-create/fruitboard/issues/33)
   and issues
   [#34](https://github.com/guilhermebmichelin-create/fruitboard/issues/34)–[#41](https://github.com/guilhermebmichelin-create/fruitboard/issues/41)
   created from `ROADMAP.md`, plus spike issues
   [#42](https://github.com/guilhermebmichelin-create/fruitboard/issues/42)
   (P0-D) and
   [#43](https://github.com/guilhermebmichelin-create/fruitboard/issues/43)
   (P0-E);
2. filesystem spikes P0-D and P0-E land written results before Scanner MVP
   design finalizes;
3. P0-G stays satisfied by #18 while P0-A/P0-B/P0-C stay closed;
4. PyFLP stays absent until compatibility and GPL distribution decisions pass.

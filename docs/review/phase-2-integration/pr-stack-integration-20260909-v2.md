# PR-stack integration validation report — 2026-09-09 (v2)

Status: **Draft integration only; no merge, no acceptance, no policy change.**

Prior worktree `C:/Users/guilh/AppData/Local/Temp/opencode/wt-integration`
remained at `main 0b7612d` with branch `docs/41-pr-stack-integration-20260909`
and no combined commit or CI evidence. That task is not assumed successful.
This report owns a new isolated worktree, integration branch, build directory,
and validation evidence.

## Ownership

- Base: `origin/main 0b7612db3570e6235d4d2c86a678dd9004264f30` (PR #89).
- New worktree: `C:/Users/guilh/AppData/Local/Temp/opencode/wt-integration-20260909-v2`.
- New branch: `docs/41-pr-stack-integration-20260909-v2`.
- Isolated build outputs: `C:/Users/guilh/AppData/Local/Temp/opencode/build-integration-20260909-v2/`
  with `target/` and `logs/` (`pnpm-check.log`, `desktop-feature-tests.log`,
  `desktop-feature-clippy.log`, `enumeration-diagnostics-*.log`,
  `packaging-build-*.log`, `packaging-smoke-hosted.log`).
- Pinned env via `C:/Users/guilh/fruitboard/.tools`
  (`node-v24.20.0-win-x64`, `cargo-home`, `rustup-home`, `uv-0.12.9`):
  `node v24.20.0`, `pnpm 11.25.0`, `uv 0.12.9`, `rustc 1.98.1`.

## Input SHAs integrated (verified 2026-09-09 before merge)

| PR | Head branch | Head SHA integrated | Base at verification |
| --- | --- | --- | --- |
| #90 platform evidence | `docs/47-48-platform-followup` | `17e571742634eceb16f09b166edd3d77b57fcf61` | `main` |
| #92 installed evidence | `docs/41-installed-app-journey-20260908` | `49a5e649c688ae767c9189801dd033f2288b61f5` | `main` |
| #93 performance followup | `docs/41-performance-followup-20260908` | `59faefc2806a725368a59e7b6fc9be7f863f4fec` | `main` |
| #94 enumeration optimization | `perf/93-enumeration-optimization` | `588867bca154e798f189f6c99de8a2668005d141` (stack includes `1d7c298`, `b35c01a`) | `docs/41-performance-followup-20260908` |
| #98 validate PR94 candidate | `perf/94-validation-20260909` | `15b7f170c579d42c4f23f9619949236af4971ba2` | `perf/93-enumeration-optimization` |
| #95 application fix | `fix/92-installed-scan-console` | `bf0aeac38ac46dbb90990611b940429e232f67ef` | `main` |
| #97 regression coverage | `test/95-durable-queue-watcher-20260909` | `aba18121f6fa6d80d5fcc67be6985277154969f5` | `fix/92-installed-scan-console` |
| #96 design proposal | `docs/41-bounded-scan-design-20260909` | `3be014074b385747e51d6d6b859180315a5b80bc` | `main` |
| #91 reconciliation (last) | `docs/41-reconciliation-20260908` | `55fefaf9d5cf32a392839c136cd1902d317c4e41` (stack includes `c0a0bf1`, `70fa20f`) | `main` |

Merge order preserved dependency order above. Stacked PRs were merged by head
(`#93` then `#94` head then `#98` head; `#95` then `#97` head), so git ancestry
supplied parent changes once and no cherry-pick duplication occurred.

## Resulting combined commit

Integration merges (all `--no-ff` on the v2 branch):

- `a85d1316a633cec1cda394bee78ca8a8303cdbb1` integrate #90
- `5874b41ac8b167a1ee9499969062e29d26656b5e` integrate #92
- `8fae3c5d23b23e3a797d31a937f5fbb0aa93bb83` integrate #93
- `cd7efadb53b8befe7f756d2cca35b006960af7b4` integrate #94
- `d479fbf80bea697d63ef12a9f61d982c9e67eae5` integrate #98
- `dd5eeaec3857318b450d4bb46206285e629dc568` integrate #95
- `0feff97dfff88c45c82404b7ca840d636be5c4cf` integrate #97
- `e9eb19bfda04f5dc0f3cc01d50229ec50f889708` integrate #96
- `dcfa32334c4ec4a613347c4e7822dbf143582a8f` integrate #91 (combined HEAD)

`git rev-parse HEAD` in the v2 worktree is
`dcfa32334c4ec4a613347c4e7822dbf143582a8f`.

## Conflict record

All nine merges completed with `ort` without conflict markers, including the
shared `docs/review/phase-2-integration/README.md` touched by #95/#97 and
rewritten by #91. The final file was verified to retain both sides:

- #95 additive paragraph
  (`installed-journey/run-20260909.md` preserved, not overwritten) remains at
  lines ~57-59;
- #91 full reconciliation rewrite (status, baseline table, companion-evidence
  table, acceptance map, P2-08 conflict, standing gates) is present.

No manual conflict edit was required; no content was dropped. If a future
re-integration hits a true conflict, it must be resolved explicitly and
re-validated.

## Local gates on combined HEAD (isolated outputs)

Run in the v2 worktree with pinned toolchains. `CARGO_TARGET_DIR` was isolated
to `build-integration-20260909-v2/target` for `pnpm check` and feature tests;
packaging build used the worktree-local `target/` because
`scripts/prepare-foundation-sidecar.mjs` expects the repo-relative sidecar
path (isolated target breaks that copy step; recorded as a script limitation,
not a product failure).

- `pnpm.cmd check`: **PASS** (`LASTEXITCODE 0`; second run confirms).
  Covers `verify:toolchains`, `verify:sqlite:embedded`, `privacy:check`,
  `format:check`, `lint` (docs/scripts/recursive/rust), `typecheck`, `test`
  (`node --test` 77 pass, client Vitest 13 files/134 tests pass, Rust
  workspace: desktop 37, enumeration 44+2 ignored, watcher 32+4 ignored,
  reconciliation 17, scan-execution 56+1 ignored, storage 76), and `build`
  (client Vite + Tauri release exe at isolated target).
  Log: `logs/pnpm-check.log`.
- Feature-on desktop: **PASS**.
  `cargo test -p fruitboard-desktop --features scan-console --locked`: 91
  passed, 0 failed. Log: `logs/desktop-feature-tests.log`.
  `cargo clippy -p fruitboard-desktop --features scan-console --all-targets --locked -- -D warnings`: pass (exit 0).
  Log: `logs/desktop-feature-clippy.log`.
- Diagnostics-enabled enumeration: **PASS**.
  `cargo fmt --all --check`: pass.
  `cargo clippy -p fruitboard-filesystem-enumeration --features diagnostics --all-targets --locked -- -D warnings`: pass.
  `cargo test -p fruitboard-filesystem-enumeration --features diagnostics --locked`: 44 passed, 2 ignored (ACL, DriveFS), 0 failed.
  Logs: `logs/enumeration-diagnostics-*.log`.
- Packaging: **PARTIAL**.
  `pnpm.cmd package:windows:smoke` (NSIS bundle build, worktree-local target):
  **PASS**, produced
  `target/release/bundle/nsis/Fruitboard Foundation Smoke_0.1.0_x64-setup.exe`.
  Log: `logs/packaging-build-default-target.log`.
  `pnpm.cmd smoke:windows:foundation:hosted` (install/launch/uninstall):
  **BLOCKED** locally — `The dedicated synthetic data directory already exists;
  preserve it for inspection` (`$env:LOCALAPPDATA/com.fruitboard.desktop.foundation-smoke`
  from 2026-09-08/09 runs, no `fruitboard-desktop` process running). The script
  refuses to overwrite shared host state, which cannot be isolated per worktree.
  No deletion was performed to avoid destroying another agent's evidence.
  Log: `logs/packaging-smoke-hosted.log`. Integration packaging must therefore
  rely on the draft-PR hosted CI run, not a local full-smoke claim.

Heavy runs used isolated cargo target/logs and did not defer the full gate
because other worktrees exist. The shared `LOCALAPPDATA` smoke directory
remains the only non-isolatable resource; coordinate its cleanup with other
agents before any local full-smoke retry.

## Exact-head CI via workflow dispatch (existing workflows only)

Both workflows support `workflow_dispatch`; dispatch was used with `--ref`
branch heads. No required-check policy was changed.

- #94 (`588867b`, branch `perf/93-enumeration-optimization`):
  - Foundation CI dispatch `34352810308`: **SUCCESS** (exact head).
  - Packaging dispatch `34352813397`: **SUCCESS** (exact head).
- #98 (`15b7f17`, branch `perf/94-validation-20260909`):
  - Foundation CI dispatch `34352820062`: **SUCCESS** (exact head).
  - Packaging dispatch `34352822867`: **FAILURE** — `The installed sidecar
    smoke timed out` at `scripts/windows-foundation-smoke.ps1:147`
    (same mode as #92 attempt-1). Build+package succeeded; the failure is in
    the installed sidecar launch wait, not compilation. This gap is reported
    as-is; no policy change hides it.

Individual PR PR-triggered CI remains: #90, #92, #93, #95, #97, #91, #96 each
have green Foundation+packaging on their exact heads except #94/#98 which lack
PR-triggered runs because their bases are not `main` (expected for stacked
PRs). Dispatch above supplies the missing exact-head evidence, with #98
packaging still failing.

Integration CI (this v2 branch/PR) is separate from individual PR merge
requirements. This report does not promote any PR or amend any budget.

## Branch drift during validation (unresolved)

While local gates ran, other agents pushed:

- #97: `aba1812` → `d699ecdb49a77bbadac08e049ca056f50ce32aad`
  (`c4754a2` pin terminal retryAvailable flags + `d699ecd` stack review record).
- #96: `3be0140` → `9aa97abab57f90589bdce8b55f3820d6bfe9cd7a`
  (independent review corrections).
- #91: `55fefaf` → `986ca61a1a840f2a4418c2007fc6838c192c311c`
  (second 2026-09-09 refresh).

This integration validates the SHAs in the table above, not the newer heads.
Do not merge on this report without re-integrating drift and re-running gates.

## Dependency-aware owner merge order (do not merge)

Owner merges manually only, in this order; each PR needs its own green
required checks with branch up to date:

1. #90 (`17e5717`, base `main`) — independent platform evidence.
2. #92 (`49a5e64`, base `main`) — independent installed evidence (retain
   packaging attempt-1 timeout incident).
3. #93 (`59faefc`, base `main`) — performance followup base.
4. #94 (`588867b`, base #93 branch) — after #93; exact-head CI now green via
   dispatch, but still needs quiet-host A/B rerun per #91/#98 reports.
5. #98 (`15b7f17`, base #94 branch) — after #94; Foundation dispatch green,
   packaging dispatch failed (sidecar timeout); do not merge until packaging
   is green or owner explicitly accepts the gap with a follow-up.
6. #95 (`bf0aeac`, base `main`) — independent application fix.
7. #97 (integrated as `aba1812`, now `d699ecd`, base #95 branch) — after #95;
   re-validate new head.
8. #96 (integrated as `3be0140`, now `9aa97ab`, base `main`) — design proposal
   only; independent position, but keep after perf/fix stacks for review
   clarity.
9. #91 (integrated as `55fefaf`, now `986ca61`, base `main`) — **last**;
   reconciliation must refresh again against merged positions and drift.

No PR was merged by this task. No acceptance ID is promoted. F1/F3 budgets,
#47/#48 platform prerequisites, durable-queued/burst gaps, and owner
acceptance remain open per #91.

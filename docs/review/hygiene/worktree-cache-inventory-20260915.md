# Worktree and cache hygiene inventory — 2026-09-15

Read-only audit. **No deletion, no build, and no cache write was performed.**
This report is the tracked summary produced by the
`chore/148-security-inventory` task; the raw, full-path inventory stays outside
Git (see Evidence below).

- Governed by
  [DEVELOPMENT.md](../../DEVELOPMENT.md#local-build-and-disk-space-rules) and
  [AGENTS.md](../../AGENTS.md#build-and-cleanup-boundaries).
- Scope: `git worktree list` (70 worktrees), `%USERPROFILE%\fruitboard-*`,
  `%TEMP%\fruitboard-*` (including linked worktrees and generated fixtures), and
  `%TEMP%\opencode\*` (scratch directories and external Cargo target caches).
- Method: `git worktree list --porcelain`/`-v`, `git rev-parse`,
  `git status --porcelain`, `git for-each-ref --contains`,
  `git merge-base --is-ancestor`, `git worktree prune --dry-run`, `Get-PSDrive`,
  bounded `Get-ChildItem -Recurse -Force | Measure-Object`,
  `Get-Item -Force` reparse checks, `Get-Process` and `Win32_Process` lookups.
- Personal path segments are replaced with `%USERPROFILE%` and `%TEMP%` so this
  tracked file passes the repository privacy gate. The raw report keeps the
  resolved absolute paths outside Git.

## Evidence (outside Git)

| Artifact | Path | Notes |
| -------- | ---- | ----- |
| Raw inventory (full paths, sizes, reparse, processes) | `%TEMP%\opencode\hygiene-inventory-raw-20260915.txt` | ~1 040 lines; not committed |
| Commit reachability per worktree | `%TEMP%\opencode\hygiene-inventory-reachability-20260915.txt` | not committed |
| Merged-into-`main` status | `%TEMP%\opencode\hygiene-inventory-merged-20260915.txt` | not committed |
| Bounded scan scripts | `%TEMP%\opencode\hygiene-inventory-20260915*.ps1` | four read-only passes |

All raw evidence lives outside disposable compiler caches but inside the
temporary directory; copy anything still needed elsewhere before any future
`%TEMP%` cleanup.

## Capacity at audit time

| Volume | Free at first capture (21:10 UTC) | Free at 21:14 UTC | Reserve |
| ------ | --------------------------------- | ----------------- | ------- |
| C: | 52.99 GiB | 51.75 GiB | 30 GiB reserve held; 1.2 GiB was consumed by other live tasks during the audit |
| D: | 0 GiB (not a usable build volume) | 0 GiB | n/a |
| G: | 14.98 GiB | 14.98 GiB | not used by this task; below the 30 GiB reserve, do not place builds or fixtures here |

The audited generated footprint is roughly **48 GiB** across all locations
(worktree `target/` caches 31.82 GiB, `%USERPROFILE%` non-worktree caches
6.94 GiB, and `%TEMP%` fixture/cache content 9.33 GiB), measured as logical
`File.Length` sums. Cargo trees contain hardlinks, so on-disk unique bytes are
lower; measure with `Get-PSDrive` before and after any authorized cleanup.

## Path, reparse, and active-user checks

- All 70 worktree roots and all 16 worktree `target/` directories resolved to
  real directories. **0 reparse points, 0 reparse entries at the top level of
  any scanned cache.** No junction/symlink aliasing was found.
- `git worktree prune --dry-run` reported no stale worktree admin entries.
- No `cargo`, `rustc`, `pnpm`, or `git` build process was running at capture.
  Node harness processes were present (expected).
- A concurrent task was observed mid-audit: it measured
  `%TEMP%\fruitboard-validation-target` from a read-only PowerShell process, and
  it created the clean `%USERPROFILE%\fruitboard-fix-p208-retry` worktree at
  21:12:53 UTC. **Both are live work; do not touch them.**
- C: free space moved from 52.99 to 51.75 GiB during the 4-minute reachability
  pass because of that concurrent activity, not because of this audit.

## Summary

| Group | Entries | GiB (logical) | Recommendation |
| ----- | ------- | ------------- | -------------- |
| Primary checkout `%USERPROFILE%\fruitboard` + primary `target/` | 1 | 16.61 target | **KEEP** (designated primary development cache) |
| Linked-worktree `target/` caches | 16 | 31.82 total | mixed; 8.83 of it is on retirement candidates |
| Recorded reusable validation cache (`%USERPROFILE%\fruitboard-validation-cache`) | 1 | 6.83 | **KEEP** (single recorded reusable cache) |
| Duplicate/orphan validation caches (`%TEMP%\fruitboard-validation-target`, `%TEMP%\opencode\validation-target`) | 2 | 8.38 | one **HOLD** (live measurement), one **REMOVE-CANDIDATE** |
| Retained evidence (`fruitboard-validation-evidence`, perf93 profile DBs) | 3 | 0.09 | **KEEP** |
| Generated fixture/run directories (non-worktree, `%TEMP%`) | ~440 | 6.84 | **FIXTURE/EPHEMERAL** after evidence check |
| Other scratch caches (`pinned-tools`, `benchmark-*`, fixture, empty stray dir) | 6 | 0.18 | mixed; `pinned-tools` **HOLD**, the rest **REMOVE-CANDIDATE** |

Strong immediate reclaim (orphan validation target, retired worktrees with
preserved refs, regenerable fixtures) is roughly **10 GiB logical**. Another
~13 GiB is held by open-PR/recent worktrees or by the live validation-target
measurement; those stay until their owner confirms completion.

## External caches and non-worktree directories

| Path (redacted) | GiB | Reparse | Owning task / contents | Proposal |
| --------------- | --- | ------- | ---------------------- | -------- |
| `%USERPROFILE%\fruitboard-validation-cache` | 6.826 | no | the one recorded reusable validation cache | **KEEP** |
| `%USERPROFILE%\fruitboard-validation-evidence` | 0.046 | no | retained evidence (`warm-reconciliation-20260913`) | **KEEP** |
| `%USERPROFILE%\fruitboard-perf93-profile-before-db` | 0.020 | no | #93 profile evidence | **KEEP** |
| `%USERPROFILE%\fruitboard-perf93-profile-after-db` | 0.020 | no | #93 profile evidence | **KEEP** |
| `%TEMP%\fruitboard-validation-target` | 5.891 | no | later duplicate validation cache; measured by a live task at capture | **HOLD** — confirm the live task finished, then consolidate to one cache (owner decision) |
| `%TEMP%\opencode\validation-target` | 2.486 | no | orphan external `CARGO_TARGET_DIR`, no worktree or process attached | **REMOVE-CANDIDATE** after owner confirmation |
| `%TEMP%\fruitboard-pinned-tools-20260912` | 0.051 | no | pinned tool copies | **HOLD** — verify not the active toolchain |
| `%USERPROFILE%\fruitboard-perf93-fixture-custom-9995` | 0.026 | no | #93 synthetic fixture | **REMOVE-CANDIDATE** (regenerable) |
| `%TEMP%\fruitboard-benchmark-*` (4 dirs) | 0.082 | no | synthetic benchmark fixtures | **REMOVE-CANDIDATE** (regenerable) |
| `%TEMP%\fruitboard-cache-cleanup-20260913-145216` | 0.022 | no | leftover cleanup scratch | **REMOVE-CANDIDATE** after owner confirmation |
| `%USERPROFILE%\fruitboard-phase-2-library-ui` | 0.000 | no | empty stray directory | **REMOVE-CANDIDATE** (empty) |

The working agreement allows one primary development cache plus one reusable
temporary validation cache. Three writable Cargo caches currently exist
(`%USERPROFILE%\fruitboard-validation-cache`,
`%TEMP%\fruitboard-validation-target`, `%TEMP%\opencode\validation-target`);
the owner should name one as the reusable cache and retire the others after
their tasks finish.

## Linked worktrees

`dirty` is the `git status --porcelain` entry count. `preserved` means the HEAD
commit is contained in at least one named local, remote, or archive ref; `main`
means it is already an ancestor of `main`. `no-ref` commits must be archived
before removal. Rebuild cost scales with `target` GiB.

### Primary checkout

| Worktree | HEAD | dirty | target GiB | State | Proposal |
| -------- | ---- | ----- | ---------- | ----- | -------- |
| `%USERPROFILE%\fruitboard` | `9f74791` | 9 | 16.61 | `main` head; owner working files | **KEEP** (never clean) |

### `%TEMP%\fruitboard-*` linked worktrees (16)

| Worktree (suffix) | HEAD | dirty | target GiB | State | Proposal |
| ----------------- | ---- | ----- | ---------- | ----- | -------- |
| `fruitboard-final-regression-20260909` | `9c211ad` | 0 | 2.00 | preserved | **RETIRE-CANDIDATE** (rebuild ~full test) |
| `fruitboard-fix-pr92-20260908` | `bf0aeac` | 0 | 1.99 | preserved | **RETIRE-CANDIDATE** (rebuild ~full) |
| `fruitboard-installed-journey-docs-20260908` | `49a5e64` | 0 | – | preserved | **RETIRE-CANDIDATE** |
| `fruitboard-perf94-validate-after` | `b35c01a` | 0 | – | preserved | **RETIRE-CANDIDATE** |
| `fruitboard-perf94-validate-before` | `1d7c298` | 0 | – | preserved | **RETIRE-CANDIDATE** |
| `fruitboard-perf94-validation` | `a3e738b` | 0 | 0.01 | preserved | **RETIRE-CANDIDATE** |
| `fruitboard-perfqual-20260912` | `a44653b` | 0 | – | preserved | **HOLD** (qualification branch) |
| `fruitboard-perfqual-before-20260912` | `f63a1d3` | 0 | – | preserved | **RETIRE-CANDIDATE** |
| `fruitboard-perfqual-before94-20260912` | `1d7c298` | 0 | – | preserved | **RETIRE-CANDIDATE** |
| `fruitboard-perfqual-candidate-20260912` | `d342ec1` | 0 | – | preserved | **RETIRE-CANDIDATE** |
| `fruitboard-perfqual-candidate94-20260912` | `b35c01a` | 0 | – | preserved | **RETIRE-CANDIDATE** |
| `fruitboard-phase2-closeout-20260912` | `6591895` | 0 | – | branch + `origin` | **HOLD** |
| `fruitboard-phase2-integration-20260912` | `e90b03c` | 0 | – | branch + `origin` | **HOLD** |
| `fruitboard-phase2-ntfs-cases-20260912` | `ef08522` | 0 | 1.65 | preserved | **HOLD** (rebase source for #111) |
| `fruitboard-phase2-source-stack-20260912` | `9da0272` | 1 | – | preserved | **KEEP** (untracked handoff doc) |
| `fruitboard-pr91-reconcile-20260908` | `70fa20f` | 0 | – | preserved | **RETIRE-CANDIDATE** |

### `%TEMP%\opencode\*` linked worktrees (23)

| Worktree (suffix) | HEAD | dirty | target GiB | State | Proposal |
| ----------------- | ---- | ----- | ---------- | ----- | -------- |
| `agent1-104-main` | `00bb3e8` | 0 | – | preserved | **RETIRE-CANDIDATE** |
| `agent1-106-main` | `9b86cbf` | 0 | – | preserved | **RETIRE-CANDIDATE** |
| `agent1-91-main` | `caa1053` | 0 | – | preserved | **RETIRE-CANDIDATE** |
| `agent1-94-main` | `5cda879` | 0 | – | preserved | **RETIRE-CANDIDATE** |
| `agent1-95-main` | `2398b2a` | 0 | – | preserved | **RETIRE-CANDIDATE** |
| `agent1-97-main` | `9aa1100` | 0 | – | preserved | **RETIRE-CANDIDATE** |
| `agent1-98-main` | `0f15ca2` | 0 | – | **no-ref** | **KEEP** — archive the commit before any removal |
| `agent1-98-main2` | `38509e4` | 0 | – | preserved | **RETIRE-CANDIDATE** |
| `agent1-phase2-closeout` | `3ebac5f` | 1 | – | merged | **KEEP** (untracked acceptance packet) |
| `agent2-s5-restart-pin` | `e209b8a` | 0 | 1.74 | preserved | **HOLD** |
| `phase2-closeout-drafts` | `8d6a1d0` | 0 | – | branch + `origin` | **HOLD** |
| `queue-stall-fix` | `ded02ac` | 0 | 1.79 | preserved | **HOLD** |
| `wt-103-fix` | `90c988d` | 0 | – | **no-ref** | **KEEP** — archive the commit first |
| `wt-136-react193` | `d25d68a` | 0 | – | branch + `origin` | **HOLD** (React 19.3 decision) |
| `wt-91-final` | `cccd6da` | 0 | – | **no-ref** | **KEEP** — archive the commit first |
| `wt-91-publish` | `f847fd1` | 0 | – | **no-ref** | **KEEP** — archive the commit first |
| `wt-a-postmerge` | `0b7612d` | 2 | – | merged | **KEEP** (modified/untracked docs) |
| `wt-b-plan` | `0b7612d` | 1 | – | merged | **KEEP** (modified plan doc) |
| `wt-bounded-scan-20260909` | `9aa97ab` | 0 | – | preserved | **RETIRE-CANDIDATE** |
| `wt-integration` | `0b7612d` | 0 | – | merged into `main` | **RETIRE-CANDIDATE** (strongest case) |
| `wt-integration-20260909-v2` | `9c211ad` | 0 | 1.49 | preserved | **RETIRE-CANDIDATE** |
| `wt-pr91-20260909` | `8437906` | 0 | – | preserved | **RETIRE-CANDIDATE** |
| `wt-review-95-97` | `3e73329` | 0 | 1.76 | preserved | **RETIRE-CANDIDATE** |

### `%USERPROFILE%\fruitboard-*` linked worktrees (30)

| Worktree (suffix) | HEAD | dirty | target GiB | State | Proposal |
| ----------------- | ---- | ----- | ---------- | ----- | -------- |
| `fruitboard-acceptance-gap-refresh-20260913` | `01d1837` | 0 | – | branch + `origin` | **HOLD** (recent) |
| `fruitboard-acceptance-record-20260914` | `4b882e5` | 0 | – | branch + `origin` | **HOLD** (recent) |
| `fruitboard-agent-disk-rules-20260913` | `c868a91` | 0 | – | branch + `origin` | **KEEP** (source of the disk rules) |
| `fruitboard-benchmark-investigation-20260912` | `5164a6f` | 1 | 0.33 | preserved | **KEEP** (untracked output) |
| `fruitboard-build-storage-preflight` | `9c225a9` | 0 | 0.21 | branch + `origin` | **HOLD** |
| `fruitboard-chore-ci-privacy` | `da637fb` | 0 | – | branch only | **KEEP** (live parallel change) |
| `fruitboard-chore-sec-inventory` | `9f74791` | dirty | – | this task | **KEEP** |
| `fruitboard-feat-library-split` | `9f74791` | 0 | – | branch only | **HOLD** |
| `fruitboard-fix-p208-retry` | `9f74791` | 5 | – | created mid-audit | **KEEP** (live task) |
| `fruitboard-fix-scan-dedupe` | `9f74791` | 10 | – | preserved | **KEEP** (dirty) |
| `fruitboard-fix-storage-errors` | `25adb1f` | 0 | – | branch + `origin` | **HOLD** (recent) |
| `fruitboard-gap-refresh-20260914` | `d8bfc2f` | 0 | – | branch + `origin` | **HOLD** (recent) |
| `fruitboard-p2o8-evidence-20260915` | `7241932` | 0 | 0.003 | branch + `origin` | **KEEP** (recent evidence) |
| `fruitboard-p2o8-verify` | `1342971` | 0 | 0.21 | branch + `origin` | **HOLD** (recent) |
| `fruitboard-parser-spike-20260914` | `66fc446` | 1 | – | preserved | **KEEP** (dirty) |
| `fruitboard-perf93-enumeration` | `588867b` | 0 | 0.15 | preserved | **HOLD** |
| `fruitboard-perf93-enumeration-before` | `1d7c298` | 0 | 0.14 | preserved | **RETIRE-CANDIDATE** |
| `fruitboard-postmerge-correction-20260914` | `5b01a20` | 0 | – | branch + `origin` | **HOLD** (recent) |
| `fruitboard-pr111-hardening` | `f814034` | 0 | 1.83 | preserved | **HOLD** (open PR) |
| `fruitboard-pr111-rebase-20260913` | `606635c` | 0 | – | preserved | **HOLD** |
| `fruitboard-qualification-execution-20260913` | `484c5eb` | 0 | – | branch + `origin` | **KEEP** (recent) |
| `fruitboard-qualification-restore-cccaa67` | `cccaa67` | 0 | – | merged into `main` | **RETIRE-CANDIDATE** (strongest case) |
| `fruitboard-qualification-runbook-20260913` | `348f9ab` | 0 | – | preserved | **KEEP** (recent) |
| `fruitboard-queue-verify-20260909` | `aba1812` | 0 | 0.27 | preserved | **HOLD** |
| `fruitboard-spike-watcher-scope` | `9f74791` | 2 | – | branch only | **KEEP** (live, dirty) |
| `fruitboard-test-client-gaps` | `9f74791` | 16 | – | preserved | **KEEP** (dirty) |
| `fruitboard-test-p210-spy` | `9f74791` | 4 | – | preserved | **KEEP** (dirty) |
| `fruitboard-warm-reconciliation-profile-20260913` | `7697f46` | 0 | 0.21 | branch + `origin` | **KEEP** (profile evidence) |
| `fruitboard-wt-pr112-rebase-20260913` | `386b4c9` | 0 | 0.07 | preserved | **HOLD** |
| `fruitboard-wt-pr114-rebase-20260913` | `0aa9020` | 0 | – | preserved | **HOLD** |

## Generated fixture classes (`%TEMP%\fruitboard-*`, non-worktree)

| Class | Dirs | GiB | Proposal |
| ----- | ---- | --- | -------- |
| `fruitboard-validation-target` (duplicate cache) | 1 | 5.89 | **HOLD** (live measurement) |
| installed/regression runs (`final-*`, `installed-regression-*`, `fixed-*`, `p2o8-*`, `ntfs-gap1*`) | 9 | ~0.75 | **FIXTURE/EPHEMERAL** after copying any reports |
| `fruitboard-benchmark-*` | 4 | 0.08 | **REMOVE-CANDIDATE** (regenerable) |
| `fruitboard-scan-execution-*` | 365 | 0.06 | **FIXTURE/EPHEMERAL** |
| `fruitboard-scan-console-*` | 22 | 0.004 | **FIXTURE/EPHEMERAL** |
| `fruitboard-watcher-supervisor-tests-*` and watcher scratch | 18 | 0.002 | **FIXTURE/EPHEMERAL** |
| `fruitboard-review*` and other empty scratch | ~10 | 0 | **REMOVE-CANDIDATE** (empty) |

## Proposed cleanup, not executed

Nothing below was performed. Every item requires explicit owner authorization
and the pre-deletion checks in the next section.

1. **Orphan validation cache** — `%TEMP%\opencode\validation-target`
   (2.49 GiB): no worktree or process attached; consolidate to one reusable
   cache.
2. **Duplicate validation cache** — `%TEMP%\fruitboard-validation-target`
   (5.89 GiB): **hold** until the live measuring task confirms completion.
3. **Retired worktree targets** — clean worktrees whose commits are preserved
   in named refs (or merged into `main`): about 7.2 GiB of `target/` across
   `fruitboard-final-regression-20260909` (2.00), `fruitboard-fix-pr92-20260908`
   (1.99), `wt-integration-20260909-v2` (1.49), `wt-review-95-97` (1.76),
   `fruitboard-perf93-enumeration-before` (0.14), `perf94-validation`, and the
   tiny `perf94`/`perfqual`/`agent1`/`wt-pr91` helpers.
4. **Regenerable fixtures** — `%USERPROFILE%\fruitboard-perf93-fixture-custom-9995`
   (0.03), `%TEMP%\fruitboard-benchmark-*` (0.08), and the empty stray
   `%USERPROFILE%\fruitboard-phase-2-library-ui`.
5. **Installed-run fixture directories** — copy any embedded reports to
   `%USERPROFILE%\fruitboard-validation-evidence` first, then remove the
   specific directories; never broad-delete `%TEMP%` or a worktree path.
6. **Four no-ref worktree commits** — archive `0f15ca2` (`agent1-98-main`),
   `90c988d` (`wt-103-fix`), `cccd6da` (`wt-91-final`), and `f847fd1`
   (`wt-91-publish`) under `refs/archive/` before those worktrees are retired.

Rebuild implications: retiring a clean worktree removes its local `target/`
cache; a later checkout of the same source must rebuild (full Rust test builds
for the 1.5–2.0 GiB entries). Source commits and retained evidence are not
affected. The primary development cache and the recorded reusable validation
cache must remain.

## Pre-deletion checklist

Before any authorized removal:

1. Re-run `git worktree list` and resolve each absolute path; confirm no task
   is using it (no running cargo/node, no dirty/untracked content).
2. Archive the four no-ref commits above, or confirm the change already landed
   through a PR.
3. Preserve the primary checkout and every dirty worktree exactly as-is.
4. Use `git worktree remove` (not raw directory deletion) for linked worktrees
   so the admin entry is cleaned. Never use `git clean -fdx` or a blanket
   `cargo clean`.
5. Copy reports, profile databases, and logs intended as evidence to
   `%USERPROFILE%\fruitboard-validation-evidence` (or another recorded path)
   before deleting their source directories.
6. Delete only specific, verified directories — never `%TEMP%`,
   `%TEMP%\opencode`, `%TEMP%\fruitboard-*` as a whole, or a whole worktree
   path.
7. Verify protected worktrees and evidence remain, then measure actual
   free-space gain with `Get-PSDrive`. Directory sums overstate reclaims
   because of hardlinks.

## Handoff

- Cache ownership: primary development cache
  `%USERPROFILE%\fruitboard\target`; recorded reusable validation cache
  `%USERPROFILE%\fruitboard-validation-cache`. Writable caches are not shared
  between concurrent tasks.
- No cache was created, used, or written by this audit; **no local build was
  run** and no compiler output was produced.
- Disk: C: 51.75–52.99 GiB free during the audit; the 30 GiB reserve held.
  G: was not used. The small decline during the audit came from other live
  tasks.
- Evidence: raw reports and scan scripts under `%TEMP%\opencode\` (paths above);
  this summary is the only tracked artifact.
- Deletions proposed: none executed. The list above is advisory and needs owner
  authorization plus the pre-deletion checklist.
- Follow-up: confirm the live `%TEMP%\fruitboard-validation-target` measurement
  finished, and decide the single reusable validation cache before retiring
  duplicates.

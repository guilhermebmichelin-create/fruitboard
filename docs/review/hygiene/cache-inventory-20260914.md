# Cache & worktree inventory — 2026-09-14

Read-only audit. **No deletion, no build, no cache writes were performed.**
This file is a local deliverable only and is **not** tracked or committed.

- Scope: `git worktree list`, `C:\Users\person\fruitboard-*`,
  `%TEMP%\fruitboard-*`, `%TEMP%\opencode\*` (worktrees and `*-target` caches).
- Method: `git worktree list --porcelain`/`-v`, `git rev-parse`,
  `git merge-base --is-ancestor`, `git status --porcelain`,
  `git for-each-ref --contains`, `Get-ChildItem -Recurse -Force | Measure-Object`,
  `Get-Item -Force` (reparse check), `git worktree prune --dry-run`.
- Governed by [DEVELOPMENT.md](DEVELOPMENT.md#local-build-and-disk-space-rules)
  and [AGENTS.md](AGENTS.md#build-and-cleanup-boundaries). Recommendations are
  advisory; deletion requires explicit owner authorization plus the pre-deletion
  verification described at the end.

## Capacity at audit time

| Volume | Used | Free |
| ------ | ---- | ---- |
| C: | 412.04 GiB | **63.41 GiB** |
| G: | 184.97 GiB | 15.03 GiB |

C: currently holds the 30 GiB local reserve. The audited C: tree accounts for
roughly **50 GiB** of worktree, cache, and fixture data.

## Recommendation legend

- **KEEP** — active source, authoritative cache/evidence, or uncommitted/unique
  work that must survive.
- **HOLD** — recent (2026-09-12/13) or tied to an open PR/rebase; remove only
  after the owning task confirms it is finished.
- **REMOVE** — clean, no uncommitted content, and every commit is preserved in a
  named local/remote/archive ref. Rebuild cost noted per entry.
- **FIXTURE/EPHEMERAL** — generated test or installed-app run output; safe to
  remove once any embedded reports/evidence are copied out.

Reparse-point status: **all 55 worktree roots (1 primary + 54 linked) and all
cache roots were verified as real directories (0 reparse points)**. None are
junctions/symlinks, so no path-aliasing risk was found.

## Summary

| Group | Entries | GiB | Keep | Remove-candidate |
| ----- | ------- | --- | ---- | ---------------- |
| Primary checkout (`C:\Users\person\fruitboard`) | 1 | 18.805 | yes | – |
| Linked worktrees, `C:\Users\person\fruitboard-*` | 16 | 4.463 | 10 | 2 (+4 HOLD) |
| Linked worktrees, `%TEMP%\fruitboard-*` | 16 | 5.752 | 3 | 11 (+2 HOLD) |
| Linked worktrees, `%TEMP%\opencode\*` | 22 | 6.909 | 5 | 13 (+4 HOLD) |
| Non-worktree `C:\Users\person\fruitboard-*` caches/evidence | 6 | 6.938 | 3 | 3 |
| Orphan external Cargo targets, `%TEMP%\opencode\*` | 8 | 1.195 | 0 | 8 |
| `%TEMP%\fruitboard-*` fixture/run dirs (non-worktree) | ~5 800 dirs | 5.944 | evidence only | bulk |
| **Total** | | **~50.0** | | |

Estimated reclaim if every REMOVE/FIXTURE item is authorized: **~10–12 GiB**
(logical; see hardlink caveat). Holding the 6 HOLD worktrees would forgo a
further ~7 GiB.

`git worktree prune --dry-run` reported **no prunable worktrees**: every linked
worktree directory is present and its admin entry is valid.

---

## 1. Primary checkout

| Path | HEAD | Branch | GiB | target GiB | Reparse | Dirty | Rec |
| ---- | ---- | ------ | --- | ---------- | ------- | ----- | --- |
| `C:\Users\person\fruitboard` | `cccaa678` | `main` | 18.805 | 16.612 | no | 13 | **KEEP** |

- Owning task: repo root; primary development cache per DEVELOPMENT.md.
- Dirty set is in-progress Phase 2 closeout/qualification documentation and
  includes untracked `AGENTS.md`, acceptance-packet, runbook, and journey docs.
  Preserve as-is; do not clean.
- Local `main` (`cccaa67`, 2026-09-09) is behind `origin/main` (`adab234`,
  2026-09-13).
- This `target/` is the designated primary development cache. Do not delete it
  as part of any worktree cleanup.

## 2. Non-worktree caches and evidence, `C:\Users\person\fruitboard-*`

| Path | GiB | Reparse | Created | Owning task | Rec |
| ---- | --- | ------- | ------- | ----------- | --- |
| `C:\Users\person\fruitboard-validation-cache` | 6.826 | no | 2026-09-13 | reusable temp validation cache (debug/release/tmp/.rustc_info.json) | **KEEP** |
| `C:\Users\person\fruitboard-validation-evidence` | 0.046 | no | 2026-09-13 | retained evidence (`warm-reconciliation-20260913`) | **KEEP** |
| `C:\Users\person\fruitboard-perf93-profile-before-db` | 0.020 | no | 2026-09-08 | #93 profile evidence (`storage/` db) | **KEEP** (evidence) |
| `C:\Users\person\fruitboard-perf93-profile-after-db` | 0.020 | no | 2026-09-08 | #93 profile evidence (`storage/` db) | **KEEP** (evidence) |
| `C:\Users\person\fruitboard-perf93-fixture-custom-9995` | 0.026 | no | 2026-09-08 | #93 synthetic fixture | **REMOVE** (regenerable; ~0.03 GiB) |
| `C:\Users\person\fruitboard-phase-2-library-ui` | 0.000 | no | 2026-09-06 | empty stray dir | **REMOVE** (empty) |

`fruitboard-validation-cache` is the one reusable temporary validation cache
allowed per machine by the working agreement; it must stay the single owner of
that role. New validation tasks should set `CARGO_TARGET_DIR` to this path
rather than creating more caches.

## 3. Linked worktrees, `C:\Users\person\fruitboard-*` (16)

| Path | HEAD | Ref | GiB | target | Dirty | Owning task | Rec |
| ---- | ---- | --- | --- | ------ | ----- | ----------- | --- |
| `…-acceptance-gap-refresh-20260913` | `01d1837` | `docs/41-acceptance-gap-refresh` (+origin) | 0.006 | – | 0 | #41 / #118 | **KEEP** (recent) |
| `…-agent-disk-rules-20260913` | `c868a91` | `docs/agent-disk-rules-20260913` (+origin) | 0.006 | – | 0 | disk-budget policy (this work) | **KEEP** (recent) |
| `…-benchmark-investigation-20260912` | `5164a6f` | `investigate/pr108-benchmark-20260912` | 0.908 | 0.331 | 1 | #108/#112 | **KEEP** (untracked `target-owned-final/`) |
| `…-build-storage-preflight` | `9c225a9` | `chore/41-build-storage-preflight` (+origin) | 0.006 | – | 0 | #41 | **KEEP** (recent) |
| `…-p2o8-verify` | `adab234` | detached (`chore/41-build-storage-preflight`, `docs/41-closeout-drafts`) | 0.006 | – | 0 | #41 Phase 2 O8 verify | **KEEP** (recent) |
| `…-perf93-enumeration` | `588867b` | `perf/93-enumeration-optimization` | 0.155 | 0.150 | 0 | #93 | **HOLD** |
| `…-perf93-enumeration-before` | `1d7c298` | detached (reachable via `docs/41-…-v2`) | 0.141 | 0.136 | 0 | #93 baseline | **REMOVE** (baseline target; rebuild ~few min) |
| `…-pr111-hardening` | `f814034` | `fix/111-closed-diagnostics` | 2.039 | 1.829 | 0 | #111 | **HOLD** (open PR) |
| `…-pr111-rebase-20260913` | `606635c` | detached (`origin/test/phase2-ntfs-cases-20260912`) | 0.006 | – | 0 | #111 rebase helper | **HOLD** |
| `…-qualification-execution-20260913` | `484c5eb` | `docs/qualification-evidence-20260913` (+origin) | 0.006 | – | 0 | #41 qualification | **KEEP** (recent) |
| `…-qualification-restore-cccaa67` | `cccaa67` | detached (== local `main`) | 0.006 | – | 0 | #41 qualification | **REMOVE** (redundant with primary; rebuild 0) |
| `…-qualification-runbook-20260913` | `348f9ab` | `docs/qualification-runbook-20260913` | 0.209 | – | 0 | #41 qualification | **KEEP** (recent) |
| `…-queue-verify-20260909` | `aba1812` | `test/95-durable-queue-watcher-20260909` | 0.478 | 0.269 | 0 | #37/#38/#95 | **HOLD** |
| `…-warm-reconciliation-profile-20260913` | `341dfd2` | `spike/41-warm-reconciliation-profile` (+origin) | 0.210 | – | 0 | #41 spike | **KEEP** (recent) |
| `…-wt-pr112-rebase-20260913` | `386b4c9` | detached (`origin/investigate/pr108-…`) | 0.275 | 0.066 | 0 | #112 rebase helper | **HOLD** |
| `…-wt-pr114-rebase-20260913` | `0aa9020` | detached (`origin/docs/qualification-runbook-…`) | 0.006 | – | 0 | #114 rebase helper | **HOLD** |

## 4. Linked worktrees, `%TEMP%\fruitboard-*` (16)

All under `C:\Users\person\AppData\Local\Temp\`.

| Path (suffix) | HEAD | Ref | GiB | target | Dirty | Owning task | Rec |
| ------------- | ---- | --- | --- | ------ | ----- | ----------- | --- |
| `fruitboard-final-regression-20260909` | `9c211ad` | detached (reachable via `docs/41-…-v2`) | 2.006 | 1.998 | 0 | #41 regression | **REMOVE** (rebuild ~full crate test) |
| `fruitboard-fix-pr92-20260908` | `bf0aeac` | `fix/92-installed-scan-console` | 2.001 | 1.987 | 0 | #92 | **REMOVE** (rebuild full) |
| `fruitboard-installed-journey-docs-20260908` | `49a5e64` | `docs/41-installed-app-journey-20260908` | 0.006 | – | 0 | #41 | **REMOVE** (tiny) |
| `fruitboard-perf94-validate-after` | `b35c01a` | detached (reachable) | 0.005 | – | 0 | #94 | **REMOVE** |
| `fruitboard-perf94-validate-before` | `1d7c298` | detached (reachable) | 0.005 | – | 0 | #94 baseline | **REMOVE** |
| `fruitboard-perf94-validation` | `a3e738b` | `perf/94-validation-20260909` | 0.013 | 0.007 | 0 | #94 | **REMOVE** |
| `fruitboard-perfqual-20260912` | `a44653b` | `perf/41-qualification-20260912` (+origin) | 0.007 | – | 0 | #41 perf qual | **HOLD** |
| `fruitboard-perfqual-before-20260912` | `f63a1d3` | detached (archive ref) | 0.005 | – | 0 | #41 perf qual | **REMOVE** |
| `fruitboard-perfqual-before94-20260912` | `1d7c298` | detached (reachable) | 0.005 | – | 0 | #94 baseline | **REMOVE** |
| `fruitboard-perfqual-candidate-20260912` | `d342ec1` | detached (archive ref) | 0.005 | – | 0 | #94/#41 | **REMOVE** |
| `fruitboard-perfqual-candidate94-20260912` | `b35c01a` | detached (reachable) | 0.005 | – | 0 | #94 | **REMOVE** |
| `fruitboard-phase2-closeout-20260912` | `6591895` | `docs/41-phase2-closeout-20260912` (+origin) | 0.007 | – | 0 | #41/#109 | **HOLD** |
| `fruitboard-phase2-integration-20260912` | `e90b03c` | `integration/phase2-stack-20260912` (+origin) | 0.008 | – | 0 | #41 | **HOLD** |
| `fruitboard-phase2-ntfs-cases-20260912` | `ef08522` | `test/phase2-ntfs-cases-20260912` | 1.661 | 1.653 | 0 | #41 NTFS cases | **HOLD** |
| `fruitboard-phase2-source-stack-20260912` | `9da0272` | `integration/phase2-source-stack-112-20260912` | 0.007 | – | 1 | #112 | **KEEP** (untracked handoff doc) |
| `fruitboard-pr91-reconcile-20260908` | `70fa20f` | detached (reachable) | 0.006 | – | 0 | #91 | **REMOVE** |

## 5. Linked worktrees, `%TEMP%\opencode\*` (22)

All under `C:\Users\person\AppData\Local\Temp\opencode\`.

| Path (suffix) | HEAD | Ref | GiB | target | Dirty | Owning task | Rec |
| ------------- | ---- | --- | --- | ------ | ----- | ----------- | --- |
| `agent1-104-main` | `00bb3e8` | (reachable via `origin/fix/41-queue-stall-…`) | 0.007 | – | 0 | #104 | **REMOVE** |
| `agent1-106-main` | `9b86cbf` | `docs/41-fixed-validation-20260910` | 0.006 | – | 0 | #106 | **REMOVE** |
| `agent1-91-main` | `caa1053` | `origin/docs/41-reconciliation-20260908` | 0.006 | – | 0 | #91 | **REMOVE** |
| `agent1-94-main` | `5cda879` | `origin/perf/93-enumeration-optimization` | 0.005 | – | 0 | #94 | **REMOVE** |
| `agent1-95-main` | `2398b2a` | `origin/fix/92-installed-scan-console` | 0.006 | – | 0 | #95 | **REMOVE** |
| `agent1-97-main` | `9aa1100` | `origin/test/95-durable-queue-watcher-20260909` | 0.006 | – | 0 | #97 | **REMOVE** |
| `agent1-98-main` | `0f15ca2` | **no ref contains it** | 0.005 | – | 0 | #98 | **KEEP** (unique commit; archive before removal) |
| `agent1-98-main2` | `38509e4` | `origin/perf/94-validation-20260909` | 0.005 | – | 0 | #98 | **REMOVE** |
| `agent1-phase2-closeout` | `3ebac5f` | `docs/41-phase2-closeout-20260910` | 0.006 | – | 1 | #41 | **KEEP** (untracked acceptance packet) |
| `agent2-s5-restart-pin` | `e209b8a` | `test/107-s5-restart-pin` (+origin, archive) | 1.750 | 1.742 | 0 | #107 | **HOLD** |
| `phase2-closeout-drafts` | `8d6a1d0` | `docs/41-closeout-drafts` (+origin) | 0.006 | – | 0 | #41 | **KEEP** (HELD moved `adab234→8d6a1d0` mid-audit = live) |
| `queue-stall-fix` | `ded02ac` | `fix/41-queue-stall-retry-sweep` | 1.801 | 1.794 | 0 | #41 | **HOLD** |
| `wt-103-fix` | `90c988d` | **no ref contains it** | 0.005 | – | 0 | #103 | **KEEP** (unique commit; archive before removal) |
| `wt-91-final` | `cccd6da` | **no ref contains it** | 0.005 | – | 0 | #91 | **KEEP** (unique commit; archive before removal) |
| `wt-91-publish` | `f847fd1` | **no ref contains it** | 0.006 | – | 0 | #91 | **KEEP** (unique commit; archive before removal) |
| `wt-a-postmerge` | `0b7612d` | `docs/89-postmerge-status` (merged) | 0.005 | – | 2 | #89 | **KEEP** (modified README + untracked doc) |
| `wt-b-plan` | `0b7612d` | `docs/41-execution-plan-refresh` (merged) | 0.005 | – | 1 | #41 | **KEEP** (modified plan doc) |
| `wt-bounded-scan-20260909` | `9aa97ab` | `docs/41-bounded-scan-design-20260909` | 0.006 | – | 0 | #41 | **REMOVE** |
| `wt-integration` | `0b7612d` | `docs/41-pr-stack-integration-20260909` (merged) | 0.005 | – | 0 | #41 | **REMOVE** |
| `wt-integration-20260909-v2` | `9c211ad` | `docs/41-pr-stack-integration-20260909-v2` (+origin) | 1.494 | 1.487 | 0 | #41 | **REMOVE** (rebuild ~full) |
| `wt-pr91-20260909` | `8437906` | `docs/41-reconciliation-20260908` | 0.006 | – | 0 | #91 | **REMOVE** |
| `wt-review-95-97` | `3e73329` | `docs/41-installed-regression-20260909` | 1.763 | 1.756 | 0 | #95/#97 | **REMOVE** (rebuild ~full) |

> Note: `phase2-closeout-drafts` advanced from `adab234` to `8d6a1d0` between the
> two audit reads. That is evidence of a live concurrent task; keep it and do not
> touch it during cleanup.

## 6. Orphan external Cargo target caches, `%TEMP%\opencode\*`

These are standalone `CARGO_TARGET_DIR` caches not attached to any current
worktree. They duplicate what the single reusable validation cache already
provides and are the cleanest reclaim in this audit.

| Path (suffix) | GiB | Reparse | Created | Rec |
| ------------- | --- | ------- | ------- | --- |
| `agent1-104-target` | 0.237 | no | 2026-09-09 | **REMOVE** (orphan; rebuild ~minutes) |
| `agent1-97-target` | 0.214 | no | 2026-09-09 | **REMOVE** |
| `agent1-98-target` | 0.007 | no | 2026-09-09 | **REMOVE** |
| `agent1-94-target` | 0.007 | no | 2026-09-09 | **REMOVE** |
| `cargo-target-s5` | 0.265 | no | 2026-09-09 | **REMOVE** |
| `target-perf94-after` | 0.133 | no | 2026-09-09 | **REMOVE** |
| `target-perf94-before` | 0.133 | no | 2026-09-09 | **REMOVE** |
| `target-perf94-validation` | 0.199 | no | 2026-09-09 | **REMOVE** |

## 7. Temporary fixture / installed-app run directories

These are generated by tests and installed-app qualification loops. They are not
worktrees and hold no Git state. Aggregate by class under `%TEMP%`:

| Class (pattern) | Dirs | Files | GiB | Rec |
| --------------- | ---- | ----- | --- | --- |
| `fruitboard-scan-execution-*` | 3 569 | 7 138 | 0.597 | **FIXTURE/EPHEMERAL** (delete all) |
| `fruitboard-scan-console-*` | 1 520 | 3 040 | 0.265 | **FIXTURE/EPHEMERAL** (delete all) |
| `fruitboard-watcher-supervisor-tests-*` | 688 | 1 376 | 0.105 | **FIXTURE/EPHEMERAL** (delete all) |
| other `fruitboard-*` run/fixture dirs | ~40 | ~180 000 | 3.247 | mixed; see notable list |

Total `%TEMP%\fruitboard-*` = **11.696 GiB**, of which linked worktrees (sections
4 + part of the mixed class) are 5.752 GiB; the remainder is fixture/run output.

Notable non-worktree fixtures to review before removal:

| Path | GiB | Contains | Rec |
| ---- | --- | -------- | --- |
| `fruitboard-qualification-20260913-8c3f0c2a` | 1.956 | `fixture/`, `reports/`, `target/`, `target-fresh/` | copy `reports/` to retained evidence, then **REMOVE** |
| `fruitboard-perfqual-target-{current,candidate,before,before94,candidate94}-20260912` | 0.72 total | external cargo targets | **REMOVE** |
| `fruitboard-ntfs-cases-run-20260912-{a…j}` | 0.53 total | NTFS case run output | **REMOVE** |
| `fruitboard-phase2-queued-restart-*` (6) | 0.53 total | installed restart runs | **REMOVE** |
| `fruitboard-installed-journey-20260908-*` / `fruitboard-fix-pr92-installed-20260908` | ~0.3 total | installed journey runs | **REMOVE** |
| `fruitboard-perfqual-fixtures-20260912` | 0.052 | synthetic fixtures | **REMOVE** (regenerable) |
| `fruitboard-pinned-tools-20260912` | 0.052 | pinned tool copies | **HOLD** (verify not the active toolchain) |

`%TEMP%\opencode\*` also contains non-`target` scratch dirs (`pw`, `diffcheck`,
`build-gate-*`, `pinned-tools`, `fixture-custom-9995-validate`,
`queue-stall-evidence`, `fruitboard-capture`, `build-integration-*`). They were
outside the requested `*-target` scope; sizes are small and not itemised here.

## Measurement caveats

- Sizes are logical sums of `File.Length` from `Get-ChildItem -Recurse -Force`.
  **Cargo `target/` trees contain hardlinks** (e.g. copied `.pdb`/`.exe`/`.rlib`),
  so logical size overstates unique on-disk bytes. Actual reclaimed space after a
  cleanup will be somewhat lower than the table sum; measure with
  `Get-PSDrive` before/after rather than trusting directory sums.
- `git worktree prune --dry-run` returned no output: there are no stale
  worktree admin entries, only real directories.
- Commit reachability was checked against local heads, `origin/*`, and
  `refs/archive/fruitboard/stack-20260913/*`. "REMOVE" means the HEAD is contained
  in at least one such ref. Four worktrees have HEADs contained in **no ref**
  and are marked KEEP until their commits are archived (section 5).
- This audit ran concurrently with other agents. Worktree HEADs can move; the
  `phase2-closeout-drafts` change observed mid-audit is called out above.

## Pre-deletion checklist (from AGENTS.md / DEVELOPMENT.md)

Before any authorized removal:

1. Re-run `git worktree list` and resolve each absolute path; re-check that no
   task is using it (no running cargo/node, no dirty/untracked content).
2. For the four unique-commit worktrees, first create an archive ref
   (`git update-ref refs/archive/... <sha>`) or confirm the change is already in
   `main`/a PR before deleting.
3. Preserve the dirty worktrees and the primary checkout exactly as-is.
4. Use `git worktree remove` (not raw delete) for linked worktrees so the admin
   entry is cleaned. Never use `git clean -fdx` or a blanket `cargo clean`.
5. Copy `reports/` and any profile DBs/logs intended as evidence to
   `fruitboard-validation-evidence` (or another recorded evidence path) first.
6. Delete only the specific `%TEMP%` fixture directories and `*-target` caches;
   never broad-delete `%TEMP%`, `%TEMP%\opencode`, or a whole worktree path.
7. Verify protected worktrees/evidence remain, then measure actual free-space
   gain with `Get-PSDrive` and report rebuild implications in the handoff.

## Provenance appendix (SHA → preserving ref)

| SHA | Preserving ref(s) |
| --- | ----------------- |
| `9c211ad` | `docs/41-pr-stack-integration-20260909-v2`, `origin/…` |
| `bf0aeac` | `fix/92-installed-scan-console` |
| `49a5e64` | `docs/41-installed-app-journey-20260908`, `docs/41-pr-stack-integration-…` |
| `b35c01a` | `docs/41-pr-stack-integration-20260909-v2`, `fix/41-queue-stall-…` |
| `1d7c298` | `docs/41-pr-stack-integration-20260909-v2`, `fix/41-queue-stall-…` |
| `a3e738b` | `perf/94-validation-20260909` |
| `f63a1d3` | `archive/fruitboard/stack-20260913/pr111-head-a3b90a4` |
| `d342ec1` | `archive/fruitboard/stack-20260913/pr111-head-a3b90a4` |
| `70fa20f` | `docs/41-pr-stack-integration-20260909-v2`, `docs/41-reconciliation-…` |
| `00bb3e8` | `origin/fix/41-queue-stall-retry-sweep` |
| `9b86cbf` | `docs/41-fixed-validation-20260910`, `origin/…` |
| `caa1053` | `origin/docs/41-reconciliation-20260908` |
| `5cda879` | `origin/perf/93-enumeration-optimization` |
| `2398b2a` | `origin/fix/92-installed-scan-console` |
| `9aa1100` | `origin/test/95-durable-queue-watcher-20260909` |
| `38509e4` | `origin/perf/94-validation-20260909` |
| `9aa97ab` | `docs/41-bounded-scan-design-20260909`, `docs/41-pr-stack-integration-…` |
| `0b7612d` | `docs/89-postmerge-status`, `docs/41-execution-plan-refresh`, `docs/41-pr-stack-integration-20260909` |
| `8437906` | `docs/41-reconciliation-20260908` |
| `3e73329` | `docs/41-installed-regression-20260909` |
| `cccaa67` | `refs/heads/main` |
| `588867b` | `perf/93-enumeration-optimization` |

Unique-commit worktrees **without** a preserving ref (do not remove first):
`0f15ca2` (`agent1-98-main`), `90c988d` (`wt-103-fix`), `cccd6da` (`wt-91-final`),
`f847fd1` (`wt-91-publish`).

## Handoff

- Cache owner: `C:\Users\person\fruitboard\target` (primary) and
  `C:\Users\person\fruitboard-validation-cache` (reusable validation).
- Disk: C: 63.41 GiB free; reserve maintained. Estimated reclaim if fully
  authorized: ~10–12 GiB logical, less after hardlink accounting.
- Retained evidence: `fruitboard-validation-evidence`, `fruitboard-perf93-*-db`,
  plus any `reports/` copied out of `fruitboard-qualification-20260913-8c3f0c2a`.
- No files were deleted, no builds were run, and no caches were written.

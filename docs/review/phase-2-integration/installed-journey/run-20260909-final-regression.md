# Final installed regression — 2026-09-09 (frozen integration 9c211ad)

Status: **Evidence only; no merge, no production scanning, no Phase 2 acceptance.**

This record completes the final installed regression at the frozen
integration commit after PR #99 turned green. It preserves the historical
records `run-20260908.md` (PR #92, D1–D3) and `run-20260909.md` (PR #95 fix)
unchanged and adds actual observations from the frozen package. Where work
could not converge it reports the exact blocker instead of a new checklist.

## 1. Freeze and provenance

- Integration PR #99 verified before the run: head
  `9c211adeb555aeacd6a12cb074cca5d359b9f0d7`, state `OPEN`, `MERGEABLE`,
  all 10 required checks `SUCCESS` (`docs-policy`, `client`,
  `rust-portable`, `migration`, `windows-foundation` 6m56s, `security`,
  `enumeration-windows`, `filesystem-watcher-windows`,
  `scan-execution-windows` via Foundation run `34360568998`;
  `windows-packaging-smoke` 4m7s via run `34360569032`). Frozen for this
  run; no wait for another integration wave.
- Isolated checkout: detached worktree at the frozen SHA (outside the
  repository; absolute path omitted for privacy). `git rev-parse HEAD`
  returns the frozen SHA. No code changes after freeze.
- Pinned toolchains via the repository-relative `.tools` directory:
  `node v24.20.0`, `pnpm 11.25.0`, `uv 0.12.9`, `rustc 1.98.1`,
  `python 3.11.16` (isolated `.venv`). No global fallback.
- Host: `Microsoft Windows 11 Home Single Language`, `10.0.26200`, 64-bit.
- Local gates at the frozen SHA (isolated worktree, logs preserved under
  the run-specific journey root outside the repository):
  `pnpm.cmd check` **PASS** (toolchains, SQLite embedded 3.53.2,
  privacy 275 files, format, docs 69 files 0 issues, scripts, lint,
  typecheck, `node --test` 78 pass, client 13 files/134 tests, Rust
  workspace, release build); `cargo test -p fruitboard-desktop --features
  scan-console --locked` **91 passed**; `cargo clippy -p
  fruitboard-desktop --features scan-console --all-targets --locked -- -D
  warnings` **PASS**. These are automated evidence, not installed-app
  evidence.

## 2. Package (packaging-smoke,scan-console explicitly enabled)

Built in the isolated checkout (worktree-local `target/` because
`scripts/prepare-foundation-sidecar.mjs` expects the repo-relative sidecar
path):

```text
node scripts/prepare-foundation-sidecar.mjs
pnpm.cmd --filter @fruitboard/desktop exec tauri build --ci --no-sign
  --config src-tauri/tauri.package.conf.json
  --features packaging-smoke,scan-console --bundles nsis
```

Both features were passed explicitly as a comma-separated list. The
existing `pnpm.cmd package:windows:smoke` (only `packaging-smoke`) was not
used. The unsigned NSIS bundle is `Fruitboard Foundation Smoke 0.1.0`:

| Artifact | Size | SHA-256 |
| --- | --- | --- |
| `target/release/bundle/nsis/Fruitboard Foundation Smoke_0.1.0_x64-setup.exe` | 2,937,398 bytes | `5B455375D61B8C64DCF0AFF4F1FDD0F2835D683DB658371A0771286D6784C820` |
| `target/release/fruitboard-desktop.exe` (build) | 9,888,768 bytes | `B98C0C1C8C84F5C400100C216275F47C883BC1B9B27A829ED44CE7EDD363E1C5` |
| Installed `fruitboard-desktop.exe` | 9,888,768 bytes | `8278106F9F68713CE123137F40C4C42CB7194C5E6FAEA3C83737416BA99C63DC` |
| `fruitboard-sidecar-smoke.exe` (build and installed, identical) | 156,160 bytes | `3B1C8B17A978036968F0CE33AD9A90912627114D9892BD2053E7A33F09A4D6D5` |

The installed-exe hash differs from the build-exe hash because the bundler
patches bundle-type information during NSIS packaging; sizes match. The
installer path used was the exact path printed by the completed build
inside `target/release/bundle/nsis`; no stale artifact was selected.

## 3. Fixtures, isolation, and smoke-data collision

Synthetic fixtures only (marker bytes, no personal FLP content):

| Fixture | Generator | Manifest SHA-256 | Selected root |
| --- | --- | --- | --- |
| Primary | baseline, seed `20260908` | `36d8c2c7d92298caca1a394c5c1dc718e924a1bf49935ebdcd457fea5cc4ac50` | `fixture/roots/shard-0000-sunset-beat` (ten FLP-named files plus one decoy; four-record Library page size exercised) |
| Cancellation | baseline, seed `20260909` | `bba313903d3ce4a873bca028649341c3ead6d13638d3f490171ca6aaf0b756d0` | `cancellation-fixture/roots` (large subtree for cancel/interruption observability) |

Run root: `%TEMP%\fruitboard-final-20260909-20260909-135637`
(`journeyId 20260909-135637`). Install root:
`<journeyRoot>\Installed Fruitboard` (silent `/S /D=` install, exit 0).
The app was launched only with a loopback WebView2 remote-debugging port
to make the installed DOM observable; the probe enforces the packaged
`http://tauri.localhost` origin and the `· Fruitboard` title suffix.

Dedicated Foundation Smoke data:
`%LOCALAPPDATA%\com.fruitboard.desktop.foundation-smoke`. Before setup no
`fruitboard-desktop` process was left running for this run: one orphaned
installed process (started 10:39, idle over 3 h, no DB/log writes, no owned
TCP port, `Responding True`) was closed gracefully via `CloseMainWindow`
(verified gone), then the existing smoke directory was preserved through a
reversible move to
`<journeyRoot>\archived-foundation-smoke-20260909-135637`. A concurrent
run sharing the same host smoke directory launched competing installed
processes during this regression (old install path under a different
`%TEMP%` regression root); each was closed gracefully before proceeding to
keep exclusive use, and its two foreign roots (`Regression Primary`,
`Regression Cancel`) polluted the first live database (four roots total).
That polluted database was then archived reversibly to
`<journeyRoot>\archived-polluted-4roots-20260909-1438` and the remaining
clean run used a fresh database with only the two `Clean`/`Final` roots
below. Prior evidence under `%TEMP%\fruitboard-installed-journey-*` and
`%TEMP%\fruitboard-installed-regression-20260909-102110` was not deleted.
The normal `%LOCALAPPDATA%\com.fruitboard.desktop` directory was never
opened, changed, or deleted.

Restoration: close every `fruitboard-desktop` process, move the current
live smoke directory to a new run-specific archive, then move the chosen
archive back to
`%LOCALAPPDATA%\com.fruitboard.desktop.foundation-smoke`. No restore was
performed during the run; both archives plus the final live database remain
for owner review.

## 4. Installed observations (synthetic only)

All native calls below went through the installed app's Tauri bridge
(`window.__TAURI_INTERNALS__.invoke`; the earlier
`window.__TAURI__.core.invoke` shape fails with `Cannot read properties of
undefined`). Statuses, pages, and DOM state were read from the installed
app. Full JSONL transcripts (`regression-full.jsonl` for the polluted
four-root run, `regression-clean.jsonl` for the clean two-root run) remain
under the journey root for review.

### 4.1 Native Library labeling — PASS

- Empty Library: `data-review-adapter="native"`, no `Review harness`
  badge.
- Populated Library (ten-file leaf, three pages 4+4+2): same `native`
  marker, `data-library-state="populated"`, `4 committed locations on
  page 1`, per-root pages (`Regression Primary`, `Regression Cancel`,
  `Final`/`Clean Primary`), each row with filename, owning root,
  root-relative path, byte size (decimal string), modified time (RFC 3339),
  and `present`/`missing`; `TOTAL FILES Unknown`, no percentage, no FLP
  metadata or grouping. The `scan-console` state reports
  `{enabled:true}`. D1 is fixed.

### 4.2 Cancel → retained → fresh Scan now — PASS

On the large `roots` subtree: first scan completed (8000 committed rows);
second `scan_now` then immediate `cancel_scan` reached `cancelled`
(`cancellationRequested true`, `retryAvailable false`,
`errorCode cancelled`); `get_library_page` still returned the prior
committed snapshot (four rows on page 1, same `snapshotId` family). A fresh
`scan_now` created a different job (`...0cc8...` cancelled vs
`...1057...` completed) and completed through `running`. The cancelled job
and its chain remain in SQLite. Queued visibility was honestly captured on
the ten-file leaf: `queued` with `jobId` and null `runId` for about 7 s
before `completed` (job `...9b11...`, run `...b4bc...`, `filesObserved
10`). No percentage was displayed or inferred.

### 4.3 Exhausted failure → new chain, old preserved — PASS (clean run)

Clean database, `Clean Primary` (ten files): after a completed baseline,
the leaf was moved away and `scan_now` failed (`errorCode unavailable`
after automatic retries; first failure `retryAvailable true`). Automatic
retries continued without operator action and reached `failed`,
`attempt 4/4`, `max_attempts 4`, `last_error_code worker_failed`,
`retryAvailable false` (job `...2001...`, four runs attempts 1–4, all
`worker_failed`). The committed ten rows stayed visible. After restoring
the leaf there was no auto-retry; `retry_scan` on the exhausted job
returned typed `conflict`; a fresh `scan_now` created a different job
(`...7df1...`, `queued` → `running` → `completed`, ten rows) with a
different `retry_chain_id`. SQLite confirms the exhausted `...2001...`
row remains `failed 4/4` alongside the completed `...7df1...` row.

### 4.4 Disabled/removed-root stale actions — PASS

- Disabled `Final`/`Clean Cancel` root: `list_scan_statuses` keeps prior
  committed rows; `scan_now` returns typed `conflict`; `retry_scan` on a
  prior terminal job returns `conflict`; `retryAvailable false` with the
  `Enable this root in Preferences` guidance. Re-enabled after the check.
- Removed temp root: `remove_scan_root` succeeds; later `scan_now` on the
  removed id returns typed `not_found` (`retryable false`). Fixture files
  were not touched by removal.

### 4.5 Unavailable-root retention and recovery — PASS via automatic retry; explicit follow-up stalled (see §5)

Clean `Clean Primary`: moved away, `scan_now` failed (`unavailable`,
`retryAvailable true`, ten committed rows retained), restored within
seconds. The durable automatic retry then completed (`...807f...`,
attempt 2, `completed`, ten rows) without operator action. The
subsequent explicit `scan_now` coalesced to an already-queued watcher
follow-up (`...8908...`, `already_queued`) that never ran while the app
was running (see §5), so no explicit-retry convergence is claimed beyond
the automatic recovery.

### 4.6 Interruption → relaunch — PARTIAL (same job preserved as queued; no convergence)

Because the worker had already stalled on the queued follow-up (§5), the
interruption kill happened while the large-root scan was still `queued`
(job `...63bc...`, null `runId`), not `running`. The exact process was
terminated (`taskkill /F`, verified gone), the database copy showed no
`running` run, and after relaunch the same job was retained as `queued`
with counters reset and the prior committed page intact. It stayed
`queued` for the full 120 s recovery window and never reached `running`
or `completed`. This preserves the recovery-identity half (same job, no
loss) but does not demonstrate restart convergence. The earlier
`run-20260908.md` interruption (killed while `running`, resumed to
`completed` on the same job) remains the only installed running-recovery
evidence and is not superseded.

### 4.7 Watcher burst — NOT OBSERVED (installed); automated bound stands

Five synthetic `.flp` files were created in burst succession on the clean
primary leaf while the queued follow-up (§5) was already stalled. No new
follow-up ran while the app was running; `get_library_page` stayed at ten
rows (`BURST-ROWS-MISSING`). The at-most-one queued-follow-up bound was
therefore not independently counted in the installed run. The deterministic
suites (`burst_during_running_scan_sets_one_durable_follow_up`,
`watcher_burst_yields_exactly_one_follow_up_per_window`,
`burst_activity_produces_one_durable_follow_up`, lifecycle/coalescer tests)
remain the only burst evidence and are classified as automated, not
installed.

## 5. Failures and exact blockers

1. **Worker queue stall (product failure, clean isolated reproduction).**
   After the automatic unavailable recovery completed (`...807f...`
   attempt 2 at 17:39:08 UTC), one watcher follow-up (`...8908...`,
   `queued`, null `runId`, due `not_before_ms` already past) never ran
   for over three minutes while the installed app was running and
   responsive to `list_scan_statuses`/`get_library_page`. Every later
   `scan_now` coalesced to `already_queued` on the same stalled job.
   Counters stayed at the last committed values, no `running` was ever
   observed, and the final CDP evaluation timed out after the 120 s
   recovery window. The live database at close holds two due `queued`
   jobs (`...8908...` primary, `...63bc...` cancel) with no `running`
   run. This blocks installed burst convergence, explicit unavailable
   retry convergence, and running-interruption convergence in this run.
2. **Shared smoke-directory contention (external).** The host
   `%LOCALAPPDATA%\com.fruitboard.desktop.foundation-smoke` cannot be
   isolated per worktree. A concurrent run under a different `%TEMP%`
   regression root repeatedly launched its older installed build against
   the same database during this regression. Exclusive use was coordinated
   by graceful close before each phase and by the two reversible archives
   above; its two foreign roots are visible in the first transcript and
   absent from the clean transcript. No foreign process was force-killed
   while doing durable work; all closes were verified.
3. **Tooling notes (no evidence impact).** System `python` (3.8.5) cannot
   open `STRICT` tables (`malformed database schema ... near "STRICT"`);
   all database assertions used the pinned `uv run --python 3.11.16`
   interpreter (SQLite 3.53.1). One transcript line uses
   `window.__TAURI__.core.invoke` and fails; the run uses
   `window.__TAURI_INTERNALS__.invoke` throughout.

## 6. Cleanup and boundaries

- Installed app closed gracefully (`taskkill` without `/F` verified;
  forced only after the stall timeout for the final close). No
  `fruitboard-desktop` process remains.
- Uninstaller `uninstall.exe /S` from the run-specific install root exited
  `0`; the install directory and installed executables are absent
  afterward. The isolated smoke database is preserved
  (`storage\fruitboard.db` 14,954,496 bytes, SHA-256
  `52125A090D4EE4F95ED2E4C2F0CD2EFBE0171B9FC3913F3DA049F535EE39A7F1`;
  `owner.lock` present). Journey fixtures, transcripts, build logs, both
  smoke archives, and the final database remain for review. Only the
  run-specific install directory was removed; no prior `%TEMP%` evidence
  and no normal `com.fruitboard.desktop` data was deleted or modified.
- Synthetic burst files created for the burst attempt were removed after
  the stall verdict; the primary leaf is back to its ten-plus-one shape.
  Moved-away directories were restored; no `*-moved-away-*` remains.
- Production scanning was not enabled: the validated package is the
  explicitly feature-enabled Foundation Smoke build; default builds keep
  the typed `unavailable` path. No merge was performed, no budget/quota
  was amended, and Phase 2 is not declared accepted.

## 7. Evidence classes

- Merged commit plus remote CI (§1), isolated `pnpm check`/feature tests
  plus deterministic watcher/scan-execution/storage suites (§1, §4.7),
  installed observations (§4), and owner acceptance remain separate. The
  deterministic suites are automated evidence only and do not fill
  installed rows.
- DriveFS (#47), FAT32/cross-volume identity (#48), network shares, ACL
  revocation during a watch, real OS buffer-overflow timing, and the
  100,000-entry qualification were not run and are not claimed.

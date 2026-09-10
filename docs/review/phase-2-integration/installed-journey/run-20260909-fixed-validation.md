# Fixed installed validation — 2026-09-09 (tested candidate ded02ac, unmerged; PR #104 head 41ac09b patch-identical fix for PR #101 stall)

Status: **Evidence only; no merge, no production scanning, no Phase 2 acceptance.**

> Provenance correction (2026-09-10): the live installed run below tested
> unmerged candidate `ded02ac`, not PR #104 head `41ac09b` and not current
> `main`. The two commits carry an identical fix patch
> (`patch-id 7aa01a13c7450c2981011b315ac57affd5f06f18` for
> `crates/storage-sqlite/src/execution.rs` plus
> `scan_console_host_recovery_tests.rs`; pre-fix base blob `7241dc58` identical
> in both lineages), but they are different commits on different bases and must
> not be equated without that check. PR #104 remains DRAFT at `41ac09b`.
> Only the lock helper `90c988d` is merged (PR #103); `ded02ac`, `9c211ad`,
> and `41ac09b` are all unmerged. See §1 and §7.

This record completes the pending fixed installed validation specified in
PR #102 `run-20260909-queue-stall-validation.md` §8, against the Agent 1
fix in PR #104. It preserves `run-20260909-final-regression.md` (PR #101)
and `run-20260909-queue-stall-validation.md` (PR #102) unchanged and adds
live installed observations from the fixed package. Baseline artifact review
was already done; this is a live run, not a read-only report.

## 1. Candidate, lock coordination, and provenance

- Tested candidate SHA (unmerged Agent 1 fix, not PR #104 head, not `main`):
  `ded02ac55ffcc0b6c0a0b25254768e0d564cfe01`
  (`fix(#41): skip failed retry while root active slot is owned`),
  base frozen integration `9c211adeb555aeacd6a12cb074cca5d359b9f0d7`
  (PR #99 DRAFT validation, not merged). Verified by `git -C <worktree>
  rev-parse HEAD`. Transcript `launch.commit` is `ded02ac` on every launch
  with `features packaging-smoke,scan-console`; the binary was not built from
  current `main` (`f63a1d3`) or from PR #104 head `41ac09b`.
- PR #104 head for Agent 1 merge decision (separate commit, same fix patch):
  `41ac09bbf69e6db22378031625fe1b029c3bc101`, parent `f487aa1`, 2 files
  `+261`, `patch-id 7aa01a13c7450c2981011b315ac57affd5f06f18` identical to
  `ded02ac` for the two fix files; pre-fix `execution.rs` blob `7241dc58`
  identical in both lineages. Surrounding base differences (integration docs
  vs `main`-line `#90`/`#100`) do not change the fix logic, but the commits
  must not be conflated: installed evidence below applies to `ded02ac`
  directly and to `41ac09b` only via verified patch equivalence plus PR #104
  remote CI.
- Exclusive test lock (PR #103):
  `90c988defe0b88e661555a4b4468fbca25d68d0e`
  (`fix(packaging-smoke): serialize installed validation through exclusive
  host lock`). The lock helper `scripts/foundation-smoke-lock.ps1` from the
  merged `90c988d` lineage (PR #103, merged as `55668da`) was used for the
  exclusive window procedure only; it is test orchestration and is not
  compiled into the binary. The binary was built from the `ded02ac` worktree
  (worktree-local `target/`, hashes §2 match preserved build outputs), so the
  product code under test is the unmerged `ded02ac` candidate, patch-identical
  to PR #104 head `41ac09b` but not itself merged and not current `main`.
- Lock ownership: `runId 10684d0acbcf452db1210930bf842dc4`,
  `purpose fixed-installed-validation-ded02ac`, lock file
  `%LOCALAPPDATA%/com.fruitboard.desktop.foundation-smoke.lock.json`
  with `CreateNew` semantics, holder PID recorded from a dedicated sleeper
  process. Verified no lock present and no `fruitboard-desktop` process before
  acquiring. Never stopped another agent's process; never deleted
  `storage/owner.lock`; never touched normal `%LOCALAPPDATA%/
  com.fruitboard.desktop` data.
- Previous smoke data preserved reversibly before the fresh run:
  `%LOCALAPPDATA%/com.fruitboard.desktop.foundation-smoke` moved to
  `<journeyRoot>/archived-previous-foundation-smoke-<runId>` with
  `Move-PreviousFoundationSmokeData` hash verification. Archived database
  SHA-256 `52125A090D4EE4F95ED2E4C2F0CD2EFBE0171B9FC3913F3DA049F535EE39A7F1`,
  14,954,496 bytes — case-insensitive match to PR #101 §6. Original evidence
  and prior `%TEMP%` journey roots were not deleted.
- Pinned toolchains (repository-relative `.tools`):
  `node v24.20.0`, `pnpm 11.25.0`, `uv 0.12.9`, `rustc 1.98.1`,
  `cargo 1.98.1`, `python 3.11.16` (isolated `.venv`, SQLite 3.53.1 for
  read-only inspection). No global fallback.
- Host: `Microsoft Windows NT 10.0.26200.0`
  (`Windows 10 Home Single Language`, 2009).
- Locks (observed in the `ded02ac` worktree, not quoted from PR #104 which
  lists no lock hashes): `Cargo.lock` SHA-256
  `CCCB9275FA4951BE4B03B6C485DF998C7C59D523A8A3735B477D36599284BC95`;
  `pnpm-lock.yaml` SHA-256
  `BC3709BB154BDBF760BE6FF410E3ED3E95C6722F80B14A7704AF562AD526B7C9`.

## 2. Package (packaging-smoke,scan-console explicitly enabled)

Built in the isolated `ded02ac` worktree (worktree-local `target/`):

```text
node scripts/prepare-foundation-sidecar.mjs
pnpm.cmd --filter @fruitboard/desktop exec tauri build --ci --no-sign
  --config src-tauri/tauri.package.conf.json
  --features packaging-smoke,scan-console --bundles nsis
```

Both features passed explicitly as a comma-separated list.
`pnpm.cmd package:windows:smoke` (only `packaging-smoke`) was not used.
Unsigned NSIS bundle `Fruitboard Foundation Smoke 0.1.0`:

| Artifact | Size | SHA-256 |
| --- | --- | --- |
| `target/release/bundle/nsis/Fruitboard Foundation Smoke_0.1.0_x64-setup.exe` | 2,936,838 bytes | `79F6C395754C843F2CA09FE9CAEFD08C2C0FB2E25C6B4E6CECE0FA0806A38539` |
| `target/release/fruitboard-desktop.exe` (build) | 9,890,304 bytes | `BAB663FF041B3E662BFA7A1B64D2490D428E2EA123C10FE9D93A93CD32D7CB43` |
| Installed `fruitboard-desktop.exe` | 9,890,304 bytes | `9BFC0534BD7984941F7F0130484EE769AC5704B2A610204AF516DF35B0342699` |
| `fruitboard-sidecar-smoke.exe` (build and installed, identical) | 156,160 bytes | `0E30A3E2F1EEC268C07A672A1C5C5A1E962076517CF73FB1D96136BDFBE43030` |

Installed-exe hash differs from build-exe hash because the bundler patches
bundle-type information during NSIS packaging; sizes match. Installer path
used was the exact path printed by the completed build; no stale artifact
was selected. Silent install `/S /D=<journeyRoot>/Installed Fruitboard`
exited `0`.

## 3. Fixtures and isolation

Synthetic fixtures only (marker bytes, no personal FLP content),
generated by `scripts/generate-synthetic-tree.mjs` in the `ded02ac`
worktree:

| Fixture | Seed | Manifest SHA-256 | Selected root |
| --- | --- | --- | --- |
| Primary | `20260908` | `36d8c2c7d92298caca1a394c5c1dc718e924a1bf49935ebdcd457fea5cc4ac50` | `fixture/roots/shard-0000-sunset-beat` (ten FLP-named files plus one decoy; four-record Library page size exercised) |
| Cancellation | `20260909` | `bba313903d3ce4a873bca028649341c3ead6d13638d3f490171ca6aaf0b756d0` | `cancellation-fixture/roots` (large subtree, 8000 committed rows, for cancel/interruption observability) |

Manifest hashes match PR #101 §3 exactly. Run root:
`%TEMP%/fruitboard-fixed-20260909-20260909-154848`
(`journeyId 20260909-154848`, `runId 10684d0acbcf452db1210930bf842dc4`).
App launched only with loopback WebView2 remote-debugging port; probe
enforces packaged `http://tauri.localhost` origin and `· Fruitboard` title
suffix. Full JSONL transcript `fixed-clean.jsonl` (135 lines, methods
labelled `UI`/`native`) remains under the journey root for review, plus
`s5-db-copy.db` (post-termination copy) and phase marker files.

## 4. Method classes (kept separate)

- **UI actions (`method: UI`):** hash navigation (`location.hash =
  "#/library"`, `"#/preferences"`), DOM reads of
  `data-review-adapter`, `data-library-state`, `Review harness` badge
  absence, `h1`/nav readiness, Library button enumeration, and one real
  `Cancel scan Fixed Cancel` button `click()` (S6). No picker dialog was
  driven; roots were added via native `add_scan_root` with synthetic paths
  (pickers cannot select synthetic `%TEMP%` paths deterministically).
- **Direct native commands (`method: native`):**
  `window.__TAURI_INTERNALS__.invoke` for `get_scan_console_state`,
  `list_scan_roots`, `add_scan_root`, `scan_now`, `cancel_scan`,
  `retry_scan`, `list_scan_statuses`, `get_library_page`. One transcript line
  in PR #101 using the older `window.__TAURI__.core.invoke` shape is not
  repeated here.
- **Database inspection (labelled separately):** read-only SQLite
  (`mode=ro`, pinned Python 3.11.16, SQLite 3.53.1) against the live DB and
  the `s5-db-copy.db` copy. No write handle was opened; original evidence DB
  hash unchanged (see §1).
- **Authoritative completion rule:** a queued acknowledgement
  (`queued`/`already_queued`/`already_running`) or responsive
  `list_scan_statuses`/`get_library_page` alone never passes. Only a durable
  terminal `completed` run plus the stated committed Library rows counts.
  Every scenario below applies that rule.

## 5. Installed observations (synthetic only)

### 5.0 Baselines and Library labeling — PASS

- Empty Library via UI: `data-review-adapter="native"`,
  `data-library-state="empty"`, no `Review harness` badge.
- After baselines: `data-review-adapter="native"`,
  `data-library-state="populated"`, `4 committed locations on page 1`,
  per-root pages, each row with filename, owning root, root-relative path,
  byte size (decimal string), modified time (RFC 3339), and
  `present`/`missing`; `TOTAL FILES Unknown`, no percentage, no FLP metadata
  or grouping. `get_scan_console_state` reports `{enabled:true}`.
- Baselines (fresh DB, two roots `Fixed Primary`/`Fixed Cancel`):
  primary manual `...5fee...` completed in 0.12 s with 10 rows; cancel manual
  `...362d...` completed in 4.4 s with 8000 rows. Watcher follow-ups also
  completed (`...60fd...` primary periodic, `...6b13...` cancel periodic).
  Committed rows 8010 (10 + 8000). One `interrupted` periodic from the
  harness-timeout kill was preserved as history; recovery jobs completed
  after relaunch (see §5.4).

### 5.1 Scenario 1 — Unavailable → automatic recovery → watcher follow-up — PASS

- Moved primary leaf away, `scan_now` coalesced to `already_queued` on the
  restart-recovery queued job (honest coalescing, not a new failure — worker
  was busy with recovery). Restored within 3 s.
- Automatic retry completed without operator action (bound 60 s):
  `completed` with 10 rows.
- Explicit `scan_now` created a **new** `queued` job (not
  `already_queued`-forever — the PR #101 stall shape), completed in ~1.2 s
  with 10 rows (bound 120 s). UI Library confirms 10 committed rows,
  `native` adapter.
- **Fix evidence:** the previously wedged `already_queued`-forever path now
  converges. PASS.

### 5.2 Scenario 2 — Repeated manual scans after that sequence — PASS

Three sequential `scan_now` after S1 convergence, each `queued` →
`completed` with unchanged 10-row Library content (bound 60 s each).
Job IDs `...06f3...`, `...08cd...`, `...0aaf...` (all `manual`,
`completed 1/4`). PASS.

### 5.3 Scenario 3 — Burst changes appearing in committed Library results — PASS

- Created five synthetic `.flp` files in burst succession
  (`burst-alpha/beta/gamma/delta/epsilon-0000.flp`).
- Exactly one follow-up ran to `completed` and the Library page showed
  **15 present rows including all five burst markers** (bound 120 s;
  observed ~4 s). Native `get_library_page(limit 100)` plus UI body check
  confirm markers. At-most-one queued-follow-up bound holds in the installed
  run (previously automated-only).
- Removed the five burst files, explicit `scan_now` → `completed`; Library
  now shows **10 present + 5 missing** (burst rows correctly transitioned to
  `missing`, not deleted), leaf back to ten-plus-one shape. This matches the
  specified presence semantics (removed stays visible as `missing`).
- PASS (burst convergence + correct missing transition).

### 5.4 Scenario 4 — Two roots progressing while one has failed retry history; poisoned-root state — PASS

- Moved primary away longer (4 s) so the scan executed while offline:
  `failed`, `errorCode unavailable`, `retryAvailable true`
  (job `...0372...`, attempt 1). This is failed retry history.
- Restored, then overlapping `scan_now` on **both** roots:
  primary `...1dc2... queued`, cancel `...1dde... queued`.
- Both reached `completed` within 120 s (observed ~4.5 s):
  primary 15 total rows (10 present + 5 missing from S3),
  cancel 8000 rows (page limit 100 returned full page; DB confirms 8000).
  UI shows both roots `Completed`, `native` adapter.
- **Poisoned-root evidence (#104 regression):** primary held
  `{failed retry-eligible + new queued}` while cancel had due work. Before
  the fix, `service_retries` aborted the whole sweep on the `UNIQUE`
  violation (`scan_job_active_root`) and `claim_due` skipped the claim,
  wedging **every** root with no running work. Here cancel's due job ran to
  `completed` while primary carried failed history, and primary's failed
  chain later retried to `completed 2/4`. Due work on the other root
  continues. PASS.

### 5.5 Scenario 5 — Termination while genuinely Running → restart → convergence — PARTIAL (successor convergence; same-job not demonstrated)

- Started large-root scan when worker idle enough to observe genuine
  `running` with non-null `runId`: `...9cf5...`/`...a116...`,
  `filesObserved 4608/8000` (mid-scan, not `queued`). PASS for the Running
  precondition (rapid 100 ms poll, bound 30 s). UI marker `native` before
  termination.
- Terminated the exact PID (`taskkill /F`, verified gone via `process.kill`
  probe). DB copy `s5-db-copy.db` (taken after kill, before relaunch) shows
  **no `running` run** and the target run `...a116...` as `interrupted`;
  two `queued` recoveries already exist. PASS for the termination + no-
  running-lease check.
- Relaunched, awaited 120 s: the **same job `...9cf5...` stayed
  `interrupted 1/4`**; a **successor recovery job `...abb4...` completed
  `1/4`** with the committed page intact (5-row sample page non-empty, full
  DB 8000 rows for the cancel root, UI `native`). No data loss, no wedge,
  worker converged.
- **Verdict:** PARTIAL. Authoritative convergence via successor is
  demonstrated (committed rows intact, worker not wedged, UI responsive),
  but the strict PR #102 §8 item 5 requirement ("same job to reach
  completed") is **not** met in this run. Reproducible case is immediate
  from the preserved artifacts:
  `s5-job.txt` (`01a087a4-9cf5-76d1-847e-d658cc8be729`),
  `s5-db-copy.db` (no running, target interrupted, two queued),
  transcript `S5-scan-now` (`already_running` coalescing onto a startup
  recovery job), `S5-running-observed` (4608 files), `S5-terminated`,
  `S5-after-restart` (same job interrupted, successor completed).
  The prior `run-20260908.md` same-job running-recovery remains the only
  installed same-job evidence and is not superseded. Whether successor-
  recovery (new job, new chain, old `interrupted` preserved) is the intended
  hard-kill contract versus same-job requeue (as in
  `shutdown_fences_active_run_as_interrupted_for_restart_recovery`) needs an
  Agent 1 decision; no acceptance is claimed on this item.

### 5.6 Scenario 6 — Cancellation and exhausted-chain fresh scans preserving old history — PASS (with S6b exhaust)

- **Cancel:** large-root `scan_now` → `queued` (`...f1f0...`), observed
  `running` with `runId`, then **real UI click** on
  `Cancel scan Fixed Cancel` (method `UI`) followed by native `cancel_scan`
  returning `already_cancelled` (the UI click had already cancelled).
  State `cancelled`, page still returns prior committed snapshot (5-row
  sample non-empty; DB 8000 rows retained). Cancellation persisted, no
  partial publication. PASS.
- **Exhausted (S6b, longer offline window):** moved primary away,
  `scan_now` → `queued` (`...c92f...`), polls show `running` →
  `failed retryAvailable true` (attempts 1–3) → `failed retryAvailable
  false` after ~12 s. DB confirms **manual failed 4/4 and periodic failed
  4/4** (attempts 3,4 failed runs), preserved. `retry_scan` on the exhausted
  job returns typed `conflict` (`status error`, `code conflict`). Restored,
  fresh `scan_now` creates a **different job** (`...f8b4...` vs `...c92f...`,
  different `retry_chain_id`), `queued` → `completed` with 15 rows
  (10 present + 5 missing). Old exhausted rows remain `failed 4/4` alongside
  the completed chain. PASS.
- The initial S6 attempt restored after 66 ms and saw `retry → queued`
  (not yet exhausted); S6b with a 12 s offline window is the authoritative
  exhaust evidence. Both transcripts are preserved.

## 6. Cleanup and boundaries

- App closed (no `fruitboard-desktop` process remains; verified).
- Uninstaller `uninstall.exe /S` from the run-specific install root exited
  `0`; install directory absent afterward. Live database preserved:
  `storage/fruitboard.db` 14,987,264 bytes, SHA-256
  `DEA0CBB12618E0F7E48C2908F061CC0E8CA134E51FB7EB294BE5EF0956C52AEA`,
  `owner.lock` present. Journey fixtures, transcripts (`fixed-clean.jsonl`
  135 lines), build logs, smoke archive, `s5-db-copy.db`, and phase markers
  remain for review. Only the run-specific install directory was removed; no
  prior `%TEMP%` evidence and no normal `com.fruitboard.desktop` data was
  deleted or modified.
- Lock released owner-only
  (`Release-FoundationSmokeLock -RunId 10684d0acbcf452db1210930bf842dc4`);
  lock file absent afterward; foreign locks preserved (none existed).
  Sleeper holder stopped.
- Production scanning was not enabled: the validated package is the
  explicitly feature-enabled Foundation Smoke build; default builds keep the
  typed `unavailable` path. No merge was performed and Phase 2 is **not**
  declared accepted.

## 7. Evidence classes

Unmerged candidate observations (§5, tested `ded02ac`), merged lock helper
plus remote CI (merged `90c988d` via PR #103 as `55668da`; PR #104 merged as
`cccaa67` on current `main`, which includes PRs #94 `d342ec1`, #98 `7e0e9f6`,
PR #95 `ee720af`, PR #97 `ff5d8ee`, and security #105; PR #104 exact-head CI
Foundation `34426552063` + Packaging `34426552044` green on `00bb3e8` before
squash), deterministic suites (merged #104 lineage: `fruitboard-desktop
--features scan-console` 92 passed including the new regression (91 from #97
plus 1), `fruitboard-storage` 76 passed, `fruitboard-scan-execution` 56
passed, Clippy clean; the 92-count seen in the `ded02ac` integration
worktree reflects its extra integration-base tests and must not be quoted as
the PR #104 result), installed observations (§5), and owner acceptance remain
separate. DriveFS (#47), FAT32/cross-volume identity (#48), network shares,
ACL revocation during a watch, real OS buffer-overflow timing, and the
100,000-entry qualification were not run and are not claimed.

Agent 1 update (2026-09-10): the fix patch tested here (`ded02ac`,
patch-id `7aa01a13`) is identical to merged #104 (`cccaa67`, squash of
`00bb3e8`); pre-fix `execution.rs` blob `7241dc58` identical. Installed
evidence below applies to merged `cccaa67` via that verified patch
equivalence plus #104 exact-head CI. S5 PARTIAL preserved separately as issue #107;
no acceptance claimed.

## 8. Merge status (PR #104 merged; this evidence PR ready)

**PR #104 merged as `cccaa67` (not `ded02ac`) after rebase onto `ff5d8ee`
with full `pnpm check` green via exact-head CI, S5 recorded as PARTIAL.
Do not declare Phase 2 accepted on this record. Do not merge `ded02ac`
itself: it remains an unmerged integration-worktree candidate.**

- The queue-stall root cause and fix are corroborated installed: the
  `already_queued`-forever wedge (§5.1), burst non-convergence (§5.3),
  two-root starvation (§5.4), and exhausted/cancel freshness (§5.6) all
  converge on the fixed build with authoritative `completed` + committed
  rows, while the frozen `9c211ad` baseline wedged on the same shapes
  (PR #101 §5, validated read-only in PR #102).
- Poisoned-root fairness holds: a root carrying failed retry history no
  longer wedges due work on the other root.
- S5 successor-convergence versus same-job-convergence preserved separately
  as #107: either accept successor-recovery as the hard-kill contract (then
  S5 becomes PASS with the reproducible IDs above), or implement same-job
  requeue after `taskkill /F` (PR #104 already merged for the stall fix).
  Either way the stall fix itself was not blocked by S5, and independent
  scenarios (1–4, 6) are complete.
- Full `pnpm check` green via #104 exact-head CI (Foundation `34426552063` +
  Packaging `34426552044` on `00bb3e8`); this validation built and installed
  but did not run the full gate itself.

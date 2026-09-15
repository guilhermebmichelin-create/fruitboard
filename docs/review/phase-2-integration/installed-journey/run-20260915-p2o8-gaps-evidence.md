# P2-08 gaps 1-3 installed evidence run - 2026-09-15

Status: **executed once on current `origin/main` `02d5b82`; gaps 1-2 partially
evidenced and gap 3 remains open.** This is installed evidence only. It does not
merge a pull request, change any issue or comment, change a budget/fixture/target,
order a product fix, promote P2-08, or activate production scanning. The gap 4
full-criterion disposition and any P2-08 promotion remain owner-only.

This run validates the four narrowed gaps recorded in the
[2026-09-14 acceptance record](../acceptance-2026-09-14.md#p2-08-remains-partial-accepted-known-gap)
against the acceptance wording drafted in the
[P2-08 follow-up proposal](../p2o8-followup-proposal-2026-09-15.md)
(gaps 1-3 only; gap 4 is out of scope), and reproduces the D2/D3 converged
behavior recorded by the merged [#121 run](run-20260914-p2o8-retry-presentation.md).
The explicitly unproduced codes carried by that record are
`access_denied`, `resource_limit`, `unsupported`, and `worker_failed`.

## Source boundary and method

<!-- markdownlint-disable MD060 -->

| Item              | Recorded value                                                                                                                                                                                                                                                                     |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source boundary   | `02d5b82c0aa8078e28adff4105265f1781e83dd1` (`docs(#138): scaffold parser fixture corpus manifest (2026-09-15) (#145)`), `origin/main`, clean worktree at branch `docs/p2o8-gaps-evidence`                                                                                          |
| Gap 1 producers   | Documented locked local-NTFS procedure `scripts/run-installed-ntfs-cases.ps1` (NTFS ACL denial, 10,001-observation overflow), synthetic fixtures only                                                                                                                              |
| Gap 2/3 producers | Review-only evidence driver derived from `p2o8-retry-state-capture.mjs`, driving the installed WebView2 CDP surface                                                                                                                                                                |
| Toolchain         | Node `v24.20.0`, pnpm `11.25.0`, Corepack `0.36.0`, cargo/rustc `1.98.1`, uv `0.12.9`, Python `3.11.16`, `x86_64-pc-windows-msvc`                                                                                                                                                  |
| Build command     | `cargo build --locked --release -p fruitboard-foundation-sidecar-smoke --target x86_64-pc-windows-msvc` then `pnpm --filter @fruitboard/desktop exec tauri build --ci --no-sign --config src-tauri/tauri.package.conf.json --features packaging-smoke,scan-console --bundles nsis` |
| Filesystem scope  | Local NTFS only. FAT32, DriveFS, and cross-volume identity remain excluded per the [2026-09-15 non-NTFS scope decision](../../../research/non-ntfs-scope-decision-20260915.md); `G:` was below the reserve and was not used                                                        |

<!-- markdownlint-enable MD060 -->

Both installed sessions ran under the repository Foundation Smoke exclusive host
lock (`scripts/foundation-smoke-lock.ps1`). Raw screenshots, driver transcripts,
logs, and database copies were retained outside Git under
`%TEMP%\opencode\evidence\p2o8-gaps-20260915\`; only a redacted summary and
hashes are committed.

## Provenance and build

<!-- markdownlint-disable MD060 -->

| Item                 | Recorded value                                                                                                                                 |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Installer            | `Fruitboard Foundation Smoke_0.1.0_x64-setup.exe`, 2,946,043 bytes, SHA-256 `2330F0678EA88B86D06E398C266BA51DCE7839C695F68F6095F68FF37866CA40` |
| Pre-bundle cargo exe | `fruitboard-desktop.exe`, 9,891,328 bytes, SHA-256 `2C119BAA314DF12B60C48DCFDBBDF3AC92D1C01187F45094295216468A8A031E`                          |
| Installed exe (run)  | `fruitboard-desktop.exe`, 9,891,328 bytes, SHA-256 `B00973B5D12E898D44DEBF7A4DAF816357221222EC29B45DBB1D72FE418A2687`                          |
| Sidecar              | `fruitboard-sidecar-smoke.exe`, 156,160 bytes, SHA-256 `244EE19E5D45D3DECFD1BE2A442058B5E2ADB4CD4C75A3776E55F3226358957B`                      |

<!-- markdownlint-enable MD060 -->

Both independent silent installs of this installer produced the same installed
executable hash `B00973B5...`, which is the binary that actually ran. The
pre-bundle cargo output has a different hash at the same size; this run does not
claim a verified cause for that difference, consistent with the #121 record.

`pnpm install --frozen-lockfile --ignore-scripts` reused the existing store
(316 packages, 0 downloads). `pnpm verify:toolchains` passed inside the locked
NTFS procedure (Node 24.20.0, pnpm 11.25.0, Corepack 0.36.0, Rust 1.98.1,
uv 0.12.9, Python 3.11.16). The wrapper's post-run junction cleanup hit a
Windows PowerShell `Remove-Item` `NullReferenceException`; the driver result,
database inspection, uninstall, and lock release all completed before that
cleanup step, and the wrapper exit code therefore does not reflect the run
outcome. The driver transcript records `driver-result: pass=true`.

## Disk, cache, and exclusive window

- Preflight (`scripts/preflight-build-storage.mjs --source <worktree> --build
<cache> --cache <cache> --evidence <evidence> --estimated-output-gib 5`):
  `C:` available 60.19 GiB, estimated output 5 GiB, remaining 55.19 GiB; the
  30 GiB reserve held. The cache scan reported `unsafe` for 1132 hardlinked
  regular files (1684 `hardlink-alias` findings including duplicate identities)
  across 6508 scanned files, with no reparse points. `fsutil hardlink list`
  confirmed every alias is Cargo-internal (for example
  `release\fruitboard-desktop.exe` <-> `release\deps\fruitboard_desktop.exe`)
  with no alias into the source checkout or the evidence directory. No evidence
  file is a hardlink; all retained binaries are independent byte copies.
- The one reusable temporary validation cache
  `%TEMP%\opencode\validation-target` was set explicitly as `CARGO_TARGET_DIR`
  and owned exclusively by this task for the build window; it was 2.49 GiB after
  the run. The primary development cache was not written.
- Capacity after both installed sessions: `C:` 59.06 GiB free; `G:` 14.98 GiB
  (below reserve, unused).
- The task owned the exclusive installed-test window. Before acquisition and
  after release: no `com.fruitboard.desktop.foundation-smoke.lock.json`, no
  `fruitboard-desktop` process, and no concurrent `cargo`/`rustc`/benchmark
  workload. The lock was acquired once per installed session and released by its
  owning run.

## Gap 1 - typed presentations for the four unproduced codes

### `access_denied` - evidenced

The documented local-NTFS procedure committed a four-row baseline, then denied
the current user read/traverse access to one subtree with `icacls`, confirmed a
same-user `EPERM` denial probe, and rescanned.

<!-- markdownlint-disable MD060 -->

| Observation             | Installed result                                                                                                                  |
| ----------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Durable terminal state  | job `01a0a4fb-f2fe-75ec-9b2b-f09faafd4adc`, run `01a0a4fc-2cfa-70ed-b32d-4eb32bb668ba`, `state=failed`, `errorCode=access_denied` |
| `retryAvailable`        | `false`                                                                                                                           |
| Rendered copy           | "ExecutionFailed ... The folder could not be read. Previous committed results were kept."                                         |
| Rendered control        | enabled `Scan now`; no non-actionable Retry                                                                                       |
| Prior committed results | `lastSuccessfulScanAt` unchanged; page still exactly 4 rows, all `present` (no false-missing rows)                                |

<!-- markdownlint-enable MD060 -->

Durable code and rendered copy agree. The NTFS case also asserts the last-success
marker and every committed row are unchanged.

### `resource_limit` - evidenced

The procedure committed an 8000-row quota-fitting baseline, then added 2001 more
synthetic `.flp`-named locations so the next authoritative scan exceeded the
unchanged 10,000-observation bound.

<!-- markdownlint-disable MD060 -->

| Observation             | Installed result                                                                                                                   |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Durable terminal state  | job `01a0a4fc-48ba-7328-ae09-66493a2a08bb`, run `01a0a4fc-a66b-727f-baee-89a1669ac47f`, `state=failed`, `errorCode=resource_limit` |
| `retryAvailable`        | `false`                                                                                                                            |
| Rendered copy           | "ExecutionFailed ... The scan reached a safe resource limit. Previous committed results were kept."                                |
| Rendered control        | enabled `Scan now`; no non-actionable Retry                                                                                        |
| Prior committed results | `lastSuccessfulScanAt` unchanged; page rows all `present`, no false-missing rows                                                   |

<!-- markdownlint-enable MD060 -->

The 10,000-observation contract, the `custom-9995`/seed `0` qualification
fixture, and the `10,000 ms` target are unchanged.

### `unsupported` - still open (not produced)

`unsupported` is emitted only when the root's filesystem qualification is not
local NTFS. Producing it requires a non-NTFS volume. The only other writable
volume (`G:`) is below the 30 GiB reserve and must not be used, and the
2026-09-15 owner scope decision excludes FAT32/DriveFS/cross-volume roots from
the supported scanner scope. No non-NTFS root was mounted or scanned in this run.

### `worker_failed` - still open (not produced)

`worker_failed` is written for enumeration `Invalid`/`SinkFailed`, a partial
failure class, or a failed worker run. The supported installed path exposes no
deterministic, non-destructive producer without fault injection, and the run did
not inject storage/worker faults. It was not produced.

### Gap 1 verdict

`access_denied` and `resource_limit` are evidenced on current head with durable
code, rendered-copy agreement, correct `retryAvailable`, and committed-result
preservation. `unsupported` and `worker_failed` remain open with the boundary
reasons above; closing them needs either an owner exclusion or a dedicated
fault/non-NTFS run.

## Gap 2 - keyboard and axe evidence for the transient states

`queued` and `running` were captured on the installed current head with a desktop
screenshot, a 390x844 narrow screenshot, a keyboard focus trace over
`Input.dispatchKeyEvent`, an accessibility-tree capture, and a best-effort axe run
under the production CSP (which does not allow `unsafe-eval`).

<!-- markdownlint-disable MD060 -->

| State     | Native status                                                                          | Keyboard trace                                                                   | AX tree                       | axe                                  |
| --------- | -------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- | ----------------------------- | ------------------------------------ |
| `running` | job `01a0a4ff-c731-7508-908b-303701e2cee8`, run `01a0a4ff-c76a-738d-92db-c31bb01f36d2` | 8 Tab steps; reached `Cancel scan Long Scan Root`, root selector, navigation     | 11 controls, 15 announcements | ran; exactly one `region` (moderate) |
| `queued`  | job `01a0a4ff-cf1e-74d6-97b0-feb2b8f32c51`, `runId=null`                               | 8 Tab steps; reached `Cancel scan Long Scan Root` and `Cancel scan Primary Root` | 11 controls, 15 announcements | ran; exactly one `region` (moderate) |

<!-- markdownlint-enable MD060 -->

The previously recorded minor finding reproduced exactly: one moderate `region`
violation per captured state (content not contained by a landmark) and no
critical or serious violations. It is carried for owner review as a non-defect
finding, not a claimed accessibility defect. `cancelled`, `completed`, and
`failed`/`unavailable` also carried keyboard, AX-tree, and axe captures.

### Gap 2 verdict

Evidenced for both transient states on current head. The one moderate `region`
finding still needs the owner's resolve-or-accept decision.

## Gap 3 - a stable rendered interrupted state - still open

Repeated coalesced enqueue during a running scan (`scan_now` on the same root
while it runs) deterministically produced durable interrupted work: the retained
database contains 4 `scan_run` rows with `state=interrupted`, `outcome=interrupted`,
`error_code=follow_up_requested`, and 4 matching `scan_job` rows
(`last_error_code=follow_up_requested`). The installed call returned
`outcome=already_running` for job `01a0a500-1a82-76e1-a949-54dbedabeecf`
(run `01a0a500-1aab-76c3-8312-a5c94aea2a4e`).

However, no stable rendered `interrupted` state was observable. Across three
attempts with 25 ms polling, the per-root rendered status never presented as
`interrupted`; the interrupted attempt is immediately superseded by a fresh
queued/running successor, so the rendered presentation is `queued`/`running` or
the next terminal state. `renderedState` was `null` for all three attempts.

### Gap 3 verdict

Still open. The durable `interrupted`/`follow_up_requested` code is reproduced on
current head, but the acceptance wording requires a stable rendered installed
state that agrees with it, and the current product flow does not expose one.
This is an evidence finding, not a product fix.

## D2/D3 reproduction on current head

The merged #121 converged D2/D3 behavior reproduced on `02d5b82`:

<!-- markdownlint-disable MD060 -->

| Scenario                             | Installed result                                                                                                                                                                                                      |
| ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D2 cancel a running scan             | durable `cancelled` (`cancelled` error code), `retryAvailable=false`, no rejected Retry; enabled `Scan now` converged to `completed`                                                                                  |
| D3 unavailable root, exhausted retry | durable `failed`/`unavailable`, `requestedJobId` advanced to a successor (`successorObserved=true`), `retryAvailable=false`; after restore `Scan now` converged to `completed` with prior committed results preserved |

<!-- markdownlint-enable MD060 -->

## Per-gap verdict summary

<!-- markdownlint-disable MD060 -->

| Gap | Requirement                                                       | Verdict    | Evidence anchor                                                                    |
| --- | ----------------------------------------------------------------- | ---------- | ---------------------------------------------------------------------------------- |
| 1a  | `access_denied` typed presentation                                | Evidenced  | NTFS `denied-after-acl`, durable + rendered agreement, committed rows preserved    |
| 1b  | `resource_limit` typed presentation                               | Evidenced  | NTFS `resource-over-bound`, durable + rendered agreement, committed rows preserved |
| 1c  | `unsupported` typed presentation                                  | Still open | Requires a non-NTFS root; non-NTFS is excluded and `G:` is below reserve           |
| 1d  | `worker_failed` typed presentation                                | Still open | No deterministic non-destructive producer without fault injection                  |
| 2   | Keyboard trace + axe for `queued` and `running`                   | Evidenced  | Current-head installed captures under production CSP; one moderate `region` each   |
| 3   | Stable rendered `interrupted` agreeing with `follow_up_requested` | Still open | Durable interrupted runs (4) produced, but no stable rendered state captured       |
| 4   | Full-criterion P2-08 disposition                                  | Owner only | Not produced by this run; gaps 1c/1d/3 remain unless the owner excludes them       |

<!-- markdownlint-enable MD060 -->

## Retained evidence and classification

Raw evidence is outside Git under
`%TEMP%\opencode\evidence\p2o8-gaps-20260915\`:

- `run\` - `gap23-p2o8-retry-state.jsonl`, `gap23-driver-result.json`,
  `gap23-root-ids.json`, driver patch diff, path records.
- `captures\gap23\` - 12 desktop/narrow PNGs for running, queued, cancelled,
  completed, and failed-unavailable.
- `db\gap23-database-after-run\` - the isolated run database (durable interrupted
  runs and D3 retry chain).
- `ntfs-gap1\` - the gap 1 transcript, provenance, installed artifacts,
  toolchain output, and four database snapshots with read-only DB evidence.
- `provenance\` - preflight JSON, build artifacts, fixture manifests, and an
  `evidence-manifest.json` of per-file sizes and SHA-256 hashes.
- `logs\` - build, fixture, wrapper, driver, and orchestration logs.

The original gap 1 journey root
`%TEMP%\fruitboard-ntfs-gap1b-20260915-091213` and the gap 2/3 journey root
`%TEMP%\fruitboard-p2o8-gaps-20260915-091603` are retained in place. The patched
driver (`p2o8-gaps-driver.mjs`, SHA-256
`E5C6A8D859B8ED95E374326F1287EF1E29302CE661136B7227C0C02CEEDBF72B`) is derived from
the merged driver (SHA-256
`DF6E942DE48F71329750D4AF01570356ECD9C90BB2C7D68C6E85269B09284E30`); the diff is
retained with the evidence and is not committed.

Classification:

- **Active/reusable cache:** `%TEMP%\opencode\validation-target` remains the
  reusable temporary validation cache, 2.49 GiB, owned by this task for its
  build window and free for sequential reuse.
- **Retained evidence:** everything under
  `%TEMP%\opencode\evidence\p2o8-gaps-20260915\` and the two journey roots.
- **Obsolete compiler intermediates:** none were cleaned; the copy-fork driver
  and evidence scripts live under the evidence root.

## Checks and provenance

- Required checks for the owned change: `pnpm.cmd lint:docs` and
  `git diff --check` scoped to the new report. No product, contract, workflow, or
  fixture file changed.
- Full `pnpm.cmd check` is **not** run on this evidence branch: there is no
  product change, the change is documentation-only, and the installed build
  already exercises the packaged artifact. The branch's own required PR contexts
  are the publication gate if a PR is later opened; opening that PR is out of
  scope here.
- Applicable existing CI: Foundation CI success on `origin/main` `02d5b82`
  covers all unchanged code.
- No issue, pull request, label, or comment was created, edited, or closed, and
  nothing was merged.

## What remains owner-only

- The gap 4 explicit P2-08 disposition: promote P2-08 on this evidence, or write
  a per-gap exclusion for `unsupported` (1c), `worker_failed` (1d), and the
  stable rendered `interrupted` state (3).
- The one moderate `region` axe finding: resolve or explicitly accept it.
- Any product fix for gap 3's rendered state; this run only records the finding.
- Whether to open the dedicated P2-08 follow-up versus keeping #38/#40 open.

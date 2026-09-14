# P2-08 installed retry, presentation, and accessibility run - 2026-09-14

Status: **executed once on current `origin/main` `adab234`; the recorded D2/D3
retry gaps and the merged #111 installed presentation are revalidated on the
current head.** This is installed evidence only. It does not merge a pull
request, post an issue update, change a budget or fixture, rerun S5 or the NTFS
cases, promote P2-08, or activate production scanning. Owner acceptance remains
a separate decision.

This record closes the three [P2-08 installed-evidence
gaps](../acceptance-packet-2026-09-12.md#why-p2-08-remains-partial) as far as a
current-head installed run can:

1. explicit Retry after cancellation;
2. explicit Retry after the unavailable-root retry budget is exhausted;
3. a current-head installed revalidation of the merged #111 typed diagnostic
   presentation, with keyboard, narrow-layout, and accessibility captures of
   the installed scan-console states.

The driver is
[`p2o8-retry-state-capture.mjs`](p2o8-retry-state-capture.mjs). Raw logs,
database copies, screenshots, and the built binaries are retained outside Git
under `%TEMP%\opencode\evidence\p2o8-verify-20260914\`; none of it is committed.

## Provenance and build

<!-- markdownlint-disable MD060 -->

| Item                | Recorded value                                                                                                                                                        |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source boundary     | `adab234b9b17b45f87d30460fa643818161e7cf3` (`docs(#41): refresh Phase 2 acceptance gaps (#118)`), fetched `origin/main`, clean detached checkout                      |
| Worktree            | `%USERPROFILE%\fruitboard-p2o8-verify` (exact absolute path retained in the out-of-tree evidence)                                                                     |
| Toolchain           | Node `v24.20.0`, pnpm `11.25.0`, cargo/rustc `1.98.1`, uv `0.12.9`, `x86_64-pc-windows-msvc`                                                                          |
| Build command       | `pnpm --filter @fruitboard/desktop exec tauri build --ci --no-sign --config src-tauri/tauri.package.conf.json --features packaging-smoke,scan-console --bundles nsis` |
| Build cache         | `CARGO_TARGET_DIR=%TEMP%\opencode\validation-target` (single owner: this task; handed off to Agent C below), 1.53 GiB after the run                                   |
| Installer           | `Fruitboard Foundation Smoke_0.1.0_x64-setup.exe`, 2,935,414 bytes, SHA-256 `4262C732B8A0495F185BF8F41B897FF0922976A243433ACE51DC2AB3FF9840B6`                        |
| Cargo release exe   | `fruitboard-desktop.exe`, 9,887,744 bytes, SHA-256 `116E4E1B2ADAC2EF756540591E2DCD003E35FD8B712ABA48B683226CAC569D90`                                                 |
| Installed exe (run) | `fruitboard-desktop.exe`, 9,887,744 bytes, SHA-256 `C6A9FDD5A877BC9E9F07B6F9AF0CDE63F2DE14CDD1B527049D2C4EA35C0E1FE0`                                                 |
| Sidecar             | `fruitboard-sidecar-smoke.exe`, 156,160 bytes, SHA-256 `E0CAB7A47D2BF62271D9106009053DA04253364CD052F3386F945F3AC3F1B397`                                             |

<!-- markdownlint-enable MD060 -->

The installed executable hash is reproduced by an independent second silent
install of the same installer (exit 0, same bytes and hash) and is the binary
that actually ran. The pre-bundle cargo output has a different hash despite the
same size; the installer-to-installed mapping is reproducible and is the
retained run artifact. This report does not claim a verified cause for the
pre-bundle difference.

`pnpm install --frozen-lockfile --ignore-scripts` reused the existing store
(316 packages, 0 downloads). The release build completed with exit code 0. The
installer hash and the installed-executable hash are the provenance anchors.

## Exclusive window and disk

- The run held the existing Foundation Smoke exclusive host lock
  (`%LOCALAPPDATA%\com.fruitboard.desktop.foundation-smoke.lock.json`) for the
  whole install/launch/uninstall sequence and released only its own lock. No
  `fruitboard-desktop` process was live before acquisition or after release.
- Process inspection before and during the run found no sibling `cargo`,
  `rustc`, benchmark, or installed journey workload. `G:` was never used.
- Disk on `C:` (the only volume used; `%TEMP%` is on `C:`) was 63.42 GiB free
  before the build and 60.14 GiB free after the run; the validation cache is
  1.53 GiB. The 30 GiB reserve held throughout.
- Fixtures: `fixture` (seed `20260908`, canonical manifest SHA-256
  `36d8c2c7d92298caca1a394c5c1dc718e924a1bf49935ebdcd457fea5cc4ac50`) selected
  leaf `roots/shard-0000-sunset-beat`; `cancellation-fixture` (seed `20260909`,
  canonical manifest SHA-256
  `bba313903d3ce4a873bca028649341c3ead6d13638d3f490171ca6aaf0b756d0`) selected
  root `roots`.

### Fixture non-mutation proof

The on-disk fixtures after the run are byte-identical to a fresh deterministic
regeneration with the same seeds. The whole-tree SHA-256 (every manifest entry,
content and path) is:

<!-- markdownlint-disable MD060 -->

| Fixture                       | Files |    Bytes | Tree SHA-256 after run                                             | Fresh regeneration                                                 |
| ----------------------------- | ----: | -------: | ------------------------------------------------------------------ | ------------------------------------------------------------------ |
| `fixture` (seed 20260908)     | 10009 | 25696697 | `ff7b18af63c059394ca29dc168c2e6110b1af171ad1c76ebf6c7a1c092f20050` | `ff7b18af63c059394ca29dc168c2e6110b1af171ad1c76ebf6c7a1c092f20050` |
| `cancellation-fixture` (0909) | 10009 | 25432217 | `3e14f756df16deba86be5ee7fb7ad4c95bcc2d192327af33a45f13f12bb34b52` | `3e14f756df16deba86be5ee7fb7ad4c95bcc2d192327af33a45f13f12bb34b52` |

<!-- markdownlint-enable MD060 -->

Zero byte-length mismatches against the manifests. The installed scanner did
not mutate its source fixtures.

## Method

The driver launched the installed `fruitboard-desktop.exe` with a loopback
WebView2 remote-debugging port, connected over CDP, asserted the feature-enabled
scan console, and drove the native commands and the real DOM. Each stable state
was captured as a desktop screenshot, a 390x844 narrow screenshot, a keyboard
focus trace over `Input.dispatchKeyEvent`, the accessibility tree, and a
best-effort axe run. Fixtures were registered through `add_scan_root`; scans
were started with `scan_now`, cancelled with `cancel_scan`, and the state set
was read with `list_scan_statuses`.

## Results

### D2 - Retry after cancellation

On current head, a terminal `cancelled` job is not retryable: the native
`retry_available` flag is false (`scan_console.rs`), and the installed UI
accordingly renders **no Retry control** and offers an enabled `Scan now`
control. This removes the #92 defect where a visible Retry was rejected with
"The scan state changed. Refresh the status and try again.".

<!-- markdownlint-disable MD060 -->

| Observation                 | Installed result                                                                                           |
| --------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Cancel command outcome      | `cancellation_requested` for the running job                                                               |
| Durable job after cancel    | state `cancelled`, attempt 1/4, `last_error_code=cancelled`; run state `cancelled`, `error_code=cancelled` |
| Status `retryAvailable`     | `false`                                                                                                    |
| Rendered controls at cancel | no `Retry scan ...` button; `Scan now Long Scan Root` present and enabled                                  |
| Rendered user text          | "Execution Cancelled" and "The scan was cancelled. No new results were published."                         |
| Actionable re-run           | clicking `Scan now Long Scan Root` converged to `completed`, 8,000 files observed                          |

<!-- markdownlint-enable MD060 -->

Conclusion: the current installed D2 path is honest and actionable. The UI no
longer offers a Retry that the native command rejects, and a fresh explicit
scan converges.

### D3 - Retry after exhausted unavailable-root retry, then restore

The selected leaf was moved away and a scan was requested. The installed app
recorded the typed `unavailable` failure through the durable automatic-retry
chain (four attempts, `attempt=4/4`, `last_error_code=unavailable`), then the
root was restored.

<!-- markdownlint-disable MD060 -->

| Observation                       | Installed result                                                                                               |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Failure presentation              | "Execution Failed" and "The folder is unavailable. Previous committed results were kept."                      |
| `errorCode` while budget remained | `unavailable`, `retryAvailable=true` (Retry would be offered below the budget)                                 |
| Durable exhausted job             | state `failed`, attempt 4/4, `last_error_code=unavailable`; four runs state `failed`, `error_code=unavailable` |
| Status at exhaustion              | `retryAvailable=false`                                                                                         |
| Rendered controls at exhaustion   | no `Retry scan ...` button; `Scan now Primary Root` present and enabled                                        |
| Actionable re-run after restore   | clicking `Scan now Primary Root` converged to `completed`, 10 files observed                                   |
| Prior committed results preserved | last successful results and committed rows stayed visible; the failure did not publish missing rows            |

<!-- markdownlint-enable MD060 -->

Conclusion: the current installed D3 path is honest and actionable. After the
durable budget is exhausted the UI offers a fresh explicit scan rather than a
non-actionable Retry, and that scan reconciles once access returns.

### #111 typed diagnostic presentation (current head)

The durable database and the UI agree on the typed presentation on the current
head:

<!-- markdownlint-disable MD060 -->

| State       | Durable code observed | Client code   | `retryAvailable` | Rendered copy                                                       |
| ----------- | --------------------- | ------------- | ---------------- | ------------------------------------------------------------------- |
| `queued`    | (none)                | (none)        | false            | "Execution Queued", progress counters, "no percentage is estimated" |
| `running`   | (none)                | (none)        | false            | "Execution Running", counters, Cancel control                       |
| `completed` | none                  | none          | false            | "Execution Completed", files observed                               |
| `cancelled` | `cancelled`           | `cancelled`   | false            | "The scan was cancelled. No new results were published."            |
| `failed`    | `unavailable`         | `unavailable` | true / false     | "The folder is unavailable. Previous committed results were kept."  |

<!-- markdownlint-enable MD060 -->

`worker_failed` is not written for these paths on current head; the durable
`unavailable` code from the failing enumeration is preserved through the retry
chain, which is the #111 correction. The installed path also showed the
`follow_up_requested` durable code on an interrupted run created by repeated
enqueue during the cancel race; its client presentation is the `conflict` code.
The `access_denied`, `resource_limit`, `unsupported`, and `worker_failed`
presentations were **not** produced in this run because doing so requires the
NTFS ACL, overflow, or malformed-fixture cases that this task explicitly does
not rerun. They remain Partial at the presentation layer.

### Keyboard, narrow-layout, and accessibility

The installed native mount now renders `data-review-adapter="native"` with no
"Review harness - fake adapter" badge; D1 from the 2026-09-08 record is fixed
on current head.

- Desktop and 390x844 narrow screenshots were captured for the running,
  queued, completed, cancelled, and failed-unavailable console states.
- Keyboard focus traces (12 Tab presses) were captured for the cancelled,
  completed, and failed-unavailable states. Focus reached the scan controls,
  the root selector, the pagination control, and the skip link/navigation in a
  logical order.
- The accessibility tree exposed the interactive controls with names, e.g.
  `button=Scan now Long Scan Root`, `button=Scan now Primary Root`,
  `link=Skip to main content`, and the status/heading nodes.
- axe ran against the installed page under the production CSP (which does not
  allow `unsafe-eval`) for the cancelled, completed, and failed-unavailable
  states and reported exactly one `region` (moderate) violation per state
  (content not contained by a landmark). No critical or serious violations
  were reported. This is a minor finding for owner review, not a claimed
  accessibility defect.

The queued and running states were captured visually but not with a keyboard
trace or axe run, because their transient lifetime is short. The `interrupted`
state was observed durably (an interrupted run with `follow_up_requested`) but
was not captured as a stable rendered state in this run.

## Retained evidence and classification

All raw evidence is under
`%TEMP%\opencode\evidence\p2o8-verify-20260914\`:

- `logs\` - orchestration, install, driver stdout, fixture generation.
- `run\` - the JSONL transcript, `driver-result.json`, root ids.
- `captures\screenshots\` - 13 desktop/narrow PNGs.
- `db\database-after-run\fruitboard.db` - the isolated run database.
- `provenance\` - source boundary, build artifacts, installer copy, fixture
  manifests, fixture tree hashes, and the read-only database inspection.

### Validation cache handoff

`%TEMP%\opencode\validation-target` (1.53 GiB) is the single reusable
validation cache for this machine's next installed/desktop build. This task
owned it exclusively through the run above and hands ownership to Agent C. Agent
C may reuse it sequentially; no two tasks may write it concurrently. The cache
contains the pinned `1.98.1-x86_64-pc-windows-msvc` release build of `adab234`
and the cached `.tauri` NSIS tooling copied from the primary target. The cache
is not evidence by itself; a binary is only proof together with the recorded
source SHA, build command, and binary hash above.

Classification:

- **Active cache:** `%TEMP%\opencode\validation-target` is the reusable
  validation cache and is handed off to Agent C (see above).
- **Reusable:** the `.tools` pinned toolchain and the pnpm store are shared
  machine resources and are unchanged.
- **Retained evidence:** everything under
  `%TEMP%\opencode\evidence\p2o8-verify-20260914\`, the journey root
  `%TEMP%\fruitboard-p2o8-20260913-214337`, and the deterministic regeneration
  tree `%TEMP%\p2o8-regen-20260914` (small, disposable verification tree).
- **Obsolete compiler intermediates:** the one-off sidecar/binary intermediates
  inside `validation-target` are reusable by the next build; none is deleted
  here. This task performed no cleanup or deletion.

## Verification

Targeted checks were run because the owned change is documentation plus one
driver script, not a product or contract change:

- `node --check` on `p2o8-retry-state-capture.mjs` passed.
- Markdownlint and Prettier were run on the touched documents.
- Repository privacy check and `git diff --check` were run.

The existing `origin/main` CI (PR #118, ten required contexts) covers the
unchanged code at `adab234`; it does not cover this new evidence branch, whose
own CI is required before publication. No local `pnpm check` was run because the
checkout's pinned Python environment link is broken and the full gate builds
the whole workspace, which is out of scope for this evidence slice.

## What converged and what remains Partial

Converged on current head `adab234`:

- D2: cancellation is durably typed, the UI does not offer a rejected Retry,
  and the offered explicit scan converges.
- D3: the unavailable failure is durably typed as `unavailable`, the exhausted
  budget removes Retry and offers a fresh scan, and the restored root
  reconciles.
- D1 is fixed (`data-review-adapter="native"`), and the installed console has
  desktop/narrow screenshots, keyboard traces, an AX-tree capture, and an axe
  run.

Remains Partial:

- The `access_denied`, `resource_limit`, `unsupported`, and `worker_failed`
  typed presentations were not produced on current head. Producing them
  requires the NTFS ACL/overflow/malformed cases this task does not rerun.
- The `interrupted` state was observed durably but not captured as a stable
  rendered state.
- No explicit owner acceptance of the full P2-08 criterion is recorded here.
  The D2/D3 disposition (policy versus defect) still requires the owner's
  decision, though current head no longer exposes the rejected-Retry behavior.

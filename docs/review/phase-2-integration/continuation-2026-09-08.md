# Phase 2 continuation review — 2026-09-08

Status: **planning and status update; owner acceptance remains pending.** This
document records the next integration lanes after the P2-12 checkpoint. It
does not rewrite the historical evidence in
[`checkpoint-2026-09-07.md`](checkpoint-2026-09-07.md), amend a budget, promote
an acceptance ID, or activate production scanning.

## Evidence boundary

At the start of this review, remote `main` was `dbabb50` (PR #80). Its Foundation CI run
[`34177098425`](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34177098425)
completed successfully. The local `feat/41-phase2-continuation` branch initially pointed
at `34ce4fc`, an unpublished continuation commit on top of that remote main.
The local commit is evidence in progress and must not be described as merged.

The merged implementation baseline is:

| Area | Merged evidence | Current boundary |
| --- | --- | --- |
| Durable staging and responsiveness | #62 fenced staging/publication and #77 short per-batch database transactions | Atomic publication and bounded batches are implemented; integrated fault and installed-app evidence remains open |
| Native Library surface | #73 feature-gated scan-console IPC and #78 native client adapter/capability seam | Six typed commands are wired behind `scan-console`; default builds keep the scanner hidden and feature-enabled CI plus installed-app proof remain open |
| Enumeration and worker | #64 bounded Windows enumeration and #70 hidden scan worker | The real worker path exists behind the hidden surface; production activation and the remaining acceptance gates are open |
| Watcher and follow-ups | #68/#69 watcher foundation and #71 durable follow-up adapter | The crate and storage halves are merged; desktop lifecycle, root mapping, and activation remain to be integrated |
| Benchmark and checkpoint | #72 first measured report, #79 decision brief, and #80 P2-12 checkpoint | F1–F3 and owner acceptance are still open; measurements do not qualify an installed app |

The fake-adapter captures and native adapter tests provide client and typed
contract evidence. They do not prove a packaged or installed desktop journey.
The local no-parser additions provide static manifest/source policy guards and
separate synthetic-fixture source-byte preservation assertions; they are not a
runtime content-read spy and are not installed-app evidence.

A review of the merged native adapter also found that the desktop capability
did not yet grant `core:event:allow-listen` and `core:event:allow-unlisten`,
even though the client subscribes to `scan-status-changed`. The local working
tree now includes those narrowly scoped permissions and a boundary regression
in `tests/desktop-boundary.test.mjs`; this correction remains unmerged and
requires the feature-enabled CI and installed-app checks below.

## Prioritized parallel lanes

These three lanes can proceed together. Each lane has a concrete exit condition
and preserves the feature gate and evidence labels.

### 1. Native watcher host integration

This wave delivers a deterministic lifecycle integration harness with five
passing scenarios. Automatic desktop watcher activation is deferred after review
of the supervisor draft. The next production slice must preserve undelivered
hints across database errors, coalesce worker wakeups, retry ended watches with
bounded backoff, and explicitly stop/join watcher threads. Root configuration
acknowledgements must not wait for the supervisor while holding the database
lock, or report a failed mutation after the durable change has committed.

The test-only `foundation/watcher_lifecycle.rs` composes the actual watcher
coalescer and follow-up adapter with a disposable durable database. It verifies
one follow-up during a running scan, retry retention after a simulated delivery
failure, overflow precedence, disable/restart generation fencing, removal and
re-addition, and storage suppression before host configuration catches up.
It neither starts native handles nor proves supervisor thread behavior.

Connect the merged `filesystem-watcher` and scan-execution follow-up seams to
the desktop host. The host work should own watcher lifecycle, configured-root
to watcher mapping, generation fencing, restart/disable/remove handling, and
translation of watcher hints into one durable reconciliation follow-up. Keep
coverage loss as a full-reconciliation trigger and retain the rule that hints
never decide presence or absence. The host boundary must carry opaque root and
generation identifiers, not private path data.

The lane is complete when the feature-gated host starts and stops watches for
the configured roots, drops stale generations, schedules bounded follow-ups,
and has deterministic restart/disable/remove coverage. It remains an
integration slice until the native journey and P2-09 acceptance are reviewed;
DriveFS and non-NTFS behavior stays outside its qualification.

### 2. Feature-enabled CI coverage

The local working tree adds the enabled desktop checks to the Windows
foundation job: `cargo test -p fruitboard-desktop --features scan-console
--locked` and warnings-denied Clippy for the same feature. It also adds the
`core:event:allow-listen` and `core:event:allow-unlisten` permissions required
by the client's `scan-status-changed` subscription. Land and run these changes
in CI while keeping the existing feature-off `pnpm.cmd check` and packaging
gates. A green default build cannot stand in for a feature-on result. Keep the
command and capability contracts typed, and retain the recoverable
`unavailable` behavior when the feature is off.

The lane is complete when the feature-on commands are visible in the relevant
CI job and a green run is available for the merged commit. Passing unit or
integration tests still does not promote P2-08 or P2-12 and does not replace
the installed-app journey. The local
`cargo test -p fruitboard-desktop --features scan-console --locked` run passes
60/60 tests, including the five lifecycle harness scenarios. Feature-enabled
Clippy with `--all-targets --locked -- -D warnings` also passes. These are
local results, not a merged CI result or proof of production watcher activation.

The event permission fix also has release-build evidence: before the change,
Tauri removed `listen`, `unlisten`, `emit`, and `emit_to`; after the change it
removes only `emit` and `emit_to`. Subscription commands remain available
without granting the renderer permission to emit events. This verifies command
retention, not the installed-app interaction journey.

### 3. Documentation and status

This lane updates the root README, the current-status and sequencing portions
of [`PHASE_2_EXECUTION_PLAN.md`](../../PHASE_2_EXECUTION_PLAN.md), and this
continuation record. It keeps merged commits, unpublished local work, hidden
worker evidence, fake-adapter rendering, native typed tests, and installed-app
observations labeled separately. It also keeps the benchmark findings linked
without converting them into approved budgets. The lane is complete after the
documentation lint passes and the links and status snapshot agree with remote
`main`.

## Gated work after the parallel lanes

### 4. Installed-app Windows journey

Once the host and feature-on CI lanes are ready, exercise the native surface in
the installed Tauri application. Record the exact merged commit, pinned build,
Windows environment, disposable fixture seed and manifest hash, and disposable
database handling. The journey must cover root selection and cancellation,
enabled settings, Scan now queued/running status with no fabricated percentage,
Library pages, restart survival, add/rename/modify/remove/restore convergence,
cancel and unavailable-root retention of the last committed list, and retry.

The existing hidden worker/driver transcript and fake-adapter screenshots are
useful preparation, but they do not satisfy this installed-app gate. Keep
production scanning hidden until the integrated P2-03 through P2-08 evidence is
reviewed.

### 5. Benchmark profiling and owner decisions

Resolve the findings in the decision brief in this order:

1. **F1 — quota versus baseline.** The accepted baseline generates 10,000
   FLP-named files plus five hardlink aliases, which is 10,005 observations.
   `MAX_STAGED_RECORDS` is 10,000, so the baseline runs end at
   `ResourceLimit` and do not publish. The owner must either raise the quota
   above 10,005 or redefine the fixture as exactly 10,000 observations,
   including aliases, before an authoritative baseline claim.
2. **F2 — warm reconciliation.** The quota-fitting 10,000-observation run
   measured a nearest-rank warm p95 of 32,876 ms (32.876 s), with a median of
   12,875.5 ms, against the provisional 10 s target. First discovery was
   11,886 ms on that comparison fixture. Re-run on an appropriately isolated
   host and/or profile the handle, metadata, and commit paths before any
   re-budgeting; the miss remains a miss while that work is pending.
3. **F3 — 100,000-entry qualification.** The 100,000-entry set cannot complete
   under the current 10,000-record quota, so the `<=128 MiB` qualification is
   unmeasured. The approximately 53 MiB result is a `tasklist` working-set
   sample of the driver on the 10,000-location set. It is not a private-memory
   qualification for the 100,000-entry set or for the installed desktop app.

The authoritative source for the options and owner approval sentences is the
[budget decision brief](budget-decision-brief.md). No benchmark result or
local test changes a provisional budget without an explicit owner decision.

### 6. Manual platform limitations

DriveFS modes (#47) and FAT32/cross-volume identity (#48) remain manual and
unverified. Network-share roots, ACL revocation during a watch, and real OS
buffer-overflow timing carry the same limitation. An ignored fixture or an
unavailable environment is not a passing qualification. The owner must either
land evidence for the environment or record an explicit scope exclusion before
any blanket platform claim.

### 7. Owner acceptance

The owner acceptance record must cover the native watcher host integration,
feature-enabled CI result, installed-app journey, F1/F2/F3 decisions, #47/#48
scope, and the remaining Partial/Pending P2 criteria. The owner may accept a
criterion as pending at close only with a named follow-up; that decision does
not make the criterion complete. Production scanning, the parser, and later
Phase 3 work remain gated by the accepted plan.

## Local continuation note

Commit `34ce4fc` adds bounded coalescing/no-parser continuation checks and an
addendum to the historical checkpoint. Its no-parser test checks dependency
manifests and production source text for forbidden parser/content-I/O patterns;
the source-byte checks refer to separate synthetic-fixture tests that compare
marker bytes before and after discovery. These are useful policy and
preservation guards, but the commit is unpublished, the source check is static,
and neither check supplies installed-app evidence or promotes P2-09/P2-10.

## Local validation for this continuation

- Pinned `pnpm.cmd check` passed again after all code, capability, workflow,
  and harness changes, including the default Windows release build.
- Repository policy tests: 77 passed; shared client tests: 128 passed.
- Feature-enabled desktop tests: 60 passed; feature-enabled Clippy passed.
- Documentation lint, changed-file formatting, `git diff --check`, and
  Actionlint for `foundation.yml` passed (external ShellCheck/Pyflakes disabled).
- No installed-app scan journey or new performance benchmark was run.

All continuation changes remain local on `feat/41-phase2-continuation`; no PR,
push, merge, issue closure, or acceptance decision is recorded by this review.

## Integration review follow-up

The follow-up review found no concrete code blocker in this continuation
slice. The unpublished checkpoint addendum and policy-test names now describe
static source checks and fixture byte equality accurately; they do not claim
a runtime content-read spy. The historical merged checkpoint sections remain
unchanged.

The next assignments are recorded in the
[next agent wave](next-agent-wave-2026-09-08.md): production supervisor
implementation, independent fault verification, and installed-app journey
preparation. Start that wave from this continuation's merged commit after its
required checks pass. The pre-integration local snapshot above is historical;
the PR and GitHub merge record provide the eventual commit and CI provenance.

## Merged continuation and next-wave baseline

PR #81 merged the continuation as
`e5e777d3c192d4d3da35284e6e3c8e5af3ec3f58`. The earlier local/unpublished
statements above describe the pre-merge review snapshot. Post-merge
[Foundation CI run 34218497618](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34218497618)
passed for that exact commit. Its
[Windows foundation job](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34218497618/job/102035821402)
passed the integrated Windows gate, enabled scan-console tests, and enabled
warnings-denied Clippy. The merged capability includes only event listen and
unlisten permissions for the subscription correction.

This establishes the starting boundary for the next parallel agent wave.
Supervisor implementation and independent fault verification are in progress;
their combined commit and CI result remain pending. Installed-app observations,
F1–F3 decisions, platform scope decisions, and owner acceptance remain pending.
This baseline record does not activate production scanning or promote an
acceptance criterion.

## Local next-wave integration

The parallel wave ran from merged `e5e777d` in separate worktrees: GPT 5.6
Luna max owned the native host, and GPT 5.6 Luna xhigh owned independent
failure verification and installed-app journey preparation. The parent
integrated the lanes on `wave/watcher-integration`. The final code snapshot is
local commit `4e72d06`; it is not a merged-main or installed-app result.

The feature-gated host now owns separate scan and watcher loops over the same
database, capacity-one wake channels, configured-root revision fences,
monotonic native generations, bounded reconnect backoff, and one retained
coalesced hint per root. Coverage loss takes precedence over activity. A
terminal hint whose delivery fails stays pending across reconnect failures;
it is delivered after a fresh native watch starts. Root commands notify the
host after their durable transaction has returned. Shutdown joins both loops
and interrupts owned work for restart recovery while preserving user
cancellation, including a cancellation mirror set before its durable write.
If the shutdown fence cannot be written, the active traversal may finish
naturally before the join completes rather than becoming a false cancellation.

The parent review corrected transient configuration-read handling, wakeup
bounds, the disable/re-enable-between-polls race, startup recovery versus
native coverage-loss handling, thread-start error cleanup, host ownership on
exit, and shutdown/cancellation ordering. Independent tests exercise the real
supervisor with injected watcher handles, clocks, and database contention.
Separate host tests execute the real empty-root threads and check joins,
idempotence, and release of the host reference. These tests do not qualify an
installed app or a configured native watch inside the installed host.

Local validation:

- Pinned Windows `pnpm.cmd check` passed at `7f42dd7`, including 77 repository
  policy tests, 128 client tests, workspace Rust checks, and the default release
  build. Watcher and scan-execution crate tests passed; ignored environment
  fixtures remain unverified.
- After the final feature-gated cancellation correction at `4e72d06`,
  `cargo test -p fruitboard-desktop --features scan-console --locked` passed
  86 tests, and enabled Clippy with `--all-targets --locked -- -D warnings`
  passed. The correction touches only enabled host code and its tests.
- The [installed-app checklist](installed-app-journey-checklist.md) records
  pinned feature-enabled packaging, separate disposable fixtures and database
  handling, expected behavior, and an empty observation table.

The wave's owner merge and post-merge feature-on CI remain pending. Remote
`main` is still `e5e777d` at this record. Installed-app acceptance, F1–F3,
the #47/#48 and other platform scope decisions, production scanning, parser work,
and Phase 3 remain gated. No acceptance ID or provisional budget is promoted.

## Post-merge addendum — PR #82 (2026-09-08)

Owner merge is done: PR #82 merged as
`05d39ff074ab6c568d998175744551561928de89` on 2026-09-08T15:58:03Z
(squash-merge from `wave/watcher-integration`,
"feat(#37): integrate native watcher supervision and shutdown recovery (#82)").

Post-merge [Foundation CI run 34248128449](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34248128449)
for the merged commit `05d39ff` completed `success`. Its
[Windows foundation job](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34248128449/job/102135306455)
passed the integrated Windows gate, the enabled scan-console suite
(`cargo test -p fruitboard-desktop --features scan-console --locked`), and
enabled warnings-denied Clippy
(`cargo clippy -p fruitboard-desktop --features scan-console --all-targets --locked -- -D warnings`).
A green default build alone was not used as evidence.

Required checks on `main` are docs-policy, client, rust-portable, migration,
windows-foundation, security, windows-packaging-smoke, enumeration-windows,
filesystem-watcher-windows, and scan-execution-windows. The post-merge
Foundation run covers 9/9 Foundation jobs on `05d39ff`; `windows-packaging-smoke`
runs only on `pull_request` (plus schedule/dispatch), so its evidence for this
wave is PR run
[34236211276](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34236211276)
(`success` on PR head `445f4dd`), not a post-merge push run.

Installed-app journey observations, F1–F3 decisions, #47/#48 scope decisions,
and owner acceptance remain pending. This addendum does not promote an
acceptance ID, amend a budget or quota, claim installed-app evidence, or
activate production scanning.

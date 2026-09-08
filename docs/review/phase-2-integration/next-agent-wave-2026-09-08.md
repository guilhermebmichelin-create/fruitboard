# Phase 2 next agent wave — 2026-09-08

Status: **planning handoff only.** This file assigns the next bounded work
after the current continuation is reviewed and landed. It does not activate
production scanning, promote an acceptance ID, change a provisional budget, or
claim installed-app evidence.

Execution record: the starting continuation merged as `e5e777d` (PR #81),
and its [Foundation CI run](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34218497618)
passed, including the feature-enabled Windows tests and Clippy. The assignments
below are the original handoff. Their local implementation and validation are
recorded in the [continuation addendum](continuation-2026-09-08.md#local-next-wave-integration).
The wave's owner merge and post-merge feature-on CI remain pending. The
[installed-app checklist](installed-app-journey-checklist.md) is preparation
only; observations and owner decisions remain pending.

## Starting boundary

The parent agent owns the current continuation review and its merge. The
handoff starts only from a recorded merged commit containing the continuation
changes that are currently local: the feature-enabled Windows checks, the
`core:event:allow-listen` and `core:event:allow-unlisten` capability correction,
the desktop boundary regression, and the deterministic watcher lifecycle
harness. A green default build alone is not a valid starting signal; record the
feature-on CI run for the merged commit.

The relevant architecture is:

- `NativeFoundation` creates one shared `Arc<Mutex<Database>>`. Root commands
  mutate durable configuration through `ScanRootsService`; the scan console
  owns the feature-gated `ScanConsoleHost`.
- `ScanConsoleHost` starts the worker/session and currently runs a detached
  poll loop for `claim -> execute`; filesystem traversal does not hold the
  database mutex, while staging and publication use short durable
  transactions.
- `fruitboard-filesystem-watcher` exposes `HandleBoundWatcher`, `WatcherPort`,
  opaque `RootId`/generation values, coalesced hints, and typed end outcomes.
  It has no storage or desktop lifecycle owner.
- `fruitboard-scan-execution` exposes `RootIdMapping` and
  `WatcherFollowUpAdapter`. The adapter fences generations and converts an
  activity or coverage-loss hint into one durable, full-root reconciliation
  follow-up.
- `foundation/watcher_lifecycle.rs` composes the real coalescer, follow-up
  adapter, and durable database for deterministic tests. It deliberately does
  not start native watch handles or prove supervisor shutdown behavior.

The host integration must therefore connect existing seams rather than create
a second queue or a second database owner. Keep the implementation behind
`scan-console`; the feature-off path remains a recoverable `unavailable`
surface.

## Parallel assignments

Run the first three assignments in parallel after the starting boundary is
recorded. The named model is a recommendation for the existing OpenCode
agents; only GPT 5.6 Luna xhigh/max and Muse Spark 1.3 xhigh are permitted for
this wave.

### A — native watcher supervisor and host lifecycle

**Owner:** GPT 5.6 Luna max. This is the complex supervisor assignment.

**Owned files and seams:**

- `apps/desktop/src-tauri/src/foundation/scan_console_host.rs`;
- a new focused host module under
  `apps/desktop/src-tauri/src/foundation/` if that keeps lifecycle state out
  of command serialization;
- feature-gated host tests, including the existing
  `apps/desktop/src-tauri/src/foundation/watcher_lifecycle.rs` or a new
  adjacent integration test;
- narrow setup/root-command hooks in `scan_console.rs` or `lib.rs` only when
  required to install, reconfigure, or shut down the host.

Use `HandleBoundWatcher::start`, `stop`, `poll_hints`, and `outcome` as the
native boundary; use `WatcherFollowUpAdapter::watch_started`, `watch_ended`,
and `process_hints` for the durable boundary. The supervisor owns the mapping
from configured storage roots to opaque watcher IDs, monotonic generations,
watch restart/disable/remove handling, bounded reconnect backoff, pending-hint
retention, and explicit thread/handle shutdown and join. Root paths may remain
inside the native host only; no path crosses the watcher/follow-up seam.

The implementation exit condition is a feature-gated host that starts watches
for enabled configured roots, drops stale generations, turns activity and
coverage loss into bounded durable follow-ups, and handles disable, remove,
restart, and shutdown deterministically. A transient database delivery error
must retain one pending coalesced hint until delivery succeeds. Coverage loss
must preserve overflow precedence and request a full reconciliation. A watch
end must be observable and retried with bounded backoff, with old handles
joined before they are released.

Add deterministic coverage for startup and initial root mapping, burst
coalescing, overflow, delivery failure and retry, watch end and restart,
disable, remove/re-add with a fresh storage identity, stale-generation replay,
and clean shutdown. A real Windows-handle test may complement this coverage,
but ignored, unavailable, DriveFS, network-share, and non-NTFS fixtures remain
unverified.

Do not change storage migrations, public watcher contracts, client contracts,
or benchmark budgets in this assignment. If an existing crate API is
insufficient, stop at a small typed seam proposal and report it for a separate
review rather than broadening this slice.

### B — independent lifecycle failure verification

**Owner:** GPT 5.6 Luna xhigh, with Muse Spark 1.3 xhigh acceptable for the
bounded check-only follow-up.

**Owned files:**

- `.github/workflows/foundation.yml`;
- `apps/desktop/src-tauri/capabilities/main.json` only for the scoped event
  subscription permissions;
- a new `foundation/watcher_supervisor_tests.rs`, after agreeing its seam with A;
- `tests/desktop-boundary.test.mjs` and closely related CI policy assertions.

The starting continuation already supplies the CI and capability changes.
Do not duplicate that implementation. First agree on a test port with A, then
own a new adjacent `watcher_supervisor_tests.rs` module. A owns production
code and its smoke tests; B owns independent fault scenarios for failed hint
delivery, delayed configuration, ended watches, bounded wakeups, stale hints,
and shutdown. Use deterministic barriers/fake clocks instead of timing races.

Verify the Windows foundation commands
`cargo test -p fruitboard-desktop --features scan-console --locked` and
`cargo clippy -p fruitboard-desktop --features scan-console --all-targets
--locked -- -D warnings`, while retaining `pnpm.cmd check` and its default
feature-off/release gates. Verify that the capability grants only
`core:event:allow-listen` and `core:event:allow-unlisten` for the native
`scan-status-changed` subscription; it must not grant event emit, filesystem,
shell, SQL, process, or opener permissions.

The lane exits only with a visible green feature-on result for the exact
merged commit and passing boundary tests. Unit tests and Clippy establish
contract/build evidence; they do not establish a packaged or installed
desktop journey and do not promote P2-08, P2-09, or P2-12.

Do not edit watcher host behavior or client serialization in this lane. If the
workflow is already correct in the starting commit, record the run and leave
the files unchanged.

### C — installed-app journey preparation and evidence documentation

**Owner:** GPT 5.6 Luna xhigh. Muse Spark 1.3 xhigh through the existing
OpenCode setup is an allowed alternative.

**Owned files:**

- `README.md`;
- `docs/PHASE_2_EXECUTION_PLAN.md` current-status and sequencing sections;
- this handoff file only for follow-up corrections discovered during review.

Prepare a new installed-app journey checklist under this review directory:
exact build/feature commands, disposable fixture and database setup, expected
observations for each journey step, and an empty evidence table. Preparation
can run alongside A/B; actual installed-app qualification waits for their
combined green commit. Do not fill observation rows with expected results.

The documentation owner updates status from merged commit IDs and CI URLs
provided by A/B. Keep hidden worker evidence, fake-adapter rendering, native
typed tests, deterministic lifecycle tests, installed-app observations, and
owner acceptance as separate evidence classes. Link the existing benchmark
report and decision brief without rewriting provisional targets as approved
budgets. The parent agent owns any final integration-status addendum to
`continuation-2026-09-08.md` so that its evidence boundary stays consistent
with the merge record.

The lane exits when documentation lint, link checks, and policy tests pass and
every status claim names the corresponding merged commit or says pending.
It must not mark P2-03 through P2-12 complete based on local tests or a green
feature-on job alone.

## Failure invariants for every assignment

These are review blockers, not optional test ideas:

1. A watcher hint is a scheduling observation. Neither activity nor overflow,
   root loss, a dropped event, or an unavailable watch may mark a file missing.
   Only an authoritative full reconciliation can publish presence or absence.
2. At most one running scan and one queued follow-up may exist per root. Bursts
   coalesce, and coverage loss is retained with precedence. Delivery failures
   cannot silently discard the pending follow-up.
3. Every watch generation is strictly newer after restart. Hints from an ended,
   removed, disabled, or superseded generation are dropped. Re-adding a root
   receives a fresh durable root identity and cannot receive old work.
4. Disable/remove invalidates queued and running work through durable storage;
   configuration acknowledgement must not wait on the supervisor while a
   database lock is held. A durable mutation must not be reported as failed
   merely because host reconfiguration is delayed or needs a retry.
5. Stop is idempotent and joins the watcher worker before its OS handle or stop
   event is closed. Shutdown cannot leave a detached watcher capable of
   enqueueing work after removal.
6. Staging and publication remain lease/generation/cancellation fenced and
   atomic. Partial, denied, cancelled, unavailable, resource-limited, or
   uncertain traversal retains the previous committed Library view.
7. No public watcher or follow-up payload, event, diagnostic, or log contains
   an absolute path, private suffix, file content, parser result, or hydration
   request. Root paths are management display data only.
8. Feature-off builds keep the scanner hidden and return the typed
   `unavailable` behavior. A default-build or fake-adapter pass cannot qualify
   the enabled native path.

## Dependencies and integration order

1. **Parent baseline.** The parent records the continuation merge commit,
   confirms that the local event-permission and feature-on workflow changes are
   included, and points this handoff at that commit. Unpublished local output
   remains labeled local until then.
2. **Parallel A/B/C.** Use separate worktrees and branches from the recorded
   baseline. Agree A/B test interfaces before concurrent edits; one agent owns
   each file. Share the pinned `.tools` toolchains and serialize Cargo builds
   on a shared target directory to avoid lock contention and cache churn.
    A develops the supervisor against the existing watcher
   and follow-up APIs. B validates the enabled build and boundary contract in
   parallel. C prepares only status text whose factual inputs are already
   merged; it does not invent a CI result or acceptance decision.
3. **Host merge and feature-on rerun.** Review A's lifecycle tests and merge
   the host slice. Re-run B's feature-on tests and warnings-denied Clippy on
   the combined commit, plus `pnpm.cmd check`, repository policy tests, and
   changed-file/documentation checks. This combined run is the CI gate for the
   next step.
4. **Status merge.** After the combined green run, merge C's status update and
   have the parent add the matching continuation evidence paragraph. Preserve
   the distinction between remote `main`, the merged wave commit, and any
   unmerged local work.
5. **Installed-app gate.** On Windows, the owner or explicitly assigned
   operator exercises the pinned installed Tauri application at the combined
   commit. Record the commit, toolchain/build, fixture seed and manifest hash,
   disposable database handling, and results for root selection/cancellation,
   enabled settings, queued/running status, Library paging, restart survival,
   add/rename/modify/remove/restore convergence, cancellation, unavailable-root
   retention, and retry. Screenshots from a fake adapter or a hidden worker
   transcript do not satisfy this gate.
6. **Benchmark and owner decisions.** Resolve F1 (10,005 observations versus
   the 10,000 staging quota), F2 (warm p95 32.876 s versus the provisional
   10 s target), and F3 (the 100,000-entry memory set is unmeasured) using the
   [budget decision brief](budget-decision-brief.md). Only the owner may raise
   a quota, redefine a fixture, amend a budget, or record an explicit scope
   exclusion for DriveFS (#47), cross-volume/FAT32 identity (#48), network
   shares, ACL revocation, or real overflow timing.
7. **Acceptance review.** The owner acceptance record covers the host,
   feature-on CI, installed-app journey, F1/F2/F3 decisions, #47/#48 scope,
   and each remaining Partial/Pending criterion. Production scanning, parser
   work, and Phase 3 remain gated until that review is complete.

## Validation command matrix

| Owner | Required validation before handoff | Evidence label |
| --- | --- | --- |
| A | `cargo test -p fruitboard-desktop --features scan-console --locked`; enabled Clippy with `--all-targets --locked -- -D warnings`; watcher and scan-execution tests; Windows lifecycle/handle results where available | Host integration; does not prove installed app |
| B | `pnpm.cmd check`; feature-on desktop test and Clippy commands; `node --test tests/*.test.mjs`; Actionlint for workflow changes | Merged CI/build contract |
| C | `pnpm lint:docs`; `git diff --check`; changed-file formatting and link/policy checks | Documentation/status only |
| Owner/operator | Pinned installed-app Windows journey with disposable fixture/database record | Installed-app evidence |
| Owner | Budget decision brief updates and explicit platform scope decisions | Acceptance/decision record |

No ignored test, unavailable environment, local-only run, screenshot, or
benchmark sample may be promoted to a passing qualification without the
corresponding owner review.

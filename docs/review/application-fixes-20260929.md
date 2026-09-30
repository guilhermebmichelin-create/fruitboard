# Application review fixes — 2026-09-29

This note maps the six application-review findings to their fixes and
regression coverage across [parser PR #224](https://github.com/guilhermebmichelin-create/fruitboard/pull/224)
and the companion scanner changes. It describes the combined fix set; the
parser correction is delivered independently. The
[Phase 2 integration contract](../PHASE_2_INTEGRATION_CONTRACT.md) retains its
original acceptance criteria and now has a short implementation addendum.

## Findings and changes

### F1 — channel ID was confused with channel type ([issue #218](https://github.com/guilhermebmichelin-create/fruitboard/issues/218))

Event 64 supplies a channel ID; Event 21 supplies its type. The parser now
tracks channel identity separately and uses only validated type evidence in
the matching channel context. Sparse and reordered IDs, multiple samplers, a
synth before a sampler, missing/duplicate/invalid type events, and stored-name
preservation are covered by [parser regressions](https://github.com/guilhermebmichelin-create/fruitboard/blob/49b3231db60d1631e3fe6abe43d11cf4c01a1478/crates/flp-parser/tests/generators.rs)
and the bounded [research spike](https://github.com/guilhermebmichelin-create/fruitboard/blob/49b3231db60d1631e3fe6abe43d11cf4c01a1478/research/parser-spike-138/src/main.rs).
Sampler-default naming remains limited to existing supported-build/class
evidence. Approved FLP fixture bytes and compatibility claims are unchanged.

### F2 — worker-created finalized stages held old runs past retention ([issue #219](https://github.com/guilhermebmichelin-create/fruitboard/issues/219))

The existing terminal policy still requires a run to be older than 30 days and
outside the newest 200 terminal runs for its root. When such a run is pruned,
retention can remove its empty published/discarded stage header in the same
transaction, but only after staged observations are gone and no publication
marker or file-history row refers to the run. Active/open stages, current
successful publication references, locations, and file history remain
protected. The regression
[`retention_prunes_real_worker_finalized_stages_only_past_age_and_count`](../../crates/scan-execution/src/tests.rs)
exercises actual worker-created stages; storage also tests rollback if the
guarded run deletion fails in [`storage tests`](../../crates/storage-sqlite/src/tests.rs).

### F3 — stopped worker health looked like enabled scanning ([issue #220](https://github.com/guilhermebmichelin-create/fruitboard/issues/220))

Native state now reports compile-time `enabled` separately from process-level
`runtimeAvailable`. After the worker stops, the Library and scan status remain
readable, but scan, retry, and cancel actions are disabled or rejected; the UI
explains that restarting Fruitboard is required. The worker is not restarted
automatically. Coverage includes the stopped-host health and UI action states
in [native scan-console tests](../../apps/desktop/src-tauri/src/foundation/scan_console/tests.rs),
[host tests](../../apps/desktop/src-tauri/src/foundation/scan_console_host_tests.rs),
and [Library page tests](../../apps/client/src/library/LibraryPage.test.tsx).

### F4 — native response relationships were not validated ([issue #221](https://github.com/guilhermebmichelin-create/fruitboard/issues/221))

The client rejects Library records whose `rootId` differs from the enclosing
page, malformed calendar timestamps, and job/run ID combinations that do not
match the job state. Queued jobs have no run yet; terminal jobs can legitimately
have no retained run, for example after cancellation before lease or bounded
terminal cleanup. Those valid lifecycle states remain accepted. Decoder and
page regressions are in [native adapter tests](../../apps/client/src/library/native.test.ts)
and [Library page tests](../../apps/client/src/library/LibraryPage.test.tsx).

### F5 — event-listener failures could leave stale Library and scan status ([issue #222](https://github.com/guilhermebmichelin-create/fruitboard/issues/222))

The client exposes event attachment health, rereads snapshots when attachment
settles, coalesces notifications, and reconciles on window focus or visibility.
For active work, it refreshes status every five seconds while the listener is
unavailable. Publication also refreshes the Library page. This is recovery
behavior for attachment failures; no installed-app failure-rate or
performance qualification is claimed. Regressions are in
[Library page tests](../../apps/client/src/library/LibraryPage.test.tsx).

### F6 — publication loaded historical paths and identities too broadly ([issue #223](https://github.com/guilhermebmichelin-create/fruitboard/issues/223))

Publication now uses temporary lookup tables bounded by staged observations.
Exact-path continuity loads only observed paths, retaining the exact-path
evidence needed to restore a missing row. A separate indexed query uses only
identities seen in this complete scan and present rows from the same root; it
keeps at most two project IDs per identity to distinguish a unique match from
a conflict. Planning and all committed updates remain atomic. Hardlink aliases
remain separate locations, ambiguity stays conservative, and file-history
restoration is retained. [Migration 009](../../crates/storage-sqlite/migrations/009_root_identity_lookup.sql)
adds the partial root-local live-identity index.

Migration 009 is an additive index, but it advances the database schema to
version 9. Older builds refuse a newer-schema database, so the existing
pre-upgrade backup and recovery safeguards apply to upgrade and rollback
planning. The indexed count and mark-unseen paths may still scale with stored
location history. No performance qualification is claimed. Publication and
migration regressions are in [storage tests](../../crates/storage-sqlite/src/tests.rs).

## Validation boundary

The source gates are the pinned Windows `pnpm.cmd check`, feature-enabled
desktop tests and warning-denied Clippy, storage/worker migration and
publication tests, diagnostics checks, and separate research-parser checks.
The PR descriptions record their final source-head results and CI links;
baseline CI covers only unchanged source.

The browser evidence below uses the actual client and a synthetic native
transport. At desktop (1440 by 1000) and narrow (430 by 900) sizes it checks
listener failure, stopped controls, preserved saved results, and publication
reconciliation on focus. Both layouts have no horizontal overflow, and axe
reports no WCAG violations in these four captured states.

| State | Desktop | Narrow |
| --- | --- | --- |
| Listener unavailable | [Screenshot](application-fixes-20260929/listener-unavailable-desktop.png) | [Screenshot](application-fixes-20260929/listener-unavailable-narrow.png) |
| Scanner stopped | [Screenshot](application-fixes-20260929/runtime-stopped-desktop.png) | [Screenshot](application-fixes-20260929/runtime-stopped-narrow.png) |

This review adds no installed-app or performance qualification and does not
enable production scanning.

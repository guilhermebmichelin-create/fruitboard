# Explicit saved sample checks

Status: ADR-007 slice 3 implemented for owner review; broader G5 stays open.
Issue [#284](https://github.com/guilhermebmichelin-create/fruitboard/issues/284).
Baseline: PR283 merge `b8c065c3f10b6b64d96b8224fadcdbb4e19af3d6`.
Final source/check receipts and installed binary provenance belong to this PR's
private handoff. This is opt-in development composition, not production activation,
performance qualification or a Phase 3 checkpoint.

## Behavior and authority

Opening/refreshing details reads saved results. Only **Check saved samples**
starts a bounded metadata check inside the selected project's local NTFS root.
Existing saved reference text stays visible. Ordered observations distinguish
**Present when checked**, **Not found at saved path when checked**, fixed
**Not checked** reasons and no saved reference. A timestamp explains that these
are observations, not playable-audio or full FL Studio search-resolution claims.
Relative paths, placeholders, other folders and unsupported paths stay unchecked.
There is no open/play/link action.

Cancel clears the report, fences late replies and restores keyboard focus to
Check. Check-again uses a fresh request ID. Navigation/new scan snapshots,
row fingerprints, details snapshots and adapter changes clear ephemeral results
and cancel pending work. Close/restart retains no report. Old snapshots ask for
analysis; empty references do not start a worker or become missing claims.

The [command](../../../apps/desktop/src-tauri/src/foundation/sample_check.rs)
accepts a strict 4 KiB version/identity/fingerprint request, never paths or sample
strings. Only two named local main-window permissions are added. Default builds
return disabled; PWA/older adapters have no functional probe. No general file,
audio or process capability is added.

The [host](../../../apps/desktop/src-tauri/src/foundation/sample_check/host.rs)
validates the complete current saved projection. The additive
[sealed source read](../../../crates/storage-sqlite/src/metadata/source.rs)
captures every root/file/location/publication revision, native locator and
qualified source identity. A lightweight current-snapshot query preserves the
original snapshot-origin fences without repeatedly loading JSON. There is no
migration, snapshot rewrite or result table.

Storage holds a configured folder grant/revision, not a root directory ID.
Native-first root capture qualifies the actual local NTFS root/source and pins
its ancestry; final root/name observations must match that captured held ID.
Earlier independently captured root IDs, when supplied, must also match.
This changes no operation/handle budget and accepts no renderer identity.
Source ID, size, high-resolution time and binding must match the stored scan.

The DB guard is released before filesystem operations. Durable authorization is
checked at native boundaries and by a separate waiter during blocked work.
Short concurrent DB reads are retried within the original deadline, not turned
into absence. The reviewed attributes-only exact-child no-follow/no-recall
adapter and all bounds remain intact. Final native and transport checks discard
results on root/source/snapshot/session changes. Metadata-preserving content
edits remain undetectable; content is not hashed or reread by this feature.

One global gate covers the worker and pending delivery, with no queue. Cancel or
the two-second monotonic deadline can finish the UI response while an old call
remains blocked; admission stays busy until it retires. A separate delivery flag
also rejects cancellation after worker publication but before response return.
Shutdown closes admission. This is cooperative cancellation, not forced kernel
interruption.

## Validation and limits

[Host tests](../../../apps/desktop/src-tauri/src/foundation/sample_check/host/tests.rs)
cover actual Windows-port present/absent/no-reference results, unchanged source/
target/database bytes, stale identities/fingerprints, source saves before watcher
publication, superseding snapshots, released DB guard, root/runtime revocation,
blocked cancel/deadline, busy admission and post-publication cancellation.
Blocked operations are injected; actual Windows/cloud blocking is not qualified.
Existing native rights/reparse/replacement/case tests remain required, with a
new native-first root capture/pin regression.

Client tests reject wrong contexts/count/order/status/timestamps, path/error
extensions and request mismatches. They cover duplicate slots, 256 references,
transport privacy, cancel/navigation/fingerprint races, keyboard focus and axe
states. The list still reveals 20 channels at a time.

Installed review uses a fresh, separately named unsigned development package and
profile with private metadata-only sentinels. Unchanged approved FLP copies and
constructed stored expectations prove host behavior, not new FL Studio path
mapping. Rendered desktop/narrow/200%-text states and installed check/cancel/
stale/reinstall evidence are recorded in the final PR. Delayed transport UI
cancellation is distinguished from injected native blocked-worker tests.

Required gates: pinned Windows `pnpm check`, changed `analysis-jobs` tests and
warning-denied all-target Clippy, sample-presence native/portable and storage
fence tests, unchanged approved corpus and normal final-head CI. No new public
FLP or owner profile access is part of this work.

## Resource ownership and rollback

Reuse the existing `bounded-probes` validation cache, sole owner Codex `/root`,
with its absolute path in the private handoff, explicit `CARGO_TARGET_DIR` and
one heavyweight window.
Preflight charges 6 GiB and preserves at least 30 GiB free on every used volume.
C is used; G is not. Installed tests hold the existing Foundation Smoke lock.
Keep reports, fixture/database hashes and independently copied binaries with
source/build provenance outside the cache. The private handoff records final
capacity, ownership and evidence. No new permanent cache or broad cleanup is
authorized; current compiler outputs remain reusable.

Rollback removes host/commands/view state and its optional dependency edge.
Keep immutable project details and saved reference display. No migration or
metadata deletion is needed. Preserve PR283's independent existing-only scanner
fix. Broader relative/placeholder/external-folder/cloud resolution, resource
acceptance, compatibility, release and Phase 3 gates stay open.

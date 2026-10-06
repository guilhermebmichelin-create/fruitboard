# Explicit saved sample checks

Status: ADR-007 slice 3 merged as PR285 on 2026-10-06,
`d19587f52132f31065bbaab97a572d3c5487cf00`; broader G5 stays open.
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

Installed review on 2026-10-06 used a fresh, separately named unsigned development
package/profile with private metadata-only sentinels. Package source:
`bd8f8adbd3e6b3d83bcada5d5c9817c583012042`, tree
`4f7df2452c57b2aaa500b233c9c897d81f443ad8`; later feature changes add only review
documentation/screenshots. The separate [security prerequisite](../security/source-map-js-286.md)
updates transitive CSS/source-map tooling; final check receipts include native/client
source equality and patched-build web-output comparison with this installed package.
Native Library check, repeat check, strict request
rejection, stale snapshot/root revocation, keyboard cancellation/focus, late reply
discard and restart were verified through the installed WebView2/native adapter.
The cancelled UI test held a completed native response at the renderer's fetch
boundary; it did not block a Windows syscall. Native blocked-worker tests above
cover that separate lifetime requirement.

Unchanged approved F15 bytes with constructed stored references prove the host's
present/absent/no-reference mapping, not new FL Studio path mapping. Actual parser
reanalysis of F15 produces three empty-Sampler references, disables the action
and returns `no_references` without a worker. An unchanged approved F12 copy's
real saved external reference returns `not_checked / outside_root`; its screenshot
stays private. Source and sentinel hashes remain unchanged. Uninstall/reinstall
preserved the review DB; final uninstall removed only the review registration,
retained its DB/fixtures and left both protected owner profiles and all 13 approved
corpus files byte-identical. The exclusive host lock was released after cleanup.

The sample section had zero installed axe WCAG A/AA violations, including actual
contrast evaluation. Narrow 390px and doubled 32px text had no horizontal page
overflow. Component tests cover additional busy/stale/error/deadline states;
installed evidence does not claim cloud blocking or performance qualification.

Required gates: pinned Windows `pnpm check`, `scan-console` and `analysis-jobs` tests and
warning-denied all-target Clippy, sample-presence native/portable and storage
fence tests, unchanged approved corpus and normal final-head CI. No new public
FLP or owner profile mutation is part of this work.

## Installed screenshots

These are native review-package captures. Paths shown are owned synthetic test
locators, not personal projects. Only the no-reference screenshots use the actual
empty-Sampler parser projection; the present/absent screenshots use the explicitly
constructed host projection described above.

| Capture                                                  | State / scope                                                 |
| -------------------------------------------------------- | ------------------------------------------------------------- |
| [Desktop idle](samples-idle-desktop.png)                 | Details display alone starts no check.                        |
| [Desktop results](samples-results-desktop.png)           | Present, absent at the exact path, and no saved reference.    |
| [Checking](samples-checking-desktop.png)                 | Injected renderer response delay; Cancel remains available.   |
| [Cancelled](samples-cancelled-desktop.png)               | No report shown; keyboard focus returns to Check.             |
| [Narrow results](samples-narrow-viewport.png)            | 390px viewport; long saved paths wrap.                        |
| [No references](samples-no-references-desktop.png)       | Actual approved empty-Sampler analysis; disabled action.      |
| [Narrow no references](samples-no-references-narrow.png) | 390px viewport with the section heading and action.           |
| [Doubled text](samples-text-200-viewport.png)            | 32px text; actual no-reference state, no horizontal overflow. |

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

# Library channels and instruments

Date: 2026-10-01. Related issue: #242. Baseline: merged PR #241
(`fdcc9ee2949cda88c91f5345bb8b6bfc542c7024`).

## Delivered behavior

Saved project details now shows channel labels and separately verified built-in
instruments. The channel positions describe parser order, not FL's saved numeric
IDs. Each value preserves saved/inferred/missing/unsupported provenance and
explains inference confidence. Unknown instruments do not discard verified
siblings. Renamed labels are not used as plugin identity. Empty labels and
zero-channel results are explicit; control/bidi characters are visible escapes.
Lists render 20 entries at a time, with keyboard controls and preserved focus.

## Native and persistence boundary

The existing generator-name parser extension is now validated by the full
metadata validator when the selected descriptor advertises it. Exact build
26.1.0.5530 supports extracted `3x Osc` and inferred built-in `Sampler`. Count,
index/order, item/status/aggregate value, method/confidence and reason checks
reject contradictory output. Classes/builds/layouts outside the existing matrix
stay unsupported. Typed data retains no arbitrary class names or extensions.

The immutable private version-1 projection gains one additive field; old results
remain readable with unsupported instrument copy. No schema migration, parser
decoding, scheduling, dependency or capability change. Future analyses may save
the new information; reading/refreshing never reparses or updates history.

The current-source/root/fingerprint guard and Library scan snapshot/late-reply
fences from #241 apply to the complete channel view. Native and client validate
channel counts, ordering, names, allowed instruments, statuses and bounds.
Returned names are local display text; raw JSON/sample paths/diagnostics are not
exposed, logged or synced. Instrument metadata grants no file/plugin/process access.

## Rendered evidence

Screenshots use the actual client and response validator with an explicit
synthetic native transport. They prove browser layout/states, not installed
application or broader parser compatibility qualification.
Component crops suppress shell navigation/skip-link overlays so the whole
channel section is visible. Actual viewport and accessibility checks use
unmodified styles at 1280 × 1800 and 390 × 844.

- [Desktop channels](channels-desktop.png)
- [Narrow channels](channels-narrow.png)
- [Older saved result](older-narrow.png)
- [Partial label/instrument coverage](partial-narrow.png)
- [No channels](empty-narrow.png)
- [Long Unicode label](long-label-narrow.png)

## Validation and limits

Required: full pinned Windows `pnpm check`, enabled `analysis-jobs` native tests
and all-target warning-denied Clippy, parser/storage/client regressions,
unchanged approved FLP hashes, desktop/narrow keyboard/accessibility/200% text
review, and final-head CI. Actual F14 parser bytes travel through validation,
storage and the authorized native details command; malformed generator claims
cannot replace a current valid snapshot. Existing packaging/worker behavior is
unchanged; prior installed evidence applies to that implementation only. A copied
old installed snapshot probes backwards-readable display separately.

Local full pinned `pnpm check` passes with 194 client tests, workspace
lint/type/tests and the desktop release build. Enabled native tests pass
(123 tests; one explicit copied-data probe ignored in the ordinary suite), as
does warning-denied all-target feature Clippy. The ignored probe was separately
run on an independent copy of older installed metadata; its 262,144-byte database
hash was unchanged. The F14 fixture follows the real parser/validation/storage
and authorized read boundary. Browser checks pass for normal/older/partial/empty
and long-label states, all 256 channels via keyboard batches, preserved focus,
reduced motion and a maximum 4,095-unit Unicode label at 200% text, including
rendered contrast/accessibility. Final-head CI is required before merge.

Metadata remains development-only. This is limited built-in channel coverage,
not Plugin Explorer, sample resolution, mixer/nested/VST or installed plugin
inventory. Older snapshots have no retroactive instrument data. Compatibility,
distribution and performance gates remain open; no installed-window or
performance qualification is claimed. Owner reviews and merges manually.

The clean source worktree and single reusable validation cache were reused.
Coordinator: Codex `/root`; the absolute cache path is recorded in the private handoff.
One sequential heavy build window, explicit `CARGO_TARGET_DIR`, 4 GiB output
budget and at least 30 GiB free reserve. Private receipts/databases and provenance
stay outside caches in `fruitboard-review-evidence/library-channels-20261001`.
Prior evidence, fixtures and primary cache remain protected. No cleanup selected;
reusable outputs remain under the coordinator and independent evidence is retained.

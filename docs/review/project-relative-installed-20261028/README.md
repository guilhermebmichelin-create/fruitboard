# Installed project-relative candidate checks, 2026-10-28

**Result: the feature-enabled installed correctness and accessibility checks
passed.** This completes PR348's installed inspection, not resource qualification,
FL Studio resolver compatibility, G5 completion or the Phase 3 exit.

## Exact source and constructed evidence

The unsigned review package and retained native test were built at
`5ff89546251fe92171b0fa306c173074e5ceff75` with `analysis-jobs`, pinned Node
24.20.0, pnpm 11.25.0 and Rust 1.98.1. The package/test executables were copied
independently outside the writable compiler cache; hashes and source/tree
provenance were frozen before installation. Installed executable identity was
recorded separately from the post-Tauri executable. See [results.json](results.json).

The native test copies an existing approved FLP unchanged, then publishes
constructed relative references into a fresh, dedicated review database. This
qualifies Fruitboard's candidate policy and UI only. It does not assert that the
references came from FL Studio or describe FL Studio's resolver.

The review corrected two fixture fields: its nested locator now includes the
parent component, and its display path uses the Windows scanner's backslash
spelling. Previously, startup reconciliation correctly treated the fixture as
changed and replaced its constructed metadata. No production freshness fence,
parser grammar, path authority or sample-check limit was relaxed.

## Verified behavior

- Opening details starts no sample check. The UI explains the project-folder
  candidate interpretation before the explicit action and retains literal text.
- The explicit check reports present, not found at the checked candidate, and
  outside-root refusal for the three constructed references. The wrong-root
  sentinel prevents a selected-root implementation from passing accidentally.
- Requests contain only the seven approved fields; results contain no resolved
  path or reference text. No open/play action or audio read is introduced.
- Keyboard activation and cancellation restore focus. Holding a genuine completed
  native reply at renderer JSON consumption proves that late delivery cannot
  republish a cancelled report. This does not claim kernel cancellation.
- The installed view fits 390 CSS pixels and a 32 px default text size without
  horizontal overflow. Desktop and post-restart axe checks each report zero
  violations and zero incomplete rules. Nine screenshots remain privately retained.
- Restart clears the ephemeral report while retaining saved references. A stale
  snapshot and a disabled root return no sample report.
- Both app launches exit with one graceful close each, exit code zero and the
  unchanged ten-second close deadline. The owned product is uninstalled, its
  registration removed, and the matching host lease released.
- All 21 approved corpus files and 671 protected profile files remain unchanged.
  Constructed source/target/sentinel bytes are unchanged; the closed review
  database is independently retained and preserved by uninstall.

## Native checks and historical harness failures

The corrected source passes all 169 native analysis-jobs tests, with four
existing ignored tests, and warning-denied Clippy. Sidecar and NSIS package builds
pass. The installed native command test also passes against the exact retained
test executable. The normal updated-head CI remains required, including the full
pinned Windows gate, client suite, security audits and packaging smoke.

Ten earlier attempts remain retained rather than being counted as passes. They
exposed process-exit-code capture, route/reload and selector assumptions, the
fixture reconciliation errors above, CSP-sensitive audit injection and the
reply-delay probe. Harness corrections preserve the app's CSP, native adapter,
ownership checks, original data and single-close contract. All partial attempts
were inspected and archived; the first three required verified operator cleanup
before releasing their own host lease. Later attempts have complete cleanup
receipts. The successful run is a separate, fresh installation and profile.

## Current merge boundary and storage

PR350's security prerequisite is merged. Merging that main ancestry into this
feature changed no files relative to the installed source; this review adds only
documentation. The installed package is reused only for the identical compiled
graph. Documentation-head CI still runs independently; green CI does not prove
performance qualification.

This thread owns the existing reusable validation cache, with its write window
closed. The primary development cache is preserved. Fresh four-path preflights
passed before builds and installation; the latest observed C: reserve is above
31 GiB, and G: was not used. Compiler aliases were verified internal to the cache
and replaced with hash-verified independent copies. No compiler intermediates,
profiles, databases, fixtures or earlier evidence were deleted. Private raw logs,
images, manifests, binaries and review databases stay outside Git and disposable
compiler caches. Reuse that cache sequentially after a new preflight.

# Immutable validated parser metadata storage

Date: 2026-09-30. Related issue: #236. Baseline: merged PR #235
(`097920b3f09cb0afa4cea72df2a4dec829fbfdc1`). Compiled implementation:
`057ea3d988d116c53b8b0e800e3a6a047487fa69`; this summary is documentation only.

## Delivered boundary

Migration 010 adds immutable completed parser snapshots, a current pointer,
and monotonic source/publication revisions. The optional native storage
`parser-metadata` feature captures database-owned input fences, validates the
selected parser reply, and atomically inserts the result and advances the
pointer. A late result cannot overwrite a newer publication or survive an
observed source change, even if the old values are subsequently restored.

Current reads hide stale snapshots; history remains after missing files,
disabled roots, and root removal. History pagination is indexed and bounded
to 100 headers per page. Individual payloads have a 256 KiB byte limit.
Complete/partial results contain an explicit allowlisted private projection;
unsupported/failed/rejected results retain their distinct safe outcomes.
Availability, reasons, inference method, confidence, assumptions, and selected
adapter/protocol/schema provenance are preserved. Raw extensions and arbitrary
diagnostics are discarded. Native read objects have no automatic Debug or
Serialize implementation.

The default desktop does not enable metadata publication. No analysis queue,
FLP file I/O in storage, renderer command, user-facing metadata view, fixture,
dependency pin, signing, or release change is included. The persisted JSON is
not a new validated parser capability. The future worker must freshly observe
an authorized file handle; a database observation and parser-claimed digest
alone cannot establish filesystem freshness.

## Validation

- `cargo test -p fruitboard-storage --features parser-metadata --locked`:
  111 tests passed. One subprocess helper is ignored in ordinary discovery
  and explicitly invoked by the passing interruption test.
- All-target, warning-denied Clippy passed with `parser-metadata` enabled.
- Full pinned Windows `pnpm.cmd check` passed, including the default storage
  and parser tests, repository/client gates, and production desktop build.
- Focused tests cover schema-9 upgrade with verified pre-upgrade backup,
  preservation of roots/locations/Library preference, backup recovery with
  snapshots, invalid/oversized replies, immutable rows, source change and
  restoration, root disable/removal, foreign locations, publication races,
  backwards wall clock, no-op observations, revision overflow, indexed cursor
  pagination, SQL rollback, and a real child process killed before commit.
- Installed noninteractive Windows development smoke passed under its
  exclusive host lock: real parser reuse/rejection, forced-crash recovery,
  shutdown/restart, missing-binary containment/restoration, and reinstall.
  Both uninstalls preserved the 237,568-byte synthetic database byte-for-byte.
  Read-only inspection confirmed schema/ledger version 10, Library preference,
  and zero metadata snapshots, as expected while publication is disabled.
  This is fresh installed startup/reinstall evidence; schema-9 upgrade is
  covered by the storage tests.
- All 13 tracked corpus files match the merged baseline byte-for-byte.
- CI adds optional storage-feature tests/Clippy on Linux and Windows and
  focused metadata tests to the migration gate. All required checks must pass
  on the final submitted head before owner review/merge.

## Evidence and cache handoff

Raw logs/reports, synthetic data, copied installer, independent binaries and
their source/build hashes remain in the external retained
`parser-metadata-storage-20260930` evidence directory. Its local handoff
records absolute paths, cache ownership, source equivalence, storage capacity,
corpus hashes, CI receipts, and smoke archive/restoration responsibilities.
No private metadata, FLPs, databases, logs, or machine paths are committed here.

Only the existing reusable `bounded-probes` validation cache was written,
under the exclusive ownership of Codex `/root`. The primary development cache
was preserved. Preflight allowed 4 GiB additional output above the 30 GiB
reserve. Cargo-created internal aliases were path/hash inventoried and replaced
with independent byte copies while compiler/app processes were quiescent.
No hardlink cloning or blanket cleanup occurred. Reusable intermediates stay
in this one cache; retained evidence must remain independent.

## Remaining work

After owner review/merge, the next slice is native analysis jobs: fresh file
authority, bounded scheduling/cancellation, parser lifecycle integration, and
publication through this storage boundary. Library metadata views follow that
integration. The supported saved-build set, broader GUI compatibility gates,
performance qualification, interactive/audio evidence, and public distribution
requirements remain unchanged.

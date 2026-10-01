# Native analysis jobs

Date: 2026-09-30. Related issue: #238. Baseline: merged PR #237
(`fc554b22096aed904a98a81ec8fcec325dfa0b2c`). Compiled implementation:
`e0928f577569275e25cf979e9c66dd6794beebd5`; this summary is documentation only.

## Delivered boundary

Migration 011 adds one mutable desired-input cell per location, capped at 128
pending cells, indexed discovery/claims and one globally running lease. The
native host rotates its process-session fence, recovers interrupted jobs and
retains the three-attempt budget. Changed source/parser revisions supersede
work; unchanged terminal inputs and cancellation survive restart without loops.
An alias's negative publication changes commit ordering, not the desired
source. Terminal work stays coalesced, while pending fence refresh preserves
its retry budget and delay.
The metadata snapshot table remains immutable completed history.

The new native worker uses the selected enabled local NTFS source from storage,
checks its opened-object identity, rejects reparses and offline/recall flags,
and independently checks size, nanosecond modification time and content digest
before/after parsing. Held Windows source and ancestor handles deny writes and
replacement through publication. Legacy/unqualified identities and Drive
virtual roots do not grant content-read authority.

The desktop's explicit `analysis-jobs` development feature composes one worker
with the fixed installed sibling parser and existing supervisor. Source and
parser work release the database mutex. Scoped cancellation observes durable
source/root/session/lease revocation and shutdown retires the owned process.
Health and descriptor validation precede each parse. Final publication rechecks
the lease and source/publication fences, validates against the independently
observed digest, and writes snapshot and terminal job in one transaction.
Negative attempts preserve a still-fresh last good snapshot. Alias reuse avoids
repeated parsing of the same physical project file.

The default desktop remains without automatic FLP content reads. No metadata
view, new renderer command/permission, sample/plugin loading, corpus addition,
dependency pin change, production signing or release is included. Raw paths,
references and private projection data remain native/local. See the
[worker documentation](../../../crates/analysis-execution/README.md) for limits.

## Validation

- Full pinned Windows `pnpm.cmd check` passed: format, lint, typecheck,
  repository/privacy/toolchain policies, 127 Node tests, 158 client tests,
  workspace Rust tests and the default production desktop build. This rerun
  follows the alias fix; the subsequent parser test-only isolation change
  separately passed all eight protocol tests and all-target Clippy. Final-head
  CI repeats the complete gate.
- Full `analysis-jobs` storage tests: 122 passed. Two ignored subprocess
  helpers are explicitly invoked by the passing interruption tests. The new
  forced-stop test terminates its owned child after snapshot and terminal-job
  writes but before commit; reopening sees neither partial publication nor a
  lost retry budget, and restart completes the recovered job.
  Additional regressions cover negative aliases/cancellation without a good
  snapshot and publication-fence changes preserving retry budgets/backoff.
- Worker portable tests: four passed. The separately invoked Windows native
  test passed with the freshly built parser at the implementation SHA. It
  copies approved corpus bytes, verifies held-object write/rename denial,
  executes the real parser, saves validated metadata and preserves source bytes.
- Desktop enabled-feature tests: 118 passed. Warning-denied all-target Clippy
  passed for enabled storage, worker and desktop code.
- Windows CI exposed a pre-existing parser-test temporary-folder collision
  when concurrent tests observed the same timestamp. A per-process sequence
  now isolates the folders; a forced equal-clock concurrent regression passes.
  Production parser behavior and external dependency pins are unchanged.
- Installed noninteractive Windows `-AnalysisJobs` development smoke passed
  under the exclusive host lock. Real native root admission/enumeration,
  durable queue, parser and publication produced one complete snapshot. Seed
  and reinstalled launches reported the same snapshot ID and payload/source
  digests. Both uninstalls preserved the synthetic database byte-for-byte.
  The approved source copy stayed byte-identical. Parser reuse, forced-crash
  recovery, shutdown/restart, missing-binary containment/restoration and inert
  lifecycle checks also passed; no owned parser remained after exit.
- Independent read-only database inspection confirmed schema/ledger 11,
  Library preference, one completed job on attempt 1, one complete current
  snapshot with valid source fences, matching source/payload digests, integrity
  and foreign keys. The retained synthetic database is 262,144 bytes.
- The baseline installed smoke retains its unchanged-database relaunch check.
  The opt-in analysis variant preserves immutable snapshot identity/digests
  across relaunch because native session records legitimately change; both
  variants retain byte checks at each uninstall. Packaging policy tests guard
  these boundaries. CI runs both installed variants sequentially.
- All 13 tracked corpus files match the merged baseline byte-for-byte. Final
  submitted-head CI must pass before owner review/merge; Linux storage/worker
  and migration coverage, Windows native feature/real parser tests and both
  installed variants are additive to the existing required gates.

## Evidence and cache handoff

Private raw reports, copied fixture/installer, independently copied binaries,
synthetic database and prior smoke archive remain outside Git and compiler
caches in the external `analysis-jobs-20260930` evidence directory. Its local
handoff records exact source/build/binary hashes, cache paths/ownership,
capacity, corpus equality, CI receipts and smoke restoration obligations.

The existing `bounded-probes` cache has one owner, Codex `/root`, with explicit
`CARGO_TARGET_DIR` and sequential heavy checks. Primary development source/cache
remain untouched. Disk preflight budgets 4 GiB extra above the 30 GiB reserve;
Cargo-created internal aliases are inventoried and converted to independent
byte copies only while compilers/apps are quiescent. No hardlink cloning,
blanket cleanup, new permanent cache or performance qualification occurred.
Reusable intermediates stay in the one cache. Retained evidence must remain.

## Remaining work

Owner review and merge precede the next slice: a typed local metadata
projection for Library, with clear extracted/inferred/unavailable states.
Explicit retry controls and production activation remain separate. A source
unavailable at execution becomes terminal until its desired input changes;
there is no user retry UI in this development slice. Broader compatibility,
resource/throughput qualification, interactive/audio evidence, signing and
distribution requirements remain open. This test is one approved fixture's
end-to-end development behavior, not a general compatibility or speed claim.

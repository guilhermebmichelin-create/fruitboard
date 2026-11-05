# Qualified ordinary-note pilot promotion

Issue: [#395](https://github.com/guilhermebmichelin-create/fruitboard/issues/395).
Source baseline: merged main `6cb560e` (PR383), plus the documentation-only
status refresh in PR394. Status: **Implementation with local parser/storage
checks; normal updated-head CI and installed note verification remain required.**

Current follow-up: the [installed review #397](../installed-notes-397/README.md)
verifies all fifteen note cases plus missing/build controls, restart and source
authority on its exact recorded source. It also fixes enlarged mobile text;
later heads retain normal CI/source-equivalence requirements. Earlier limitations
below record this slice's original local handoff rather than a current blocker.

## Behavior and evidence

Adapter 0.1.5 promotes the independently observed clean F31-F38 pilot cases
without changing the record layout or detail bounds. Alongside F24-F30, the
accepted numeric sets are saved keys 60/62/64/67, start ticks 0/24/48/96,
duration ticks 24/48/96 and stored velocity bytes 74/89/128. These are discrete
raw-value sets, not continuous ranges, MIDI mappings or velocity percentages.
Build 26.1.0.5530, 96 PPQ and the existing one/two-Sampler declaration boundary
remain unchanged. The [independent pilot report](../../research/ordinary-note-coverage-381-result.md)
and immutable public expectations supply ground truth.

N10/N11/N12 add observed start/duration/velocity values. N13/N14 add a three-note
melody and chord, preserving all saved ordinals, cardinality and the chord's
descending source key order 67/64/60. N16 retains its already-qualified note on
the renumbered channel; it supplies no sparse-ID support. Cross-products of the
observed raw fields use the same existing grammar; this is not independent
GUI evidence for every combination or a guarantee for arbitrary projects.

N09 remains refused because raw velocity 90 is unqualified. N15 remains refused
because one complete record has unexplained byte19; its supported sibling is
never emitted alone. Counts and other metadata survive both whole-pattern
refusals. Unknown neighboring values, flags, slides, steps, unknown channel
targets and other builds/PPQs remain unsupported.

The native validator and immutable readers reuse the parser's qualified-value
function. The strict client decoder now matches the promoted numeric sets.
No schema/JSON shape, capability ceiling, filesystem grant, migration,
automatic reanalysis, diagnostics, sync, playback or editing is added.
The adapter version distinguishes new parses; old immutable payloads retain
their saved states until explicit reanalysis. Older 0.1.4 readers may reject
projections containing the new numeric values. A binary downgrade is therefore
not automatically read-compatible: preserve the database/backups and use a
reviewed rollback that stops selecting this optional field, without rewriting
historical data.

## Verification

- Local pinned Rust parser suite passes, including the actual 36-file supervised
  exchange, six positive pilot cases and both whole-pattern negative controls.
  The final focused note suite passes after additional neighboring-value tests.
- Nine focused storage tests pass with `parser-metadata`, including all eight
  pilot files through publication, close/reopen, current reads, exact saved
  values, cardinality, negative states and unchanged source hashes.
- Parser and feature-enabled storage all-target Clippy pass with warnings denied;
  Rustfmt passes. The native details regression now covers all fifteen genuine
  note cases through publication/restart and root/source authority loss. That
  desktop regression requires normal Windows CI; it was not run locally.
- Fourteen focused decoder tests pass, including all pilot states and rejection
  of neighboring values. The local run uses existing Vitest 5.0.3 and a temporary
  Node-only configuration for the pure decoder test. No committed test setup or
  required check is changed. Full fresh-install client/native/Windows/packaging
  checks remain mandatory on the PR head.
- Full local client typecheck still reports the unchanged missing installed
  `@napi-rs/canvas` dependency in the existing test setup. This is a local
  dependency mismatch, not a passing typecheck; CI must use the committed locks.
- Changed-client ESLint, scoped formatting/Markdown checks, 26 corpus/privacy
  tests, the repository privacy scan and whitespace checks pass.

Existing renderer layout/keyboard/missing/older/limit coverage is unchanged;
only accepted numeric content changes. No new visual layout or installed pass
is claimed. All 36 approved FLP files remain byte-identical, and no new fixture
or musical-field GUI observation is generated.

## Build and handoff

Owner `/root` works in the primary checkout on the dedicated feature branch.
The owned source boundary is the parser grammar/version/tests, client numeric
decoder/tests, storage/details integration regressions and matching documents.
The lock change records only the local parser's version, not a dependency update.

The existing `bounded-probes` cache is reused sequentially for local parser and
storage checks, with explicit `CARGO_TARGET_DIR`, incremental compilation off
and debug information off. Before its write window, 36 hardlinked cache paths
were replaced by independently copied, hash-verified bytes. All inventoried
alias hashes were preserved; no output was deleted or hardlink-cloned. The
following preflight certified isolation with 31.64 GiB free and a 0.50 GiB output
allowance, leaving 31.14 GiB against the unchanged 30 GiB reserve. No competing
build/app task was active. The primary `target` and larger `desktop-feature-tests`
cache remain unwritten. No new permanent cache is introduced.

Full desktop packaging/installed tests need their own larger output budget,
fresh isolation preflight and exclusive host lock. They must not reuse a
non-equivalent old executable as current-source evidence. Exact absolute cache
and evidence paths, source/check receipts, cache-copy inventory and current
capacity remain privately retained in `status-notes-20261104`, outside compiler
caches. No performance qualification, public activation or phase acceptance is
part of this change. Preserve both owner planning drafts, all source/corpus,
personal profiles, historical binaries and review evidence. Reused outputs are
development cache only, not independently retained evidence binaries.

The final installed-build preflight measured 31.29 GiB free. With a conservative
3 GiB full desktop/package/evidence estimate it projects 28.29 GiB, below reserve;
the larger desktop cache also retains 44 hardlinked paths. No full desktop
build or installed test was started. This is the specific remaining local
verification blocker; hosted checks do not replace an installed note workflow.

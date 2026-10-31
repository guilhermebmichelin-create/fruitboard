# Note research validation and development continuation

Date: 2026-10-31. Reviewed main:
`8a4caa5b84ffadbbc91118cd6892b6b459a34c46`, including merged PR373.
Related issue: [#366](https://github.com/guilhermebmichelin-create/fruitboard/issues/366).

## Progress review

Phase 3 remains active. The development Library already shows saved project
facts, channels, sample/plugin references, pattern names and note-record counts,
and qualified ordinary mixer names. PR348 delivered bounded project-relative
sample checks. PR367 and PR373 delivered numeric note observations and controlled
comparison tooling. These are merged steps, not outstanding implementations.
The [October 31 assessment](../development-status-20261031/README.md) retains the
broader gaps and historical evidence boundaries.

The next product slice is saved ordinary note details. Its prerequisite remains
independent matching-build observations of pitch, start, duration, velocity and
channel binding. Actual sequences, MIDI export, Phase 3 exit, resource budgets
and production activation remain unaccepted or undelivered as previously recorded.

## Review finding and repair

The research observer compared both chunk signatures after Node's ASCII
decoding. That decoding clears each byte's high bit. Flipping that bit in any
of the eight `FLhd`/`FLdt` signature bytes therefore still returned all nine F16
records. A pinned digest identifies bytes; it does not make malformed framing
valid. This matters especially for the explicit-input comparison command.

Compare the signature bytes directly instead. Constructed in-memory regressions
cover ordinary corruption and high-bit corruption at every signature position,
and refusal of either corrupted comparison input. Approved corpus bytes stay
unchanged. This repair affects research tooling only; the application's Rust
parser and supported musical fields do not change.

## Next development boundary

1. Restore storage capacity and pass the four-path preflight with an allowance
   for new private saves and evidence. Current C: capacity is about 28.23 GiB;
   G: is about 12.33 GiB. Both are below the required 30 GiB reserve. No build,
   dependency installation, fixture generation or installed test ran here.
2. Confirm ownership of the open matching-build FL Studio session. A read-only
   probe found an accessible input desktop and an existing FL Studio process;
   no keys, clicks, playback, open/save or process termination were sent to it.
3. Complete the [registered genuine-save experiment](../../research/parser-note-sequence-366.md#genuine-save-experiment)
   and independent channel-binding observations. Retain exact GUI properties,
   save/reopen evidence, hashes and sanitation differences privately. The
   [comparison tool](../../research/note-record-differentials-366.md) reports
   numeric changes; it cannot approve a musical interpretation.
4. Present exact candidate bytes and expectations for fixture privacy approval.
   Then implement bounded parser/full validation, immutable storage and the
   accessible Library note list as separate reviewed slices under the
   [proposed sequence contract](../../research/note-sequence-contract-366.md).

## Validation and storage handoff

Owned source boundary: the research observer, its observer/differential tests
and this review document, on the main baseline above. Required checks are the
pinned Node policy/script tests, script syntax, scoped formatting/Markdown lint,
privacy/fixture hashes and whitespace checks. Fresh updated-head CI remains
required; PR373's fifteen successful checks cover its unchanged tested source,
not this repair or any new musical-field evidence.

Local Node 24.20.0 results: 183 full-suite passes, four failures and one
Linux-only FIFO skip. The four unchanged tooling-glob regressions still resolve
the stale installed minimatch 10.2.6 rather than the lockfile's patched 11.0.3;
no install or test suppression was used. All 34 focused observer/comparison
tests pass, with that same Linux-only skip. Script syntax, scoped Prettier and
Markdown lint, repository privacy/fixture hashes and whitespace checks pass.
Fresh frozen-install CI must validate the complete updated head. The reviewed
main integration's fifteen successful PR372 checks apply only to unchanged code.

No local native build or compiler-cache write is needed for this boundary.
The primary `target` and reusable `desktop-feature-tests` caches remain under
existing `/root` coordination, with no acquired build/install window. A narrow
inventory found about 2.49 GiB of September compiler libraries/metadata outside
those active caches, but cleanup was rejected before execution. Nothing was
deleted, no space was reclaimed and no rebuild cost was introduced.

Private reproduction, inventory, check logs and the handoff remain outside
source/compiler caches in `note-qualification-continuation-20261031`. The
handoff records absolute cache/evidence paths and current capacity. Preserve
all existing source, caches, binaries, fixtures, databases and prior evidence.

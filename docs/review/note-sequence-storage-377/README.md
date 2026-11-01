# Immutable ordinary-note sequence storage

Issue: [#377](https://github.com/guilhermebmichelin-create/fruitboard/issues/377).
Source baseline: merged PR376, `d8d80f926276eb38d83b9828a41c23c638ef5616`.
Status: **Persistence slice; Library sequence display remains pending.**

## Saved behavior

The full-validation projection now selects optional `patternNoteSequences` into
the existing private immutable metadata payload. Records retain pattern ID,
source ordinal, dense channel index, raw saved key, pattern-local start/duration
ticks and raw velocity byte. Coverage remains `stored-pattern-note-records` at
96 PPQ. Records are neither sorted nor deduplicated, and repeated Playlist
placements do not expand them.

Only the parser's typed accessors supply this projection. There is no raw JSON
copy, additional logging, serialization of private types, renderer field or
sample/plugin/process authority. Supported siblings coexist with explicit
unavailable/unsupported patterns; unknown records never become a partial list.
Whole-section and per-pattern detail-limit states remain explicit.

This preserves adapter 0.1.4's exact 26.1.0.5530 build, observed raw values and
one/two-channel Sampler subset. It introduces no musical pitch names, MIDI,
seconds or velocity conversion. See the [parser boundary](../pattern-note-sequences-366/README.md)
and [approved expectations](../../../fixtures/parser-corpus/note-sequence-expectations.json).

## Compatibility, bounds and reads

Projection version 1 already supports additive optional fields; there is no SQL
migration or version increment. A descriptor without sequence advertisement
selects no sequence field. Old snapshots lacking it remain readable and are not
rewritten or reparsed. Absence means details were not selected/saved, never zero.

A present field is revalidated on both historical and current payload reads
against the saved build, pattern IDs, note counts and channel count. The new
stored-sequence validator reuses the parser's existing grammar, reasons and
qualified values at fixed ceilings rather than implementing another decoder.
Malformed present fields reject the payload; they do not fall back to an older
shape. An opaque historical payload remains a saved fact, not a live-source or
display capability. ID-only freshness queries still do not load payloads.

The existing limits remain 512 detailed records per pattern, 1,024 total,
128 KiB sequence JSON and 256 KiB complete reply/persisted projection. The bounded
writer rejects an oversized complete projection transactionally; no limits are
raised and no records are truncated. Root/location/fingerprint/publication and
worker session/lease fences remain in their existing transaction. Failed,
rejected or stale attempts cannot overwrite a good historical payload.

## Validation and handoff

Owned checkout: the dedicated `fruitboard-note-sequence-storage` worktree, branch
`feat/377-note-sequence-storage`, owner `/root`. The changed source boundary is
storage metadata encoding/read validation/tests, one reusable parser validator,
the real native-analysis regression and matching documentation. No fixtures,
dependencies, lockfiles, database schema, UI, commands or CI workflows change.

Storage regressions cover all seven approved F24-F30 cases against independent
expectations after database close/reopen, old/unadvertised results, immutable
history, malformed advertised replacements, mixed states, duplicate ordering,
512/513 and 1,024/1,025 boundaries, corrupted stored fields, negative outcomes
and source loss. Existing tests retain the complete payload ceiling and wider
publication/cancellation/session/lease fences. The Windows native-analysis test
uses the freshly built supervised parser and real authorized source handles for
all seven cases; existing feature CI runs it through the `native_analysis`
filter. Approved source bytes are checked unchanged.

Required checks are pinned formatting, privacy and repository policy tests;
storage default/parser-metadata/analysis-jobs tests and warning-denied Clippy;
parser tests/Clippy; native analysis and freshly built parser integration; and
the documented integrated Windows gate plus normal updated-head CI. Prior CI
applies only to unchanged source. No installed UI or performance claim follows
from this storage slice.

The local four-path preflight fails at 27.53 GiB free on C: with a 1 GiB output
estimate leaving 26.53 GiB against the 30 GiB reserve. The selected reusable
cache also reports hardlink aliases and is not certified isolated. No local
compilation, fixture generation, dependency installation or installed test is
started. Changed-head hosted checks provide compilation/test evidence; their
results do not certify local cache isolation or performance.

The existing reusable `desktop-feature-tests` cache remains under sole owner
`/root`, write window closed. The private `note-sequence-storage-20261101`
handoff records the absolute source/cache/evidence paths and check receipts
outside the source worktree and compiler caches. Primary development cache and dirty
primary documents are preserved. No cleanup or new compiler cache is introduced.
Restore the reserve and resolve isolation before any local heavyweight work;
allow one writer/build at a time, hold the installed-test host lock where
applicable and run qualification alone. Retain evidence and source; there are no
new obsolete compiler intermediates or local rebuild costs from this task.

The next slice is accessible Library display with strict native/client decoding,
old-result copy and source/snapshot-loss states. Broader grammar/build coverage,
resource budgets, Phase 3 acceptance and production activation stay separate.

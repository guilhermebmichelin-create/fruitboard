# Bounded ordinary pattern note extraction

Related issue: [#366](https://github.com/guilhermebmichelin-create/fruitboard/issues/366).
Source baseline: merged `884a0c232408bb7d1f3e0263667582179e41fb5e` (PR375).
Status: **Parser and full native validation slice; no Library sequence display.**

## Behavior and evidence boundary

The [approved F24-F30 experiment](../../research/parser-note-sequence-366-result.md)
supports ordinary saved-note records on exact FL Studio 26.1.0.5530, 96 PPQ.
Adapter 0.1.4 advertises additive `patternNoteSequences` on protocol 1/schema 2.
The field uses coverage `stored-pattern-note-records`, coordinate system
`pattern-local-ticks`, PPQ 96 and patterns in the existing sorted ID order.
Each pattern contains either complete source-ordered records or a fixed
unavailable/unsupported reason. Missing data never becomes an extracted zero.

Records contain `ordinal`, `channelIndex`, `savedKey`, `startTick`, `durationTick`
and `storedVelocityByte`. Only the observed raw values are accepted: keys 60/64,
starts 0/48, durations 48/96 and velocity bytes 74/89. Flags must be `0x4000`;
remaining uninterpreted bytes must match the approved ordinary records. This is
deliberately narrower than a general piano-roll decoder. Broader value ranges,
articulations, velocity conversions, other builds/PPQs and MIDI remain open.

Saved channel IDs are resolved through declarations from the same file; rack
reordering does not establish persistent IDs. Duplicate/sparse IDs, more than
two channels, missing targets and targets other than a qualified Sampler are
refused. The same pattern is decoded once, independently of Playlist reuse.
One unqualified record refuses that entire pattern, without silently hiding
notes or discarding supported siblings. Existing saved-note counts are separate.

## Limits and native validation

Sequence advertisement requires existing counts/pattern/project-fact fields
and `maxDetailedNotesPerPattern`, `maxDetailedNotesTotal`, `maxNoteSequenceBytes`.
Ceilings are 512 records per pattern, 1,024 candidates total and 128 KiB detail
JSON. Candidates charge the budget before classification, including duplicates,
malformed payloads and refused records. Input/count/event/text/time limits stay
unchanged. Payloads are borrowed rather than copying large note sections.

Exceeding a per-pattern cap refuses that pattern. Exceeding the total or byte
budget refuses the whole optional section. Before emitting a wire response,
the encoder replaces the whole section with its fixed limit state if the
complete envelope would exceed 256 KiB; otherwise the existing response-limit
error remains. No arrays are truncated or undocumented limits increased.

Full validation checks descriptor dependencies, exact shapes, fixed reasons,
qualified build/values/PPQ, pattern order, count agreement, contiguous ordinals,
channel bounds, integer types, caps and checked arithmetic. Malformed advertised
fields reject the complete reply. Initial-only validation still discards these
extensions, and older descriptors without sequence advertisement select no data.
Private typed accessors have no automatic logging or serialization. Existing
source/root/fingerprint/lease authority and immutable publication are unchanged.

## Validation and handoff

Owned worktree: the dedicated `fruitboard-pattern-note-sequences` checkout,
branch `feat/366-pattern-note-sequences`, owner `/root`. Validate the parser crate,
its CLI/descriptor/transport and full native projection against the stated
baseline, plus downstream parser-metadata/analysis tests. No fixture bytes,
database schema, storage selection, desktop command or renderer code changes.

Regression coverage includes all seven independent cases and unchanged hashes;
mixed supported/refused patterns; source order/duplicates; explicit declaration
mapping; sparse/ambiguous bindings; missing/duplicate/late/misaligned payloads;
record/candidate/JSON/wire limits; descriptor dependencies; malformed advertised
replies; and older-descriptor/build behavior. Constructed tests prove defenses,
not additional GUI compatibility. Normal updated-head CI is required; prior CI
applies only to unchanged code, not this extraction slice.

Local heavyweight validation is blocked by the storage preflight: C: has
27.70 GiB free, estimated additional output 1 GiB leaves 26.70 GiB against the
required 30 GiB reserve. The selected reusable cache also reports hardlink
aliases and is not certified isolated. No compile or installed test starts.
The existing reusable `desktop-feature-tests` cache
remains owned by `/root`, write window closed; primary cache ownership is
unchanged. No new compiler cache is created and no generated files are deleted.
Absolute source/cache/evidence paths and check receipts are recorded in the
private `pattern-note-sequences-366` handoff outside compiler caches.
Restore capacity and resolve cache isolation before local builds. Installed
tests require the existing exclusive host window; qualification runs alone.

The [storage slice #377](../note-sequence-storage-377/README.md) selects fully
validated sequences into immutable storage with backward-compatible bounded
reads. The next slice adds an accessible Library note list with old-result and
source-loss states.
Neither this parser slice nor green CI completes those slices, performance
qualification, Phase 3 acceptance or production activation.

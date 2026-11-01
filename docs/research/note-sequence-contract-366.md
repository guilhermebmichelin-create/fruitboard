# Proposed saved-note sequence contract

Status: **Parser/full native validation and immutable storage implemented;
Library display remains pending.** See the [storage slice](../review/note-sequence-storage-377/README.md)
and the
[implementation boundary](../review/pattern-note-sequences-366/README.md).
Seven narrow genuine-save cases now have [approved qualification evidence](parser-note-sequence-366-result.md).
Related [research and genuine-save plan](parser-note-sequence-366.md),
issue [#366](https://github.com/guilhermebmichelin-create/fruitboard/issues/366).
Design source: research commit `ecd16e996991f5151b7319b693472a9963bb00b1`.

Contract review can now use F24-F30's independent GUI observations. The first
implementation must preserve their exact build, raw-value and channel-binding
limits; numeric candidate words alone never replace independent ground truth.

## First useful delivery

After independent qualification, show the saved ordinary note sequence of a
small pattern, grouped by its existing channel labels. Preserve note start,
stored duration, numeric key and velocity with verified units. Each pattern is
interpreted once, independently of Playlist reuse. No playback, MIDI export,
project editing or full-song expansion enters this initial slice.

The initial slice is deliberately complete-or-unavailable **per pattern**.
A verified ordinary pattern can coexist with another unsupported pattern.
Never omit a slide, step or unknown record and present the remaining records
as that pattern's complete sequence. Existing note counts stay independent.

## Proposed structure

The optional `patternNoteSequences` field requires an advertising
descriptor and explicit caps. This example is **constructed**, not an observed
F16 expectation or the implemented wire format. Adapter 0.1.4's actual field
and stricter qualified-value limits are documented in the implementation review:

```json
{
  "coverage": "stored-pattern-note-records",
  "coordinateSystem": "pattern-local-ticks",
  "ppq": 96,
  "patterns": [
    {
      "patternId": 1,
      "status": "extracted",
      "records": [
        {
          "ordinal": 0,
          "channelIndex": 0,
          "savedKey": 60,
          "startTick": 0,
          "durationTick": 48,
          "velocity": { "status": "unavailable", "reason": "unqualified" }
        }
      ]
    }
  ]
}
```

`savedKey` initially denotes a saved numeric key only. It is not automatically a
standard MIDI key, audible frequency or octave label. A qualified mapping and
range must precede musical pitch naming or MIDI conversion. Raw candidate words
do not gain authority from fitting the example's integer ranges.

`channelIndex` refers to the existing validated dense channel projection, **not**
the note's raw saved-channel word. Extraction needs an independently qualified
map from saved channel identity to that projection. Do not guess from channel
count, current list order or the assumption that IDs are contiguous. Refuse a
pattern's details if binding cannot be proved.

Keep PPQ and tick coordinates together. Musical quarter-note position may be
derived as ticks / PPQ only after units and origin are qualified. Display seconds
only under a separately qualified tempo policy; base BPM alone does not describe
tempo automation. Retain source ordinals even if the UI sorts chronologically.
Identical notes remain separate records; an ordinal is local to this immutable
result, not a persistent ID across edits.

## States and exclusions

| Condition                                       | Proposed behavior                                                                                |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| No descriptor advertisement                     | Details unsupported; never an empty list                                                         |
| Older immutable snapshot without this field     | Explain that sequence details were not saved; no automatic reparse                               |
| Missing pattern payload, including current F17  | Unavailable, not extracted zero                                                                  |
| Independently qualified explicit empty layout   | Empty extracted sequence only after its own evidence; not supported by current corpus            |
| Entire pattern has qualified ordinary records   | Complete records in source order, with optional unqualified properties explicitly unavailable    |
| Slide/portamento/unknown articulation           | Pattern details unsupported until separately qualified; retain existing counts                   |
| Stored zero-length step record                  | Pattern details unsupported until duration semantics are qualified; never invent note-off timing |
| Unknown channel binding or PPQ interpretation   | Pattern details unsupported                                                                      |
| Missing velocity qualification                  | Velocity unavailable; do not copy byte 21 into a MIDI velocity field                             |
| Detail cap exceeded                             | Explicit detail-limit state; no clipped arrays or hidden omissions                               |
| Malformed advertised data                       | Reject the complete parser reply; do not demote invalid data to an older shape                   |
| Changed/disabled/missing source or stale result | Existing details authority refuses content; no last-project sequence retained in the UI          |

Fixed reason codes are selected during implementation, aligned across Rust,
private storage, desktop DTO and strict client decoding. The lowercase example
reason above is explanatory design text, not a new accepted reason vocabulary.

Unknown nonessential properties can remain unavailable without losing a
qualified note's position/key/duration. Unsupported record _kinds_, ambiguous
binding or missing records invalidate that pattern's completeness. Distinguish
these two cases explicitly in the final validator.

## Bounds and consistency

Candidate initial ceilings are **512 detailed records per pattern and 1,024 total**.
These are proposed safety bounds, not accepted performance targets. Every
candidate record charges the processing budget before filtering, including
malformed, duplicate, unsupported and ultimately refused records. The existing
count caps do not authorize equally large detail arrays.

Keep the existing 64 MiB input cap, 2 MiB interpreted-event cap, 100,000-event
cap, 256-channel cap, 1,024-pattern cap and supervisor deadline. Keep both
256 KiB reply and stored-projection caps. A proposed detail section budget is
**128 KiB**, but the whole serialized result must also fit: existing metadata,
envelope, statuses and JSON escaping count. If details cannot fit, replace the
whole detail section with a fixed limit state before the existing final output
check. Do not silently truncate patterns or increase existing caps. If even the
fallback does not fit, retain the existing bounded-result failure policy.

Full validation must check:

1. Optional field and caps advertised together; exact adapter/schema/build
   compatibility and qualified record grammar, without broadening by version band.
2. Exact shapes and fixed states; reject extra keys, value/reason contradictions,
   fractional numbers, negatives, non-finite values and integer overflow.
3. Pattern IDs in exactly the existing validated pattern order. An extracted
   pattern's record cardinality must match its qualified saved-note count.
4. Unique contiguous source ordinals; known channel indexes through the verified
   mapping; exact per-property qualified ranges, including saved key and velocity.
5. Positive qualified PPQ and ordinary-note durations. Use checked arithmetic for
   start plus duration. Do not infer positivity for currently unqualified steps.
6. Per-pattern and total record limits, full serialized size, and count/detail
   consistency before any immutable publication. Check all advertised fields;
   initial-only validators must continue discarding extensions they do not select.

## Storage, display and authority

Select only fully validated data into a versioned immutable projection. Inspect
the actual payload reader before deciding whether an additive version is enough
or a migration is required. Old missing fields remain readable; malformed new
fields do not fall back silently. Rollback may stop selecting this optional
content without rewriting historical snapshots.

Every detail read retains the existing root/location/source fingerprint,
snapshot and current-result checks. Publication keeps the independent digest,
source/root/session/lease and transactional fences. Notes grant no additional
filesystem, sample, plugin or process authority. Do not include sequences in
diagnostics or automatically add them to sync.

Start display with a read-only note list under the selected pattern/channel,
showing raw numeric key and verified tick positions/durations. Musical labels
require their qualified mapping. Render a bounded first 64 rows and an accessible
explicit reveal action; this is UI disclosure, not data truncation. Clear state
on snapshot/root loss and invalidate in-flight responses. Any eventual piano-roll
needs keyboard-accessible equivalent information and narrow/enlarged-text tests.

## Work that can proceed while Windows is locked

This contract, descriptor/limit review, state mapping and constructed defensive
test design can proceed. Production extraction does not proceed on guessed
semantics. After GUI qualification, implement in separate parser/validation,
immutable storage and Library slices with the relevant tests and installed
evidence. Keep fixture exact-byte approval separate from design acceptance.

Constructed tests should cover cap boundaries, unknown flags, repeated patterns,
two channels with identical notes, sparse/reordered saved IDs, duplicate notes,
old snapshots, malformed advertised replies and late/stale completions. They
prove defenses only; they do not replace genuine FL Studio observations.

This documentation-only follow-up needs scoped Markdown lint, privacy and
whitespace checks plus updated-head CI. No local build, new dependencies,
compiler cache writes or fixture generation is required. The unchanged research
scripts retain their 164 passing Node 24.20.0 tests at `ecd16e9`; the prior 15
hosted passes apply to that head, not this changed documentation head.

# Automation object and control-point counts: current evidence and qualification plan

Issue [#324](https://github.com/guilhermebmichelin-create/fruitboard/issues/324)
asks whether saved automation objects, Playlist placements and editable curve
points can be counted independently. This report records what the existing
approved fixtures establish, separates outer-event framing from unproven inner
semantics, and defines a small genuine-save matrix that can distinguish the
remaining hypotheses.

Status: **Research only; automation count extraction is not qualified or
delivered.** No parser field, schema, extraction, persistence or Library view is
introduced here. The current exact-build GUI evidence is useful but does not
separate object count from placement count, or establish a byte-level point
grammar.

## Existing GUI evidence

The owner approved F23's exact sanitized FLP and expectations on 2026-10-16.
The independent GUI oracle was recorded before candidate-byte or parser
comparison. In FL Studio 26.1.0.5530, F23 contains one native automation clip
object linked to Sampler channel volume, placed once on Playlist Track 1 from
bar 1 to bar 2. The three displayed editable control points are:

| Ordinal | Displayed B:S:T position | Displayed value |
| --- | --- | --- |
| 1/3 | `001:01:000` | `-5.2dB` |
| 2/3 | `001:09:000` | `-12.8dB` |
| 3/3 | `002:01:000` | `-5.2dB` |

The two hollow segment-tension handles are not control points. The approved
F23 file is 47,788 bytes, SHA-256
`23b764e4220fc5e48ffbd4a99c602bda092c032ef35b9b2d5b785250b622cf7f`.
Its exact fixture and GUI provenance remain in the [#322 fixture
result](parser-automation-322-result.md) and [corpus
manifest](../../fixtures/parser-corpus/manifest.md#saved-automation-gui-case-f23).

F11 is the existing genuine empty-Sampler control from the same exact build
and tempo. Its GUI has no Playlist clips. It is 46,589 bytes, SHA-256
`e0471d55032b7dfaa8bbcd496c487147149b9ce18e0b250882cc6f67f47480ee`.
F11 is evidence for this one no-clip save, not a universal proof that an absent
record means zero automation in every layout or build.

The [FL Studio Automation Clips manual](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/playlist_automationclip.htm)
describes an Automation Clip as a Generator loaded into a Channel and says its
data can appear in the Playlist. It documents selecting an Automation Clip in
the Playlist selector and placing it, deleting a Playlist clip while leaving
the Channel in place, and editing control points separately from tension
handles. Reusing one object for multiple placements remains an experimental
question; the manual does not specify the FLP byte grammar.

## What the independent byte walk establishes

An independent bounded walk checked the `FLhd` magic and declared six-byte
header, the `FLdt` magic and declared data size against the file length, the
event-ID-dependent fixed widths (including the three-byte ID 172 form for this
saved major version), and ULEB lengths for event IDs at or above 192. Both
walks ended exactly at EOF without trailing bytes. F23 has 1,470 framed events
and F11 has 1,421. The F23 count is for the already-approved sanitized fixture;
its prior sanitation removed 18 registration bytes from event 200 and adjusted
the enclosing FLdt length. No approved fixture or expectation bytes were
changed for this research.

The relevant framed records are:

| Fixture | Event | Ordinal | Event-ID offset | Length encoding | Payload range |
| --- | ---: | ---: | ---: | --- | --- |
| F23 | 234 | 118 | 1,789 | `cd 01` = 205 | `[1792,1997)` |
| F23 | 233 | 134 | 2,430 | `58` = 88 | `[2432,2520)` |
| F11 | 233 | 85 | 1,319 | `00` = 0 | empty at offset 1,321 |

F11 contains no event 234. F23's event 234 precedes event 233. A scanner's
last-seen channel marker is 1 for both F23 records, after markers 0 and 1 were
seen in the file. That is a chronological scanner state only; it does not prove
the records belong to channel 1 or reference each other. Neither payload
contains the target channel name, and repeated zero/one values do not prove a
channel reference.

The F23 event-233 payload starts
`00 00 00 00 00 50 01 00 80 01 00 00 f3 01 00 00`. One heuristic reading
would yield candidate values start 0, item `0x00015000`, length 384 and token
499. The candidate item value fails the current pattern-ID encoding heuristic;
the apparent bar-1-to-bar-2 extent is only a coincidence-compatible
hypothesis. Event 234 starts
`01 00 00 00 40 00 00 00 00 04 00 00 00 03 00 00 00 03...`; unaligned
16-bit windows at relative offsets 13 and 17 happen to equal 3. Neither those
windows nor any other inner value is a qualified point count. Exact divisors
of its 205-byte payload permit candidate counts 1, 5, 41 or 205; one possible
segmentation is `13 + 3×64`, among many. These are falsifiable hypotheses, not
field definitions.

The parser currently exposes no automation summary. F23's basic metadata can
be complete while its generator class remains
`GENERATOR_CLASS_UNVERIFIED` and Playlist clip interpretation remains
`PLAYLIST_CLIP_KIND_UNVERIFIED`. The current tests preserve those refusals and
the exact approved bytes. The unrelated 24-byte note-record layout is not
evidence for an automation record size. Outer framing, the GUI's three visible
points, and a matching-looking candidate integer do not together establish a
saved point grammar or a binding from event 234 to event 233.

## Semantic definitions for future review

These definitions clarify what the experiments should distinguish. They do
not name or propose public parser fields, APIs, persistence properties or
Library labels before the saved grammar is qualified.

| Research quantity | Proposed semantic definition | Required separation |
| --- | --- | --- |
| Saved automation objects | Count each unique saved native automation object definition once, regardless of whether it has zero, one or several Playlist placements. | A timeline occurrence is not another object. Treat a “Make unique” result as a second object only when save/reopen evidence establishes an independent definition. |
| Playlist placement | Use each saved timeline occurrence only as a research control that can vary independently from object count. | The GUI manual says deleting a Playlist placement leaves its Channel object. A repeated placement must not multiply the object's control-point count. This is not a proposed product field. |
| Control points | Count editable nodes on each unique object's saved main curve once, then sum across unique objects. | Exclude tension handles, interpolation/segment metadata, LFO-generated behavior, separate event/MIDI automation, target values, playback and repeated placements. |

The [FL Studio Automation Clips manual](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/playlist_automationclip.htm)
distinguishes control points from tension handles, says deleting a Playlist
clip leaves its Channel in place, describes selecting and placing a clip from
the Playlist selector, and defines “Make unique” as an independent clip. Those
GUI behaviors inform the experimental controls only. Target/parameter
identity, persistent IDs, curve evaluation, values over time and MIDI remain
outside this count research. A reference-differential experiment is warranted
only if later needed to prove object binding, not to claim target semantics.

## Absence, zero and result states

An absent marker is not itself a zero. A future object total may report zero
only when a complete, recognized and qualified layout proves that the entire
counted object set is empty. A future aggregate point total may report zero
when that object set is empty, or when every present object in the counted
scope has a recognized, explicitly empty point list. One empty object's curve
does not make the aggregate zero if another object has points. A missing event
in an unknown, truncated, mixed, ambiguous or otherwise unqualified layout
remains unsupported or an appropriate parse failure; it is never silently
converted to zero.

The proposed state rules are:

- **Extracted zero:** only on an exact qualified build/layout after complete
  validation proves the entire counted object set is empty, or proves that
  every present object's point list is explicitly empty. Object count zero
  and aggregate point count zero are separate claims; a single empty object
  does not prove an aggregate point total of zero. F11 alone does not prove
  either saved-byte zero rule.
- **Unavailable:** only when a qualified layout establishes that the relevant
  data is not stored or cannot be observed for that case. “No record found” in
  an unknown layout is not unavailable evidence.
- **Unsupported:** a build/layout, record form, or relationship whose meaning
  has not been qualified, including today's automation fields. This is the
  current honest state for automation interpretation.
- **Malformed/partial:** truncation, contradictory lengths/references, or an
  ambiguous record in a layout otherwise being parsed must reject the complete
  numeric automation summary or remain unsupported under a fixed reviewed
  safe contract. Never emit a partial numeric summary or silently convert it
  to zero.
- **Older immutable result:** a historical result that predates a field did
  not capture that field. Preserve it as an older/missing-result state, never
  backfill zero or reparse it during refresh. A new reply that advertises a
  field but omits its required value is invalid, not an older-result case.

## Smallest useful differential save matrix

The present F11/F23 pair provides the baseline rows below, but cannot
discriminate all competing grammars. Each proposed new row should start from a
separate copy of an approved synthetic save, use only exact build 26.1.0.5530,
and have the GUI outcome and expected counts preregistered before anyone
inspects candidate bytes. Save once, reopen in the matching build without
further saving or playback, record the visible objects/placements/points, and
hash the saved bytes. Keep screenshots and saves private until an exact owner
approval packet covers any proposed public registration.

| Case | GUI construction and expected result | Hypothesis it can falsify |
| --- | --- | --- |
| F11 control | Existing same-build GUI observation: one Sampler and no Playlist clips. Do not infer saved-byte object or point count zero until a future grammar is complete and qualified. | Negative GUI control for the tested save only; does not qualify all zero cases by itself. |
| F13 pattern-only Playlist control | Reuse the approved pattern project whose registered Playlist view shows four one-bar pattern placements (A, A, B, C); see the [F13 placement result](parser-playlist-188-result.md). Do not recreate or re-register it. | Controls for separating ordinary pattern placements from automation semantics, within the recorded GUI scope. |
| F23 baseline | Existing save: `(1, 1, 3)`. | A candidate that cannot account for the independently observed three-point, one-placement example. |
| F23 with one point removed | Separate copy; retain one object and one placement, with two visible points: `(1, 1, 2)`. | Fixed-size/constant-count interpretations and the candidate that treats tension handles as points. |
| F23 with a point added | Separate copy; retain one object and one placement, with four visible points: `(1, 1, 4)`. | Competing count-field, stride and per-point segmentation hypotheses. Together with the two-point row, distinguishes a value that merely coincides with 3. |
| Same point count, changed point location | Separate copy; keep one object, one placement and three points, but move one point to a different registered position/value: `(1, 1, 3)`. | Whether candidate framing tolerates changed per-point content rather than only the original byte coincidence. GUI expectations check visible position/value; this does not qualify byte-level position/value extraction. |
| Repeated placement | Separate copy; place the same object twice, retain its three points: `(1, 2, 3)`. | Placement-count-as-object-count and point-total-multiplied-by-placement interpretations. |
| Two unique objects | Separate copy; make a second independent object and give the two objects different point counts, preferably two and four: `(2, 2, 6)`. | One definition with two references versus two definitions, and per-object versus aggregate point count. Confirm uniqueness by GUI reopen. |
| Object without placement | Separate copy; remove the Playlist placement while retaining the channel/object and its three points: `(1, 0, 3)`. | A parser that equates object existence with a Playlist placement. The GUI manual says deleting a Playlist clip can leave the Channel object. |
| Explicit empty curve, if the GUI permits it | Attempt a saved object with an explicitly empty curve; record its actual placement state and expected `(1, 0-or-1, 0)` only if reopen confirms no editable points. If FL Studio forces a point, document that this row is unavailable through this authoring path; do not synthesize bytes. | Explicit empty point-list semantics versus absent object/record semantics. |

The point-count and uniqueness cases should vary one factor at a time and use
the same tempo, target type, build, and otherwise-empty synthetic project.
Store each result independently; do not use playback or personal projects. The
existing exact-build installation alone does not authorize a public fixture.
The new genuine saves, screenshots, preregistration, reopen proof, sanitation,
hashes, privacy review and expectations must stay outside Git until the owner
approves their exact packet. Only then should a separately reviewed change
register exact bytes/expectations and add parser tests. Preserve all current
F22/F23 bytes and dated approvals unchanged.

## Qualification boundary and next review

F23 qualifies one GUI-observed native automation example and F11 one genuine
no-clip control, both in 26.1.0.5530. The complete event walks qualify only
outer record boundaries for those exact files. They do not qualify any saved
automation object count, placement count, point count, reference, or other
saved build. No differential save was made for this report; therefore no
automation feature or broader qualification claim is delivered. The next
research review should compare independently preregistered genuine saves from
the matrix above, update this report with exact hashes and GUI outcomes, and
decide whether a bounded grammar is falsifiably established before proposing
extraction or application fields.

This docs-only result changes no parser, native authority, persistence,
renderer, corpus, dependencies or CI. Existing approved fixtures and
expectations remain byte-identical. Repository documentation/privacy/script
checks and updated-head CI are owned by the orchestrator; no local build or
installed test was required for this report.

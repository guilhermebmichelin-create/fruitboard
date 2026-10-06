# Saved pattern note-count proposal

Status: **Design for review; no implementation or fixture approval.**
Issue [#288](https://github.com/guilhermebmichelin-create/fruitboard/issues/288).
Planning source: PR285 merge `d19587f52132f31065bbaab97a572d3c5487cf00`.
This starts [readiness gap G6](../flp-intelligence-readiness/gap-ledger.md)
inside Phase 3. It does not accept the phase checkpoint or resource budgets.

## Owner outcome

After a new analysis, each saved pattern could show how many note records it
contains. Reusing a pattern in the Playlist would not multiply its count.
Unknown contents would stay unknown; a missing record would not become zero.
The first step is to qualify the counts against deliberately made projects in
the matching FL Studio build, then add parsing and Library display separately.

## Meaning and format evidence

Count the records saved inside each pattern, across its channels, once each.
Do not infer sounding notes, MIDI messages, note density, composition quality,
song totals or duration. Identical records remain separate saved entries;
Playlist copies do not add entries. No pitches, positions, lengths, flags,
velocities or raw record arrays reach storage or the renderer in this slice.

Image-Line documents [Piano roll notes and slides](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/pianoroll.htm),
including slide entries that do not themselves produce sound. This is why the
display says **Saved note records**, rather than promising audible notes.
The [PyFLP pattern format reference](https://pyflp.readthedocs.io/en/latest/_modules/pyflp/pattern.html)
suggests event 224, 24-byte entries and association with event-65 pattern IDs.
These are candidate format facts, not verified support for the selected build.
Its GPL source is not copied or translated into product code, and no PyFLP
dependency is proposed. Independent GUI saves and byte observations must qualify
the record layout and section binding before an independent Rust implementation.

Existing approved F13 verifies three pattern names and four placements. Its
note counts, empty-pattern encoding, Step Sequencer representation and slide
behavior have not been independently registered or qualified. F13 is not
silently assigned new note expectations. Existing F15 verifies empty Sampler
labels, not stored pattern-note counts.

## Proposed parser contract

The first candidate build is **26.1.0.5530** only, conditional on the
[fixture gate](fixture-plan.md). Other saved builds retain a field-level
unsupported state while their currently supported metadata remains readable.
Keep protocol 1/schema 2 with an advertised additive `patternNoteCounts` field.
An absent advertisement is unsupported, not an empty collection. The parser
package/adapter version changes with extraction; descriptor consumers validate
the new limits only when the field is advertised.

When stored patterns are known, return one entry per saved pattern ID, in the
same strictly ascending order as validated pattern names. Every entry carries
`patternId` and a `noteCount` field. A proposed fully qualified example is:

```json
{
  "status": "extracted",
  "coverage": "stored-pattern-note-records",
  "value": [
    { "patternId": 1, "noteCount": { "status": "extracted", "value": 3 } },
    { "patternId": 2, "noteCount": { "status": "extracted", "value": 1 } }
  ]
}
```

This example is a proposed shape, not an observed fixture result. The aggregate
status is `extracted` only when all entries are extracted; otherwise it is
`unavailable` with reason `PATTERN_NOTE_COUNTS_INCOMPLETE`, or `unsupported`
with that reason if any entry is unsupported, and carries `items` instead of
`value`. Preserve each entry's state. A global build/no-pattern/binding failure
has a fixed reason and no entries. Every non-failed summary carries the literal
coverage `stored-pattern-note-records`; the coverage label does not convert a
missing or unsupported count into an extracted count. A failed whole parse keeps the existing fixed
failed outcome; it never publishes fabricated counts.

| Evidence or condition                                                           | Proposed state / fixed reason                                                   |
| ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| One qualified payload bound to a known pattern                                  | `extracted`; checked record count                                               |
| Qualified, explicitly stored zero-length payload                                | `extracted: 0`, only after independent empty-payload qualification              |
| Known pattern with no note payload                                              | `unavailable / PATTERN_NOTES_NOT_STORED`                                        |
| No saved pattern markers                                                        | `unavailable / PATTERN_DATA_NOT_STORED`; no implicit Pattern 1                  |
| Other saved build                                                               | `unsupported / PATTERN_NOTES_UNVERIFIED_BUILD`                                  |
| Unqualified record length or layout                                             | `unsupported / PATTERN_NOTE_LAYOUT_UNVERIFIED` for that pattern                 |
| More than one note payload for the same ID, including byte-identical duplicates | `unsupported / MULTIPLE_PATTERN_NOTE_PAYLOADS`; never concatenate or select one |
| Payload without a qualified active ID, or unqualified section ordering          | Whole summary `unsupported / PATTERN_NOTE_BINDING_UNVERIFIED`                   |
| Per-pattern or total candidate limit exceeded                                   | Whole summary `unsupported / PATTERN_NOTE_LIMIT_EXCEEDED`; no truncated count   |

Event-65 repeats for content and property sections must not multiply counts.
Bind through qualified saved IDs, never channel order, names, Playlist positions
or the currently selected pattern. The implementation must define the verified
section transitions; channel/arrangement/Playlist boundaries end note authority.
An unknown intervening section transition cannot leave an old ID authorized.
The fixture gate records the actual allowed ordering before code is written.
Empty payload support may remain unqualified: then an empty GUI pattern whose
payload is omitted remains unknown. GUI emptiness alone does not authorize zero.

## Bounds and typed validation

Proposed new ceilings are **65,536 records per pattern** and **262,144 records
per project**. These are safety ceilings, not performance acceptance targets.
A qualified 24-byte layout would use at most 1.5 MiB for one allowed payload.
Keep the existing 64 MiB file, 2 MiB interpreted-event, 100,000-event,
256-channel, 1024-pattern, 256 KiB process-reply and 256 KiB stored-projection
caps, supervisor deadline, worker lifecycle and analysis-attempt policy intact.
Advertise `maxNoteRecordsPerPattern` and `maxNoteRecordsTotal` as positive limits
at or below these ceilings. Count through
checked borrowed slices, retain only a bounded ID/count/state map, and allocate
no per-note objects. Check total limits even for duplicate or unsupported
payloads so ambiguous input cannot bypass the processing budget. Charge a checked
ceiling of payload length divided by the candidate record size before interpreting
each note payload; do not allocate or traverse unbounded records to classify it.

The full native validator requires the optional field and its limits together,
the qualified saved build, exact aggregate/per-entry shapes, fixed reasons,
positive unique ordered 16-bit IDs, integer counts within advertised limits,
matching pattern IDs/count/list length and a checked sum within the project cap.
Reject value/items/reason contradictions, unexpected methods/confidence,
fractions, negatives, overflow, duplicate/missing IDs and malformed present data.
Do not drop an invalid advertised summary and publish it as old metadata.
Unrelated extensions are discarded. Counts are extracted observations with
coverage, not musical inference. The initial-only validator remains usable.

## Native authority and persistence review

| Boundary      | Required invariant / review                                                                                                                 |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Input         | Existing held, read-only FLP source and enabled-root authorization; no extra file open, search root, sample read or plugin execution        |
| Subprocess    | Existing fixed native-selected parser, bounded protocol, timeout/cancellation and replacement lifecycle; no new renderer process permission |
| Validation    | Envelope correlation, descriptor/version, fingerprint and complete typed projection validation precede publication                          |
| Publication   | Existing source/root/location/publication/session/lease rechecks; stale or invalid work preserves last-good immutable results               |
| Storage       | Optional `patternNoteCounts` in private projection version 1; no raw records, note strings, migration, rewrite or new table                 |
| Old snapshots | Absent optional field becomes `not_saved`; malformed present fields fail safely, never fall back to an older shape                          |
| Details read  | Existing current-result root/location/presence/fingerprint gates, bounded JSON and explicit selected DTO projection                         |
| Renderer      | Counts and fixed reasons only; no new commands, filesystem/plugin/MIDI capability or persistence from display                               |
| Diagnostics   | Fixed allowlisted codes only; no note content, source paths or raw parser payload in logs/sync                                              |

Authority follows [ADR-002](../../adr/002-flp-parser-process.md); this adds
content interpretation inside that grant, not a new filesystem grant. Review
these invariants again against actual extraction and final native call sites.
Rollback removes extraction/projection/display, keeps old immutable results,
and treats retained additive fields as unselected data. Opening or refreshing
details never reparses a file. A new analysis is explicit and keeps the existing
unchanged-file attempt limit.

## Library layout and states

Extend the existing **Saved patterns** list: keep each inert saved name/ID and
add a second text line, **Saved note records: N**, or a fixed explanation of an
unknown/unsupported count. The section description explains that this is saved
pattern content, including non-sounding records, independent of Playlist reuse.
Do not add a project-wide musical total or per-channel detail.

| State                                        | Owner-facing behavior                                                                         |
| -------------------------------------------- | --------------------------------------------------------------------------------------------- |
| Current qualified data                       | Count under each pattern; existing 20-item progressive disclosure                             |
| Qualified explicit zero                      | `Saved note records: 0`; only under the qualified encoding rule above                         |
| Missing note payload                         | `Saved note count is unknown; no note data was stored for this pattern.`                      |
| Unqualified/ambiguous data                   | Fixed explanation; existing names and other valid facts remain visible                        |
| Older snapshot                               | `Note counts were not saved with this result. A new analysis is needed to add them.`          |
| Loading or refresh                           | Clear counts with the existing details loading state; do not retain another snapshot's counts |
| Missing/changed/disabled/stale result        | Existing unavailable details state; no note content shown                                     |
| Malformed response or transport failure      | Existing safe details error; no partial decoded numbers                                       |
| Default desktop/PWA adapter without analysis | Existing disabled/unavailable behavior; no probe or synthesized count                         |

Preserve list keys, focus on progressive-disclosure actions, escaped private
names, live status and reduced-motion behavior. Require desktop/390px/200%-text
visuals and axe/contrast/keyboard checks for the eventual actual UI. This design
introduces no visual implementation; the table is the proposed information layout.

## Delivery boundary

The [implementation sequence](implementation-plan.md) separates fixture
qualification, parser/typed validation and immutable Library integration.
Each code PR needs actual-source review and its normal required CI. This design
needs pinned documentation/privacy/script/policy checks, not a local app build.
No new FLP bytes or expectations are approved by merging a design.

Automation/controllers, MIDI file import/export, piano-roll drawing/playback,
editing, mixer/nested plugins, tempo changes and richer arrangements remain
open G6 work. Broader G5 relative/placeholder/external/cloud resolution, G1/G4
resource decisions, G3 compatibility, G7 release and the Phase 3 checkpoint
also remain open. This is a first note-summary slice, not completion of them.

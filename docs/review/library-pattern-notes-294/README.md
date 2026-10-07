# Saved pattern note records in Library

Status: issue [#294](https://github.com/guilhermebmichelin-create/fruitboard/issues/294),
accepted note-count design slice 3; owner review/manual merge required.
Baseline: PR293 merge `ff48da24fc15151cc81d82edb168126811825706`, tree
`acb63317fe62b4c111c6ffb0b5cbd71823f7cb1e`.

## Behavior and boundary

The existing **Saved patterns** list adds **Saved note records** beneath each
saved name. Counts include stored slide/step records and entries that may not
sound; reusing a pattern in the Playlist does not increase its stored count.
There is no project musical total or per-channel count. Only exact build
26.1.0.5530 and the previously qualified initial content series are supported.
Explicit-zero payloads remain unqualified. Missing payloads stay unknown.

New analysis saves selected numeric counts and fixed states in the additive
`patternNoteCounts` field of immutable private projection version 1. There is
no migration, new table or rewrite. Older snapshots without the field remain
readable and explain that another analysis is needed. Unadvertised, ambiguous,
unsupported-layout and limit states use fixed explanations. Malformed present
data fails the complete details read; it cannot silently become an older result.

The authorized native read revalidates the exact build, coverage, ordered IDs
against saved patterns, positive integer counts, 65,536 per-pattern and 262,144
total ceilings, aggregate/state relationships and exact shapes. The renderer
independently checks the selected IDs, counts, states and ceilings. Raw notes,
pitches, velocities, positions, lengths and flags are never persisted or sent
to the renderer. Names remain inert escaped text.

Existing root/location/row-fingerprint/current-snapshot, publication/session/
lease fences remain authoritative. Disabled, changed or missing sources expose
no current counts. Loading clears previous results; late replies after closing
are discarded. Refresh reads results without analysis. Explicit analysis keeps
the existing attempt policy. The existing 20-pattern disclosure retains keyboard
focus. Default desktop and PWA analysis behavior remains unchanged.

## Validation record

Validation receipts, final source identity, installed artifact hashes and visual
evidence are recorded after qualification in this packet and the private handoff.
The approved F16/F17 corpus and its recorded expectations remain unchanged.
Constructed malformed/older/limit/paging states test defenses and display only;
they do not qualify a new FL Studio encoding.

## Remaining work and recovery

This completes the three narrow saved note-count slices when accepted. Broader
G6 automation/MIDI, mixer/nested plugins, tempo changes and richer arrangements
remain open. It does not accept Phase 3, production activation/distribution,
resource budgets or performance qualification. No new command, permission,
filesystem resolver, audio/process capability or dependency is added.

Rollback removes selection/display, retains immutable history and ignores the
additive field. Owner review decides whether to accept this slice; fixture
publication approval already covers the exact unchanged F16/F17 files.

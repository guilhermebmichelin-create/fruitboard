# Pattern-clip timeline end and nominal seconds (issue 190)

The approved F13 FL Studio 2026 project has pattern clips A, A, B, C in bars
1–4 in the matching-build GUI. PR #189 verified that its saved records start at
ticks 0, 384, 768, and 1152, and each lasts 384 ticks. The greatest start plus
length is therefore 1536 ticks. Adding the four clip lengths happens to give
the same number in this fixture, so separate in-memory overlap and gap cases
test that the parser uses the greatest end position instead.

F13 stores PPQ 96 in its FLhd header, a base tempo of 130.000 BPM, and one
each of the byte-valued timing events 17 and 18 with value 4. The approved GUI
case has a four-beat bar. [Image-Line's Project Settings manual](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/songsettings_settings.htm)
defines PPQ as pulses per quarter note for Playlist placement and describes
BPM in terms of quarter notes per minute. On this verified layout, a
constant-base-tempo conversion is `1536 / 96 × 60 / 130`, or about 7.384615
seconds. The nominal field keeps that assumption explicit, with `inferred`
status, `low` confidence, and a method name. Event IDs 17 and 18 are used only
as an exact match to the F13 timing values here; their general FLP semantics
are not established by this one fixture.

`playlistPatternEndTick` is extracted only when the verified pattern-clip list
has at least one clip. Empty saved lists return `unavailable` with
`NO_PLAYLIST_PATTERN_CLIPS`; absent or unsupported lists carry their existing
reason. `playlistPatternNominalSeconds` needs that end, PPQ 96, exactly one
4-valued event each for IDs 17 and 18, and a saved base tempo. Unknown timing
values report `unsupported`; missing base tempo reports `unavailable`.

Neither field describes a complete rendered song. Tempo automation can change
actual elapsed time; audio and automation clips, release tails, and export
settings may extend or change what a listener hears. F13's constant-tempo
estimate is a controlled stepping stone toward duration reporting. A broader
version and arrangement matrix remains necessary before these values can be
presented as general project length. No fixture bytes, scanner, database,
plugin, or packaging path is changed here.

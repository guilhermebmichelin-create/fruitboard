# Saved-pattern note fixtures (issue #290)

Status: **Exact F16/F17 bytes and expectations approved on 2026-10-07.**
This completes the fixture qualification slice of the accepted
[note-count design #288](../review/pattern-notes-288/README.md), PR289 merge
`491abefc9d3a4d74337d541efa1ff1c1a2ab89cc`. No note-count extraction,
typed projection, persistence or Library display is delivered by this slice.

## Independent genuine saves

The recipe was registered privately before GUI creation and byte inspection.
Two independent copies of approved F11 were opened in FL Studio **26.1.0.5530**,
with 130 BPM and empty built-in Samplers (File `(none)`). No personal song,
sample, plugin import, recording, artwork, comment or playback was used.
The matching executable's SHA-256 was
`a3ef478a54d660d763113b04dae0a8b985e8610eca213eb3bf0222eed5df4b67`.

Every channel/pattern was inspected before save, with Piano roll ghost display
disabled. After narrow sanitation, both candidates reopened in the matching
GUI without saving or playback. Names, counts, slide properties, enabled steps,
empty sample fields and placements matched; hashes remained unchanged after
closing. Private screenshots and original saves stay outside Git.

| Fixture/case | Registered GUI observation, preserved on reopen | Subsequent byte observation |
| ------------ | ----------------------------------------------- | --------------------------- |
| F16 A, ID 1 | `Fixture Notes One`: 2 ordinary entries; `Fixture Notes Two`: 1. | One event-224 payload, 72 bytes / 3 records. |
| F16 B, ID 2 | One: 1 ordinary entry + 1 overlapping slide; Two: empty. Reopened note properties say `slide to D5`. | One 48-byte payload / 2 records, including slide. |
| F16 C, ID 3 | One: 4 enabled steps and 4 Piano roll entries; Two: empty. | One 96-byte payload / 4 records with length word zero. |
| F16 Playlist | A twice, B once, C once. | Three unique stored pattern contents; four placements. |
| F17, ID 1 | `Fixture Named Empty` survives reopen; no entries or enabled steps; one empty Sampler, no clips. | Property/name marker exists; event 224 is absent. |

The exact filenames, sizes, hashes, provenance/license and owner approval are
in the [manifest](../../fixtures/parser-corpus/manifest.md#saved-pattern-note-follow-ups-f16f17).
The approved [expectation record](../../fixtures/parser-corpus/pattern-note-expectations.json)
is copied byte-identically from the private review packet (SHA-256
`6a7b161bc9d2d6240887a1ae04d5e0978bb4f9fcf072aa41e803a46b6f9a6295`).
No registered value was changed to fit parser output.

## Independent framing qualification

Only after GUI reopen verification, a bounded independent byte walk observed
24-byte records in F16. Ordinary and slide entries each occupy one record.
The demonstrated slide flags word is `0x4008`, versus ordinary `0x4000`.
Each enabled step occupies a record with stored length word zero; a zero-length
record is different from a zero-length payload.

| F16 section | Event-start offsets and ordering |
| ----------- | -------------------------------- |
| Initial content | `(65 ID 1, 224)` at `(358, 361)`; ID 2 at `(435, 438)`; ID 3 at `(488, 491)`. Each ID is immediately followed by its sole payload. |
| Content ends | Event 226 at 589; first channel event 64 at 655. |
| Repeated properties | IDs 1/2/3 at 1539/1657/1775, followed by names/properties, with no new payloads. These blocks occur between channel sections. |
| Later boundaries | Second channel event 64 at 1893; arrangement event 99 at 2777; Playlist event 233 at 2808. |

F17 has only the property/name ID marker at 1270, after its channel section.
It stores no initial content marker/payload and no explicit zero-length payload.
The future count must remain `unavailable / PATTERN_NOTES_NOT_STORED`.
GUI emptiness does not authorize extracted zero. Explicit-zero payload support
remains unqualified and must be rejected as an unverified layout.

This qualifies the demonstrated initial contiguous `(65, 224)` series in the
exact build, not arbitrary intervening sections or controller-interleaved
layouts. The eventual parser must distinguish content from properties, lose
authority at channel/arrangement/Playlist or unknown boundaries, reject
duplicate payloads, and apply the accepted record/processing limits. Repeated
property markers and Playlist reuse never multiply record counts.

## Sanitation, approval and regression boundary

Each candidate differs from its genuine save only by removal of event 200 at
original offset 110: 16 registration bytes plus 2 framing bytes, and the FLdt
length correction. All retained events were independently compared byte-for-byte.
The inherited project-data path is the synthetic Public Documents fixture path.
Latin-1/both UTF-16LE alignment scans found no non-Public home path, account name
or email; signature checks found no embedded RIFF or PNG/JPEG/GIF data; no
registration event remained. Controlled GUI provenance corroborates empty
Samplers. These are bounded checks, not a full decoder of opaque state.
The nonexistent synthetic folder prompt was dismissed with **Do nothing**.

The owner approved the exact two candidate hashes and expectations with
"Approve these exact two fixtures and expectations" on 2026-10-07 under the
[fixture rules](../../DEVELOPMENT.md#parser-fixture-rules). Raw saves and
account-bearing screenshots were not approved for publication and stay private.
The repository privacy policy permits only the exact named paths/hashes.

The current parser also successfully reads registered build, tempo, channel
labels/generators, pattern IDs/names and four/zero clips from both candidates,
preserving hashes. Before publication, this check reused an independently
retained parser whose crate/Cargo/toolchain inputs equal the accepted merge.
The new corpus regression exercises these existing metadata fields read-only;
it does not pretend that the current parser returns note counts.

No new saved build, controller/MIDI/playback behavior, sounding-note total,
duration formula, resource acceptance or Phase 3 checkpoint is qualified.
Parser/full typed validation and immutable Library integration remain the
next separate slices. Broader G6 and other readiness gaps stay open.

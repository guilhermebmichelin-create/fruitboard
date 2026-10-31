# Ordinary saved-note qualification, issue #366

Date: 2026-10-31. Status: **Seven exact sanitized genuine-save fixtures and
observed expectations approved for publication. Product extraction remains
separate.** The owner answered "Continue" to the exact-byte approval packet.
The [manifest](../../fixtures/parser-corpus/manifest.md#ordinary-note-sequence-follow-ups-f24-f30)
owns the seven filenames, hashes, provenance and publication decision.

## Independent GUI evidence

The session used FL Studio **26.1.0.5530**, executable SHA-256
`a3ef478a54d660d763113b04dae0a8b985e8610eca213eb3bf0222eed5df4b67`,
and an independent copy of approved F11. Every case uses 130 BPM, header PPQ
96, Pattern 1 and one ordinary note on a built-in Sampler with File `(none)`.
Recipes and actual GUI properties were registered before saved-byte inspection.
Each sanitized candidate was reopened in the matching application and checked
again before comparing note bytes. No approved fixture was edited.

| Slot / experiment | GUI pitch/key | GUI start | GUI duration | GUI velocity | GUI owner / rack order |
| --- | --- | --- | --- | ---: | --- |
| F24 / N01 baseline | C5 / 60 | 1:01:00 | 0:02:00 | 70% | Sampler |
| F25 / N02 pitch | E5 / 64 | 1:01:00 | 0:02:00 | 70% | Sampler |
| F26 / N03 position | C5 / 60 | 1:03:00 | 0:02:00 | 70% | Sampler |
| F27 / N04 duration | C5 / 60 | 1:01:00 | 0:04:00 | 70% | Sampler |
| F28 / N05 velocity | C5 / 60 | 1:01:00 | 0:02:00 | 58% | Sampler |
| F29 / N06 second channel | C5 / 60 | 1:01:00 | 0:02:00 | 70% | NoteTwo; NoteOne then NoteTwo |
| F30 / N07 reordered | C5 / 60 | 1:01:00 | 0:02:00 | 70% | NoteTwo; NoteTwo then NoteOne |

NoteOne is empty in both two-channel saves. Stored velocity was checked using
the actual note-properties knob hint. Lane-hover values only describe cursor
height and were excluded. The reversible inspection gesture could mark a
project dirty; those inspection edits were discarded without saving.

## Saved-byte result

The [expectation record](../../fixtures/parser-corpus/note-sequence-expectations.json)
separates independent GUI facts from byte observations. It contains all 24
record bytes, the six candidate words, declaration offsets and exact hashes.
Its SHA-256 is
`372c9f99a23844850b512d47f871ab8a3754eca84187c6d8a5fdd844f82caed8`.
Observation tooling source was `309e534244deaa2c37d27f8f9b71ea79134ce1a6`
([PR374](https://github.com/guilhermebmichelin-create/fruitboard/pull/374));
that header-validation repair is a separate change.

| Case | Position u32, offset 0 | Length u32, offset 8 | Key u16, offset 12 | Byte 21 | Channel u16, offset 6 |
| --- | ---: | ---: | ---: | ---: | ---: |
| N01 | 0 | 48 | 60 | 89 | 0 |
| N02 | 0 | 48 | 64 | 89 | 0 |
| N03 | 48 | 48 | 60 | 89 | 0 |
| N04 | 0 | 96 | 60 | 89 | 0 |
| N05 | 0 | 48 | 60 | 74 | 0 |
| N06 | 0 | 48 | 60 | 89 | 1 |
| N07 | 0 | 48 | 60 | 89 | 0 |

All flags words at offset 4 are `0x4000`. Against N01, N02 changes only byte
12, N03 only byte 0, N04 only byte 8, N05 only byte 21 and N06 only byte 6.
N07's note bytes equal N01. N06 to N07 changes only byte 6, from 1 to 0.
Other project sections are outside a note-only comparison's equality claim.

N06 declares saved ID 0 NoteOne and ID 1 NoteTwo; N07 declares ID 0 NoteTwo
and ID 1 NoteOne. The note follows NoteTwo while its saved reference changes.
Resolve a note through declarations in its own save; IDs are not persistent
identities across edits. Sparse/deleted-channel case N08 was not created.

The two observed velocity percentages correspond to bytes 89 and 74. They do
not qualify a general conversion, range, rounding policy or MIDI velocity.
Likewise the two keys and timing values do not qualify every pitch, another
octave-label convention, other PPQs or time signatures. Keep raw saved values
in the next implementation. Slide, portamento, step length, simultaneous or
duplicate notes, arrangement rendering and MIDI export remain unqualified.

The existing research observer/comparator remains numeric and keeps
`musicalFieldsQualified: false`. Semantic evidence comes from this independent
GUI experiment; a constructed mutation or successful parse cannot supply it.

## Sanitation, privacy and retained failures

Every candidate removes only one registration event 200: 16 payload bytes plus
two framing bytes, with the corresponding FLdt body-length correction. Every
retained event, including notes, channel declarations and plugin state, is
byte-identical to its original. Independent Python 3.11.16 verification checked
all seven original/candidate pairs, exact hashes and the unchanged 21 earlier
fixtures. Both text alignments and single-byte scans found no non-Public home
path, known account name, email, embedded RIFF/image signature or registration
event. Opaque plugin state is not fully decoded; the purpose-built empty-Sampler
GUI provenance supplies the content boundary.

Originals, screenshots containing account UI/chooser paths, raw logs and
sanitation receipts remain private outside Git and compiler caches. Only the
seven approved candidates, sanitized expectations and this result are public.
No personal project, imported audio or external plugin was used.

The retained record includes failed GUI focus/capture attempts. One rename
focus error briefly started the empty synthetic pattern; transport was stopped.
An unintended Mixer preset chooser was cancelled without saving. These attempts
are excluded from qualification. A private Node 24.20.0 sanitation helper exited
with native heap-corruption status while converting encoded text; its cause
was initially unestablished. Python independently sanitized the last two cases
and verified all seven. Six pinned comparison-CLI runs completed successfully
afterward. Publication reproduced the same native exit on F29's odd-length
UTF-16 slice; decoding its complete even-length range succeeded. The repository
privacy helper now explicitly bounds both alignments to complete code units.
Regression cases keep private paths at the last complete unit in both size
parities, and all approved corpus files still receive the scan. This addresses
the observed trigger without claiming an upstream Node root cause or fix.

## Regression and resource boundary

[Research tests](../../tests/note-sequence-corpus.test.mjs) preserve every record
byte and the controlled differences. Privacy tests pin exact approved paths,
fixture hashes and expectation bytes and reject changed or relocated files.
[Rust corpus tests](../../crates/flp-parser/tests/corpus.rs) check existing saved
metadata, channel order and one saved note-record count without mutating input.
The actual supervised parser/typed-validation corpus now includes 28 fixtures.
These tests add evidence; they do not expose note sequences in the app.

Local publication checks: 52 focused Node tests passed, with one Linux-only
FIFO test skipped on Windows. The full repository Node run had 186 passes,
the same four tooling-glob failures from stale installed dependencies, and
one platform skip. No dependency installation was attempted below the reserve.
Repository privacy, changed-file formatting, documentation lint, pinned Rust
formatting, script syntax and independent Python approval/hash checks passed.
Rust compilation/tests and integrated builds require fresh CI on this PR head;
PR374's successful checks do not certify the new fixture inputs.

The owner allowed this one GUI fixture experiment below the normal 30 GiB
reserve, capped at 20 MiB of new output and no app build. Retained controlled
experiment output was about 17.9 MiB. The owned FL session closed gracefully;
no app build, dependency install, benchmark or compiler-cache write occurred.
Publication independently copies 327,964 already-approved fixture bytes and
runs lightweight checks. The exception does not authorize a later app build.
The primary `target/` and reusable temporary validation cache remain owned by
`/root`, with write windows closed. No new cache or obsolete compiler output
was created, and no cleanup was performed. Normal updated-head CI is required.

Next, implement bounded ordinary-note extraction and full native validation as
one reviewed slice, then immutable storage and accessible Library display.
Broader build support, installed end-to-end qualification, production activation
and Phase 3 acceptance remain separate gates.

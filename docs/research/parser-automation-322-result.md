# Saved automation GUI fixture F23 — issue #322

Status: **Exact fixture and expectation bytes approved on 2026-10-16.**
This publishes independent GUI evidence and conservative parser regression
coverage. Automation record extraction and Library summaries remain unfinished.
The [manifest](../../fixtures/parser-corpus/manifest.md#saved-automation-gui-case-f23)
owns the two exact files, byte counts, hashes, provenance and dated approval.
F22 / PR320 is separate and excluded from this source tree.

## Independent GUI observations

The recipe was registered before GUI authoring; the actual GUI oracle was
recorded before inspecting candidate bytes or comparing parser output. An
independent approved F11 copy was edited in the matching runnable FL Studio
2026 Producer Edition, version **26.1.0.5530**, About v26.1 build 5530 Windows
64-bit. Base tempo stayed 130 BPM. The All channel filter showed `Sampler`
and `Sampler - Channel volume`; Sampler File was `(none)`.

One native automation clip linked to Sampler channel volume occupied Playlist
Track 1 from bar 1 to bar 2. The GUI showed three control points, excluding
two hollow segment-tension handles:

| GUI point ordinal | Literal displayed B:S:T position | Displayed value |
| --- | --- | --- |
| 1/3 | `001:01:000` | `-5.2dB` |
| 2/3 | `001:09:000` | `-12.8dB` |
| 3/3 | `002:01:000` | `-5.2dB` |

No note, sample, imported audio, external plugin, recording or playback was
added. The current Sampler Pattern 1 Piano roll viewport was observed empty;
that is not an exhaustive inspection of every pattern. Raw and sanitized
saves each reopened in separate matching-build processes without saving or
playback, preserving these observations and their exact hashes. Owned GUI
processes were closed gracefully. Private screenshots and observations remain
outside Git because application chrome can display account information.

## Sanitation, provenance and exact approval

The raw 47,806-byte save remains private. Sanitation removes only event 200
at raw offset 110: sixteen registration payload bytes and two framing bytes,
then corrects FLdt length by eighteen. Independent direct-splice verification
confirms that all retained project-event bytes are identical. The sanitized
file is 47,788 bytes, SHA-256
`23b764e4220fc5e48ffbd4a99c602bda092c032ef35b9b2d5b785250b622cf7f`.

Bounded Latin-1 and both UTF-16LE alignments find no registration event, known
account identifier, non-Public Windows home or email. RIFF/PNG/JPEG/GIF
signatures, nonempty event-196 sample filenames and event-224 note payloads
are absent. These checks and controlled native empty-Sampler provenance do
not completely decode opaque state or establish exhaustive privacy proof.
The nonexistent inherited synthetic Public Documents project-data-folder
prompt was dismissed with **Do nothing**, without saving or rewriting it.

The owner answered **"Yes"** to the exact two-file approval packet on
2026-10-16. Content is owner-controlled purpose-built synthetic material,
with no third-party project/audio material or external attribution requirement.
The approved [expectations](../../fixtures/parser-corpus/fl2026-automation-expectations.json)
are 5,942 bytes, SHA-256
`f59bee2a0f2860cb5fc1739b7381113c2eb0db6d6aec5dc4b3356c1f972b64a9`.
Their preparation-time pending status is preserved byte-for-byte; this dated
decision records subsequent approval. It does not approve raw saves,
screenshots, personal paths or detailed process logs for publication.

## Framing and current parser behavior

Bounded independent framing observes 1,470 retained events, header count two,
channel markers 0/1, 96 PPQ, one event-233 payload of 88 bytes and one
event-234 payload of 205 bytes. F11 has empty event 233 and no event 234.
These are complete event boundaries, **not qualified automation point-record
semantics**. No point stride or count-field relationship is established, and
the unrelated 24-byte note-record format is not used to infer one.

The unchanged parser reports basic metadata as complete: saved version
26.1.0.5530, 130 BPM and two channels. It retains the inferred `Sampler` and
extracted `Sampler - Channel volume` labels with medium aggregate confidence.
The second generator remains unsupported (`GENERATOR_CLASS_UNVERIFIED`).
Playlist clips, endpoint, bars and nominal seconds remain unsupported
(`PLAYLIST_CLIP_KIND_UNVERIFIED`), without guessed values. No automation
summary field is advertised or emitted. A complete basic-metadata outcome
does not mean automation extraction is supported.

The [corpus regression](../../crates/flp-parser/tests/corpus.rs) pins both
approved hashes, independent GUI counts and current refusal, using read-only
file parsing and an unchanged F11 control. The [actual supervisor test](../../crates/flp-parser/tests/supervisor.rs)
passes all twenty corpus files through the real parser and full native reply
validation, including F23's unsupported arrangement fields and unchanged bytes.
The privacy registration permits only the exact F23 path/hash; regressions
reject changed bytes, missing hashes, alternate paths and private home paths.
The existing 2026 repository binary scan remains bounded text inspection;
the complete framing/sanitation review is retained separately.

## Validation and next boundary

Source baseline is merged PR321,
`803ef43c420a3fc7a4f5b3d05228a17b720159d9`. Product parser, native application,
renderer, storage, dependencies, schema, CI and feature flags are unchanged.
Its fifteen passing checks apply only to unchanged source. This publication
requires scoped parser tests/Clippy, Rustfmt, repository Node/privacy/script
and documentation/format checks, plus normal updated-head CI. Source
equivalence avoids a redundant local full app rebuild; updated-head Windows
CI still runs the complete pinned check and packaging smoke. Private logs
retain actual results and the exact source boundary. No installed test or
performance qualification is implied.

The existing sole validation cache remains owned by Codex/root and is reused
sequentially after the four-path storage preflight, retaining at least 30 GiB
after expected output. Evidence remains outside compiler caches; no new cache
or cleanup is introduced. All nineteen prior fixture hashes are preserved.

Next, independently establish complete saved automation-record framing and
count relationships before bounded extraction and full typed validation.
Unknown, ambiguous or opaque layouts stay unsupported. A later separately
reviewed immutable Library slice may display qualified saved counts. Parameter
identity/value/curve evaluation, MIDI, richer arrangements, new saved-build
support, production activation and Phase 3 acceptance remain outside this PR.

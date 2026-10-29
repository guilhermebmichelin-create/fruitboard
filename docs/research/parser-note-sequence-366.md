# Saved note sequences: qualification and first implementation boundary

Issue [#366](https://github.com/guilhermebmichelin-create/fruitboard/issues/366).
Started 2026-10-29 at main `6971eb4008ed57fadbc12b75d6e4cbf4a69fd497`.
The owner requested prioritizing FLP parsing and exploring actual MIDI/note
sequences. This stays inside Phase 3 and preserves all existing phase gates.

Status: **Reproducible numeric observations delivered; musical field
qualification pending. No product note-sequence extraction or MIDI export.**

## Initial result

The selected Rust parser already counts saved note records. It does not decode
or publish their pitch, position, duration, velocity or channel binding.
Approved F16 contains nine 24-byte records in three pattern payloads (3/2/4).
The new research command reproduces numeric observations from those records:

```text
node scripts/research-note-records.mjs
```

This command has no input arguments. It reads only the fixed, exact-hash F16/F17
corpus files, bounds reads to their approved sizes and writes no files. Output
contains fixed fixture IDs, digests, record offsets and numeric candidate words;
it contains no arbitrary strings, paths, plugin state or user data. Results stay
research observations. The command is outside the app and its parser protocol.

| Pattern | Record count | Candidate position words | Candidate channel words | Candidate length words | Candidate key words |
| --- | ---: | --- | --- | --- | --- |
| F16 A, ID 1 | 3 | 0, 96, 192 | 0, 1, 0 | 48, 48, 48 | 60, 64, 62 |
| F16 B, ID 2 | 2 | 0, 24 | 0, 0 | 48, 48 | 60, 62 |
| F16 C, ID 3 | 4 | 0, 48, 96, 144 | 0, 0, 0, 0 | 0, 0, 0, 0 | 60, 60, 60, 60 |

Candidate record offsets are position u32 at 0, flags u16 at 4, channel u16 at 6,
length u32 at 8 and key u16 at 12. Byte 21 is 100 in all nine records; calling it
velocity requires an independently observed velocity-only change. The F16
slide record has flags 0x4008 while its ordinary control has 0x4000.
The header PPQ is 96. These are byte facts, not a complete format specification.

The existing [F16/F17 qualification](parser-pattern-notes-290-result.md)
independently established counts, the demonstrated slide difference and zero
stored lengths for steps. Its original expectations did not register all note
positions, pitches and velocities. The research tests' candidate-word values are
byte regressions, not new independent GUI ground truth. F17 has no note payload;
this must not be promoted into a qualified extracted empty sequence.

No PyFLP implementation was copied or translated. The observer uses the
existing Fruitboard framing and independent approved-fixture observations. It
is not an alternative production parser or a filesystem security boundary.

## Genuine-save experiment

Use independent private copies of approved F11 in exact FL Studio **26.1.0.5530**,
130 BPM, header PPQ 96 and an empty built-in Sampler with File `(none)`.
Register recipes and actual GUI properties before inspecting each new save.
Do not edit existing approved corpus bytes or use personal projects.

| Case | Controlled intended GUI state | Independent observation required |
| --- | --- | --- |
| N01 baseline | One ordinary C5 note; start 001:01:000; duration 000:02:000 | Pitch label, numeric Start/Duration and actual default velocity before save |
| N02 pitch-only | Baseline with E5 | Only the intended pitch changes; all other GUI fields retained |
| N03 position-only | Baseline with start 001:03:000 | Exact Start changes; pitch, duration and velocity retained |
| N04 duration-only | Baseline with duration 000:04:000 | Exact Duration changes; other fields retained |
| N05 velocity-only | Baseline with distinctly lower velocity | Register exact GUI velocity value before saved-byte inspection |

If the GUI uses a different representation, register that actual representation
before inspecting bytes; never rewrite expectations to match decoder output.
After saving, hash each original, sanitize narrowly identified private metadata,
reopen the candidate without saving or playback and recheck all fields. Retain
GUI evidence, original/candidate hashes and a complete sanitation difference
inventory privately. Compare byte changes only after those observations.

The matching installed executable was verified as build 26.1.0.5530 with SHA-256
`a3ef478a54d660d763113b04dae0a8b985e8610eca213eb3bf0222eed5df4b67`.
The first GUI attempt encountered a locked Windows session. Ownership checks
refused input because LockApp held the foreground; no note creation, save or
candidate-byte interpretation was performed. Genuine-save qualification remains
pending an unlocked session. The preregistration and helper stay in private
retained evidence; this report does not claim those fixtures exist.

Prepare a concrete exact-byte privacy/expectation approval packet after
qualification. New FLP bytes enter the public corpus only after the owner's
explicit approval under the [fixture rules](../../DEVELOPMENT.md#parser-fixture-rules).
Starting this research is not approval of future public fixture bytes.

## First product slice after qualification

The [proposed sequence contract](note-sequence-contract-366.md) defines the
per-pattern completeness, field states, channel binding and output budgets that
can be reviewed while the GUI experiments remain pending. It is design only.

Deliver per-pattern saved note data before a song-level sequence:

1. Independently qualify pitch/key, raw start tick, stored duration and velocity
   for ordinary notes in the initial exact-build layout. Qualify binding from
   each stored channel word to a saved channel identity; it is not automatically
   a renderer list index or MIDI channel. Preserve source order and an ordinal
   so simultaneous and duplicate notes are not lost.
2. Design a bounded optional parser field and full typed validation. Existing
   count support permits 65,536 records per pattern and 262,144 total, whereas
   replies and stored projections are capped at 256 KiB. Do not add unbounded
   arrays or silently increase those caps. Prefer complete small-pattern detail
   with an explicit detail-limit state; measure the entire serialized result.
   Existing counts can remain available when details exceed their own limit.
3. Add immutable selected persistence, old-snapshot semantics and native details
   authorization. Keep root/source/digest/session/lease/publication checks.
   A refresh does not reparse or rewrite previous metadata.
4. Add an accessible read-only note list or piano-roll. Unknown data stays
   unknown; neither display nor logging may reinterpret unknown flags as
   ordinary notes. Run the actual parser-to-storage-to-display installed journey
   when integration is implemented.

The initial research does not qualify slide/portamento semantics, zero-length
step duration, fine pitch, color groups, sparse/reordered channels, other PPQs or
other saved builds. These need their own controlled cases. Preserve numeric keys
independently of octave labels: the already observed FL label D5 and candidate
key 62 do not establish a universal octave-name convention.

[Image-Line's export manual](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/fformats_save_export.htm)
confirms that FLPs save note data and that Piano roll data can be exported as
MIDI. Its [Piano roll manual](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/pianoroll.htm)
explains slide behavior and PPQ-dependent timing. Those sources describe musical
behavior, not saved byte offsets. Ordinary MIDI export is a separate follow-on;
slides do not independently sound, and stored zero-length steps must not become
guessed one-tick or zero-duration MIDI notes. Do not apply the project-wide
Prepare for MIDI export macro to approved source fixtures: it replaces channels.

Whole-song reconstruction additionally requires qualified placements, repeats,
clip offsets, clipping, mute state, arrangement selection, time signatures and
tempo changes. Pattern definitions are counted once; song occurrences are a
separate transformation. Existing automation research #324, richer arrangements,
compatibility, reference handling and resource qualification remain open.

## Validation and handoff boundary

The observer caps files at 64 MiB, events at 100,000, interpreted payloads at
2 MiB and retained note records at **32 total**. It rejects unsupported builds,
unbound/reopened content, duplicate pattern payloads, malformed lengths and
unqualified empty/fractional records. This tiny research cap is not a product
budget. Its CLI rejects arbitrary input paths without reflecting them in errors.

Required checks for this scripts/tests/docs-only change are the pinned Node
script/policy suite, script syntax, privacy/hash check, scoped formatting and
Markdown lint, plus normal updated-head CI. No Rust/app build, package install,
database mutation or installed-app test is needed. Existing main CI covers only
unchanged product source; it does not validate a changed PR head or musical
field qualification.

Local Node 24.20.0 validation passed: all 164 script/policy tests (including
15 new observer tests), syntax checks for all 38 script/test files, scoped
Prettier and Markdown lint, repository privacy and `git diff --check`. The
observer reproduced F16's nine records and F17's absent payload; approved
fixture digests match. Updated-head CI remains required. No genuine-save,
musical-field, app-integration or performance pass is claimed.

The worktree and new evidence are owned by `/root`; actual absolute locations
are recorded in the private handoff. No writable compiler cache is used. The
existing primary cache and reusable `desktop-feature-tests` cache remain
unchanged. A four-path fixture preflight passed with 33.53 GiB available and
0.02 GiB output allowance, preserving the 30 GiB reserve. No other FL Studio or
heavyweight build process was observed before the owned GUI launch. Future
builds require a fresh preflight and a named exclusive cache write window;
installed/resource runs retain their host-lock and all-process quietness rules.
No cleanup or deletion of source, profiles, approved fixtures or caches is
authorized by this research.

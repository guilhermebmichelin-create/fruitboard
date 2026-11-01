# Ordinary-note coverage pilot result

Issue: [#381](https://github.com/guilhermebmichelin-create/fruitboard/issues/381).
Date: 2026-11-01. Status: **Fixture publication approved after merged PR382;
runtime support is unchanged.**

The [protocol](ordinary-note-coverage-381.md) was registered at `6dfb206`, before
authoring or inspecting new note bytes. The application baseline is merged
PR380, `e2ba0ee`; its code matches the tested `e960cec` tree. The installed
executable matched the registered FL Studio **26.1.0.5530** hash. Every case
started from an independent private copy of approved F24, at 130 BPM/96 PPQ,
using empty built-in Samplers. No playback, imported audio or external plugin
was used. The original report PR382 contained no FLP bytes, screenshots or
private receipts. The approved publication follow-up is recorded below.

## Outcomes

Eight originals were genuinely saved, narrowly sanitized, reopened without
saving inspection edits, then decoded with the existing bounded research tool.
Independent Python 3.11.16 decoding corroborated all thirteen record payloads,
channel declarations, original/candidate hashes and sanitation differences.
Actual GUI observations were registered before new note-byte inspection.

The common GUI baseline is C5, Start `1:01:00`, Duration `0:02:00`, velocity 70%.
Its raw position/length/key/velocity values are `0 / 48 / 60 / 89`.

| Case | Reopened GUI outcome                                               | Raw result and qualification                                                                                  |
| ---- | ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------- |
| N09  | D5/62, baseline timing, displayed 70%                              | Key `62`, velocity `90`; bytes 12 and 21 changed. **Failed pitch-only control.**                              |
| N10  | C5, Start `1:02:00`, baseline duration/70%                         | Position `24`; only byte 0 changed. Clean start case.                                                         |
| N11  | C5, Duration `0:01:00`, baseline start/70%                         | Length `24`; only byte 8 changed. Clean duration case.                                                        |
| N12  | C5, baseline timing, actual properties-knob hint 100%              | Velocity `128`; only byte 21 changed. Clean velocity case.                                                    |
| N13  | Three-note C5/D5/E5 melody, all 70%                                | `(start,length,key)` = `(0,48,60)`, `(48,48,62)`, `(96,96,64)`. Clean melody case.                            |
| N14  | Simultaneous C5/E5/G5, baseline timing/70% each                    | Three records at position `0`, length `48`; source key order `67,64,60`. Clean chord case.                    |
| N15  | Two retained coincident C5 notes with matching observed properties | Raw byte 19 is `64` in one record and `0` in the other. **Exact identical duplicate not qualified.**          |
| N16  | DeleteMe removed; NoteTwo retains the baseline note; Reserve empty | Saved IDs are `0` NoteTwo, `1` Reserve; note resolves to ID `0`. Clean ownership case; no sparse-ID evidence. |

Every observed flags word is `0x4000`. These are exact observations, not a
qualification of arbitrary flags, pitches, velocities, lengths, PPQs or builds.
The tools continue to report `musicalFieldsQualified: false`; musical facts
come from the independently recorded GUI observations.

## Failed controls and limits

N09's displayed 70% did not guarantee the baseline raw velocity byte. The
properties-knob gesture used to expose its hint may have affected quantization;
this pilot does not isolate the cause. Both raw `89` and `90` displayed 70% in
the observed files. Do not label this a clean pitch differential or infer a
general percentage conversion. D5/62 is also observed in N13 with raw velocity
`89`, but the failed N09 control remains retained.

N15's cardinality was established through the GUI: separating one coincident
note exposed the second, including after sanitized reopen. The two visible
notes have the intended observed properties, yet their complete records differ
at byte 19. Its meaning is unqualified. Do not normalize, ignore or silently
drop that byte or record. A later decoder must preserve whole-pattern refusal
until the additional state is independently qualified.

The chord is saved in descending pitch order despite ascending authoring order.
Preserve source ordinal; do not infer chronology or note identity from a sort.
Deletion renumbered channels in this case. Saved IDs remain local file bindings,
not persistent channel identities, and this supplies no sparse-ID support.

Initial focus/capture and asynchronous-load failures remain in the private
evidence. A navigation-guard failure caused a wrong-file inspection; its
in-memory edits were discarded, its captures excluded, and final candidate
hashes remained unchanged. No failed attempt was removed or counted as a pass.
The owned FL process closed gracefully with inspection edits discarded.

## Privacy and publication boundary

Each candidate differs from its pinned original only by removal of one
16-byte registration event 200, its two framing bytes and the FLdt length
adjustment. Every retained event, including notes and channel declarations,
is byte-identical. Eight candidates pass independent Latin-1 and both UTF-16
alignment checks for account/home paths and email, plus registration, RIFF
and image-signature checks. Empty-Sampler provenance supplements those bounded
checks; this is not a complete decoder of opaque plugin state.

The private exact-byte approval packet lists eight proposed names, lengths,
SHA-256 hashes, thirteen raw records and corresponding GUI observations. It
distinguishes six clean cases from N09/N15 as negative controls. Publication
approval was pending when PR382 was prepared. Originals, screenshots, scripts,
failures and private receipts remain outside Git. Existing approved fixtures
and the primary dirty documents were preserved.

On 2026-11-01 the owner reviewed and merged PR382, then requested the next step
in response to that exact-file approval packet. The eight sanitized candidates
are published as F31-F38 in the [manifest](../../fixtures/parser-corpus/manifest.md#ordinary-note-pilot-follow-ups-f31-f38).
The [public expectation record](../../fixtures/parser-corpus/ordinary-note-coverage-expectations.json)
pins their names, hashes, GUI facts, thirteen complete records and six
clean/two-negative classification. Public fixture content totals 373,802 bytes.

Publication adds read-only research/privacy regressions and expands the actual
supervisor corpus from 28 to 36 files. Native regression expectations preserve
seven unsupported pattern sequences (N09-N15) and the already-supported N16
NoteTwo note. Counts and metadata survive refusal; N15 never exposes only its
supported sibling. Parser/runtime logic, raw-value limits, native validation,
storage and Library behavior are unchanged. New native tests and fixture inputs
require normal updated-head CI; no local app/Rust build or cache write is
needed for this publication. Prior CI covers unchanged implementation only.

After this fixture PR is reviewed and merged, a separate runtime
slice can consider the sampled new values and multi-note layouts while
retaining strict unknown-record refusal. No general MIDI conversion, runtime
range widening, sparse IDs, alternate build/PPQ, steps or slides are approved
by this report. Installed Library verification and phase/resource qualification
remain separate work.

## Resources and checks

The owner chose to restore the reserve before fixture generation. Selective
cleanup removed 1,388 verified obsolete September compiler library/metadata
files, recovering **2.49 GiB measured**. Single-link identity, exclusive file
access, exact paths, timestamps and hashes were checked before removal. All
127 protected hashes and both worktree states were verified afterward.

The four-path preflight passed with **30.07 GiB free**, projecting **30.04 GiB**
after the conservative 32 MiB allowance. The GUI finished with about 30.04 GiB
free. A later reporting-time check found that available capacity had fallen
below reserve again; the cause of that volume change was not established.
Further fixture work remained stopped. A second exact inventory removed 529
retired compiler intermediates, including Cargo build-script helpers and their
debug symbols, recovering another **0.64 GiB measured**. Complete hardlink
groups were counted once; two incomplete/external alias groups were preserved.
All 291 protected hashes and both worktree states passed verification. C:
then had **30.28 GiB free**. Reusing the retired profiles requires recompilation.
Application binaries, retained debug symbols/evidence, active caches,
dependencies, source and fixtures remained protected.

The pilot retained approximately 27 MiB including both cleanup receipts, below
the 32 MiB cap. Exact measurements and absolute paths are in the private handoff.
No app build, dependency install, compiler-cache production write, benchmark or
installed-app test ran. No new compiler cache was created.

Owner `/root` coordinated the exclusive GUI window. The existing empty
`fixture-generation/tmp` location was checked for preflight only and remains
unwritten. Primary `target` and reusable `desktop-feature-tests` cache write
windows remain closed; the latter's earlier hardlink-isolation limitation is
not cleared by this GUI preflight. A future build needs its own output budget,
cache-isolation check and exclusive ownership window.

PR382 changed only this report and its protocol link/status; scoped
Markdown/format/privacy/whitespace checks covered those documents. The fixture
publication changes and native regression coverage are listed above. Prior
fifteen green checks cover unchanged implementation only; normal required
checks must verify the publication PR head and its new fixture/test inputs.

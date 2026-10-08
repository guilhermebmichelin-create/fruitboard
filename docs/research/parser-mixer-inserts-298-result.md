# Genuine saved mixer fixtures (issue #298)

Status: **Exact F18-F21 bytes and expectations approved on 2026-10-08.**
This completes the fixture slice of the accepted
[mixer design #296](../review/mixer-inserts-296/README.md), PR297 merge
`8336797950934719e4544b3ea4278c6413dd0131`, tree
`d260a0b66fa201c9d241f049c6b95198a1241492`. This PR also presents the narrow
positional identity mapping for review before implementation. It adds no
parser fields, native validation, storage, client or Library support.

## Independently registered GUI cases

Recipes and matching executable/baseline hashes were recorded privately before
creation and mixer-byte inspection. Four independent approved F11 copies were
saved through FL Studio **26.1.0.5530**: 130 BPM, one built-in Sampler with File
`(none)`, no notes/steps, Playlist clips, personal song, imported audio, external
plugins, effects, routing edits, recording, artwork, comments or playback.
The executable's SHA-256 is
`a3ef478a54d660d763113b04dae0a8b985e8610eca213eb3bf0222eed5df4b67`.
F20 activates the registered duplicate-name case; F21 was separately registered
to identify special tracks before creation and byte inspection.

| Fixture | Matching GUI observation | Subsequent independent structural observation |
| --- | --- | --- |
| F18 untouched mixer | Sixteen ordinary inserts, default Master/Current and visible insert labels | Eighteen complete records; sixteen ordinary name events omitted |
| F19 named inserts | Positions 2/7 retain `Fixture Mixer A` / `Fixture Mixer B`; channel/pattern retain `Fixture Channel Context` / `Fixture Pattern Context` | Name events at section ordinals 2/7; channel/pattern names outside the mixer section |
| F20 duplicate names | Distinct positions 2/7 both retain `Fixture Mixer Same` | Two distinct records with identical text, not a deduplicated name count |
| F21 special tracks | Master/Current retain `Fixture Master Context` / `Fixture Current Context`; ordinary inserts untouched | Special names at section ordinals 0/17; all sixteen ordinary name events omitted |

Originals were reopened without saving/playback. A fresh process opened F18
then F21 to distinguish saved Current state from session state before byte
inspection. Exact sanitized candidates subsequently repeated that fresh
contrast, followed by F19/F20. All original and sanitized hashes stayed unchanged.
Current's narrow column hides its custom label; selecting it shows the full
name in the mixer title. An initial inference from the narrow column was
corrected before byte inspection. F19 also exposes the full channel name in
Sampler controls, File `(none)`, full pattern name and an empty Playlist.

GUI establishes labels and roles, not the number of saved records. The bounded
independent walk and controlled comparisons below establish the explicit count.
No Fruitboard or other decoder output supplied ground truth, and no unofficial
implementation was copied, translated or used as a dependency.

## Complete framing and identity observation

Checked FLhd/FLdt lengths, bounded event counts and payload lengths frame each
file. In this exact build event 172 has its separately qualified three-byte
width; event 199 must identify 26.1.0.5530 before that width is accepted. This
does not qualify framing in any additional build.

Each case has one two-byte event 103 with value **18**. The immediately
preceding sequence is 242 (28-byte payload), 100 (two-byte value 0), then
one-byte events 29/39/40/31/38 with values 1/1/0/0/1. That boundary is identical
across all four saves; a bare count/name event in another context is not
authority. No intervening unknown transition is qualified. The complete
record series immediately after 103 is:

| Component | Observed representation |
| --- | --- |
| Start | Event 42, one-byte payload |
| Optional name | Event 204, UTF-16LE with one trailing NUL, between 42 and 236 |
| Body | Event 236, exactly 12 payload bytes |
| Empty effect-slot controls | Ten event-98 two-byte values, in order 0 through 9 |
| Remaining controls | Event 235, then 165/166/49/154/147 |
| Following records | Eighteen complete sequences in total |
| Termination | Event 225 with 6,924 payload bytes, then 133/243/47 and exact file end |

Every non-name byte of each record is identical across all four saves. The
event-236 body is `000000004c00000000000000` for ordinary records and
`000000000c00000000000000` for both special records. These are controlled
observations, not a general flag-bit, routing or audio interpretation. Renaming
only adds a name event to the corresponding unchanged record sequence. F19's
channel name uses event 203 and pattern name uses 193 outside this section;
neither can authorize a mixer name.

**No independent per-record numeric ID payload was observed.** Identity is
position in the complete saved sequence. Controlled labels bind GUI positions
2/7 to section ordinals 2/7 and Master/Current to ordinals 0/17. Those roles
and all eighteen complete records establish **sixteen explicit ordinary
insert records**. The count is not inferred from GUI defaults, custom names,
the highest named position, or whether tracks are used or audible.

Only this complete eighteen-record layout is qualified. Other totals/orderings,
sparse numeric identity, stable IDs across add/delete/reorder edits, explicit
zero/empty names, populated effects, routing semantics and other builds remain
unqualified. Constructed mutations can test rejection but cannot expand
genuine compatibility.

## Proposed mapping for later implementation

The accepted design made the actual identity representation conditional on
qualification. This clarification is reviewed in the corpus PR before coding:

- `savedInsertId` denotes the record position within the complete qualified
  saved section, not a separately stored scalar or stable ID across edits.
- In this observed layout only, ordinary IDs are exactly **1 through 16**,
  count 16. Master position 0 and Current position 17 are excluded.
- Every ordinary record receives one name entry, including omitted names.
  All four fixtures have incomplete names aggregates. F19/F20 have two named
  entries; F18/F21 have none. Missing names use
  `MIXER_INSERT_NAME_NOT_STORED`, never generated `Insert N` labels.
- Limits charge all eighteen candidates before filtering special records.
  The 512 ceiling does not authorize other layouts/IDs. Full native, selected
  storage and renderer validation must enforce the exact ordinary ID set,
  count 16 and sixteen corresponding name entries.
- Entry, complete framing/order and qualified termination must all hold.
  Unknown transitions, extra name assignments, repeated sections or incomplete
  records never authorize a partial result; unknown layout/binding states
  remain unsupported as designed.

The [approved expectations](../../fixtures/parser-corpus/mixer-insert-expectations.json)
record `sectionRecordOrdinal`, not fabricated scalar IDs or parser output.
Any broader mapping requires separately registered genuine evidence and a
reviewed contract update. The proposed API name remains unchanged, with its
positional scope explicit.

## Exact sanitation, public approval and validation

The [manifest](../../fixtures/parser-corpus/manifest.md#saved-mixer-insert-follow-ups-f18-f21)
pins filenames, sizes, hashes, provenance and license. The four FLPs total
186,714 bytes. The unchanged expectations are 19,969 bytes, SHA-256
`cb89b238d9c7b836a6a5ddebaf044ae3f95559b0943364b4faa098fac1cb9d8e`.
Their pending-publication text records preparation time; the owner subsequently
answered **"Yes"** on 2026-10-08 to the exact packet's publication question.
No expectation bytes were rewritten after approval.

Sanitation removes event 200 at offset 110 (16 registration bytes plus two
framing bytes) and corrects FLdt length by 18. Every retained event remains
byte-identical, including mixer data and unrelated metadata. The inherited
folder remains the synthetic Public Documents fixture path; its missing-folder
prompt was dismissed with **Do nothing**, without saving or path changes.

Bounded Latin-1/both UTF-16LE scans find no non-Public home path, known account
identifier or email. Known text fields contain only version information,
controlled names, the synthetic Public path, `Unsorted`, `Arrangement` or
empty text. No registration event, RIFF/PNG/JPEG/GIF signature, event-196 sample
filename or event-224 note payload remains. Each header has one channel and
event-233 Playlist payload is empty. Matching GUI and empty-Sampler provenance
corroborate these checks; this is not a complete decoder of opaque state.
Raw originals, account-bearing screenshots and diagnostics remain private.

Policy allows only the exact named paths/hashes. Tests pin the expectation
hash, scan all nineteen fixtures and reject changed FLP bytes or approved
bytes at another path. All fifteen prior FLPs and expectations remain unchanged.
No dependency, protocol or app behavior changes. No local build is needed for
extraction/app code. The existing supervised corpus integration test's
maintained-file count changes from fifteen to nineteen; a targeted local
parser integration build verifies all nineteen real supervised replies,
full metadata validation and unchanged bytes. Pinned privacy/docs/script/policy
checks, parser all-target Clippy and normal updated-head CI apply. Prior PR297
CI covers unchanged product code only.

Broader G6, Phase 3 exit, resource budgets, production activation, licensing/
distribution and performance acceptance remain open. After owner review/merge,
bounded parser extraction and full native validation come next; immutable
Library integration and installed qualification follow later.

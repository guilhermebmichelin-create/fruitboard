# Native automation reorder and removal controls

Issues: [#324](https://github.com/guilhermebmichelin-create/fruitboard/issues/324)
and [publication #417](https://github.com/guilhermebmichelin-create/fruitboard/issues/417).
Status: **Three exact sanitized files and expectations approved on 2026-11-16;
controlled association/empty-set evidence, no runtime automation field.**
Authoring/publication baseline: merged PR416,
`341f4494df5f7e0224e50176bf98e43ad92e34d5`.

## Independent GUI controls

This continues the [object/minimum-point examples](automation-objects-324-result.md)
in the installed FL Studio **26.1.0.5530**, at 130 BPM. About and executable
hash matched prior authoring evidence. Each case began from an independent copy
of approved F44: one empty native Sampler and two volume curves with two and
four points. No notes, sample file, imported audio, external plugin, personal
project or playback was introduced. Additional-build experiments stay deferred.

Operations and expected controls were preregistered privately. Actual named
channel order, curve shapes, editable-point counts and occurrences were recorded
before saving and automation-byte analysis. Hollow tension handles are excluded.

| Case / slot | Native GUI operation and reopened result | Objects / editable points / occurrences | Event 234 payload bytes | Event 233 bytes |
| --- | --- | --- | --- | ---: |
| C01 / F46 | Move the four-point channel before the two-point channel; both curves and occurrences remain | 2 / 4+2=6 / 2 | 229 + 181 | 176 |
| C02 / F47 | Delete only the original two-point channel; named volume #2 retains four points and its occurrence | 1 / 4 / 1 | 229 | 88 |
| C03 / F48 | Delete both automation channels, retaining only the Sampler; picker and Playlist empty | 0 / 0 / 0 | None | 0 |

C01's GUI order became Sampler, volume #2, volume. The four-point occurrence
stayed at Track 1 bars 3–4 and the two-point occurrence at bars 1–2. The native
Alt+Up operation is documented in the [Channel Rack manual](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/channelrack.htm).
C02/C03 used explicit Channel Rack Delete commands, inspecting confirmations
without selecting Remember my choice. This removed definitions rather than only
occurrences. In C03, both available filters, **All** and **Unsorted**, showed one
Sampler; the explicitly selected automation picker had no entries.

All three raw and all three sanitized files reopened unchanged in the matching
GUI without saving or playback. Each retained curve's final point showed
`002:01:000`, -5.2 dB, with the expected Point2/2 or Point4/4 ordinal. Other
internal coordinates were not all independently remeasured. The inherited
synthetic data-folder prompt was acknowledged with **Do nothing**. The owned
FL process closed gracefully.

## What the bytes distinguish

Independent bounded Node and Python walks agree on framing, direct-splice
sanitation and these observations. The [fixed-input regression](../../tests/automation-differential-fixtures.test.mjs)
reproduces them from hash-pinned files and independent GUI expectations:

- F44 declares channel types 0,5,5 with point lists 2,4. C01 still declares
  types 0,5,5 but its stored names and point bodies occur in order #2(4),
  original(2). FL reassigns saved IDs: #2 changes 2→1 and the original 1→2.
  The four-point payload is byte-identical to F44's; the two-point payload
  differs only at offset 67, whose meaning remains unqualified.
- C02 retains only the named #2 definition after the Sampler, now saved ID 1
  instead of 2. Its 229-byte point payload differs from F44's #2 only at
  offset 115; the byte is retained without interpretation.
- C03 has one declared type 0 channel matching its header count, no event 234
  and empty event 233. Its observed declaration/curve/Playlist structure matches
  approved F11's no-clip control. Both event walks reach EOF; the declaration
  IDs are unique, types singular and no orphan event 234 appears in these inputs.

The scoped association clears at observed channel/section boundaries
64/65/98/100/233/236. Controlled reorder/removal is stronger evidence than a
nearest-marker coincidence, but it does not establish target/parameter binding
or a complete general recognizer. Numeric IDs are not persistent cross-save
object keys. Present payloads retain the observed **21 + 24 × n + 112**
relationship, stable first 17/trailing 112 bytes and u32LE count candidate at 17.

C03 is a controlled **empty native clip object set**, not an explicitly empty
point list on a present object, nor evidence that event automation, MIDI or
initial-controller state are absent. Unknown classes/signatures, missing,
duplicate, orphaned or malformed records cannot silently become zero.

## Exact approval, privacy and failed attempts

On **2026-11-16** the owner answered **"Yes"** to the exact three-file packet.
The frozen [expectations JSON](../../fixtures/parser-corpus/automation-enumeration-expectations.json)
is **5,942 bytes**, SHA-256
`1e9978b757073e885e55659a371a0ea15a1e9ff30525d3733ccb33ad0988831c`.
Names, sizes, hashes and owner-controlled synthetic provenance are in the
[manifest](../../fixtures/parser-corpus/manifest.md#automation-enumeration-controls-f46-f48).
All 43 earlier FLPs and all existing expectations retain their hashes. The
corpus now contains 46 files across the same three exact saved builds.

Sanitation removes only one registration event 200 with 16 payload bytes and two
framing bytes and corrects FLdt length. Independent direct-splice checks prove
every retained event byte identical. Bounded Latin-1 and both complete UTF-16LE
alignments found no known account identifier, email, non-Public home path,
RIFF/image signature, nonempty sample filename or note payload. Opaque state is
not exhaustively decoded. Raw saves, screenshots and detailed receipts stay
private outside Git.

An initial sanitizer stopped after C01 without completing the summary or other
candidates. Its unsafe odd-length UTF-16 slices were replaced with complete
code-unit ranges; the exact initial termination cause was not independently
reproduced. C01 remained byte-identical, and corrected independent checks
completed all three cases. Premature missing-file GUI attempts and stale hints
were excluded, file-existence/exit guards added, and fresh verified captures
completed the actual reopen checks. No failed attempt was saved as qualifying
source. Initial/failed receipts remain private for traceability.

## Parser boundary and next review

Publication changes only fixtures, tests, exact privacy admission and matching
documentation. Real-parser and full supervised native-validation regressions
cover all 46 files read-only. Basic metadata and GUI channel order remain
available. Retained automation cases preserve unsupported generator/Playlist
interpretation. F48 retains ordinary empty-pattern-Playlist handling and
unavailable pattern extent; it adds no automation numeric zero or new field.

The next step is review of a strict recognizer for this tested family: exact
build/framing, complete unique declarations with header agreement, singular
recognized classes, scoped point payloads and reviewed signatures/count bounds.
Constructed rejection cases must cover missing/duplicate/orphan records,
unfamiliar state, contradictory lengths/references and arithmetic/aggregate
bounds. Zero requires a complete recognized empty set, not record absence alone.
Review those conditions before separate extraction/native validation, followed
by immutable persistence/Library integration. Keep #324 open. No public field
contract, phase acceptance, production activation or distribution is delivered.

## Validation and storage boundary

Owned scope is the three exact FLPs/expectations, privacy/tests and associated
documentation in the primary checkout. Checks include root Node tests, script
syntax, privacy, scoped formatting/Markdown, whitespace, Rustfmt, release-profile
default/research parser tests and warning-denied all-target Clippy. Full
updated-head CI is required. Earlier CI covers unchanged application code only;
no installed/performance qualification is extended.

Owner `/root` sequentially reuses `desktop-feature-tests` with explicit
CARGO_TARGET_DIR, preserving primary `target`. Four-path preflight budgets
0.50 GiB beyond current usage while preserving the 30 GiB reserve. One native
build/test window runs at a time. Absolute cache/evidence paths, commands,
source hashes and private receipts are in the `automation-enumeration-20261116`
publication handoff outside compiler caches. Cache outputs remain reusable;
private raw/candidate files and previous evidence must remain. No new permanent
cache or obsolete-output cleanup is introduced. Cargo-created internal aliases
are verified and isolated with independent copies after the write window.

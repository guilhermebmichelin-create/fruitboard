# Saved automation object and minimum-point examples

Issues: [#324](https://github.com/guilhermebmichelin-create/fruitboard/issues/324)
and [publication #415](https://github.com/guilhermebmichelin-create/fruitboard/issues/415).
Status: **Three exact sanitized files and expectations approved on 2026-11-15;
research evidence, no runtime automation field.**
Authoring source: merged `db1c35f0b7c5b5e2094fef00bc8d34e7ba203a84`;
publication baseline: `d7e76955ca2563214e45120c455c6e84921f5d74`.

## Independent genuine-save observations

This continues the [point/placement differentials](automation-differentials-324-result.md)
in the installed FL Studio **26.1.0.5530**, at 130 BPM. About and the executable
hash matched prior evidence. Each case began from an independent copy of an
approved synthetic file: F23 for B01/B03 and F39 for B02. Only an empty native
Sampler and its volume automation were used. No personal project, notes,
sample file, imported audio, external plugin or playback was introduced.
Additional-build work remains deferred.

Operations and intended outcomes were preregistered privately before authoring;
actual GUI observations were frozen before saving and automation-byte analysis.
Hollow tension handles are excluded from editable point counts.

| Case / slot | GUI operation and observed result | Unique objects / points | Event 234 payload bytes | Event 233 bytes |
| --- | --- | --- | --- | ---: |
| B01 / F43 | Move only the middle point vertically, retaining its displayed time and both endpoints | 1 / 3 | 205 | 88 |
| B02 / F44 | Place a second occurrence, Make unique, then add two points only to that new curve; original stays at two | 2 / 2+4=6 | 181 + 229 | 176 |
| B03 / F45 | Delete middle point and final endpoint; native Delete is disabled on the remaining Point 1/1 | 1 / 1 | 157 | 88 |

B01's middle point changed from -12.8 dB to -2.5 dB at `001:09:000`;
endpoints stayed -5.2 dB. B02 reopened two separately named curves, the original
at Track1 bars1–2 and `#2` at bars3–4, with two and four points respectively.
Editing the new curve left the original GUI shape unchanged. B02's new Point2/4
displayed `001:05:000`, -11.5 dB before saving and `001:05:001`, -11.5 dB on
reopening both unchanged raw and sanitized saves. The exact saved expectations
retain the latter actual observation; no timing decoder or explanation of that
one-tick difference is claimed. Point3/4 displayed `001:13:001`, -1.4 dB.

B03 did **not** achieve an empty curve. The final point remained at
`001:01:000`, -5.2 dB and its Delete command stayed disabled after reopening.
The original occurrence no longer had a visible nonempty rectangle, but the
saved Playlist payload still contained 88 bytes. No occurrence was manually
deleted. Its saved placement count is unqualified (`null` in expectations),
not zero. This authoring path supplies no explicit empty-list or general
automation-zero evidence.

All three raw and all three sanitized saves reopened in the matching build
without saving or playback, with unchanged hashes; B02 also had a raw timing
recheck. The inherited synthetic project-data-folder prompt was acknowledged
with **Do nothing**. The owned process closed gracefully.

## What the bytes establish

Independent bounded Node and Python event walks agree on complete framing,
exact sanitation, hashes and the following observations. The
[fixed-input regressions](../../tests/automation-differential-fixtures.test.mjs)
reproduce them from approved bytes and independently observed GUI expectations:

- Event234 u32LE offset 17 agrees with each observed point-list count. The
  relationship **21 + 24 × n + 112** now holds for observed counts one through
  four in these exact files, with the same first 17 and trailing 112 bytes as F23.
  Offset 13 remains three; it does not track the changed point count.
- B01 preserves its 205-byte payload length/count while changing offsets 56–59
  and 67. Its event233 payload is byte-identical to F23. This is a same-count
  content differential, not a time/value decoding rule.
- B02 has two event234 payloads with counts 2/4, consistent with its independently
  reopened two objects and six total points. Distinct preceding event64 markers
  and event203 names are observed, but chronology alone does not qualify a
  complete channel/object/target binding or enumeration grammar.
- B02's original two-point payload differs from F39 only at offset 67 despite
  unchanged GUI shape. That byte's meaning is unqualified; it is retained.
- B03 retains nonempty event233 data even with no visible nonempty rectangle.
  Its bytes 8,9,32 differ from F23. No duration or zero-placement semantics are
  inferred from those differences.

An initial private marker report incorrectly read two bytes from one-byte
event0 and selected event201 rather than the stored channel-name event203.
Those marker fields are excluded; a corrected event64/203 report with bounded
width checks was independently verified. The failed report remains private for
traceability. Initial loading/scaling and stale direct-rack paints are excluded.
A temporary Pattern1 occurrence was immediately undone before the correct
automation-picker operation and was never saved as evidence.

## Exact approval and privacy

On **2026-11-15** the owner answered **"yes"** to the concrete exact-byte packet.
Approval covers only these three sanitized candidates and the exact
[expectations JSON](../../fixtures/parser-corpus/automation-object-expectations.json),
**6,777 bytes**, SHA-256
`c206b82e18abdaf11aa8fbc32c3a695e603824bd0c69f4b84421f43f4bc6dfd7`.
The [manifest](../../fixtures/parser-corpus/manifest.md#automation-object-examples-f43-f45)
records filenames, hashes, sizes and provenance.

Each candidate removes only one registration event200 with 16 payload bytes
and two framing bytes, correcting FLdt length. Two independent direct-splice
checks prove every retained event byte identical to its raw save. Bounded
Latin-1 and both UTF-16LE alignments found no known account identifier, email or
non-Public home path, RIFF/image signature, nonempty sample filename event or
note payload. Opaque state is not exhaustively decoded. Raw saves, screenshots,
private paths and detailed receipts remain outside Git. All 40 earlier FLPs
remain unchanged; the corpus now has 43 files across the same three saved builds.

Slash-separated privacy prose in the frozen expectations resembles a Unix home
path to the repository scanner. Only that exact phrase in the exact approved
file/hash is exempted from that match. Tests retain rejection for changed bytes,
other paths, actual personal paths and credentials; the approved JSON is unchanged.

## Parser boundary, validation and next step

The subsequent [reorder/removal controls](automation-enumeration-324-result.md),
approved on 2026-11-16, add F46-F48. They strengthen controlled channel/name/curve
association and add an independently observed empty native clip set. This
earlier three-file result and its exact expectations remain unchanged.

Publication changes fixtures, privacy admission, tests and documentation only.
Direct parser and full supervised native-validation regressions preserve
complete basic metadata, unsupported automation generator/Playlist
interpretation and the absence of automation numeric fields. F45 must not take
F42's recognized empty-pattern-Playlist path: its saved payload is nonempty.

Checks cover root Node tests, script syntax, scoped formatting/Markdown,
privacy/whitespace, Rustfmt, release-profile parser tests and all-target
warning-denied Clippy in default and compatibility-research configurations.
Full updated-head CI is required. Earlier application CI covers unchanged code
only; installed/performance qualification is not extended.

Owner `/root` reuses the existing `desktop-feature-tests` validation cache
sequentially with explicit CARGO_TARGET_DIR, preserving primary `target`.
Four-path preflight budgets 0.30 GiB after inspecting compatible cached test
outputs and maintains the 30 GiB reserve. One native build/test window runs at
a time; there is no installed/performance window. Absolute paths, commands,
source hashes and retained private evidence are in the
`automation-bindings-20261115` publication handoff outside compiler caches.
Prior storage repair reclaimed two obsolete October 28 debug link/symbol outputs
and made internal Cargo aliases independent; future desktop debug linking may
regenerate them. Remaining cache outputs are reusable; prior evidence remains
retained. Cargo recreated internal same-cache aliases during checks; those were
verified internal and independently copied after the write window closed,
without changing their bytes. No new permanent cache or additional obsolete-file
cleanup is introduced.

Keep #324 open. Complete object-set boundaries and binding, explicit zero/empty
lists, malformed/ambiguous relationships, LFO and other targets remain
unqualified. Review that grammar before a separate extraction/native-validation
and immutable Library change. Resources, Phase 3 exit, production activation and
distribution remain open.

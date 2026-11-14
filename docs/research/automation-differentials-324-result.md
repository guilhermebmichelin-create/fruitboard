# Saved automation point and placement differentials

Issues: [#324](https://github.com/guilhermebmichelin-create/fruitboard/issues/324)
and [publication #413](https://github.com/guilhermebmichelin-create/fruitboard/issues/413).
Research collected 2026-11-12 against merged
`30cb3f90846926f908e611750e12ecb38c1e757d`.
Status: **Four exact sanitized files and expectations approved on 2026-11-14;
research evidence only, no new runtime automation field.**

## Independent genuine-save evidence

The owner deferred additional-build experiments and requested continued Phase 3
work. This follows the [registered automation matrix](parser-automation-counts-324-result.md#smallest-useful-differential-save-matrix)
in the installed, already represented FL Studio **26.1.0.5530**. Its executable
hash matched the earlier authoring receipt and About showed v26.1 build 5530.
Every case began from an independent approved F23 copy at 130 BPM with an empty
Sampler and its native channel-volume automation. No playback, audio, external
plugin or personal project was introduced.

Operations and intended counts were registered privately before authoring;
actual GUI observations were frozen before saving and event inspection. Raw and
sanitized files each reopened unchanged in the matching build without saving:
eight checks. Hollow tension handles were excluded from editable point counts.
The inherited synthetic project-data-folder prompt was dismissed with **Do
nothing**. The owned process closed gracefully after verification.

| Case / slot | Independently observed change                        | GUI objects / placements / points | Event 233 bytes | Event 234 bytes | u32LE at offset 17 |
| ----------- | ---------------------------------------------------- | --------------------------------- | --------------: | --------------: | -----------------: |
| F23 control | Existing approved curve                              | 1 / 1 / 3                         |              88 |             205 |                  3 |
| A01 / F39   | Delete the middle point                              | 1 / 1 / 2                         |              88 |             181 |                  2 |
| A02 / F40   | Add one editable point                               | 1 / 1 / 4                         |              88 |             229 |                  4 |
| A03 / F41   | Place the same object twice; no clone or Make unique | 1 / 2 / 3                         |             176 |             205 |                  3 |
| A04 / F42   | Delete the occurrence, retaining channel and curve   | 1 / 0 / 3                         |               0 |             205 |                  3 |

A02's added point displayed `001:05:001`, `-19.8dB`, recorded as observed rather
than relabeled as an exactly snapped beat. The original middle point displayed
`001:09:000`, `-12.8dB`; endpoints retained F23's observations. A04's first
in-memory attempt opened a point menu and later created an unwanted Pattern 1
placement. It was discarded without saving; the original was reloaded and the
explicit Playlist Delete tool used. Failed/stale/loading captures remain private
and are excluded from passes.

## What the bytes establish

Independent bounded Node and Python walks agree on complete exact-build framing,
candidate hashes, sanitation and these observations. The
[fixed-input regression](../../tests/automation-differential-fixtures.test.mjs)
reproduces them from approved bytes and independent GUI expectations:

- The little-endian u32 at event-234 payload offset **17** tracks two, three and
  four editable points. Offset **13** stays three, falsifying its earlier
  interpretation as this changing count.
- These payloads fit **21 + 24 × n + 112** bytes, with identical first 17 and
  trailing 112 bytes. The earlier `13 + 64 × n` hypothesis matches F23 by
  coincidence but fails the two-/four-point examples. A matching 24-byte size
  does not identify this as the unrelated note format.
- A03/A04 preserve the entire event-234 payload while the Playlist payload
  doubles or becomes empty. Occurrences cannot substitute for this object's
  point count.
- A01 preserves event 233 exactly. A02 also changes its byte 32; its meaning
  remains unqualified and the byte is retained, not normalized away.

This is one native object/target and exact build, not a complete automation
grammar. Multiple-object enumeration/totals, channel/reference binding, explicit
empty/zero lists, same-count point-content changes, LFO, other targets, curve
time/value interpretation and MIDI remain unqualified. An empty Playlist or
absent record is not an automation-count zero.

## Exact approval and privacy

On **2026-11-14** the owner answered **"Yes I approve"** to the four exact
sanitized candidates and expectations packet. Names, sizes, hashes and provenance
are in the [manifest](../../fixtures/parser-corpus/manifest.md#automation-differentials-f39-f42).
The exact [expectations](../../fixtures/parser-corpus/automation-differential-expectations.json)
are **5,977 bytes**, SHA-256
`a09f62bbe3a432dcec27649c4f469ce073f253734a7995b5e33ce1f68ed81162`.
Preparation-time pending wording is preserved byte-identically; this dated
approval supersedes it and approves fixture publication only.

Each candidate removes only one 16-byte registration event 200 plus two framing
bytes, correcting FLdt length. Independent direct-splice comparison proves all
retained event bytes identical to the private original. Bounded Latin-1 and both
UTF-16LE alignments found no known account identifier, email or non-Public home
path, embedded RIFF/image signature, nonempty sample filename event or note
payload. Controlled native-automation provenance supports these checks; opaque
state is not exhaustively decoded. Raw saves, screenshots, private paths and
detailed receipts remain outside Git. The 36 earlier FLPs are unchanged; there
are now 40 approved files across the same three saved builds.

## Parser regressions and next boundary

Publication changes no production parser, native validation, schema, storage,
client, capability or dependency. Real parser and full supervised native-validation
regressions cover all 40 files read-only. Placed cases retain unsupported
generator/Playlist interpretation. F42 retains a recognized empty pattern
Playlist and unavailable pattern extent; its automation curve still exists and
no automation numeric summary is emitted. Basic metadata remains available.

Next research should distinguish multiple definitions and their bindings,
same-count point-content changes and explicit empty lists where the GUI permits
them. Only qualified grammar should lead to separate extraction/validation and
immutable Library implementation. Keep #324 open. Additional-build work stays
deferred; Phase 3 exit, resources, production activation and distribution remain
open.

## Validation and storage boundary

Owned primary-checkout scope: four approved fixtures and exact expectations,
privacy exceptions/tests, byte-evidence and Rust corpus/supervisor tests, and
matching research/manifest/readiness documentation. Checks include root Node
tests, script syntax, scoped formatting/Markdown, privacy, whitespace, Rustfmt,
release-profile default/research parser tests and all-target warning-denied
Clippy. Normal full updated-head CI remains required. Earlier application CI
covers unchanged production code only; no installed/performance qualification
is extended.

Owner `/root` sequentially owns the existing `desktop-feature-tests` cache with
explicit `CARGO_TARGET_DIR`, preserving primary `target`. Four-path preflight
measured 31.32 GiB free and permits 0.60 GiB additional output while preserving
the 30 GiB reserve. One native build/test window runs at a time; no installed or
qualification window is used. Absolute cache paths, source hashes, commands and
private evidence stay in the `automation-differentials-20261112` handoff outside
compiler caches. Compiler outputs are reusable mutable cache; private originals,
candidates and receipts remain retained evidence. No deletion or new permanent
compiler cache is introduced.

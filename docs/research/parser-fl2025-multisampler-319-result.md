# FL 2025 three-Sampler clone case (issue #319)

Status: the owner approved the exact fixture and expectations for public
inclusion on 2026-10-15. This case adds one controlled authoring path to the
already-covered FL Studio 2025 build 25.1.3.4922. It adds no saved build or
product behavior. The [corpus manifest](../../fixtures/parser-corpus/manifest.md)
records the exact approved hashes and expectations.

## Matching-build GUI case

An independent copy of approved F06 was opened in FL Studio 25.1.3.4922. The
original empty Sampler was cloned twice without renaming. The GUI showed
130 BPM, three channels each literally named `Sampler`, File `(none)` on each
built-in Sampler, and an empty visible Playlist. No notes, steps, Playlist
edits, imported audio, external plugins, recording or playback were used.
Piano rolls were not independently inspected; bounded saved-event inspection
found no nonempty note payload.

Fresh reopens of the original save and sanitized candidate repeated the same
visible observations. Reopen sessions did not save or play the files, and the
raw and candidate hashes remained unchanged. The operating agent directly
observed the GUI and viewed the screenshots. The independent reviewer checked
the retained receipt and screenshot metadata, but did not inspect screenshot
pixels. The reviewer independently recomputed the fixture and expectations
hashes and checked the byte transformation and recorded receipts.

## Exact approved bytes and sanitation

The raw GUI save is 48,104 bytes. The approved
[`FIX-FL2025-MULTISAMPLER.flp`](../../fixtures/parser-corpus/FIX-FL2025-MULTISAMPLER.flp)
is 48,062 bytes with SHA-256
`2216b6d131360c0cafb20d2bee2ab82c0217e21303c3a1260c5c8f4a7cd42eeb`. The
exact approved [`expectations`](../../fixtures/parser-corpus/fl2025-multisampler-expectations.json)
file is 3,546 bytes with SHA-256
`b7715ef0c57a27adc3fcb0c22700fa321d668bd3a2b8eb172cc4486b1621637d`. Its
prepared pending-publication status is preserved; the manifest records the
later owner approval separately.

Sanitation removed the complete event 200 registration payload (40 payload
bytes and two framing bytes) and corrected the FLdt length. Every retained
event byte is identical. The encoded registration value was not decoded or
matched to visible account text. Earlier 16-byte-size and literal-label
assumptions failed before candidate creation and remain in the private record.

Bounded Latin-1 and both UTF-16LE alignment scans found no non-Public Windows
home path, known account text, email, or RIFF/PNG/JPEG/GIF signature; no
registration event remains. Opaque default/plugin state is not fully decoded,
so these checks are not exhaustive privacy proof. Raw saves, screenshots,
failed checks and diagnostics remain outside Git.

## Parser comparison

The retained parser from source `0e86b8efb2ccdeeed37caec37ca1ac175c2a2583`
has the same parser, Cargo and toolchain inputs as issue source
`fa0dbdd2fc9b7f2952fc6386fcb8fd444c2e180b`. Its pinned binary SHA-256 is
`50f619a0cc87c6a58e51660d84285f21b585e47c39a5dbc0d6aa9ebb908b69a2`. It
matched the GUI-observed saved build, tempo, channel count and labels. The
three channel labels are **inferred** as literal `Sampler` values with method
`sampler-label-for-known-build`; item confidence is high/medium/medium and
aggregate confidence is medium. No editable-name event is stored, and the
saved events contain three empty generator-class records.

The 2025 `channelGeneratorNames` field remains unsupported with reason
`GENERATOR_NAMES_UNVERIFIED_BUILD`. The three sample references remain
unavailable with reason `SAMPLE_REFERENCE_NOT_STORED`. Genuine F06 and F15
control results matched and their bytes stayed unchanged. All nineteen earlier
approved fixture hashes also remained unchanged. The parser already produced
the registered result, so this fixture does not justify a parser algorithm
change.

## Coverage boundary

F22 qualifies only this clone-from-F06 path in FL Studio 25.1.3.4922. It does
not qualify every way of creating channels, other saved builds, 2025 generator
name extraction, MIDI, automation or richer project layouts. The visible
Playlist was empty; piano rolls were not independently inspected. No sample
file was stored or resolved. G3 remains open for other builds, absent tempo,
zero channels and other authoring paths. G5/G6, resource budgets, Phase 3,
production activation and release remain separate pending decisions.

## Validation and evidence handoff

Local validation passed `cargo test -p fruitboard-flp-parser --locked`,
`cargo clippy -p fruitboard-flp-parser --all-targets --locked -- -D warnings`
and `cargo fmt --all --check` with the pinned Windows toolchain. The real
supervised parser regression now validates all twenty approved corpus files
without changing them. F22 coverage pins both approved hashes, GUI values,
per-channel inference confidence, unsupported 2025 generator names and absent
sample references. The privacy regression rejects changed bytes, another path,
registration metadata and malformed event bounds.

All 137 Node script/policy/privacy tests, 33 script/test syntax checks, scoped
Prettier and Markdown lint, and repository privacy verification passed. All
nineteen existing fixture hashes remained unchanged. Both new files match the
owner-approved byte counts and hashes in the working tree and Git index.

The documentation agent ran no build or tests. Root owns the scoped validation
and normal updated-head CI. The full application was not rebuilt locally:
production source and dependency/toolchain inputs are unchanged, and the fifteen
successful PR318 checks cover that unchanged source. New publication checks
remain required; no CI result proves performance qualification.

Private preregistration, raw and sanitized saves, GUI observations and images,
sanitation receipts, parser comparison, binary provenance, failed assumptions,
approval and independent review remain outside the repository and compiler
caches. The public record contains no raw registration payload, personal path,
screenshot or log.

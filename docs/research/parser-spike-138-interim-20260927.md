# Parser spike #138: Rust corpus result and fallback trigger

- Status: **Interim research result. No production parser selected.**
- Start: 2026-09-27, after owner-approved fixture PR #168 merged into `main` at
  `e62eebef4713f480fc16526cd4c7b14dbd7178ab`.
- Scope: the nine approved files in `fixtures/parser-corpus/manifest.md` only.
  F02 (FL 20) and F03 (FL 21) remain owner-recorded `not covered`; F12 remains
  reserved. No personal project, audio, or installed-app save was used.
- Research source: `17a5114d872ca66780f385222a042de10555c95e`; corrected validation
  harness: `425792d`; compiled with Rust 1.98.1 MSVC, `--release --locked
  --offline`, in the machine's reusable validation cache. The retained binary is
  an independent copy outside that cache, SHA-256
  `527be990796574a81064753e7af2b2d80a5349a26fad7da20e3a44abb93d111f`.
  Private per-file JSON report SHA-256:
  `683d3326c972687e325edc5f26c273c6f1f7477e6d9d45dd6d68d5196eae35ea`.

## Method and provenance

The research-only binary lives in `research/parser-spike-138/`, outside the
production Cargo workspace. It reads one explicit file, caps input at 4 MiB,
event payloads at 2 MiB, events at 100,000, and declared channels at 256. It
does not save, repair, load plugins, resolve sample paths, or access the network.
The harness verifies each manifest SHA-256 before and after every parse. All
approved bytes remained identical. It emits a typed failure for a bad chunk,
length, or channel count and a warning for the inserted unknown event.

Event interpretation was checked against the approved files themselves and
public format research: the [FLIndexRenders project](https://github.com/1ajh/FLIndexRenders)
(MIT licensed) documents the three-byte event 172 change in FL Studio 25.2.4+,
and the [flpdiff format notes](https://github.com/dawhubapp/flpdiff/blob/main/docs/fl-format/flp-format-spec.md)
describe the version, tempo, channel-name, and sample-reference events. No
third-party parser code was copied. These sources are format evidence, not
ground truth for the registered expected values.

## Result against pre-registered fields

| Fixture group | Saved version | Base tempo | Channel names | Raw sample references | Overall |
| --- | --- | --- | --- | --- | --- |
| F01, F04, F05 (FL 2024) | Exact build extracted | 120, 140, 141 BPM respectively | `Sampler` extracted from explicit event | `unavailable`, matching File `(none)` | Meets registered fields |
| F06 (FL 2025) | Exact build extracted | 130 BPM | `unavailable`: name not stored as an explicit event | `unavailable`, matching File `(none)` | **Name mismatch**: manifest expects `Sampler` |
| F11 (FL 2026) | Exact build extracted | 130 BPM | `unavailable`: name not stored as an explicit event | `unavailable`, matching File `(none)` | **Name mismatch**: manifest expects `Sampler` |
| F07/F09/F10 | `failed` | `failed` | `failed` | `failed` | Typed truncation, malformed length, and channel-count limits match |
| F08 | Exact 2024 build extracted | 120 BPM | `Sampler` extracted | `unavailable` | `partial` with `UNSUPPORTED_EVENT` for ID 255, as registered |

Stage 1 (version and tempo) passed every covered row, allowing stage 2 to run.
Stage 2 did not meet the approved matrix: the 2025 and 2026 saves have one
channel in the header and channel-open event but omit the explicit channel-name
event found in the 2024 saves. FL Studio displays the built-in default name in
those versions. The research parser reports the missing stored field instead of
claiming the displayed default was in the bytes. A byte search and bounded
event walk found no `Sampler` text in F06/F11; this is a corpus-specific
observation, not a universal claim about FL Studio 2025 or 2026.

None of the approved genuine saves has a loaded sample reference, so this
corpus verifies honest absence only. It cannot establish positive raw-reference
extraction. The FL 20/21 rows, an absent-tempo save, zero-channel save, rich
Unicode names, and projects with loaded samples were not tested.

## Resource observations

The optimized Windows binary is 258,048 bytes. The first observed `describe`
launch after building took 52.0 ms; the median of ten later launches was about
6.5 ms. Across the nine fixtures, median warm process-plus-parse time was
6.59-6.88 ms, the in-process read/parse time was 0.167-0.205 ms, and peak
working set was 3.89-3.96 MiB. This is one host with no OS cache flush, so the
first launch is an observation, not a controlled cold-start benchmark. No
runtime beyond the compiled Rust binary was required for the Rust runs.

## Decision input

The Rust prototype is small and read-only, and it met stage 1 and the typed
robustness cases. It **does not meet the current four-field matrix**, so the
spike has recorded a shortfall and triggered the separately gated PyFLP research
comparison under the same approved corpus. This record does not authorize
production PyFLP, change the manifest's expected values, or select a parser.
After fallback evidence is added, the owner will decide whether to select,
defer, or stop under the proposal's decision deadline.

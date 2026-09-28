# Parser spike #138: inferred default Sampler follow-up

- Date: 2026-09-28, after the original nine-file result and the F12 positive
  sample-reference result.
- Owner decision: for the FL Studio 2025/2026 projects that visibly show the
  built-in `Sampler` while storing no channel-name event, report the displayed
  name as an **inference**. The original manifest expectations and results stay
  in their historical records.
- Question: can the bounded Rust research adapter apply that decision without
  presenting inferred text as stored text or losing F12's explicit name?
- Scope: ten approved fixtures in the [manifest](../../fixtures/parser-corpus/manifest.md).
  F02 and F03 remain owner-recorded `not covered`.

## Method

F06 and F11 each contain one channel-open event with type value zero and no
explicit channel-name event. Matching FL Studio 2025 (25.1.3.4922) and 2026
(26.1.0.5530) GUI reviews showed that channel as `Sampler`. The research
adapter now infers the displayed default only when **all** of these conditions
hold: the name event is absent, the channel type is zero, and the saved build
matches one of those two verified builds. It returns `status: inferred`, method
`sampler-default-for-known-build`, and `confidence: high`, including item-level
provenance. Other unstored names remain `unavailable`. Explicit name events,
including F12's `Fixture Sample A`, remain `extracted`.

The Rust source remains research-only, outside the product Cargo workspace.
It reads one explicit file at a time with the existing 4 MiB file, 2 MiB event,
100,000 event, and 256 channel limits. The validation harness retained its
10-second child timeout and output bounds. Every approved fixture hash was
checked before and after parsing. No fixture, sample audio, PyFLP package, or
production parser code changed. A regression test rejects an unlabeled
`extracted` claim for F06 and requires the inference method and confidence.

## Result

The revised Rust comparison passed all ten approved fixtures under the owner's
new interpretation. F06 and F11 returned the visible `Sampler` name with
`inferred` status; F12 returned its explicitly stored `Fixture Sample A` name
and exact raw sample path with `extracted` status. The original byte-extraction
matrix still has its historical F06/F11 shortfall: the string `Sampler` is not
stored in those two files. This follow-up changes the interpretation of the
displayed default; it does not rewrite the original pre-registered table.

The exact Rust source was committed at `080e825`. It was built with Rust
1.98.1 MSVC using `cargo build --release --locked --offline`. The optimized
Windows research executable is 273,920 bytes, SHA-256
`ba505daaffc1e02b8938f9a77627ad2a90b1c142f4bcc2681303fc07623da30c`.
An independent copy outside the compiler cache produced the private ten-file
report, SHA-256
`8ba2ff2507d786a1f410828e61525fc534454f74bf2ed934ad393f6d373a688c`.
The run reported zero current-policy mismatches. Ten Python harness tests,
119 repository policy tests, Markdown lint, Rust formatting, and the repository
privacy scan passed locally. This is functional validation, not a cold-start,
installed-app, or hostile-input performance qualification. Stable PyFLP's
previous ten-file result is unchanged and was not rerun for this Rust-only
change.

## Remaining decision

This result supports the Rust research approach for the ten covered fixtures
under the owner's inference policy. It does not select a production parser.
The owner still needs to choose the parser approach under ADR-002 before Phase
3 implementation. FL Studio 20/21, absent tempo, zero channels, richer channel
types, and Unicode names remain untested; no blanket default-name inference is
claimed for other FL Studio builds.

# Cross-build parser pilot and baseline controls

Issue: [#399](https://github.com/guilhermebmichelin-create/fruitboard/issues/399).
Prepared against merged `c6527c562a5799a7c6e6be06707216d53e391f4e`.
Status: **Existing-build baseline passes; additional-build experiment has not
started. Exact candidate builds and genuine saves remain prerequisite inputs.**

## What is complete

PR396 promotes the independently qualified ordinary-note pilot values and PR398
records the installed workflow and narrow enlarged-text correction. Those
changes are merged. Their [installed review](../review/installed-notes-397/README.md)
passes seventeen cases on its recorded source; 437 client tests passed. The
default desktop still does not activate automatic parsing. These deliveries do
not complete the parser or accept Phase 3.

The parser covers selected saved facts in three exact builds: 24.1.0.4225,
25.1.3.4922 and 26.1.0.5530. Per-field restrictions remain in the
[maintained matrix](../review/flp-intelligence-readiness/field-reliability.md).
Additional builds, automation/MIDI, richer mixer/arrangement interpretation,
broader dependency resolution and resource qualification remain unfinished.

## Executable baseline

The [control specification](../../research/cross-build-controls.json) freezes
four already approved genuine-save derivatives before any new-build experiment:

| Control | Exact saved build | Independently recorded facts                                                                |
| ------- | ----------------- | ------------------------------------------------------------------------------------------- |
| F01     | 24.1.0.4225       | 120 BPM; one Sampler; no stored sample reference                                            |
| F06     | 25.1.3.4922       | 130 BPM; one Sampler; no stored sample reference                                            |
| F11     | 26.1.0.5530       | 130 BPM; one Sampler; no stored sample reference                                            |
| F12     | 26.1.0.5530       | 137 BPM; one channel named Fixture Sample A; exact approved synthetic sample-reference text |

The [manifest](../../fixtures/parser-corpus/manifest.md) owns the existing GUI
observations, privacy approvals and hashes. Existing parser contracts/tests
establish field status: F06/F11 default labels are inferred, not extracted;
absent reference text remains unavailable, never an empty or zero value.
These four controls do not replace the complete 36-file regression corpus.

Run the dependency-free Node tool using the pinned runtime and an independently
retained parser with known build provenance:

```text
node scripts/research-cross-build-baseline.mjs --parser <retained-parser.exe> --sha256 <expected-lowercase-sha256>
```

The tool reads only the four fixed corpus files. It accepts no project path,
candidate manifest or version override. It checks exact lengths/hashes before
launch, supplies only the fixed corpus root as native authority, and verifies
the native result fingerprint plus independently reread bytes/identity/time
after each child and after the entire run. Each parser process gets one normal
protocol request, EOF, a ten-second limit, a 256 KiB output bound and a 64 KiB
accepted stderr limit. Process failures produce a fixed diagnostic and no
qualifying report. The supplied executable hash is verified before/after; a hash
identifies bytes but does not independently prove their build source.

The report distinguishes matched extracted values, matched inferences, matched
unavailable fields, mismatches, policy-blocked versions, unsupported layouts and
failures. An overall complete parse cannot hide a mismatching field. Process
text, arbitrary returned values/reasons and local paths are not echoed; reports
include only known expectations and classifications. A mismatch needs separate
private inspection before drawing a format conclusion.

The [baseline result](cross-build-baseline-399.json) passes all **four controls /
twenty field comparisons**. The retained parser SHA-256 is
`329860b011bf48ad3390335bbabc20d09bac5a19dad9f513316a07fd8a899d66`, built at
`1de0a6c4329ce5da1fffa7cc3a977f7d51ae1f31`. Its parser crate, Cargo manifest/lock
and toolchain inputs match the merged baseline above. The previous build receipt
and independent binary remain private; no native rebuild was needed. The initial
runner request used an incorrect fingerprint key and was rejected without a
report; that runner-only error was corrected before this successful run.

This is an existing-build control pass, **not new-build compatibility evidence**.
It performs no allowlist bypass, writes no FLP and does not store/display results
in the application. No new fixture, capability, schema or support row is added.

## Next experiment registration

The [public version-comparison survey](flp-version-literature-399.md) identifies
existing research and release information. It prioritizes the 25.2.4 release
line and an adjacent 26.1.x release for investigation; exact runnable builds
remain unverified. The owner does not need to supply versions from memory for
literature and format analysis to continue.

The owner's cross-build workstream specifies two additional lawfully runnable
builds: an adjacent build and, where available, one near the known event-172
width transition. Actual exact numbers must be recorded before experimentation;
they have not been supplied. The format-transition hypothesis in existing code
is not an independently qualified version family.

| Candidate role                  | Exact build                                 | Current state |
| ------------------------------- | ------------------------------------------- | ------------- |
| Adjacent to a represented build | Pending availability and About confirmation | Not tested    |
| Near a format transition        | Pending availability and About confirmation | Not tested    |

For each available candidate, complete the following before inspecting events:

1. Pin the parser/harness source and binary hashes, exact About build, lawful
   availability, recipe and observation sheet. Record unavailable programs as
   not tested. Do not relabel an existing supported build as an additional one.
2. Create the same disposable Empty-template project in the matching build:
   130 BPM, one empty built-in Sampler explicitly named `Compatibility Sampler`,
   File `(none)`, and no notes, Playlist clips, recording or external plugin.
   Register expected version, base tempo, channel count/name and absence of a
   sample-reference string from the GUI before parser inspection.
3. Register controlled variants separately: 131 BPM only; a different explicit
   channel name only; then a generated-silence sample-reference case with exact
   GUI-confirmed inert text. Preserve any unexpected GUI result and reassess the
   recipe before looking at bytes. Default-label inference is a separate case.
4. Independently preserve raw saves, sanitize under existing fixture rules,
   reopen candidates in the matching build without saving, verify exact GUI
   values and unchanged hashes, and obtain exact-byte/expectation approval
   before public inclusion. No personal project is a substitute.
5. Record current production refusals as policy-blocked, not proof of a format
   incompatibility. Any experimental shared-decoder bypass needs a separate
   reviewed research harness and locked genuine-save expectations; no production
   override or patched version string belongs in this runner.
6. Compare each field with the independent observations and document framing,
   payload and semantic differences. Patterns, notes, Playlist, mixer and plugins
   are separate groups. Do not infer support for intervening builds from two
   passing endpoints or promote research output directly into stored app facts.

Exit: publish a reproducible per-field report recommending retention of the
current boundary, selected support expansion or specific further research.
Production extraction/validation/persistence/Library changes require their own
implementation and genuine corpus regressions. This stage can finish with an
honest not-tested outcome when matching programs are unavailable.

## Validation and storage

Owned primary-checkout files are the fixed research specification, Node runner,
comparison helper, defensive tests and matching research/readiness documents.
No Rust, client, protocol or fixture byte changes are included. Required local
checks are Node tests, script syntax, scoped formatting/Markdown, privacy and
whitespace; normal full updated-head PR CI remains required. Merged CI applies
only to unchanged application code. The baseline uses the retained binary above.

No local compiler cache or installed host window is needed. Owner `/root`
preserves the primary `target` and existing reusable `desktop-feature-tests`
validation cache. Absolute paths, provenance, source equivalence, corpus hashes,
failed/successful receipts and current capacity remain in the private
`cross-build-pilot-20261104` handoff outside compiler caches. Maintain the 30 GiB
reserve after the small evidence allowance; future builds require new explicit
four-path preflights and an exclusive cache window. No cleanup, new compiler
cache or additional rebuild cost is introduced. Preserve owner planning drafts
and all prior evidence.

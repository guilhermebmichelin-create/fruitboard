# Parser spike #138: bounded parser research result

- Status: **Research result and recommendation; owner decision pending. No
  production parser selected.**
- Start: 2026-09-27, after owner-approved fixture PR #168 merged into `main` at
  `e62eebef4713f480fc16526cd4c7b14dbd7178ab`.
- Scope: the original nine approved files in `fixtures/parser-corpus/manifest.md`
  only. F02 (FL 20) and F03 (FL 21) remain owner-recorded `not covered`. F12
  was reserved for the original run and is now approved in separate, unmerged
  PR #170; no research parser has run on F12. No personal project or audio was
  used in this spike.
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
spike recorded a shortfall before beginning the separately gated PyFLP research
comparison under the same approved corpus. This record does not authorize
production PyFLP, change the manifest's expected values, or select a parser.

## Gated PyFLP comparison

The stable [PyFLP 2.2.1](https://pypi.org/project/pyflp/) wheel was installed
only into an isolated, reusable Python 3.11.16 research environment outside the
product workspace. PyFLP and its dependencies were not added to the application
or its lockfile. The exact research package set was `pyflp 2.2.1`,
`construct-typing 0.8.1`, `construct 2.10.70`, `sortedcontainers 2.4.0`,
`typing-extensions 4.16.0`, `arrow 1.4.0`, `python-dateutil 2.9.0.post0`,
`six 1.17.0`, and `tzdata 2026.4`. The probe script calls `pyflp.parse` only;
it has no save call. It checked each approved SHA-256 before and after parsing.
All nine inputs remained byte-identical. Private report SHA-256:
`0a903ac3d17644d299df721009ead3d052134fdae3c55e4625aad38d898a0f0c`.

Stable PyFLP returned `TypeError` on eight fixtures, including all five genuine
saves and the unknown-event derivative. Its error was `EventEnum has no members
defined`, occurring during event ID construction before any requested metadata
was returned. The truncated derivative returned `HeaderCorrupted`. No fixture
yielded all four fields. The [upstream comparison](https://github.com/demberto/PyFLP/compare/v2.2.1...f937126b888ce94271bfea631b89166c74056530)
shows six commits after the 2.2.1 tag, changing only development requirements
and pre-commit configuration; parser source is unchanged. A second run of the
same parser source at that upstream commit would not add independent coverage.
These failures do not prove PyFLP cannot parse all FLPs; they show this pinned
stable source did not parse the approved current-version corpus on this host.
PyFLP remains GPL-3.0 and blocked from product distribution under ADR-002 and
`LICENSE_INTENT.md`.

## Answers and recommendation

### Harness review and correction

The continuation review found that the original Rust harness had no process
timeout or response-size cap, and the PyFLP probe parsed inside its controller
process. The earlier successful corpus runs did not establish those safety
properties. Research harness commit `95fa042` corrects this gap with a shared
process runner: 10 seconds per invocation, at most 256 KiB stdout and 64 KiB
stderr. Exceeding a bound terminates the direct child and returns a fixed failure
code. Each PyFLP file now runs in its own child process. This is a transport
guard for these trusted research executables, not an OS sandbox or descendant
process containment claim.

Input hashes are checked in `finally` blocks, including on process failure.
Reports omit arbitrary PyFLP exception messages. The PyFLP field adapter also
converts its version object to text and reads sample event text directly rather
than converting through `pathlib.Path`, which can normalize the stored string.
This follows the installed 2.2.1 API; no upstream implementation was copied.

Seven standard-library regression tests passed on Python 3.11.16. They exercise
timeout termination, stdout/stderr overflow, invalid responses, concurrent pipe
draining, recovery after a failed child, input mutation detection after failure,
and preservation of version/reference text. These transport tests use small
child programs and a temporary text file; they add no FLP fixture.

The corrected harness reran all nine approved fixtures against the independently
retained, unchanged Rust binary and the isolated PyFLP installation. The Rust
results still have exactly the F06/F11 channel-name mismatches; PyFLP still has
eight `TypeError` results and one `HeaderCorrupted`. Every fixture hash remained
unchanged. These functional reruns preserve the recommendation below and do not
replace the original resource observations with a performance qualification.

- Rust report SHA-256:
  `057573d2b799b7c9300f126b9454a3cdb3d3bcd92fba43b4bd547c0760da434f`.
- PyFLP report SHA-256:
  `261a8fc128ad6c5687db4ef77db4a89c52a222124f77e03637b4fd63297d4c6f`.
- Reproduction: `python -B -m unittest discover -s research/parser-spike-138
  -p test_harness.py -v`, followed by the existing `validate.py --binary ...
  --output ...` and isolated-environment `pyflp_probe.py --output ...` commands.
  Reports stay outside the source checkout and compiler cache. The Python tests
  are local research validation; the existing application CI does not run them.

### Decision summary

| Question | Evidence-backed answer |
| --- | --- |
| Version and base tempo | Rust extracted exact saved versions and 120/140/141/130 BPM from all five genuine saves and the unknown-event derivative. FL 20/21 and absent tempo were not covered. |
| Channel names and raw sample references | Rust extracted explicit 2024 `Sampler` names; it reported the unstored 2025/2026 names as unavailable. No approved save has a positive raw sample reference. |
| Version rows | FL 2024, 2025, and 2026 have covered version/tempo evidence; FL 20 and 21 are `not covered`, not proven `unsupported`. |
| Resource cost | 252 KiB optimized binary, roughly 6.5 ms warm launch, 0.167-0.205 ms in-process read/parse, and 3.89-3.96 MiB peak working set on this host. No extra Rust runtime. |
| Safety | The bounded Rust CLI read one approved file per call, loaded no plugin/script, and left every input hash unchanged. The four registered robustness cases returned typed outcomes. This does not qualify untested hostile files. |
| PyFLP fallback | Stable PyFLP 2.2.1 returned no complete parse on the same nine files; its latest upstream parser source is unchanged from that release. Product licensing and packaging gates remain closed. |

**Recommendation to the owner: defer production parser selection.** The Rust
prototype has a precise stage-2 mismatch against the approved expected matrix,
and PyFLP did not provide a usable fallback on this corpus. If the owner wants
to continue the Rust path, a separate approved fixture/expectation decision is
needed to distinguish an explicit saved channel name from FL Studio's displayed
default and to add a genuine sample-reference case within the proposal's bounds.
No fixture value should be altered to make the prototype pass. Any change to
the twelve-slot, six-row, or time limits requires its own owner decision.

The owner selects Rust, PyFLP, defer, or stop by the proposal's decision
deadline. Until that decision is recorded in ADR-002 and `FLP_PARSER.md`, the
production parser remains unselected and Phase 3 parser implementation does not
start.

### Proposed next research case

To continue the Rust investigation, propose assigning reserved F12 to one genuine
FL Studio 2026 (26.1.0.5530) save with base tempo 137 BPM, one built-in Sampler
explicitly named `Fixture Sample A`, and one generated silent WAV at a neutral
Public Documents fixture path. The expected raw sample path and exact bytes
would be recorded before parser execution. This adds positive sample-reference
and explicit-name evidence in the newest covered version; it does not establish
those features in every supported version or resolve the default-name question.

The proposed procedure generates the synthetic WAV locally, creates the project
through the FL Studio GUI, inspects and sanitizes its embedded metadata, reopens
the corrected file without saving, and checks its tempo/name/sample state and
unchanged SHA-256. The WAV stays outside Git. The final FLP requires owner
approval of its exact bytes and a separate fixture manifest PR before parsing.
The owner subsequently approved this case and its exact sanitized bytes, and
PR #170 contains the fixture and manifest. That PR remains unmerged. Existing
F06/F11 expectations and their recorded mismatches remain intact.

This would use ten of the twelve fixture slots, the same version rows and four
fields, and the original 15-day active-work/21-day hard-stop bounds. Production
parser selection and Phase 3 implementation remain separate owner decisions.

## 2026-09-28 continuation: PyFLP enum diagnostic

The unmodified stable PyFLP 2.2.1 baseline above remains the required fallback
result: it parsed none of the nine approved files on pinned Python 3.11.16.
This is partly an entry-point defect rather than nine independent file-format
failures. Calling its `EventEnum(199)` directly, without an FLP, raises
`TypeError: EventEnum has no members defined` before its `_missing_` hook can
resolve the known version event. [Upstream PyFLP issue #201](https://github.com/demberto/PyFLP/issues/201)
reports the same error.

An **opt-in research diagnostic** in `pyflp_probe.py` now adds one out-of-range
member to that empty enum's private map inside each bounded child process.
This allows PyFLP's existing `_missing_` lookup to run. The installed package,
product code, and baseline probe remain unchanged. The flag is
`--enum-compat-diagnostic`; it is a temporary compatibility experiment, not a
proposed production patch. The report from source commit `41462fb` is retained
outside Git with SHA-256
`117b36b685f24397a8b59808757e34736f5f6c676d97ab04875ae02d300dbb8c`.
Each child had the existing ten-second and output limits, and all nine fixture
hashes were unchanged before and after parsing. Eight local harness tests
passed, including an opt-in check that keeps the unmodified baseline separate.

| Fixture group | Diagnostic result after the temporary enum workaround |
| --- | --- |
| F01/F04/F05 | Version, tempo, explicit `Sampler` name, and absent sample reference matched their registered values. |
| F08 | The same four fields matched; the probe did not establish the registered unsupported-event diagnostic. |
| F06 | Version, tempo, and absent sample reference matched; the unstored default channel name remained unavailable. |
| F11 | Version was extracted, tempo was unavailable, and channel/sample access failed with `NoModelsFound`. |
| F07/F09 | Parsing failed with `HeaderCorrupted` and `UnicodeDecodeError` respectively; typed field-level failures were not established. |
| F10 | PyFLP parsed the deliberately excessive 65,535-channel declaration instead of enforcing the registered channel-count limit. |

This diagnostic corrects the interpretation of the baseline failure: bypassing
the enum defect exposes partial PyFLP coverage, not a complete fallback. It
still misses newer-version fields and the resource-limit safety case, and no
approved positive sample reference has been tested. It provides no performance
qualification or license exception. The recommendation to defer production
selection is unchanged. The F12 parser test remains gated on PR #170 merge;
both fixture and research PRs currently have GitHub Actions jobs that could not
start because of the account billing/spending limit.

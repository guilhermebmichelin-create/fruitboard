# FLP parser architecture and feasibility

Status: **Rust selected for the next production implementation on 2026-09-28;
compatibility and packaging gates remain open**

## Conclusion

The owner selected an independent, read-only Rust sidecar after the approved
ten-fixture research comparison. Start production implementation with saved FL
version and base tempo, then channel names and raw sample references. The
displayed built-in `Sampler` default is reported as an explicitly labeled
inference for the two verified saved builds. The selection and open gates are
recorded in [ADR-002](docs/adr/002-flp-parser-process.md#2026-09-28-owner-selection).
The [corpus manifest](fixtures/parser-corpus/manifest.md) and
[research results](docs/research/parser-spike-138-inferred-default-result-20260928.md)
provide the tested source boundary. Arrangements, plugin-state decoding, and
duration remain outside the initial implementation slice.

PyFLP was tested as the comparison candidate. Its stable release returned no
complete parse on the approved corpus. It remains outside the product, and the
owner's intended public open-source release does not select the exact license.

PyFLP is unofficial. Its [PyPI package](https://pypi.org/project/pyflp/) is
currently version 2.2.1, published June 5, 2023, marked Alpha, and licensed
GPL-3.0. Its documentation describes project settings, arrangements, playlist,
patterns, channels, mixer inserts/slots, native plugin models, and VST2/VST3.
An [open FL Studio 2025 issue](https://github.com/demberto/PyFLP/issues/200)
reports missing playlist data after an apparent structure change. The format
must therefore be treated as evolving and partially understood.

## Non-destructive contract

The production parser surface contains no `save`, mutate, repair, zip, plugin
load, DLL load, or script execution operation.

- Rust opens or validates the selected locator and permits only files inside an
  enabled scan root.
- The sidecar opens the FLP read-only and returns inert JSON.
- Embedded paths, comments, plugin state blobs, and strings are untrusted data.
- The sidecar never resolves an embedded path by executing or loading its
  target. Dependency resolution is a separate Rust service.
- No partially parsed object is written back to the FLP.
- A parser failure affects one file and leaves the last known good snapshot
  active.

## Adapter shape

The Rust core depends on an interface, not PyFLP:

```text
FlpParser
  describe() -> ParserDescriptor
  parse(ParseRequest) -> ParseOutcome
  health_check() -> HealthStatus
```

The initial transport is UTF-8 JSON Lines over stdin/stdout. Avoid a localhost
HTTP server: stdio has no listening port, discovery, or local-origin
authentication problem. Every message contains `protocolVersion`, request ID,
and schema version. Stdout is protocol-only; diagnostics go to stderr.

Initial version-1 request shape (values shown are illustrative). Schema
version 2 added the `allowedRoots` allowlist to `parse` params: the
supervisor passes the enabled scan roots, and the sidecar refuses any path
that is not a descendant of one of them (`INVALID_PATH`). Paths and roots
must be absolute without parent-directory components. Symlinks, junctions,
and other reparse points are refused throughout the ancestor chain and at
the leaf. Windows device paths, alternate streams, and trailing-dot/space
aliases are refused. An empty allowlist denies every path. The parser reads
the same handle it authorized: Windows ancestor/leaf handles deny write and
delete sharing until parsing finishes; Linux checks object identities and
the opened descriptor through `/proc/self/fd`. Authorization fails closed
on other platforms until an equivalent backend is qualified:

```json
{"protocolVersion":1,"schemaVersion":2,"id":"01234567-89ab-cdef-0123-456789abcdef","method":"parse","params":{"path":"C:\\approved\\project.flp","expected":{"size":46703,"modifiedAtMs":1234567890000},"features":["basic-metadata"],"allowedRoots":["C:\\approved"]}}
```

The absolute path exists only in the private Rust-to-sidecar request needed to
open the selected file. Exact request payloads and paths never enter logs or
sync payloads; logs, progress events, and diagnostics use opaque request,
location, and project-file IDs. Any user-facing path display is fetched through
an explicit local-only use case.

Initial response shape (the actual result includes all six fields). Schema
version 2 replaced the null `hash` with the SHA-256 content digest of the
exact bytes read. After parsing, a second bounded read through the same
authorized handle checks the digest, size, and modified timestamp again;
a difference fails `INPUT_CHANGED`. This detects observed changes even when
size and timestamp were preserved. It is a before/after observation, not a
guarantee against a change-and-restore between observations or a change
after the response; the future supervisor still owns result freshness:

```json
{
  "protocolVersion": 1,
  "schemaVersion": 2,
  "id": "01234567-89ab-cdef-0123-456789abcdef",
  "result": {
    "outcome": "complete",
    "savedVersion": {"status": "extracted", "value": "26.1.0.5530"},
    "baseTempoBpm": {"status": "extracted", "value": 137.0},
    "channelCount": {"status": "extracted", "value": 1},
    "patternCount": {"status": "unavailable", "reason": "PATTERN_DATA_NOT_STORED"},
    "channelNames": {"status": "extracted", "value": ["Fixture Sample A"]},
    "sampleReferences": {"status": "extracted", "value": ["sample.wav"]},
    "inputFingerprint": {"size": 46703, "modifiedAtMs": 1234567890000, "hash": {"algorithm": "sha256", "value": "2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae"}},
    "diagnostics": []
  }
}
```

Each potentially absent fact uses a status such as `extracted`, `inferred`,
`unavailable`, `unsupported`, or `failed`. `inferred` values are normally
produced by Rust after parsing; if a parser adapter infers something, it must
label the method and confidence explicitly. An inferred displayed channel name
must never be labeled as text extracted from the FLP, and an extracted name must
never be relabeled as inferred by an aggregate method.

Inferred sampler channel names follow FL Studio's own display numbering, so
successive unnamed sampler channels in a known build are reported as `Sampler`,
`Sampler 2`, `Sampler 3`, and so on. Only the first is verified against a real
save and carries `high` confidence; the numbering beyond it follows FL's display
convention and is reported at `medium` until a multi-channel approved fixture
confirms it. When a project mixes a stored name with an inferred default, the
aggregate `channelNames` object uses the method
`mixed-extracted-and-sampler-default` at `medium` confidence rather than
claiming the whole array came from the sampler-default rule.
An all-inferred array keeps the sampler-default method, but its confidence is
`medium` when any included name is `medium`; the one verified inferred Sampler
alone remains `high`.

Generator names are separate from editable channel labels. A controlled
FL Studio 2026 save shows built-in `3x Osc` stored in event 201, while the
renamed channel label is stored in event 203. The draft
`channelGeneratorNames` field extracts that verified built-in class and
labels the 2026 Event 21 type-0 Sampler default as an inference. Event 64
identifies the channel; it does not encode the channel type. Unverified
generator classes, mixer effects, external VST identity, and other saved builds remain
`unsupported` for this field. The owner approved the exact sanitized
two-channel F14 save for corpus inclusion on 2026-09-29 under the
[fixture rules](DEVELOPMENT.md#parser-fixture-rules).

The first selected Rust implementation slice is
[`crates/flp-parser/`](crates/flp-parser/README.md). It implements the four
initial fields, followed by a channel count validated against both the header
and channel events, and a bounded version-1 JSON-lines process. Its verified
saved builds are exactly 24.1.0.4225, 25.1.3.4922, and 26.1.0.5530; other saved
builds return typed `UNSUPPORTED_SAVED_VERSION` until approved fixtures expand
coverage. The crate is not yet wired to scanner jobs or desktop packaging.

The merged pattern-count extension counts distinct stored pattern IDs in
26.1.0.5530, with a 1024-pattern resource bound. Repeated note/property markers
do not inflate the result. Absent markers are `unavailable`; the 2024/2025
builds are `unsupported` for this field. The owner approved the exact
three-pattern F13 GUI save, now in the corpus; see the
[validation record](docs/research/parser-pattern-count-184-result.md).

Before persistence, application integration must validate the parser result for
schema, length/count limits, finite numeric values, path/string sizes, and known
enum values. Unknown fields may be ignored for forward compatibility; unknown
required protocol versions fail closed. This application boundary remains to
be implemented.

## Process supervision

Start with one long-lived parser process and one job at a time. This amortizes
process startup without allowing unbounded memory/concurrency. The Rust
supervisor:

- starts only the configured, bundled sidecar binary;
- passes paths as encoded arguments/messages, never through a shell;
- enforces per-request timeout and maximum response size;
- captures bounded stderr with path redaction;
- cancels by terminating and restarting the sidecar if cooperative cancellation
  fails;
- restarts after crash, malformed protocol, memory growth threshold, or a
  configured number of jobs;
- associates every result with the expected file fingerprint so stale results
  cannot replace a newer scan;
- never exposes sidecar spawn permission to the webview.

If a Python exception is recoverable, the sidecar returns a typed failure and
continues. If native dependencies or the interpreter crash, Rust records a
per-file failure, restarts the sidecar, and continues the queue. Benchmark a
small pool only if one worker cannot meet measured throughput.

### Implemented Rust transport slice

The selected crate now exports a native `ParserSupervisor` that owns one child
and one pipe worker. It sends bounded JSON-lines requests, checks versioned
reply envelopes and correlation IDs, and supports deadlines, cancellation,
crash recovery, configured request-count recycling, and explicit/drop shutdown.
Failed requests are never replayed. Paths travel in JSON rather than shell
commands, and subprocess stderr is discarded so arbitrary private text does
not enter application logs. Real parser exchanges and subprocess fault tests
cover this transport on the existing Windows and Linux CI lanes.

This is the first supervision slice for issue #226. Successful result bodies
remain untrusted. Semantic field/capability/fingerprint validation, current
file/root revision checks, persistence, memory/CPU qualification, bounded
redacted diagnostic capture, scanner integration, and packaged executable
resolution remain deferred. No renderer command or FLP content read is enabled
by this module. See the [crate documentation](crates/flp-parser/README.md) for
the exact implemented limits and lifecycle behavior.

## Packaging recommendation

If Python/PyFLP is selected, its candidate package is a PyInstaller **one-directory**
bundle registered as a Tauri external binary. One-directory avoids the unpack
work and antivirus friction commonly associated with self-extracting one-file
bundles; the PoC must validate that assumption on target machines.

Pin Python, PyFLP, all transitive dependencies, and build hashes. Produce an
SBOM. Sign the sidecar and enclosing Windows installer. Build each target on its
native CI runner and name binaries using Tauri's target-triple convention.
Future macOS builds require separate universal/architecture artifacts,
notarization, and the same protocol tests.

Tauri explicitly supports embedding sidecars built in any language, including
Python/PyInstaller examples, through `externalBin`. Run it from trusted Rust
code under the narrowest capability rather than granting generic shell access
to JavaScript.

### Alternatives considered

| Option | Advantages | Problems | Position |
| --- | --- | --- | --- |
| Packaged Python sidecar | Reuses PyFLP; process isolation; independent upgrade | Runtime size, signing/AV, license and compatibility gates | Not selected; future adoption requires a new decision |
| Require user Python | Small app download | Fragile setup, dependency conflicts, poor onboarding | Reject for product builds |
| Embed CPython in Rust process | Potentially lower IPC overhead | Crash/failure and licensing boundary become tighter; complex packaging | Reject initially |
| Local HTTP parser service | Familiar API | Port/lifecycle/auth surface with no product benefit | Reject; stdio is simpler |
| Independent Rust parser | No Python runtime; direct control of supported fields | Format research and ongoing compatibility maintenance | Selected for the next implementation; distribution gates remain |
| Remote parsing service | Central upgrades | Uploads private FLPs, breaks offline/privacy goals | Reject |

## Expected metadata reliability tiers

These are historical hypotheses to test, not product promises. PyFLP API
references below describe possible future coverage, not the selected backend.
The selected Rust implementation's current fields and limits are listed in its
[crate documentation](crates/flp-parser/README.md); the other rows stay deferred.

| Metadata | Likely source | Initial status | Required validation |
| --- | --- | --- | --- |
| Filename/path/size/filesystem times | Rust filesystem | High | Windows/DriveFS semantics; creation time is not portable |
| FL version, title, author, genre, comments, PPQ, base tempo | PyFLP project API | Medium-high | Version matrix, Unicode, absent and rich-text cases |
| Time signature | PyFLP arrangements | Medium-high | Multiple arrangements and old versions |
| Pattern/channel counts and names | PyFLP | Medium-high | Empty/deleted objects, old/new versions |
| Playlist/mixer tracks | PyFLP | Medium | Current FL versions; open 2025 playlist issue |
| VST/native plugin display names | PyFLP wrapper/plugin models | Medium | VST2 vs VST3 IDs, Patcher, Waves/shell plugins, unknown state |
| Stable plugin identifier/vendor | Wrapper events where present | Low-medium | Normalize without inventing missing values |
| Sample/audio references | Channel/plugin events | Medium | Encodings, macros, relative/data paths; do not claim resolved/missing yet |
| Automation/markers/MIDI notes | PyFLP | Medium | Summaries vs huge detail; unsupported event handling |
| Arrangement duration/bar count | Calculated from parsed timeline | Low-medium/inferred | Tempo automation, clip offsets, markers, multiple arrangements |
| Project creation time | Filesystem plus possible FL metadata | Low-medium | Label sources independently; neither is universal truth |
| FL embedded `time_spent` | PyFLP project API | Low/uncertain | Semantics, precision, version behavior; never tracked time |
| Missing dependencies | Resolver combining references + filesystem | Low/inferred | Drive placeholders, search roots, plugin inventory limitations |

Project comments and all strings must be size-limited and rendered as text or
through a sanitizer. Plugin state blobs are neither stored in sync nor exposed
to the UI unless a future diagnostic need is reviewed.

## Duration estimation

For each arrangement, find the latest meaningful playlist end in ticks and
retain both raw evidence and the calculated value. A simple constant-tempo
estimate is:

```text
quarter_notes = max_end_ticks / PPQ
seconds = quarter_notes * 60 / BPM
bars = quarter_notes / (time_signature_numerator * 4 / denominator)
```

This formula is valid only when tempo/time-signature behavior is adequately
represented. Tempo automation requires integration over tempo events. Clip end
offsets, muted clips, loop/selection markers, tail effects, unused clips,
multiple arrangements, and a distant accidental clip all affect interpretation.

Store per-arrangement raw max-end and method. The effective project duration
defaults to the current arrangement when known, otherwise the longest
non-empty arrangement, and is labeled “estimated.” Offer a later manual override.
Do not infer audio tail length from plugin state.

## Research-only probe completed

On 2026-09-04, the upstream PyFLP repository was cloned to an OS temporary
directory at commit `f937126b888ce94271bfea631b89166c74056530`. Nothing from the
fixture set was copied into Fruitboard.

Using Python 3.8.5 and the repository code:

- the 190,128-byte upstream `FL 20.8.4.flp` parsed in about 239 ms on the
  current development machine;
- it reported FL 20.8.4.2576, 69.42 BPM, 4/4, PPQ 96, five patterns, nineteen
  channels, two arrangements, and 127 mixer inserts;
- named VST data included Sylenth1 on a channel and OTT on a mixer slot;
- the first arrangement contained 22 playlist items with raw maximum end 1536
  ticks (four 4/4 bars at PPQ 96), while the second had marker data but no clips;
- six deliberately corrupted upstream FLPs each failed quickly with typed
  `HeaderCorrupted` exceptions rather than terminating the process;
- the parsed embedded time was 9,119.258001 seconds, while the upstream test has
  a disabled expectation for 9,353 seconds.

Timing is a single warm developer-machine observation, not a benchmark. The
fixture was created specifically for PyFLP and is FL 20-era data, so it says
little about current compatibility.

Python 3.8.5 is end-of-life and will not be a packaging baseline. P0-A and
P0-B/P0-G must use a supported, pinned Python toolchain (candidate: Python
3.11) and exercise the FL 21, FL 2024, FL 2025/current corpus before PyFLP can
be considered for adoption.

## Required parser matrix PoC (P0-A)

Create a separate research PR containing only harness code, fixture manifests,
expected normalized JSON, and results. Run it with the supported, pinned Python
3.11 candidate rather than the research machine's Python 3.8.5. Proposed corpus:

- empty/minimal projects from representative FL versions that can legally be
  produced: older supported baseline, FL 20, FL 21, FL 2024, FL 2025/current;
- base metadata, Unicode title/comments/path, non-4/4 signatures, fine tempo;
- multiple arrangements, muted/offset/distant clips, tempo automation, markers;
- pattern MIDI, automation clips, audio clips, mixer routing;
- native generators/effects, VST2, VST3, bridged/shell/Patcher where available;
- relative, absolute, missing, Unicode, and DriveFS sample references;
- locked/in-progress save, truncated/corrupt file, random bytes, oversized
  strings/counts, and unsupported event versions.

For each field record: expected value, observed value, status, warning/error,
FL version, parser/library version, elapsed time, and peak memory. Run the same
corpus against stable PyFLP and a reviewed upstream commit if they differ.

Acceptance is field-specific, not “the file parsed.” A successful parse with an
empty playlist is a compatibility failure when the fixture contains clips.

No personal production project may enter the repository. If representative
features can only be captured from a real project, request permission, minimize
and sanitize it, document provenance, and review its binary diff/hash before a
force-add.

## Packaging and update PoC (P0-B/P0-G)

Measure cold/warm start, first/subsequent parse, memory, artifact size, crash and
timeout recovery, paths with spaces/Unicode, read-only enforcement, antivirus
behavior, NSIS/MSI install/uninstall, code signing, and sidecar replacement
during app updates. Test Windows x64 first and keep ARM64/macOS build definitions
explicitly portable.

## Licensing gate (P0-C)

PyFLP declares GPL-3.0. Bundling it, communicating with it as a sidecar, and
distributing the combined product may impose source and license obligations.
Process separation is an engineering boundary, not a promise that copyleft
obligations disappear. Before any distribution:

1. choose Fruitboard's intended source/distribution license;
2. obtain competent legal guidance or explicit alternative licensing from the
   copyright holder as appropriate;
3. record the decision and attribution/source-offer requirements;
4. if incompatible, do not ship PyFLP—evaluate a clean-room/parser alternative.

This document identifies the issue; it is not legal advice.

## References

- [PyFLP PyPI metadata and feature summary](https://pypi.org/project/pyflp/)
- [PyFLP project API](https://pyflp.readthedocs.io/en/latest/reference/project.html)
- [PyFLP event model warning](https://pyflp.readthedocs.io/en/stable/reference/events.html)
- [PyFLP GPL-3.0 license](https://github.com/demberto/PyFLP/blob/master/LICENSE)
- [PyFLP FL Studio 2025 playlist issue](https://github.com/demberto/PyFLP/issues/200)
- [Tauri external binary documentation](https://v2.tauri.app/develop/sidecar/)

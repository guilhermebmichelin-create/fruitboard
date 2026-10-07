# Rust FLP parser process

This is the first product implementation slice after the owner selected Rust
under [ADR-002](../../docs/adr/002-flp-parser-process.md). Its code derives from
the bounded research parser at commit `080e825`. It reads one explicit FLP
read-only, returns the four initial metadata fields plus channel and pattern
counts, pattern names, playlist pattern clips and timing summaries, and the
narrow generator-name field and project facts described below. It keeps typed failure/partial
outcomes tested against the approved thirteen-file corpus.

The executable uses protocol version 1 and schema version 2 as newline-delimited
JSON on stdin/stdout. It accepts `describe`, `healthCheck`, and `parse` methods.
`parse` requires an absolute path, a non-empty `allowedRoots` list of enabled
roots, and an expected size and modified timestamp in milliseconds. Parent
traversal, linked/reparse ancestors or leaves, Windows device paths and
alternate streams are rejected. Authorization and parsing use the same open
file, with guarded ancestors on Windows and descriptor/identity checks on
Linux; other platforms fail closed pending qualification. The result reports
the SHA-256 of the bytes parsed, verified by a second bounded read from that
handle after parsing (`INPUT_CHANGED` on a mismatch). This is an observation
window, not a guarantee of future filesystem freshness. Schema-1 requests
receive `UNSUPPORTED_SCHEMA_VERSION`.
The only optional feature list is `["basic-metadata"]`. Every
response carries the request ID; malformed requests receive a fixed error
code. Requests are capped at 64 KiB and responses at 256 KiB. The parser caps
files at 64 MiB (67,108,864 bytes), events at 100,000, and channels at 256.
Event 213 (saved plugin data) may occupy the file budget: opaque state and path
subrecords are skipped by checked lengths, with at most 1024 wrapper subrecords
and unchanged name/vendor text bounds. Other event payloads remain capped at
2 MiB. No plugin state is loaded or returned. Unique saved patterns and playlist
clips are each capped at 1024. Only the three exact saved builds
in the approved corpus currently return
metadata; other build strings receive `UNSUPPORTED_SAVED_VERSION`.

The native `supervisor` module implements the process-transport slice. A trusted
native caller supplies the absolute executable path, input fingerprint, and
allowed roots; renderer input cannot select an executable or command arguments.
One supervisor owns one child and sends one request at a time, with bounded
64-KiB requests, 256-KiB replies, and at most 256 roots per request. Its default
deadline is ten seconds and its default process lifetime is 256 completed
requests. Both are configurable within fixed limits. Pipe writes and reads run
on one owned worker so a child that stops reading cannot block the caller past
the request deadline. Cancellation, timeout, malformed replies, and transport
failures retire the child without replaying the request. Subsequent requests can
start a replacement after the previous process is stopped and reaped. Explicit
shutdown and dropping the supervisor release the child and worker. Windows
launches hide the console window.

The transport checks protocol/schema versions, correlation IDs, mutually
exclusive result/error envelopes, and bounded rejection-code syntax. Successful
result objects remain untrusted: application integration must validate their
field semantics, capabilities, fingerprint, and current file/root revisions
before persistence. Raw subprocess stderr is discarded rather than captured or
logged; transport errors use fixed categories with no paths. OS process startup
and termination are outside the request timer, and this slice does not qualify
memory/CPU limits or descendant-process containment. Only the trusted parser
executable is supported.

Issue #234 includes this executable in the separate unsigned Windows desktop
development package. Native lifecycle checks cover fixed installed resolution,
validated health/capabilities, reuse, rejected requests, abrupt child exit and
recovery, explicit shutdown, missing-binary containment and reinstall. The host
owns the supervisor; the renderer receives no process permission. The native
`process_id` accessor observes ownership, not liveness or termination authority.

This crate is not yet wired to scanner analysis jobs. The existing
filesystem-only scan path continues to avoid FLP content reads. Metadata
persistence, app integration, richer compatibility, resource qualification and
production distribution remain gates in ADR-002.

## Initial native result validation

The `validation` module adds a typed projection of the initial five fields:
saved version, base tempo, channel count, channel names and raw sample
references. A validated descriptor must identify the selected adapter/version,
advertise each initial field once, and stay within the parser's resource
ceilings. `validate_reply` consumes an envelope-checked supervisor reply and
checks field forms, tempo/count ranges, list lengths and per-channel inference
method/confidence. Missing per-channel values stay missing; mixed stored and
inferred names keep their individual provenance. Text is bounded by 4095
UTF-16 code units (the source's 8192-byte limit including its terminator),
allowing valid Unicode output larger than 8192 UTF-8 bytes.

The returned fingerprint must match the requested size/mtime and contain a
lowercase SHA-256 digest. If the caller independently knows a digest, that must
match too. Without an independent digest, hash shape validates the protocol
claim; it does not independently prove the parser read those bytes. The native
caller supplies captured/current root and file IDs, revisions, enablement and
fingerprints. A changed observation discards the result. These observations
must come from native authority, not parser or renderer input; integration
still needs a freshness recheck in the eventual publication transaction.

Complete/partial metadata and fixed failed/unsupported/rejected categories are
distinct. Diagnostics retain only the known unsupported-event category, up to
1024 entries and never more than the declared event count. Unrecognized codes
fail closed. Unknown fields and the currently unvalidated pattern, generator
and playlist/timing extensions are discarded from the initial projection.
Validated metadata has no automatic Debug/Serialize implementation because
names and raw references can contain private text. Raw references are neither
resolved nor read. The development package uses descriptor validation for
lifecycle checks only. Project reply validation has no scanner/application
publication call site, renderer command or persistence effect. Application
integration remains a separate slice.

## Saved pattern note records

Adapter 0.1.2 advertises additive `patternNoteCounts` on protocol 1/schema 2.
Independent approved [F16/F17](../../docs/research/parser-pattern-notes-290-result.md)
qualify exact build 26.1.0.5530: counts 3/2/4 include ordinary, slide and step
records across channels once per saved pattern, independently of Playlist use.
No audible-note total, pitch, flags, position, length or per-note array is returned.

Only the contiguous initial event-65 ID / event-224 payload series is bound.
Before the series, only header event kinds independently observed in the F16
genuine save/candidate pass. The registration header removed during sanitation
is skipped without copying or returning it; it cannot restore later authority.
Any interruption ends authority permanently; later property IDs cannot restore
it. An unbound payload invalidates the whole summary. Duplicate payloads are
unsupported even when identical. A missing payload is unavailable, including
the GUI-empty F17. Zero-length and non-24-byte-multiple payloads are unqualified.
Other saved builds keep an unsupported field without losing existing metadata.

The summary carries `coverage: stored-pattern-note-records`. All extracted
entries use `value`; incomplete entries use `items` and fixed reason
`PATTERN_NOTE_COUNTS_INCOMPLETE`, unavailable unless any entry is unsupported.
Entries are sorted by saved ID and retain individual states. Global build,
binding, missing-pattern and limit failures have no entries. Whole parse failure
keeps its existing failed outcome.

Advertised positive ceilings are `maxNoteRecordsPerPattern: 65536` and
`maxNoteRecordsTotal: 262144`. Borrowed slices are counted using checked lengths;
every candidate charges a ceiling of payload bytes / 24 before classification,
including duplicates, malformed and unbound candidates. No records are traversed
or allocated. Existing file/event/pattern/output/deadline limits remain intact.
These are safety bounds, not performance qualification.

Full typed validation selects the field only when advertised with both limits
and pattern IDs/names. It checks exact shapes, literal coverage, fixed reasons,
verified build, ordered matching IDs, positive integer counts and checked totals.
Zero counts are rejected while explicit-zero encoding remains unqualified.
Malformed advertised fields reject the result; unadvertised extensions are
discarded. Matching adapter-version checks remain exact and initial-only
validation remains usable. Typed accessors retain counts/states only, without
automatic serialization. Storage and Library projection are a separate slice;
no native authority, command, sample read, filesystem grant or migration is added.

## Project facts

Issue #232 adds `projectCreatedLocal` (the embedded creation date with an
unspecified local timezone), `flStudioTimeSpentMs` (FL Studio's saved counter),
and optional `filesystemCreatedAtMs` from the authorized file handle. The
embedded project creation date is separate from the filesystem creation date.
Missing, malformed or repeated project-info records keep explicit states;
neither date nor time spent is guessed. The counter is never tracked work or
automatically summed across project versions.

`pluginReferences` retains bounded top-level saved class names and, for
recognized Fruity Wrapper metadata records, factory name/vendor strings.
Sampler defaults remain inferred; malformed/unknown metadata stays unsupported.
Plugin paths/state are skipped and never returned or loaded. This is a saved
reference list with incomplete nested-plugin coverage, not an installed-plugin
inventory or general VST compatibility claim.

`playlistPatternSpanBars` adds a low-confidence bar-span estimate on the same
verified 96-PPQ, 4/4 pattern-clip layout as nominal seconds. Whole-arrangement
span uses the maximum clip end, including overlap/gaps; it is not a sum of
clip lengths or a rendered song duration.

`validate_project_reply` adds a separate full typed projection containing these
facts, authoritative request size/modified time, plugin references and checked
arrangement end/bars/seconds. It preserves the initial validation and freshness
checks, recomputes the maximum clip end and estimate formulas, and drops
unrelated JSON. It requires an advertising descriptor. The initial-only
`validate_reply` remains unchanged for its callers. No scanner/UI integration
is enabled. [Format evidence and limits](../../docs/research/parser-project-facts-232.md)
distinguish constructed tests from independent GUI compatibility qualification.

`channelCount` is `extracted` only after the bounded channel-event walk agrees
with the FLP header. A malformed file has a typed `failed` count, and an
unverified saved build has an `unsupported` count. The approved valid projects
each have one channel. Arrangement length and plugin details need separate,
pre-registered fixture coverage before they are claimed.

`channelGeneratorNames` is distinct from editable `channelNames`. For the exact
26.1.0.5530 build, a controlled GUI save identified native generator class
`3x Osc` in UTF-16 event 201, while its editable channel name
`Fixture Synth A` was in event 203. A channel with Event 21 type 0 and an
explicitly empty event 201 is reported as built-in `Sampler` by a labeled,
high-confidence inference from the approved 2026 minimal saves. A verified `3x Osc` class is
`extracted`. The array retains per-channel provenance in `items` and reports
`medium` aggregate confidence when stored and inferred classes are mixed.
Unknown classes or layouts are field-level `unsupported`, even if event 201
contains a plausible string. The 2024/2025 builds are `unsupported` for this
field. The full `validate_project_reply` projection now retains this extension
only when advertised by the selected descriptor, after checking build/class
allowlists, cardinality, dense parser order, item/aggregate values and inference
method/confidence. It preserves verified siblings of unsupported classes and
returns typed reasons without arbitrary plugin text. Descriptors without the
extension produce unsupported instrument details; the initial-only projection
still discards generator metadata.

This channel-only field does not decode mixer effects or VST identity;
the separate `pluginReferences` field has the narrower metadata support above.
Plugin state is not decoded.
The owner approved the exact sanitized two-channel F14 file for corpus
inclusion on 2026-09-29. Its ordinary test checks the class and editable
label separately; it does not establish broader compatibility.

The `patternCount` extension counts distinct saved pattern IDs for
26.1.0.5530. That build repeats IDs for note and property sections; repetitions
do not add patterns. When no IDs are stored, the count is `unavailable` with
`PATTERN_DATA_NOT_STORED`, because the GUI can display an implicit empty
`Pattern 1`. Other currently supported builds receive `unsupported` for this field.
The [GUI validation](../../docs/research/parser-pattern-count-184-result.md)
verified three patterns and four playlist placements. The owner approved the
exact sanitized F13 fixture on 2026-09-29, as recorded in the corpus manifest;
PR #185 merged its ordinary corpus test.

`patternNames` reads stored names for that same 2026 build. Its extracted
`value` is an array ordered by saved ID, with entries shaped as
`{"patternId":1,"name":{"status":"extracted","value":"Fixture Pattern A"}}`.
If any name is missing, the field is `unavailable` with
`PATTERN_NAME_NOT_STORED`; `items` retains each ID and its individual name
status. No default name is invented. No stored IDs means
`PATTERN_DATA_NOT_STORED`; supported 2024/2025 builds return `unsupported` with
`PATTERN_NAMES_UNVERIFIED_BUILD`. Identical repeated names are accepted;
conflicting names, malformed UTF-16, and names outside a pattern context fail
with typed codes. Text uses the existing 8192-byte limit and response limits
remain in force. This field does not describe pattern notes or playlist length.

Issue #263 carries both advertised pattern fields through full typed validation
into the application's bounded immutable projection and authorized Library
details view. Count/ID/name relationships and playlist references are checked;
unknown extensions are discarded. Older saved projections without patterns
remain readable without reparsing. See the
[Library patterns review](../../docs/review/library-patterns-263/README.md).

`playlistPatternClips` reads event-233 pattern placements only for the exact
2026 build validated by the [F13 playlist study](../../docs/research/parser-playlist-188-result.md).
The extracted value is an array in saved record order, with entries like
`{"patternId":1,"startTick":0,"lengthTick":384,"trackToken":499}`.
Ticks and the track token are raw stored values; no bar number or track number
is inferred. F13 has A, A, B, C at ticks 0, 384, 768, and 1152, each 384
ticks long. The approved 2026 minimal projects have empty saved arrangement
payloads and return an extracted empty array. An absent payload is
`unavailable`. Other supported builds and unfamiliar layouts, clip kinds,
references, or multiple arrangement payloads return `unsupported` for this
field. The parser rejects clip counts over 1024 and overflowing end positions.
Audio clips, automation clips, track normalization, and rendered song length
are not yet decoded. The existing response-size fallback still applies.

`playlistPatternEndTick` is the greatest saved pattern-clip start plus length,
not a sum of clips. The approved F13 arrangement ends at tick 1536. An empty
saved pattern playlist reports `unavailable`, because it has no pattern-clip
end. Unsupported playlist layouts keep their field-level reason.
`playlistPatternNominalSeconds` is an `inferred`, low-confidence conversion
using the saved base BPM and the verified 96-PPQ, 4/4 timing layout. F13 yields
about 7.385 seconds if its base tempo stays constant. This is a nominal span
through the pattern clips, not the song's rendered length: tempo automation,
other clip kinds, and sound tails are outside this calculation. Unknown PPQ or
meter remains `unsupported`; missing base tempo is `unavailable`. See the
[timing evidence](../../docs/research/parser-extent-190-result.md).

The corpus tests read the exact approved fixtures under `fixtures/parser-corpus/`.
The repository privacy check validates their allowed paths and SHA-256 values;
the parser tests verify fields, typed failures, source-byte preservation, and
the process protocol. Supervisor tests also run a real parser exchange and
stdlib-only subprocess fault fixtures for crashes, stalls, blocked stdin,
oversized/truncated replies, cancellation, stderr flooding, process recycling,
and shutdown. The fault executable is compiled once per test run with the pinned
`rustc` into an independent temporary directory. It has no product parser or
input-file authority. The test command is:

```powershell
cargo test -p fruitboard-flp-parser --locked
```

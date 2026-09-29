# Rust FLP parser process

This is the first product implementation slice after the owner selected Rust
under [ADR-002](../../docs/adr/002-flp-parser-process.md). Its code derives from
the bounded research parser at commit `080e825`. It reads one explicit FLP
read-only, returns the four initial metadata fields plus channel and pattern
counts, pattern names, and playlist pattern clips. It keeps typed
failure/partial outcomes tested against the approved
eleven-file corpus.

The executable uses protocol version 1 and schema version 1 as newline-delimited
JSON on stdin/stdout. It accepts `describe`, `healthCheck`, and `parse` methods.
`parse` requires an absolute path and an expected size and modified timestamp
in milliseconds. The only optional feature list is `["basic-metadata"]`. Every
response carries the request ID; malformed requests receive a fixed error
code. Requests are capped at 64 KiB and responses at 256 KiB. The parser caps
files at 4 MiB, event payloads at 2 MiB, events at 100,000, and channels at
256. Unique saved patterns and playlist clips are each capped at 1024. Only the three exact saved builds
in the approved corpus currently return
metadata; other build strings receive `UNSUPPORTED_SAVED_VERSION`.

The trusted native supervisor must authorize a locator inside an enabled root,
enforce a per-request deadline, and validate response schema and fingerprint
before any result is persisted. This crate is not yet wired to the scanner or
packaged into the desktop application. The existing filesystem-only scan path
continues to avoid FLP content reads. Packaging, crash/restart behavior, richer
compatibility, and distribution qualification remain gates in ADR-002.

`channelCount` is `extracted` only after the bounded channel-event walk agrees
with the FLP header. A malformed file has a typed `failed` count, and an
unverified saved build has an `unsupported` count. The approved valid projects
each have one channel. Arrangement length and plugin details need separate,
pre-registered fixture coverage before they are claimed.

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
the process protocol. The test command is:

```powershell
cargo test -p fruitboard-flp-parser --locked
```

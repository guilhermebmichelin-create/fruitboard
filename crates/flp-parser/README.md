# Rust FLP parser process

This is the first product implementation slice after the owner selected Rust
under [ADR-002](../../docs/adr/002-flp-parser-process.md). Its code derives from
the bounded research parser at commit `080e825`. It reads one explicit FLP
read-only, returns the four initial metadata fields plus a verified channel
count, and keeps the typed failure/partial outcomes tested against the approved
ten-file corpus.

The executable uses protocol version 1 and schema version 1 as newline-delimited
JSON on stdin/stdout. It accepts `describe`, `healthCheck`, and `parse` methods.
`parse` requires an absolute path and an expected size and modified timestamp
in milliseconds. The only optional feature list is `["basic-metadata"]`. Every
response carries the request ID; malformed requests receive a fixed error
code. Requests are capped at 64 KiB and responses at 256 KiB. The parser caps
files at 4 MiB, event payloads at 2 MiB, events at 100,000, and channels at
256. Only the three exact saved builds in the approved corpus currently return
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
each have one channel; pattern counts, arrangement length, and plugin details
need separate, pre-registered fixture coverage before they are claimed.

The corpus tests read the exact approved fixtures under `fixtures/parser-corpus/`.
The repository privacy check validates their allowed paths and SHA-256 values;
the parser tests verify fields, typed failures, source-byte preservation, and
the process protocol. The test command is:

```powershell
cargo test -p fruitboard-flp-parser --locked
```

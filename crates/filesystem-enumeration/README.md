# Fruitboard filesystem enumeration (#36)

This is the isolated Windows metadata-enumeration boundary for Issue #36. It
does not belong to the production Cargo workspace yet: the crate is standalone
so this PR does not change the shared manifest, lockfile, SQLite code, queue,
publication path, watcher, client, or cross-agent contracts.

The implementation is deliberately an enumeration producer, not a scanner
activation. It discovers `.flp` names case-insensitively, obtains filesystem
metadata and qualified local-NTFS identity when available, and sends bounded
observation batches to a caller-owned sink. It never opens FLP content, hashes,
hydrates, parses, writes, renames, deletes, or follows nested reparse points.

## Storage-agent interface proposal

The storage owner can integrate the public port without depending on the
Windows implementation details:

```text
enumerate(port, root, limits, cancellation, sink, progress) -> Report

port.inspect_root(path) -> RootMetadata
port.open_directory(path) -> DirectoryCursor
cursor.next_entry() -> EntryName | end
port.read_metadata(path) -> FileMetadata

sink.accept(ObservationBatch) -> SinkResult
```

`FilesystemPort` is intentionally a metadata-only port. There is no read-bytes
method. A recording/fake port can deterministically inject access denial,
not-found/disappearance, cursor failure, root loss, and resource errors without
touching a real source tree. The Windows port uses `symlink_metadata`,
directory enumeration, attribute-only handles, and `FILE_ID_INFO`; all handles
share read/write/delete and request only `FILE_READ_ATTRIBUTES`.

### Locator and display path

- `DisplayRelativePath` is the root-relative path for a Library/list consumer.
  It preserves the spelling returned by the directory enumeration and uses `\`
  separators. It is never used as proof of physical identity.
- `NormalizedLocator` is the boundary-owned root-relative locator. It rejects
  absolute, parent, current, empty, NUL-containing, and malformed components;
  uses `\` separators; and compares components using Windows
  `CompareStringOrdinal(..., TRUE)` semantics. It preserves the first observed
  case-preserving spelling for serialization, so a case-only rename is not
  silently treated as a different identity. Duplicate normalized locators
  invalidate the run rather than allowing enumeration order to choose a row.
- The selected root is expected to be the canonical path produced by the #34
  native root boundary. Traversal constructs children only from validated
  single directory-entry names and never canonicalizes a nested path or follows
  it, which keeps nested junctions/symlinks outside the selected root.

### Identity and metadata

`Observation` contains both path forms, byte size, modified time as signed Unix
nanoseconds, and an optional `QualifiedIdentity`:

```text
QualifiedIdentity {
    volume_serial: u64,
    file_id: u128,
    qualification: LocalNtfs,
}
```

The identity tuple is a lookup signal, not a location key. Hardlink aliases
produce one observation per normalized path and may carry the same tuple. The
enumerator only labels an identity `LocalNtfs` after the Windows volume reports
NTFS and a fixed local drive type; DriveFS, FAT32, network, and other non-NTFS
modes are not qualified by this slice. This check is not DriveFS
qualification: DriveFS behavior remains unverified under #47. Metadata may
remain usable with `identity = None`, but the absence of identity never
establishes a move or logical-project relationship. Placeholder/recall-marked
entries do not receive an identity handle, avoiding intentional hydration.

### Batches and limits

`EnumerationLimits` bounds directories, pending directory work, entries
examined, FLP observations, path bytes, diagnostic entries, batch records,
batch bytes, and emitted batch count. The default observation batch is at most
512 records and the default batch quota is 256 MiB, matching the provisional
Phase 2 budget. Traversal is streaming: directory cursors, pending directory
paths, and the current batch are bounded; the complete observation set is never
accumulated in memory. Exceeding any limit yields `ResourceLimit`, drops the
current provisional batch, and makes the report non-authoritative. The storage
sink must discard already accepted provisional batches when the report is not
authoritative.

### Cancellation and progress

`Cancellation` is cooperative. The worker checks it before root inspection,
between directory-cursor calls, before metadata calls, before batch delivery,
and before final root validation. An OS call already in progress cannot be
preempted by this port; cancellation is still reported before any further
batch/finalization step. `ProgressSink` receives only bounded counts and a
final flag, never paths or file contents. The default reporter throttles to the
accepted four-updates-per-second budget while always allowing the terminal
update.

### Exclusions, coverage, and authority

Reparse-point files/directories and unsupported non-file entries are
`PolicyExclusion`s. They are not coverage failures and are never traversed.
Non-FLP regular files are a format filter, not an error. Denied directories,
disappearing entries, failed required metadata, root loss/change, duplicate
locators, sink rejection, cancellation, and resource exhaustion are
`CoverageFailure`s or non-authoritative outcomes. A child failure is isolated
so independent siblings can still be observed, but the final report remains
non-authoritative.

The only authoritative result is `Outcome::Complete` after all in-policy
directories and required metadata were covered, the root remained the same,
and all batches were accepted. An empty root returns `Complete` with zero
observations and zero batches. `Denied`, `Partial`, `RootUnavailable`,
`Cancelled`, `ResourceLimit`, `SinkFailed`, `Invalid`, and
`UnsupportedFilesystem` never authorize missing-file transitions. The storage
publication transaction must still fence root generation/configuration,
lease/session/deadline, and durable cancellation before applying any complete
run; this crate does not write SQLite.

## Integration instructions

1. Review the interface above with the storage/publication owner.
2. Add this crate to the root Cargo workspace in a separate shared-manifest
   change; do not copy its standalone `[workspace]` section into the root.
3. Wire `Observation` fields to the storage staging DTO without making the
   identity tuple unique. Keep one location row per locator.
4. Give the worker a run-scoped sink that stages by run ID and discards all
   provisional batches unless the report is `Complete` and the storage fence
   passes in the final transaction.
5. Map `Complete` to the reconciler's absence-authority input only after the
   storage checks. Map all other outcomes to retained prior committed results.
6. Keep production scan activation hidden until #40 publication and the
   incomplete/restart/cancellation tests pass. Watcher integration remains #37.

## Test evidence and limits

The unit suite uses deterministic fake ports and a recording sink for every
coverage outcome, batching/resource bound, cancellation, duplicate locator,
and empty-root case. Windows fixture tests use disposable files only and check
Unicode/long paths, mixed-case extensions, hardlink aliases, reparse
exclusions, marker-byte preservation, and real metadata traversal. ACL-based
denial and DriveFS/streamed-placeholder behavior are environment-dependent;
they are documented as unavailable/ignored when the host cannot provide them,
not reported as passing qualification evidence.

Local evidence on the available Windows host with the pinned Rust 1.98.1
toolchain:

```text
cargo test --manifest-path crates/filesystem-enumeration/Cargo.toml
18 passed; 2 ignored; 0 failed
cargo clippy --manifest-path crates/filesystem-enumeration/Cargo.toml --all-targets -- -D warnings
passed
cargo fmt --manifest-path crates/filesystem-enumeration/Cargo.toml -- --check
passed
```

The two ignored tests are the ACL-denied fixture (requires a disposable ACL
policy setup) and DriveFS/streamed placeholders (not installed or available on
this host). The local NTFS Unicode/long-path, mixed-case, hardlink, junction,
metadata-only marker, cancellation, disappearance, denial-injection, and
resource-bound tests ran successfully. This is correctness evidence for the
isolated crate, not a DriveFS, non-NTFS, benchmark, or production-scan claim.

An existing-workspace `cargo test --workspace --locked` attempt was not
counted as passed: after compiling the existing crates, the desktop link step
hit MSVC `LNK1318`/`no space on device` while writing its generated PDB. The
generated target was cleaned; rerun that broad workspace check on a host with
sufficient temporary/build space.

This slice does not qualify DriveFS, FAT32, cross-volume identity, network
shares, watcher delivery, parser behavior, SQLite staging/publication, crash
recovery, benchmarks, or production activation. Those remain owned by #47/#48,
#37, #40, #41, and the designated storage/execution owners.

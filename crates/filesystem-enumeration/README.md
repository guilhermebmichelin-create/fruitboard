# Fruitboard filesystem enumeration (#36)

This is the isolated Windows metadata-enumeration boundary for Issue #36. It is
now a member of the shared Cargo workspace (`crates/filesystem-enumeration`,
crate `fruitboard-filesystem-enumeration`) and has a dedicated Windows CI job
(`enumeration-windows` in `.github/workflows/foundation.yml`) that runs fmt,
clippy, and the crate's own tests. The shared lockfile only gained this crate
and its pinned dependencies; no other workspace member changed. Storage
publication, the queue, the watcher, and the client remain owned by other
agents and are not touched by this slice.

The implementation is deliberately an enumeration producer, not a scanner
activation. It discovers `.flp` names case-insensitively, obtains filesystem
metadata and qualified local-NTFS identity when available, and sends bounded
observation batches to a caller-owned sink. It never opens FLP content, hashes,
hydrates, parses, writes, renames, deletes, or follows nested reparse points.
Directory traversal is handle-bound: no child is reopened through a
path-constructed `read_dir` after an inspection.

## Storage-agent interface proposal

The storage owner can integrate the public port without depending on the
Windows implementation details:

```text
enumerate_into_run(
    port, root, limits, cancellation, storage, run_id, progress
) -> Report

port.inspect_root(path) -> RootMetadata
port.open_root(path) -> OpenedDirectory
opened.cursor.next_entry() -> EntryName | end
opened.cursor.read_metadata(entry) -> FileMetadata
opened.cursor.open_directory(entry) -> OpenedDirectory

storage.stage_batch(run_id, ObservationBatch) -> SinkResult
storage.invalidate_run(run_id)
```

`FilesystemPort` is intentionally a metadata-only port. There is no read-bytes
method. A recording/fake port can deterministically inject access denial,
not-found/disappearance, cursor failure, root loss, replacement, and resource
errors without touching a real source tree. `OpenedDirectory` owns the
directory capability and its metadata/case-mode decision. The Windows port
enumerates with `NtQueryDirectoryFile` against that handle, opens child
metadata with `NtCreateFile` relative to the parent handle, and uses
`FILE_OPEN_REPARSE_POINT` for every open. The `Nt*` imports are linked
against `ntdll.lib` explicitly so the link does not depend on the build host.
It requests directory listing and attribute access only, with read/write/delete
sharing; it never requests file content access. A truncated
`STATUS_BUFFER_OVERFLOW` reply is treated as a non-authoritative error
(`PortError::Other`), never parsed: the 64 KiB per-cursor buffer makes the
status practically unreachable, and the buffered bytes are not trustworthy
input when it does occur.

### Replacement and reparse safety

The initial root inspection and the root open are separate operations. The
enumerator compares their qualified identities before accepting the root
cursor. A child directory is opened only from its parent handle, with
`FILE_OPEN_REPARSE_POINT`; the handle is rejected if it is a reparse point,
not a directory, or has an identity different from the preceding metadata
inspection. A reparse discovered during this inspection/open window is a
`DirectoryChanged` coverage failure, not a successful policy exclusion.

The same rule applies when a directory entry turns out to be a
junction/reparse point while its metadata is read. A reparse-point **file**
(leaf entry) is a policy exclusion: it has no subtree, so excluding it cannot
hide coverage. A directory that turns out to be a junction at metadata time is
classified as a `DirectoryChanged` coverage failure with a non-authoritative
outcome instead: its subtree was never enumerated, and a policy exclusion
there would let the run finish `Complete` while storage marked the skipped
subtree's files missing. A reparse indication returned by a failed metadata
read (without complete metadata) is classified the same non-authoritative way.

Each child cursor retains a bounded chain of parent handles and expected
identities. It revalidates that chain before metadata, child-open, and each
directory-query operation. If an ancestor is replaced after a child handle is
created, the cursor returns `Changed` before yielding the next entry. Holding
the capability prevents a junction target from being enumerated; the final
root identity check separately prevents a root-path replacement from being
reported as authoritative. These are containment/safety checks, not a claim
that the filesystem is an atomic snapshot.

### Locator and display path

- `DisplayRelativePath` is the root-relative path for a Library/list consumer.
  It preserves the spelling returned by the directory enumeration and uses `\`
  separators. It is never used as proof of physical identity.
- `NormalizedLocator` is the boundary-owned root-relative locator. It rejects
  absolute, parent, current, empty, NUL-containing, separator-containing, and
  malformed components; uses `\` separators; and serializes the exact
  case-preserving spelling returned by the directory handle. It does not
  lowercase or otherwise rewrite the locator, and downstream code must treat
  the serialized value as opaque.
- Duplicate detection uses `DirectoryCaseSensitivity` from each opened
  directory. Case-insensitive components use Windows
  `CompareStringOrdinal(..., TRUE)` comparison; case-sensitive components use
  exact comparison. Thus `A.flp` and `a.flp` remain distinct in a
  case-sensitive directory while case-only duplicates in a normal directory
  invalidate the run. This decision belongs to the filesystem boundary and is
  not repeated in the reconciliation core.
- The selected root is expected to be the canonical path produced by the #34
  native root boundary. Traversal passes only validated single directory-entry
  names to handle-relative operations; it never canonicalizes a nested path or
  follows a nested reparse point.

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

The authority policy is intentionally asymmetric between entries and
directories, and the reason is structural:

- For a **file** observation, identity is a comparison/lookup signal. When the
  identity query fails (access denied, unsupported, other error) the file still
  gets its full path/size/mtime observation with `identity = None`, the count
  is reported in `identity_unavailable`, and the run can remain authoritative:
  a qualified-identity miss only weakens move/replacement evidence, it cannot
  hide coverage of the file's location.
- For a **directory**, the qualified identity is required traversal state: the
  handle-bound validation chain uses it to detect ancestor and child swaps.
  Without it the enumerator cannot prove it is enumerating the same directory
  it inspected, so the child is recorded as `DirectoryIdentityUnavailable` with
  a non-authoritative outcome.

Both sides follow one rule: identity is never a requirement where its absence
only weakens evidence, and it is required where traversal continuation depends
on it. Neither case ever proves a move or a merge on its own.

The timestamp conversion is exact at the source precision: signed Windows
FILETIME ticks are converted with
`(ticks - 116444736000000000) * 100` to signed Unix nanoseconds. The identity
DTO uses the volume serial as `u64` and preserves the opaque 16-byte Windows
file ID as a little-endian `u128`, matching the reconciliation metadata shape.
The boundary does not truncate to milliseconds; any storage-level precision
conversion must be an explicit Agent 1/storage contract decision rather than a
second implicit conversion here. Boundary tests cover epoch, one-tick
precision, and signed `i64` endpoints plus all-zero/all-one identity bytes.

### Batches and limits

`EnumerationLimits` bounds directories, pending directory work, entries
examined, FLP observations, path bytes, diagnostic entries, batch records,
batch bytes, and emitted batch count. The default observation batch is at most
512 records and the default batch quota is 256 MiB, matching the provisional
Phase 2 budget. Traversal is streaming: the complete observation set is never
accumulated in memory, and every work structure is bounded — but bounded is
not the same as small. The measured worst-case bounds are:

- Each open Windows directory cursor retains a fixed 64 KiB query buffer plus
  its validation chain. Pending directories stay open until visited, so the
  default `max_pending_directories` of 4096 cursors holds up to
  4096 × 64 KiB ≈ 256 MiB of cursor buffers (plus handles) before traversal
  exceeds the pending-directory limit and reports `ResourceLimit`. Meeting the
  provisional 128 MiB working-memory budget on wide trees therefore depends on
  this limit configuration, not on streaming alone; lowering
  `max_pending_directories` trades cursor memory for more `ResourceLimit`
  outcomes on deep trees. This is the measured structure of the current
  implementation, not a target reached by design.
- Accepted-but-unflushed batch bytes and the sink's staged batches are bounded
  by the batch quota; `estimated_bytes` and `pending_bytes` are tracked.

Exceeding any limit
yields `ResourceLimit`, and the enumerator invokes `BatchSink::discard()` for
the accepted and pending provisional work before returning a
non-authoritative report.

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

Reparse-point files (leaf entries) and unsupported non-file entries are
`PolicyExclusion`s. They are not coverage failures and are never traversed.
A directory junction discovered at metadata time is deliberately NOT a policy
exclusion; see "Replacement and reparse safety" — it is a `DirectoryChanged`
coverage failure with a non-authoritative outcome. Non-FLP regular files are a
format filter, not an error. Denied directories,
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

`RunScopedSinkAdapter` is the integration seam for that rule. It maps
`BatchSink::accept` to `RunScopedStorage::stage_batch(run_id, batch)` and maps
the enumerator's non-authoritative `discard` call to
`RunScopedStorage::invalidate_run(run_id)`. The adapter is also panic-safe:
while staging is provisional and un-invalidated, `Drop` invalidates the run,
so a panic after `stage_batch` cannot leave in-flight batches eligible; an
authoritative `Complete` run calls `release()` first and stays staged.
`enumerate_into_run` owns the
adapter lifetime, so a recording/fake storage test can assert that a failed
run has no retained staging while a successful empty run remains eligible for
the publication transaction. The adapter deliberately has no commit method.

## Integration instructions

1. Review the interface above with Agent 1/storage and keep the exact
   case-preserving locator serialization, per-directory case decision, signed
   Unix-nanosecond timestamp, and qualified identity conversions aligned.
2. Done: the crate is a member of the root Cargo workspace and
   `enumeration-windows` CI gates it on windows-latest.
3. Wire `Observation` fields to the storage staging DTO without making the
   identity tuple unique. Keep one location row per locator.
4. Use `enumerate_into_run` with a run-scoped sink that stages by run ID.
   `Complete` leaves batches eligible for the storage fence; every other
   outcome is already invalidated before the report is returned, and a panic
   during traversal invalidates in-flight staging during unwind.
5. Map `Complete` to the reconciler's absence-authority input only after the
   storage checks. Map all other outcomes to retained prior committed results.
6. Keep production scan activation hidden until #40 publication and the
   incomplete/restart/cancellation tests pass. Watcher integration remains #37.

## Test evidence and limits

The unit suite uses deterministic fake ports and recording sinks for every
coverage outcome, batching/resource bound, cancellation, duplicate locator,
run invalidation, and empty-root case. Replacement tests cover root replacement
between inspection/open, child replacement between metadata/open, and an
ancestor replacement detected by a bound cursor before its next entry; each
asserts zero out-of-root access and identifies whether a cursor was returned.
Case-sensitive-directory serialization and timestamp/identity boundary tests
are also deterministic. Windows fixture tests use disposable files only and
check Unicode/long paths, mixed-case extensions, hardlink aliases, junction
refusal, marker-byte preservation, and real handle-bound metadata
traversal. ACL-based denial and DriveFS/streamed-placeholder behavior are
environment-dependent; they are documented as unavailable/ignored when the
host cannot provide them, not reported as passing qualification evidence.

Local evidence on the available Windows host with the pinned Rust 1.98.1
toolchain, now against the shared workspace:

```text
cargo test -p fruitboard-filesystem-enumeration --locked
43 passed; 2 ignored; 0 failed
cargo clippy -p fruitboard-filesystem-enumeration --all-targets --locked -- -D warnings
passed
cargo fmt --all -- --check
passed
```

The two ignored tests are the ACL-denied fixture (requires a disposable ACL
policy setup) and DriveFS/streamed placeholders (not installed or available on
this host); they remain explicit unverified gates, never passing silently.
The local NTFS Unicode/long-path, mixed-case, hardlink, junction-refusal,
metadata-only marker, cancellation, replacement-race, case-mode,
timestamp/identity, run-invalidation, run-invalidation-on-unwind,
disappearance, denial-injection, and
resource-bound tests ran successfully. This is correctness evidence for the
crate, not a DriveFS, non-NTFS, ACL, benchmark, or production-scan
claim.

On the development shell the pinned Rust binaries were invoked from the local
rustup toolchain directory because `cargo` is not on `PATH`; the commands above
are the reproducible equivalents once the pinned toolchain is on `PATH`.
The real Windows tests ran on local NTFS. No case-sensitive NTFS directory
fixture, ACL fixture, DriveFS volume, FAT32 volume, network share, or streamed
placeholder was available; case-sensitive behavior is covered by the
deterministic port only.

An existing-workspace `cargo test --workspace --locked` attempt was not
counted as passed: after compiling the existing crates, the desktop link step
hit MSVC `LNK1318`/`no space on device` while writing its generated PDB. The
generated target was cleaned; rerun that broad workspace check on a host with
sufficient temporary/build space.

This slice does not qualify DriveFS, FAT32, cross-volume identity, network
shares, watcher delivery, parser behavior, SQLite staging/publication, crash
recovery, benchmarks, or production activation. Those remain owned by the
issues #47/#48, #37, #40, and #41 and by the designated storage/execution
owners.

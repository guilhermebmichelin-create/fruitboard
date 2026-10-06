# Bounded sample presence

ADR-007 slices 1 and 2, issues #280 and #282. This library implements portable
policy, request lifecycle rules and a Windows metadata adapter. The
[slice 3 host review](../../docs/review/sample-presence-ui-284/README.md) records
explicit command/Library integration in the opt-in `analysis-jobs` composition.
Default builds and PWA gain no functional sample probe.

## Authority contract

The trusted host captures enabled root/location revisions, qualified source
identity, size/time, snapshot and session under durable authorization. The DB
stores a folder grant/revision, not a root directory identity. When
`AuthorityFence.root_identity` is `None`, the Windows adapter captures the actual
root ID during qualification, pins its ancestry, and compares final root/name
observations to that held ID. `Some` also requires the earlier observed ID to
match. Both modes require local NTFS/source qualification and the same final
held-object/durable fences; neither accepts a renderer root identity.
`CapturedInput` validates shape; its constructor does not grant access. Old
snapshots without references return `ReferencesUnavailable`. The limits are
256 ordered slots, 256 KiB aggregate reference text and the existing per-string
4,095 UTF-16 / 12,285 UTF-8 caps. Sensitive input/authority types have no Debug.

Only an injected `MetadataPort` can qualify authority, attest exact-child
observations and revalidate held metadata plus parent/name bindings and durable
revisions. Its interface has no content read, directory enumeration, audio,
process, network fallback or mutation method. A native implementation must
separately prove syscall rights, no reparse traversal and no cloud hydration.
Fake tests do not prove these properties.

Syntax accepts ordinary drive-letter paths with backslash separators. It
conservatively rejects alternate separators, short-name spelling containing
`~`, controls/bidi format characters, reserved names, ambiguous trailing
spelling, streams, dot/parent components and wildcard/device/network/URL syntax.
Relative references and placeholders remain unchecked. Policy neither normalizes
Unicode nor folds component case. The qualified port compares each held root
component under actual filesystem semantics. Siblings and other drives cannot
inherit authority from string prefixes. Syntax approval is never an access grant.

Directory guards retain ancestry. Only authoritative exact-name absence from a
trusted parent yields `not_found`; denied, incomplete, changed or unsupported
observations cannot become missing. Root/source qualification and the final
freshness fence are mandatory even for empty/all-no-reference reports. Source
content is not rehashed: identity/size/time-preserving edits remain undetectable.

## Work and publication limits

One host-owned `WorkerGate` admits one request with no queue. Move the lease into
the sole worker; retain it until underlying work retires. A timer may cancel,
invalidate or poll the two-second monotonic deadline via `RequestControl`.
This fences publication without releasing admission. A blocked call may outlive
timeout; later requests remain busy. Shutdown closes admission and invalidates
work. A lease cannot execute a second request and reset its budgets.

Every adapter syscall must use `Operations::perform`, one syscall per closure,
including qualification, queries, opens and revalidation. Before/after checks
honor cancellation, revocation and deadline. Each live native handle, including
temporary handles, requires a non-cloneable `HandleLease` token; release the
handle before its token. The adapter must not retain guards outside the worker.
This is a trusted injection contract requiring separate implementation review.

The 2,048-operation total reserves 256 for final authority revalidation. Probe
exhaustion preserves every slot, marking remaining references unchecked while
absent reference fields keep `no_saved_reference`. The final fence still honors
the total cap and deadline; an incomplete fence makes the whole report
unavailable. Cancellation/deadline/revocation discard provisional results.
Paths have at most 64 components; the request holds at most 132 handles. Exact
literal duplicates reuse results within this request only, without case folding.

Reports contain fixed outcomes/reasons, ordered one-based positions, bounded UTC
milliseconds and opaque context, with a 64 KiB encoded cap. They exclude paths,
raw errors, file identities and fingerprints, and are not persisted. Slice 3
must supply strict IPC decoding and host/renderer context/generation fencing at
delivery; library publication alone does not guarantee transport freshness.

## Windows metadata adapter

`native::WindowsPort` requires an injected `CurrentAuthorization`; there is no
production permit-all implementation. The future host must check durable root,
location, snapshot, source and session state, release its database guard before
I/O and signal `RequestControl` when authority is revoked. The adapter compares
the captured context/fence again before publication. Database integration is
slice 3 work, not something the injected tests prove.

The adapter qualifies a local harddisk DOS mapping and held NTFS volume/root/
source identity before candidate work. Handles request attributes and synchronize
rights only. Exact child queries/opens use held parents, kernel-owned directory
case semantics, no-follow/no-recall options and existing-only NT disposition.
Unsafe calls live exclusively in the private `native/ffi.rs` boundary; no raw
handle or arbitrary error/path escapes it. Ancestor handles pin names without
blocking descendant writes. Source files allow ordinary writes and atomic
replacement; final checks compare both held metadata and the current parent/name
binding, because an old handle can survive replacement.

Root-prefix spelling must match the trusted captured spelling literally.
Potentially equivalent case variants remain `not_checked/unsupported_case_mode`;
the adapter does not grant authority from an OS string comparison. Candidate
suffixes follow the qualified directory's case mode, including sensitive
directories. Unsupported/remote/redirected mappings and reparse/offline/recall
states fail closed. The no-recall option and injected flags are reviewed evidence;
these tests do not qualify every real cloud provider or a blocked kernel call.

## Validation

`cargo test -p fruitboard-sample-presence --locked` retains 24 recording-port,
fake-clock and blocked-worker tests. On Windows, 20 additional tests exercise
actual NTFS attributes-only handles, presence/absence, access-denied metadata,
directory pins, ordinary source saves, surviving old handles, real junction
races, sensitive case mode, cancellation/revocation and maximum-depth budgets.
Offline/recall attributes use explicit injection. Fixtures are private synthetic
bytes, not new FL Studio compatibility evidence. Independent source/sample bytes
remain unchanged except in tests that deliberately simulate a save/replacement.

Warning-denied all-target Clippy/tests run in portable CI, explicit Windows CI
and the pinned Windows workspace gate. See the
[policy review](../../docs/review/sample-presence-policy-280/README.md),
[Windows review](../../docs/review/sample-presence-windows-282/README.md) and
[accepted sequence](../../docs/review/sample-presence-278/implementation-plan.md).
Installed UI, strict IPC, host database fences, wider path resolution and
performance qualification remain separate work.

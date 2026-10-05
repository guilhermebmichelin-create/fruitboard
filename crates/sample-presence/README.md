# Bounded sample presence policy

ADR-007 slice 1, issue #280. This library implements portable policy and request
lifecycle rules. It has **no filesystem implementation or desktop command**;
the app still displays saved paths without checking them.

## Authority contract

The future trusted host captures enabled root/location revisions, root/source
identities, source size/time, snapshot and session under durable authorization.
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

## Validation

`cargo test -p fruitboard-sample-presence --locked` uses recording ports, fake
clocks and a channel-controlled blocked worker. It proves policy/lifecycle,
not Windows traversal, FL Studio mapping, installed UI or performance.
Warning-denied all-target Clippy/tests run in portable CI and the pinned Windows
workspace gate. See the [review](../../docs/review/sample-presence-policy-280/README.md)
and [accepted sequence](../../docs/review/sample-presence-278/implementation-plan.md).

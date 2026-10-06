# ADR-007: Bounded checks for saved sample paths

- Status: Accepted design; policy PR281 and Windows PR283 merged; app #284 for review
- Date: 2026-10-05
- Decision owner: product owner
- Source baseline: merged PR268, `91261525383ce79adb090550329167ef347f7253`

## Context

Library details already displays validated per-channel sample reference text.
That text can name an absolute path, relative path or FL Studio placeholder.
It currently grants no filesystem access and proves neither presence nor
absence. Treating an access error or an unresolved reference as a missing
sample would give the owner incorrect information.

G5 requires a scoped native authority review before implementation. This record
defines the first slice of Phase 3 reference resolution; it does not itself implement
it, accept G5 as delivered or authorize a later phase. The
[review packet](../review/sample-presence-278/README.md) contains the threat
review, acceptance cases and implementation sequence.

## Decision

Add an explicit desktop action to check saved paths within the selected
project's enabled, qualified local-NTFS scan root. Use a dedicated Rust
metadata-only boundary. First-version eligibility is restricted to ordinary,
fully qualified drive-letter paths that are literally stored in the current
validated snapshot and lie inside that root. No environment/FL placeholder
expansion, relative-path base guess, search or fallback folder is allowed.

The folder was already selected for this project; the explicit action grants
only the scoped metadata check described by its UI. It does not grant access to
other scan roots or an unrestricted local file lookup. Checking other folders
requires a separately designed explicit grant. Default builds and PWA do not
gain a filesystem capability; implementation remains in the existing opt-in
`analysis-jobs` desktop composition until production activation is reviewed.

Presence means an ordinary file was observed at the exact permitted saved
path when checked. It does not mean playable audio, matching audio content,
successful FL Studio search resolution, or an intact dependency set. A result
is an observation, not a promise that the filesystem will remain unchanged.

### Result vocabulary

| Outcome | Meaning | Local display |
| --- | --- | --- |
| `present` | Qualified regular file at the permitted saved path | Present when checked |
| `not_found` | Authoritative absence at a trusted parent/name within the permitted root | Not found at saved path when checked |
| `not_checked` | Policy, permission, unsupported source/path, resource or transport restriction | Not checked, with fixed explanatory copy |
| `no_saved_reference` | This channel has no saved sample reference | No saved reference |

`not_checked` reasons are allowlisted constants: outside root, unsupported path
syntax, relative reference, unresolved placeholder, unqualified filesystem,
unsupported case mode, reparse/offline target, access denied, not a regular file,
limit reached, deadline or cancelled. OS errors do not become display strings.
Only a qualified exact-name absence may produce `not_found`. A source/root/
snapshot change invalidates the entire request instead of producing item-level
missing results. Duplicate references remain in their original channel slots;
counts describe channel references, not unique files.

### Request and freshness authority

The renderer submits a strict versioned command containing opaque root,
location, snapshot and request IDs plus the existing displayed size/modified
fingerprint. It supplies no paths, sample strings, folder lists or retry budget.
The native host obtains the current source and stored references itself.

Under the database guard, authorize the enabled root, present location,
qualified source identity, current immutable metadata snapshot and displayed
fingerprint. Copy a bounded validated input and capture root/source revisions,
snapshot identity and the current host session. Release the database guard
before filesystem operations. Unknown fields, stale/mismatched identities,
invalid sample projection or feature-disabled state fail closed without probes.

Before checking references, qualify the actual root/source objects and match
their volume/file identities, size and high-resolution modification time to the
captured source. Hold root/source authority through the request; source checks
must not hash or parse the FLP again. Candidate metadata operations use only
handles rooted in this qualified authority. Immediately before returning,
revalidate the root/source objects and the durable revisions/current snapshot/
session under the database guard. Any mismatch discards provisional results.
This is bounded freshness checking, not a globally atomic filesystem snapshot.
Without rereading content, edits that preserve the recorded identity, size and
timestamp cannot be detected; do not claim cryptographic content freshness.

Storage retains the configured folder grant/revision, not a directory ID. Slice
3 captures root identity on the actual held native root during qualification
before probing; final root/name checks compare to that ID. Earlier independently
captured root IDs, when supplied by a trusted native caller, must also match.
Both paths require the same qualified source identity, pinned ancestry, operation
bounds and final durable fences; the renderer supplies neither identity nor path.

Native cancellation and request generation fence late replies. The renderer
also requires the same selected row, displayed fingerprint, snapshot and current
request generation before showing a response. Navigation/root removal, a newer
scan/analysis result, runtime shutdown or cancellation clears the ephemeral
report. Report state never survives restart or becomes immutable FLP metadata.

### Filesystem authority

The port exposes metadata operations and owned root/parent/candidate guards;
it has no read-bytes, playback, parser, process launch, write or delete method.
Do not use the FLP content-reading helper to open a sample. Existing qualified
root/source and enumeration primitives are review references, not proof that
the new exact-child probe is already implemented.

Validate path syntax without filesystem access. Reject UNC/network paths,
device/extended namespaces, URLs, drive-relative paths, alternate streams,
dot/parent components, wildcards, reserved/ambiguous component spellings and
controls. Preserve stored spelling; do not apply Unicode normalization or
assume case equality. The native boundary owns component comparisons and
per-directory case semantics. Unqualified case mode remains `not_checked`.

Prove containment by components and actual held-object ancestry, not string
prefix or `canonicalize` followed by a path reopen. A name under `Projects2`
must not inherit authority for `Projects`. Open/query only the exact next child
relative to a trusted parent, inspect its attributes and identity, reject
reparses and offline/recall indications before continuing, and retain ancestor
authority against replacement. A narrow port review must establish that
excluded cloud/offline candidates cause no hydration or network traversal.
Where that cannot be established, stop with `not_checked` before the operation.

Never enumerate a tree or query an FL Studio search folder to find a substitute.
Candidate/source file handles permit ordinary concurrent file use and are
released after their required observations; do not block normal FLP saves or
lock every referenced file across the batch. Root/source guards must validate
both held-object metadata and parent/name bindings: a replaced source name must
not pass simply because the old held handle still has its old identity. Ancestor
pins may prevent path replacement while allowing writes to descendant files.
These sharing/binding decisions require replacement/reparse/save-race tests.
A missing intermediate component is `not_found` only when its absence was
reported authoritatively from the preceding trusted parent. Access denied,
incomplete attributes, changed ancestors, directory targets or failed queries
cannot be converted into absence.

### Bounds and cancellation

These are proposed operational stops, not accepted performance budgets.

| Boundary | First-version maximum |
| --- | --- |
| Channel entries | 256, preserving the existing validated projection bound |
| Stored reference | Existing 4,095 UTF-16 units / 12,285 UTF-8 bytes and payload bounds |
| Request body | 4 KiB; identifiers/fingerprint only |
| Full root/source/candidate path depth | 64 components; deeper paths not checked |
| Metadata query/open/revalidation operations | 2,048 per request, including qualification |
| Simultaneously held native handles | 132 total: root/source authority plus one candidate chain |
| Response body | 64 KiB; fixed status/reason entries, no paths |
| Worker | One active request globally in the native host; no queued batch |
| Cooperative deadline | 2 seconds, using a monotonic clock |

Check budgets/cancellation before and after each native operation. Exhaustion
marks remaining references `not_checked`; do not silently truncate the channel
array. Reuse a probe result only for an identical literal reference within this
same authorized request, without cross-request or guessed case-fold caching.

A blocked kernel operation may outlive a cooperative deadline. The host timer
can finish the UI request with fixed timeout copy and reject late publication,
while retaining the single-worker gate until that operation retires. Do not
spawn replacement workers or accumulate orphan tasks/handles. A subsequent
request gets a bounded busy response. Cancellation is not a claim that every
Windows syscall is forcibly interruptible; measure actual behavior separately.

### Storage, privacy and UX

The availability report is session-local and ephemeral. It includes version,
opaque request/root/location/snapshot correlation, a bounded UTC check timestamp
and one ordered status/reason per channel. No resolved paths, volume/file IDs,
raw OS errors or extra saved strings are returned. Existing saved-reference text
continues to come from the authorized details view. Strict native/client
decoders enforce count/order, fixed vocabulary, bounded timestamp and matching
request/source context.

No schema migration, snapshot rewrite, automatic parser retry or persistent
sample index is needed for this slice. No path/report telemetry, event payload,
sync operation or raw debug representation is introduced. Errors/logs use
allowlisted codes; renderer-supplied identifiers are not interpolated into logs.

Opening or refreshing project details remains a saved-results read. Only
"Check saved paths" starts a probe; its explanation names the selected folder
and narrow scope. Offer explicit check-again and cancel controls, keyboard
operation, progressive per-channel disclosure, announced bounded progress and
fixed loading/busy/stale/error states. Never add clickable/open/play actions.
Old snapshots without references ask for analysis; an empty reference list or
an empty Sampler says there is nothing to check, not that a sample is missing.
Reports show when the check happened and distinguish unchecked references.

## Alternatives

- `exists(saved_string)`: too broad an authority grant, hides error distinctions
  and loses replacement/root/source fences.
- Automatically check on parse, details open or refresh: silently adds file
  access and mutable observations to an existing read-only path.
- Search all enabled roots or FL Studio library folders: requires new explicit
  authority, mapping/precedence rules, privacy review and separate bounds.
- Resolve relative paths against the FLP parent now: that base is not qualified
  by the current corpus. Keep them unchecked until independent GUI evidence
  establishes the supported mapping.
- Read/hash/decode audio or let the parser resolve dependencies: unnecessary
  content access and a larger process/plugin/privacy boundary.

## Consequences and review gate

This narrow slice can honestly report some exact saved paths while leaving
many real references unchecked. It provides an authority boundary for later
folder grants and relative/placeholder mapping without claiming FL Studio's
full dependency resolver. Source snapshots remain immutable and old apps/data
need no migration for this design.

The design was accepted by PR279 on 2026-10-05, merge
`53e248ae906bfd0a53cbdf6cbe62442c25238669`. The authority review is documented;
[policy slice #280](../review/sample-presence-policy-280/README.md) implements
portable rules merged in PR281. The
[Windows slice #282](../review/sample-presence-windows-282/README.md) records native
implementation, adversarial tests and limitations for review. Passing tests alone
do not replace an authority review. Review/merge of the focused design PR approves
this implementation scope. Separate implementation PRs still need their own
native review, required updated-head checks, Windows adversarial tests and
rendered/installed evidence. Design approval does not activate production,
approve resource budgets, accept Phase 3 or close G5.

PR283 merged the Windows adapter as `b8c065c3f10b6b64d96b8224fadcdbb4e19af3d6`.
[App slice #284](../review/sample-presence-ui-284/README.md) maps the authorized
command, durable revocation/delivery fences and explicit Library controls.

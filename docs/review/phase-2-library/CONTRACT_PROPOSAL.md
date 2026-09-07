# Phase 2 Library/scan client contract proposal

Status: **UI-only proposal for the #38 execution and #40 publication owners.**
This document is not an accepted shared contract, native implementation, or
claim of production scanning. The client branch intentionally leaves
`apps/client/src/platform/contracts.ts`, native commands, manifests, locks, and
CI unchanged.

> Superseded for this round: the earlier combined-root query and numeric
> `byteSize` sketch are overridden by the authoritative storage integration
> contract §5. Library pages read **one root per query**
> (`LibraryPageRequest{rootId, limit 1..200, cursor}`,
> `LibraryPage{rootId, snapshotId, records, nextCursor}`), and
> `byteSize`/`modifiedAt` cross the boundary as exact decimal strings. The
> client seam in
> [`apps/client/src/library/contracts.ts`](../../../apps/client/src/library/contracts.ts)
> already implements that shape.

The client uses the local `LibraryScanAdapter` seam in
`apps/client/src/library/contracts.ts` so the Library can be reviewed against
typed data without pretending that the current Tauri adapter can scan. The
stateful fake implementation is test/rendered-evidence harness code only.

## Proposed published file/location DTO

The read model publishes one row per tracked file location. It must not collapse
hardlink aliases or infer a logical project.

```ts
type DecimalString = string;

interface PublishedFileLocation {
  locationId: string;
  rootId: string;
  rootDisplayName: string;
  rootCanonicalPath: string; // management display only; never diagnostics
  fileName: string;
  relativePath: string; // display spelling only; not a key or cursor input
  byteSize: DecimalString; // canonical unsigned decimal string
  modifiedAt: string; // RFC 3339 / UTC
  presence: "present" | "missing";
}
```

Required invariants:

- `locationId` is stable for the tracked location, and `rootId` is never
  reused. A detached historical location remains addressable without being
  silently marked missing.
- `relativePath` is a display-only, root-relative spelling supplied by the
  boundary. The renderer does not lowercase, canonicalize, enumerate, or
  check the path, and never uses it as a key or cursor input.
- `byteSize` and `modifiedAt` are the last committed metadata. A missing row
  retains those values from its last committed observation.
- `presence` changes only through an authoritative successful publication.
  Failed, incomplete, cancelled, unavailable, denied, and limited runs do not
  create new missing rows or publish partial positive updates.
- Root display names are not unique. The UI may include the canonical path in
  Library/root-management labels when needed to distinguish equal names; that
  path must not enter diagnostics or generic error text.

## Per-root bounded Library query

The authoritative storage integration contract §5 supersedes the earlier
combined-root sketch. Each request reads exactly one tracked root, and the UI
does not merge or sort pages itself. Native ordering is by
`(locator_key BINARY, location_id)` within that root; `relativePath` remains
display-only.

```ts
interface LibraryPageRequest {
  rootId: string; // exactly one tracked root
  limit: number; // caller asks for <= 200
  cursor: string | null; // opaque continuation token
}

interface LibraryPage {
  rootId: string;
  snapshotId: string; // opaque committed snapshot identity
  records: PublishedFileLocation[];
  nextCursor: string | null;
}
```

The native owner clamps the requested limit to `1..200`, rejects malformed or
expired cursors with a safe error, and reads only committed generations for the
requested root. A non-null cursor is valid only with that root and the
snapshot identity that produced it. The cursor must encode enough
committed-generation/order information to prevent duplicate or skipped rows
when a later scan publishes.

If the adapter reports `invalid_cursor` or `stale_cursor`, or returns a page
for the wrong root or whose `snapshotId` does not match the requested non-null
cursor snapshot, the renderer discards the cursor history and safely requests
page one with the same `rootId` and `cursor: null`. It does not guess a new
cursor or show a mixed-root/mixed-snapshot page. A new page-one read always
asks for the latest committed snapshot for that root.

## Scan status, queue identity, and progress

```ts
interface ScanStatus {
  root: ScanRoot;
  state:
    | "idle"
    | "queued"
    | "running"
    | "completed"
    | "cancelled"
    | "failed"
    | "interrupted";
  jobId: string | null;
  runId: string | null;
  counters: {
    filesObserved: number; // validated safe integer
    directoriesVisited: number; // validated safe integer
    totalFiles: number | null; // validated safe integer when present
  };
  lastSuccessfulScanAt: string | null; // RFC 3339 / UTC
  lastOutcomeAt: string | null; // RFC 3339 / UTC
  errorCode: ScanErrorCode | null;
}
```

`jobId` identifies queued work and remains the cancellation/retry identity.
`runId` is allocated only when a worker leases an attempt, so it is null while
the job is queued. Running work has both identities; terminal work keeps its
job identity and may keep its completed run identity.

The native boundary must make timestamp and integer conversions explicit:
native integer milliseconds such as `*_at_ms` become RFC 3339 UTC strings;
JSON integer fields are validated as safe, non-negative integers rather than
accepted as numeric strings or floats. The renderer formats timestamps for
display and never treats an integer timestamp as a JavaScript `Date` input.

`root.availability` is an observation, not freshness proof. `available` and
`unavailable` are not live reachability checks, and `unknown` is not a reason to
disable recovery. `lastSuccessfulScanAt` describes the committed dataset. The
client renders those facts separately and marks results as previous/stale when
a run is active or terminally unsuccessful. A null `totalFiles` is honest
unknown work: the UI shows counters and does not render a percentage.

The owner should expose a bounded status subscription (or equivalent Tauri
event stream) so queued/running progress can update without polling storms. It
must be safe to unsubscribe on route changes and carry root/job/run identity
with every update.

## Proposed controls and outcomes

```ts
scanNow(rootId): Promise<{
  rootId: string;
  jobId: string;
  runId: string | null; // null while queued
  outcome: "queued" | "already_queued" | "already_running";
}>;

cancelScan(jobId): Promise<{
  rootId: string;
  jobId: string;
  runId: string | null;
  outcome:
    | "cancelled"
    | "cancellation_requested"
    | "already_cancelled"
    | "already_completed"
    | "already_failed"
    | "not_found";
}>;

retryScan(jobId): Promise<ScanStartResult>;
```

Queued cancellation is immediate; running cancellation persists a request for
the leased worker. A complete-before-cancel race returns the completed outcome
and does not claim rollback. For a root with no prior terminal job, the UI uses
`scanNow(rootId)` for its recovery button; it does not invent a job ID.

Safe error codes should be a closed, versioned set:

```text
access_denied | unavailable | unsupported | resource_limit |
conflict | not_found | cancelled | internal | invalid_cursor | stale_cursor
```

`invalid_cursor` and `stale_cursor` are Library read-recovery conditions; they
are handled by restarting pagination and are not shown as scan failures. The
client maps other codes to fixed safe copy. It never displays an arbitrary
native error message, SQL detail, correlation token, lease token, or full path
from an error.

## Async ordering rules exercised by the client

- Page and status responses carry a monotonically ordered client request
  sequence. Only the latest sequence for the current adapter, cursor, and
  snapshot may update state.
- Subscription refreshes use the same sequence gates, so an older refresh that
  resolves after a newer refresh cannot roll the UI back.
- Cursor navigation invalidates the prior page sequence immediately. Adapter
  replacement invalidates both channels before the replacement effects run.
- All response and focus continuations check mounted state. Unsubscription is
  still required; it is not treated as cancellation of an already-started
  Promise.
- Failed page refreshes retain the last page; failed, incomplete, cancelled,
  or unavailable scans retain the last committed dataset and never infer new
  missing locations.

## Integration checklist for the designated owners

- [ ] #38 owner accepts the job/run/status/outcome shape, including a null
      queued `runId`, queued cancellation, queued follow-up, and cancellation
      race semantics.
- [ ] #40 owner accepts the per-root DTO, committed-snapshot read rule,
      opaque snapshot-bound cursor with `invalid_cursor`/`stale_cursor`
      semantics, `snapshotId`, 200-record bound, and stable per-root
      ordering. (The earlier combined-root variant of this item is
      superseded by §5 of the authoritative storage integration contract.)
- [ ] #36/#40 owners confirm that incomplete coverage never becomes new
      `presence: "missing"` rows and that prior committed rows survive every
      failed/partial outcome.
- [ ] Native/platform owner adds versioned IPC parsers and commands only after
      the proposal is accepted; the current client `PlatformPort` remains the
      compatibility boundary until then.
- [ ] Native parsers explicitly convert millisecond timestamps and validate
      safe integer counters/byte sizes before constructing UI DTOs.
- [ ] Native integration supplies committed results and statuses from
      SQLite/publication state, not renderer-owned arrays or filesystem guesses.
- [ ] Integration tests cover restart/interrupted leases, stale workers,
      atomic publication, root disable/removal, hardlink aliases, queued and
      running cancellation, stale cursors, snapshot changes, and safe errors
      before any production Scan now control is enabled.
- [ ] The Windows/DriveFS and cross-volume qualification limits remain
      explicit; fake-adapter screenshots are not native or filesystem
      evidence.

## Review evidence in this branch

`docs/review/phase-2-library/` contains fake-adapter desktop/narrow captures and
keyboard assertions for the client state machine. The rendered badge says
“Review harness · fake adapter”. These artifacts prove client state, focus,
keyboard operation, responsive layout, and overflow assertions only. They do
not prove native permissions, enumeration, SQLite, restart persistence, or
production scan execution.

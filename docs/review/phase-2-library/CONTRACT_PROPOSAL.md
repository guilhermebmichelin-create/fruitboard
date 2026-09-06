# Phase 2 Library/scan client contract proposal

Status: **Client proposal for #38 execution and #40 publication owners.** This
document is not an accepted shared contract, native implementation, or claim of
production scanning. The client branch intentionally leaves
`apps/client/src/platform/contracts.ts`, native commands, manifests, locks, and
CI unchanged.

The client uses the local `LibraryScanAdapter` seam in
`apps/client/src/library/contracts.ts` so the Library can be reviewed against
typed data without pretending that the current Tauri adapter can scan. The
stateful fake implementation is test/dev harness code only.

## Proposed published file/location DTO

The read model should publish one row per tracked file location. It must not
collapse hardlink aliases or infer a logical project.

```ts
interface PublishedFileLocation {
  locationId: string;
  rootId: string;
  rootDisplayName: string;
  rootCanonicalPath: string; // management display only; never diagnostics
  fileName: string;
  relativePath: string; // normalized root-relative locator
  byteSize: number;
  modifiedAt: string; // RFC 3339 / UTC
  presence: "present" | "missing";
}
```

Required invariants:

- `locationId` is stable for the tracked location, and `rootId` is never
  reused. A detached historical location must remain addressable without
  being silently marked missing.
- `relativePath` is already normalized and root-scoped by the boundary. The
  renderer must not lowercase, canonicalize, enumerate, or check the path.
- `byteSize` and `modifiedAt` are the last committed metadata. A missing row
  retains those values from its last committed observation.
- `presence` changes only through an authoritative successful publication.
  Failed, incomplete, cancelled, unavailable, denied, and limited runs do not
  create new missing rows or publish partial positive updates.
- Root display names are not unique. The UI may include the canonical path in
  Library/root-management labels when needed to distinguish equal names; that
  path must not enter diagnostics or generic error text.

## Proposed bounded Library query

```ts
interface LibraryPageRequest {
  cursor: string | null; // opaque continuation token
  limit: number; // caller asks for <= 200
}

interface LibraryPage {
  records: PublishedFileLocation[];
  nextCursor: string | null;
}
```

The native owner should clamp the requested limit to `1..200`, reject malformed
or expired cursors with a safe error, and read only committed generations. The
ordering must be deterministic across calls and pages:

1. `rootId` ascending;
2. normalized `relativePath` ascending;
3. `locationId` ascending as the final tie-breaker.

The cursor must encode enough committed-generation/order information to prevent
duplicate or skipped rows when a later scan publishes. A new query after
publication may start from `cursor: null`; the renderer does not assume that a
page is a snapshot of a filesystem traversal.

## Proposed scan status and progress

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
  runId: string | null;
  counters: {
    filesObserved: number;
    directoriesVisited: number;
    totalFiles: number | null;
  };
  lastSuccessfulScanAt: string | null;
  lastOutcomeAt: string | null;
  errorCode: ScanErrorCode | null;
}
```

`root.availability` remains a current observation (`available`, `unavailable`,
or `unknown`), not a freshness result. `lastSuccessfulScanAt` describes the
committed dataset. The client renders both facts separately and labels a page
as previous/stale when a run is active or terminally unsuccessful. A null
`totalFiles` is honest unknown work: the UI shows counters and explicitly does
not render a percentage.

The owner should expose a bounded status subscription (or equivalent Tauri
event stream) so queued/running progress can update without polling storms. It
must be safe to unsubscribe on route changes and must carry root/run identity
with every update.

## Proposed controls and outcomes

```ts
scanNow(rootId): Promise<{
  rootId: string;
  runId: string;
  outcome: "queued" | "already_queued" | "already_running";
}>;

cancelScan(runId): Promise<{
  rootId: string;
  runId: string;
  outcome:
    | "cancellation_requested" | "already_cancelled"
    | "already_completed" | "already_failed" | "not_found";
}>;

retryScan(rootId): Promise<ScanStartResult>;
```

Safe error codes should be a closed, versioned set:

```text
access_denied | unavailable | unsupported | resource_limit |
conflict | not_found | cancelled | internal
```

The client maps those codes to fixed safe copy. It never displays an arbitrary
native error message, SQL detail, correlation token, lease token, or full path
from an error. Cancellation must be durably recorded before the command reports
success; a complete-before-cancel race returns the completed outcome and does
not claim rollback.

## Integration checklist for the designated owners

- [ ] #38 owner accepts the run/status/outcome shape, including queued follow-up
      and cancellation race semantics.
- [ ] #40 owner accepts the DTO, committed-generation read rule, opaque cursor,
      200-record bound, and stable ordering.
- [ ] #36/#40 owners confirm that incomplete coverage never becomes new
      `presence: "missing"` rows.
- [ ] Native/platform owner adds versioned IPC parsers and commands only after
      the proposal is accepted; the current client `PlatformPort` remains the
      compatibility boundary until then.
- [ ] Native integration supplies committed results and statuses from SQLite/
      publication state, not from renderer-owned arrays or filesystem guesses.
- [ ] Integration tests cover restart/interrupted leases, stale workers,
      atomic publication, root disable/removal, hardlink aliases, and safe
      errors before any production Scan now control is enabled.
- [ ] The Windows/DriveFS and cross-volume qualification limits remain explicit;
      fake-adapter screenshots are not native or filesystem evidence.

## Review evidence in this branch

`docs/review/phase-2-library/` contains fake-adapter desktop/narrow captures and
keyboard assertions for the client state machine. The badge in the rendered
view says “Review harness · fake adapter”. These artifacts prove client state,
focus, keyboard operation, responsive layout, and overflow assertions only.
They do not prove native permissions, enumeration, SQLite, restart persistence,
or production scan execution.

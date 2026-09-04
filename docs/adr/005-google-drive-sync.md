# ADR-005: Drive app-data operation-log synchronization

- Status: Proposed
- Date: 2026-09-04

## Context

Metadata should optionally synchronize between desktop and iPhone without a
Fruitboard account/backend. Google Drive is user-selected, but concurrent
offline edits cannot safely be handled by copying SQLite or applying global
last-write-wins. Browser authorization does not provide reliable unattended
background access.

## Decision

Use Google Drive `appDataFolder` with the narrow `drive.appdata` scope. Each
replica keeps local state plus an atomic outbox and uploads immutable,
per-device, versioned operation batches. Replicas apply unseen operations
idempotently and use type-specific conflict rules. Add immutable snapshots and
conservative operation cleanup only when scale requires it.

Desktop uses installed-app authorization code + PKCE and OS secure token
storage. The PWA uses Google Identity Services and synchronizes in the
foreground; offline operations wait durably in IndexedDB. Do not promise silent
iOS background sync.

## Alternatives

- Sync the SQLite file: unsafe with concurrent writers/WAL and leaks local data.
- One mutable JSON backup with last-write-wins: loses concurrent offline edits.
- Proprietary backend/account: contradicts the initial product/privacy model.
- General Drive folder/full Drive scope: broader permissions and user-visible
  clutter without need.
- CRDT for every field immediately: excessive complexity; apply specialized
  merge semantics where needed.

## Consequences

- Sync schema/protocol, compaction, tombstones, conflicts, and recovery require
  substantial testing before Phase 11 ships.
- Main notes and ambiguous delete/edit races preserve conflict copies.
- Simple scalar fields may use deterministic per-field ordering with history.
- PWA users may occasionally need to reauthorize after token expiry.
- `appDataFolder` is personal/app-specific and is not a future collaboration
  transport; collaboration requires a new backend/ADR.
- Absolute paths, FLPs, logs, and original exports stay local.

## References

- [Sync design](../../SYNC.md)
- [Drive application data](https://developers.google.com/workspace/drive/api/guides/appdata)
- [Drive scopes](https://developers.google.com/workspace/drive/api/guides/api-specific-auth)
- [Web authorization](https://developers.google.com/identity/oauth2/web/guides/overview)

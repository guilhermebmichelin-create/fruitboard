# ADR-004: Separate Project, ProjectFile, and FileLocation

- Status: Accepted
- Date: 2026-09-04
- Approved: 2026-09-04 by the product owner

## Context

A song may have many FLPs, one FLP may be renamed/moved or appear on multiple
devices, and similar names do not prove shared identity. Paths are mutable and
device-specific. Automatic grouping must remain reversible.

## Decision

Model:

- `Project`: the user-managed creative work;
- `ProjectFile`: an FLP/version or exact file identity associated with a project;
- `FileLocation`: a device-local path/presence record for a project file.

Assign UUIDv7 IDs. Use filesystem identity, hashes, names, proximity,
chronology, and parsed similarity as evidence. Stable filesystem ID can continue
a locator across a move, but uncertain song/version grouping is stored as a
suggestion with score/evidence/ruleset version. Only explicit confirmation or
manual merge changes grouping; reject and split remain possible.

## Alternatives

- Path as project ID: breaks on rename, move, Drive mount changes, and another
  computer.
- One project per FLP forever: cannot represent version history.
- Automatic filename merge: destructive false positives and poor reversibility.
- Full content hash as project ID: different song versions differ by design and
  identical copies may exist at several locations.

## Consequences

- Discovery, grouping, and location availability can evolve independently.
- The PWA can display projects/files without receiving absolute desktop paths.
- Schema and UI are more complex than a file browser.
- Candidate generation/scoring and rejection memory require dedicated tests.
- Exact duplicates and song versions remain distinct relationship concepts.

## References

- [Identity strategy](../../ARCHITECTURE.md#project-and-version-identity)
- [Data model](../../DATA_MODEL.md)

# ADR-003: Rust-owned local SQLite database

- Status: Accepted
- Date: 2026-09-04
- Approved: 2026-09-04 by the product owner

## Context

The desktop must work offline, protect user metadata, query thousands of
projects and dependencies, support migrations/search, and later generate a sync
operation log. Browser code should not gain arbitrary database authority. A
database file inside Google Drive would not provide safe replication.

## Decision

Use SQLite in the platform application-data directory, accessed only by Rust
repositories (initially through SQLx or an equivalently maintained Rust binding).
Use committed forward migrations, foreign keys, parameterized queries, backups,
and FTS5. Evaluate WAL after confirming the embedded SQLite includes the current
WAL-reset fix and after concurrency/durability tests; prefer one writer and
short transactions.

The UI invokes application use cases, not SQL. Synchronization serializes
domain operations, never the database file.

## Alternatives

- Tauri SQL exposed to JavaScript: quick but leaks storage structure/authority
  across the trust boundary and makes invariants harder to centralize.
- IndexedDB for desktop: weaker native tooling/queries and does not serve Rust
  scanner transactions naturally.
- Embedded document store: less suitable for relational filters, joins,
  migrations, and FTS without a demonstrated benefit.
- Cloud database: violates offline/no-account requirements.

## Consequences

- Rust repositories and migrations require integration tests.
- SQL schema can be optimized for local queries while the sync schema evolves
  independently.
- The PWA needs an IndexedDB adapter rather than opening desktop SQLite.
- Database backups and version compatibility are first-class release concerns.
- SQLite library versions must be audited, not assumed from the OS.

## Phase 1 implementation note

Issue #11 adds a tested machine-readable gate that rejects SQLite versions
below 3.51.3 unless an exact official fixed backport is explicitly allowlisted.
No SQLite binding, database, migration, or WAL mode is introduced; Issue #15
must feed the selected binding's runtime version through this gate.

## References

- [Data model](../../DATA_MODEL.md)
- [SQLite WAL](https://www.sqlite.org/wal.html)
- [SQLite FTS5](https://www.sqlite.org/fts5.html)

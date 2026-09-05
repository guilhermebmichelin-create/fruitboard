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

Issue #15 selects `rusqlite` 0.40.2 with its bundled and backup features, locking
`libsqlite3-sys` 0.38.2 and embedded SQLite 3.53.2. Its synchronous ownership and
backup API fit one small native writer without an async pool. The portable
`crates/storage-sqlite` crate owns schema, connections, and typed repositories;
the Tauri host resolves `app_local_data_dir()` and owns it behind a Mutex.

The Issue #11 policy remains the single version floor. Native opens check the
embedded runtime against the committed policy, and `pnpm verify:sqlite:embedded`
runs a linked Rust probe through the existing JavaScript gate during
`pnpm check`. Neither the OS SQLite executable nor a manifest version is used
as runtime evidence. WAL remains disabled and existing WAL databases fail
closed; version eligibility alone does not qualify concurrency or durability.

Version 1 contains only a local singleton startup-view preference and migration
ledger. SQL migrations live with the crate and all pending changes commit
atomically. An application ID and exact ordered migration history prevent
silently opening unrelated, newer, or changed schemas.

Before upgrading an existing schema, a checked SQLite backup must succeed.
Recovery validates the backup and requires the destination database and its
`-journal`, `-wal`, and `-shm` companions to be absent before staging, even when
a companion is empty. Existing files are preserved; orphan journals must not
be replayed against restored data. No automatic restore, down migration, or
backup pruning is provided. Tests include a terminated process with dirty
uncommitted pages, rollback after failure, and restoration after corruption.
These tests do not claim hardware power-loss qualification.

The complete implemented schema and recovery contract are documented in
[DATA_MODEL.md](../../DATA_MODEL.md). React/IPC settings use cases belong to
Issue #16; portable/Windows CI automation belongs to #17; installer preservation
checks belong to #18. FTS and product/scanner/parser/sync schemas remain deferred.

## References

- [Data model](../../DATA_MODEL.md)
- [SQLite WAL](https://www.sqlite.org/wal.html)
- [SQLite FTS5](https://www.sqlite.org/fts5.html)
- [rusqlite backup API](https://docs.rs/rusqlite/0.40.2/rusqlite/backup/index.html)
- [SQLite atomic commit](https://www.sqlite.org/atomiccommit.html)
- [SQLite hot-journal recovery](https://www.sqlite.org/lockingv3.html#hot_journals)

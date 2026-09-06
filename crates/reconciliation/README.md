# Filesystem-only reconciliation core (#36)

This isolated Rust library proposes per-path changes from normalized metadata
observations for one root. It has no dependencies, filesystem I/O, parser,
SQLite connection, watcher, renderer command or production scan entry point.
It implements the deterministic decision slice of the
[scanner contract proposal](https://github.com/guilhermebmichelin-create/fruitboard/pull/57),
not the integrated scanner.

An incomplete outcome rejects the entire proposal. A successful outcome can
mark previously present paths missing, while retaining missing history. Qualified
physical identity supplies conservative replacement and unambiguous same-run
rename evidence. Hardlink aliases always retain separate presence records.
Missing identity uses path continuity only. Equal size/mtime cannot prove equal
content, and this library never opens content to make that claim.

The caller supplies normalized relative paths and qualified identity. Basic
absolute/traversal/duplicate-path rejection is defensive; Windows normalization,
case-sensitive directory handling, extension filtering, reparse exclusions,
metadata-only enumeration and honest completion belong to the future native
boundary. DriveFS/non-NTFS identity remains unqualified under #47/#48.

## Bounds and integration gate

This is a bounded in-memory reference slice: at most 10,000 combined prior and
observed records, 32,768 bytes per path and 4 MiB total input path bytes. Exceeding
a limit rejects the run without output. These implementation limits are not
performance qualification or acceptance of the proposed MVP budgets.

Production must use bounded traversal/staging and integrate durable execution
and atomic SQLite apply (#38/#40). A Complete input is not sufficient authority:
the apply transaction must check root existence/enabled revision, generation,
lease token/session/deadline and cancellation. No proposal may bypass that gate.
Keep WAL disabled and PyFLP absent. Watcher hints and overflow recovery later
enter the same path; they never directly establish missing files.

## Incremental #41 evidence

Run `cargo test -p fruitboard-reconciliation --locked` and
`cargo clippy -p fruitboard-reconciliation --all-targets --locked -- -D warnings`.
The portable CI job runs both, and workspace Windows tests include the crate.

Thirteen deterministic tests cover unchanged-tree idempotence and order,
add/modify, rename/replacement, all incomplete outcomes including cancellation
and offline roots, missing/restored paths, restoration with replacement,
hardlink alias survival, ambiguous alias churn, identity fallback, historical
ID reuse, invalid/duplicate paths and resource limits. Fixtures are in-memory
metadata; no FLP files or private folders are used. This is core evidence only,
not installed-app, actual filesystem enumeration, crash recovery or benchmark
evidence. #36 and #41 remain open until their broader acceptance is satisfied.

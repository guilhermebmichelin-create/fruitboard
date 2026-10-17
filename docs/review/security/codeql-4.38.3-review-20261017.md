# github/codeql-action v4.38.3 pin review

Reviewed on 2026-10-17 against merged PR323, `929cf4df`.

The upstream [release](https://github.com/github/codeql-action/releases/tag/v4.38.3) and
GitHub tag API resolve to immutable commit
`24c54180a607b1449ed407dd24f251e4e9147c8d`. Annotated tag object
`cee97f86b972a3ded3c58d04148362b193ddc518` dereferences to that commit.
The release notes, changed source inventory and relevant execution/input/config
patches were reviewed; this is not an exhaustive audit of the upstream bundle.

Both initialization and analysis move together to the same release. The old split updates failed with a configuration-version mismatch. The reviewed source/default changes include bundle 2.27.2, per-language bundle/config handling and environment access; the Rust/JavaScript matrix and no-build modes remain unchanged. A regression rejects mismatched action pins.

The exact workflow pins and policy allowlist are updated together. Permissions,
credential persistence, trusted triggers, runner scopes and artifact restrictions
retain the existing policy. No runtime application dependency or production
feature is changed.

Validation requires the repository Node policy tests, script syntax, Markdown,
format and privacy checks, then all normal CI on the updated PR commit. Prior
PR323 CI applies only to unchanged app source. No local native build/cache write
is needed for this action-only review; actual Windows setup/package and CodeQL
compatibility must come from fresh CI, not the upstream tag alone. The owner
reviews and manually merges; historical installed/performance evidence is not
promoted. Private provenance and check logs remain outside compiler caches.

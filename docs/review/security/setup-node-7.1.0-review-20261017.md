# actions/setup-node v7.1.0 pin review

Reviewed on 2026-10-17 against merged PR323, `929cf4df`.

The upstream [release](https://github.com/actions/setup-node/releases/tag/v7.1.0) and
GitHub tag API resolve to immutable commit
`949feb2413d6458794dcd2491c4babbbce0c15c1`. The release notes, changed source inventory and relevant execution/input/config
patches were reviewed; this is not an exhaustive audit of the upstream bundle.

The reviewed execution changes add Node binary-version verification, bounded manifest-fetch retries, absolute version-file paths and mise input support. Fruitboard still supplies the exact Node 24.20.0 input and no package registry credentials. No version-file/mise input is enabled.

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

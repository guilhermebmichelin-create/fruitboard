# astral-sh/setup-uv v10.3.0 pin review

Reviewed on 2026-10-17 against merged PR323, `929cf4df`.

The upstream [release](https://github.com/astral-sh/setup-uv/releases/tag/v10.3.0) and
GitHub tag API resolve to immutable commit
`1c37ad07a6a961277cf70c0d37d6f313000f5884`. The release notes, changed source inventory and relevant execution/input/config
patches were reviewed; this is not an exhaustive audit of the upstream bundle.

The reviewed execution changes add optional Python architecture selection/probing and architecture-aware cache keys, plus bundled checksum entries. Fruitboard retains exact uv 0.12.9/Python 3.11.16 and does not set python-arch or UV_PYTHON_ARCH; the new architecture branch returns immediately for the empty input.

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

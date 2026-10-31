# Tooling glob matcher security review

The refreshed checks for #357 and #368-#372 encounter
[GHSA-g3hv-75wg-m249](https://github.com/advisories/GHSA-g3hv-75wg-m249),
published after their earlier successful checks. The advisory covers minimatch
versions below 11.0.1. The affected dependency is used by development tools;
Fruitboard does not introduce it as a product dependency.

## Dependency decision

PR #369 pins the workspace override to minimatch 11.0.3 and regenerates the
pnpm lockfile. All four consumers resolve to the patched package: Markdown
lint, ESLint, ESLint configuration matching, and the TypeScript parser.
The pinned Node 24.20.0 runtime meets the package's engine requirement.

This crosses a major version, so an audit-only check is insufficient.
[Upstream's changelog](https://github.com/isaacs/minimatch/blob/v11.0.3/changelog.md)
describes removed regular-expression APIs and changed matcher internals.
The reviewed consumers use the constructor, matching function, pattern and
partial-match behavior; they do not use the removed APIs. Their real lint
commands and the file-selection controls below pass with the override.

Retain the exact override until the upstream tools adopt a patched dependency.
Removing it requires a clean audit and the same integration checks. No advisory
is ignored, and the existing moderate-or-higher audit threshold is unchanged.

## Regression and validation

`tests/tooling-glob-compatibility.test.mjs` resolves the matcher through each
consumer. Each case constructs eight split points with an explicit expansion
limit of eight, then checks source-file matching, partial directory matching,
and dependency-directory exclusion. These are behavioral checks rather than a
version-string assertion.

The bounded negative control against the previously installed 10.2.6 package
generates 256 patterns despite the limit of eight. The patched dependency
respects the limit through all four consumers. This small regression does not
claim performance qualification or use a large adversarial fixture.

Local validation of the updated dependency tree passed:

- frozen installation with the pinned package manager;
- all 168 Node script/policy tests, including the four new cases;
- full documentation lint and script syntax checks;
- client ESLint and strict TypeScript;
- all 415 client tests across 34 files;
- `pnpm audit --audit-level moderate`.

The audit still reports the pre-existing low-severity development-only
[KaTeX advisory](https://github.com/advisories/GHSA-238p-pmpm-9mq7), which is
below the existing threshold. No new exception is added. Complete CI, including
the native Windows build and packaging smoke, must pass on the updated PR head
before merge. Other PRs receive this shared fix by integrating main after #369
merges; their previous failed audit is not treated as a passing check.

No application source, native dependency, permission, schema, migration or
private fixture changes. No local native compiler cache is written. Raw audit,
negative-control and validation logs are retained outside compiler caches.

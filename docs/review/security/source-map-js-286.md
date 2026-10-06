# source-map-js security repair

Issue [#286](https://github.com/guilhermebmichelin-create/fruitboard/issues/286).
Baseline: PR283 merge `b8c065c3f10b6b64d96b8224fadcdbb4e19af3d6`.

The existing lock resolves `source-map-js` 1.2.1 through PostCSS and CSS tooling.
The required security job now rejects it. The reviewed
[GHSA-68fv-2mgg-jv7q advisory](https://github.com/advisories/GHSA-68fv-2mgg-jv7q)
identifies indexed source-map offset validation as a denial-of-service issue and
1.2.2 as patched. This repair pins the exact transitive override in the workspace's
machine-readable override policy and regenerates the lockfile. The lock diff
changes only that override, package integrity and the two dependent resolutions.
No direct dependency/toolchain pin, runtime feature, audit exception or capability
changes. The existing `smol-toml` override remains intact.

Validation: frozen install, pinned Windows `pnpm check`, `pnpm audit --audit-level
high` and final-head CI. Private logs, corpus hashes and source receipts are
retained outside the reusable compiler cache. All 13 approved FLPs must remain
byte-identical. This is a focused prerequisite to sample-check integration;
merge it first, then review that feature against the repaired baseline.

No migration, profile mutation or new UI is included, so screenshots and new
product tests are unnecessary. Existing client/CSS build and test coverage plus
the dependency audit validate the repair. Rollback restores the old lock and
override but reintroduces the advisory and failing required security gate.

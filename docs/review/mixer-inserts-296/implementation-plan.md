# Mixer insert implementation and validation boundaries

This is a proposed sequence for [issue #296](README.md), not a delegation
request. Each slice follows issue → branch → focused PR → updated-head checks
→ owner review/manual merge. Inspect actual paths and name additions before
editing. Review the field contract if qualification changes it.

## Slice 1: genuine qualification and approved corpus

Source boundary: actual accepted design merge, recorded SHA/tree, with all
product/config/lock files unchanged. Own only the private candidate/expectation/
approval packet first; after exact owner approval, own named corpus FLPs,
`fixtures/parser-corpus/manifest.md`, named mixer expectations there, fixture
privacy allowlist in `scripts/lib/repository-privacy.mjs`, relevant policy
tests and a new research result. Inspect exact existing paths before preparing
the task. Do not change parser extraction or assign unqualified expectations
to existing approved files.

Follow the [fixture gate](fixture-plan.md). No local native build/cache write is
needed for a corpus-only change. Run pinned privacy, Markdown/script checks,
repository policy tests and normal final-head CI. A matching unchanged source
can use preceding CI for product checks, but genuine GUI/structural qualification
must be new. Record all fixture hashes before/after. Use an exclusive GUI
qualification window, the named cache/evidence boundaries and storage preflight
below. Retain exact approval, originals and observations; no broad cleanup.

## Slice 2: bounded parser and complete native validation

Source boundary: actual accepted corpus-slice merge, recorded SHA/tree and exact
approved expectations. Own `crates/flp-parser/src/lib.rs`, a new selected mixer
module, `src/validation.rs`, `src/validation/project_facts.rs`, a new full
validation module, relevant protocol/corpus/mixer/validation tests, parser
README/package version and `FLP_PARSER.md`; name any further files before edits.
Storage/client/desktop behavior remains unchanged. No dependency changes or
new build support are needed. Qualify actual descriptor consumers before adding
the paired fields/caps. Implement independently from approved format evidence.

A local parser build is required. Run pinned `pnpm.cmd check`, parser tests and
warning-denied all-target Clippy; test descriptor negotiation, unadvertised/other-
build/omitted/unnamed states, sorted sparse IDs, count/list correspondence, UTF-16
and UTF-8 limits, candidate/output caps, duplicate IDs/names, repeated sections,
special-track exclusion, unknown/incorrect boundaries and borrowed aggregate
arithmetic. Genuine observations establish compatibility; constructed negative
cases establish rejection behavior only. No invented zero/default names.

Exercise real supervised parsing with approved cases and full typed validation;
malformed advertised fields fail the reply. Preserve initial-only compatibility
and all old fields/limits. Hash the whole corpus before/after. Reserve the single
heavyweight window/cache owner below, retain source/protocol/validation logs
outside the cache, and require normal final-head CI and owner review. Successful
unchanged-code CI can avoid redundant builds outside the changed boundary;
it cannot replace required updated parser checks or installed qualification.

## Slice 3: immutable selected projection and Library

Source boundary: actual accepted parser/full-validation merge, SHA/tree and
qualified field semantics unchanged. Own `crates/storage-sqlite/src/metadata/`
selected payload/tests, `apps/desktop/src-tauri/src/foundation/project_details.rs`
and a selected mixer DTO module/tests, analysis publication/integration tests,
`apps/client/src/library/projectDetails.ts` and selected decoder/component/tests,
`ProjectDetailsPanel.tsx`, appropriate styles and review documentation. Inspect
actual publication-test locations and name further owned files before editing.
No new native command, permissions, schema/table/migration or snapshot rewrite.

Local native build and genuine installed validation are required. Run pinned
Windows `pnpm.cmd check`, changed `scan-console`/`analysis-jobs` tests and
warning-denied all-target Clippy, real-parser publication/details integration,
storage immutability/malformed replacement checks and strict renderer state/
relationship/race/accessibility tests. Current authorized details revalidate the
stored selected pair. Old snapshots are `not_saved`; malformed present data
fails safely. Refresh never adds fields or starts analysis. Explicit reanalysis
retains the existing attempts and current-source/publication/session/lease gates.

Use a fresh separately identified installed development profile. Check approved
counts/names, unnamed/missing/unsupported/old states, refresh without attempt,
restart/reinstall recovery, root revocation, changed/missing source and held late
replies. Exercise 20-entry disclosure/focus with clearly labeled constructed
scale evidence; real native desktop, 390px and 200% text views plus axe contrast.
No personal/Foundation Smoke profile seeding or swapping. Record actual source/
tree/toolchain/commands/package and binary hashes; retain independent copies,
not cache links. Installed qualification uses the existing host lock and runs
alone after builds. Hash approved FLPs before/after. Final-head CI and owner
review remain required; CI is not installed-app or performance evidence.

## Shared resources and completion requirements

Reuse the inspected `fruitboard-review-source-20260929` worktree. The actual
absolute source/cache/evidence paths are recorded in the private task handoff;
record any change before execution. Validation cache: `bounded-probes`, single
owner **Codex `/root`**. The primary checkout's `target/` is protected. No new
per-PR cache. Set
`CARGO_TARGET_DIR` explicitly for native validation. No native build/cache write
is needed for this design or a corpus-only slice.

Before a heavyweight build or fixture creation, inspect clean/dirty worktrees,
active workloads and resolved paths, then run the repository read-only storage
preflight with explicit source, build, cache and retained-evidence directories.
Maintain **30 GiB free on every used volume after expected output**. Start with
6 GiB additional native-build output or 0.1 GiB small-fixture/docs evidence,
adjusting for actual scope and independent retained copies. The design task
measured C: at 44.10 GiB; that is not a future capacity guarantee. Do not use
G: while below reserve. Check cache hardlinks/reparses; historical isolation
does not establish continuing exclusivity. Follow [DEVELOPMENT.md](../../../DEVELOPMENT.md#local-build-and-disk-space-rules).

The named owner reserves one heavyweight write window; no concurrent Cargo
writer or build/test task shares it. Genuine fixture qualification and installed
host-lock tests run with other agent/build/test work quiescent. Performance
measurements, if later separately scoped, run alone and retain existing targets.
This design proposes safety ceilings, not resource-budget acceptance.

Retain fixture originals, approval, reports, databases, screenshots and copied
binaries outside disposable caches. Current private design evidence packet:
`mixer-inserts-design-20261008`; its absolute path is in the private task handoff.
Name future slice evidence paths in their handoffs before execution. Preserve
owner profiles/data, dirty/untracked source, Git state, dependencies/toolchains,
all earlier approved fixtures and retained evidence. No broad deletion.

At each handoff record final SHA/tree/checks, single cache ownership, current
free space, reusable outputs, obsolete compiler intermediates and retained
evidence/cleanup responsibility. No intermediates are generated by this design.
Any later cleanup requires verified absolute targets, no unexpected reparses or
active users, an inventory, native `-LiteralPath` operations and protected-file/
state verification plus measured reclaimed space/rebuild cost. Never delete a
whole worktree or treat a directory named `target` as automatic authorization.

## Owner decision boundary

The completed design is reviewable now. Exact public fixture approval is a
separate decision about prepared bytes/expectations after privacy review.
Merging any slice does not accept broader G6, Phase 3, resource budgets, license/
distribution gates or later-phase work. Documentation rollback changes no data.

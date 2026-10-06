# Pattern note implementation sequence

Status: **Proposed sequence; this PR implements documentation only.**
Planning baseline: `d19587f52132f31065bbaab97a572d3c5487cf00`, PR285 merge.
Work from the accepted design merge, then the actual preceding slice merge;
record each full SHA/tree and final PR head before validation. This sequence is
not an instruction to another agent and does not request additional agents.
The [contract and authority review](README.md) and [fixture gate](fixture-plan.md)
define the source/content boundary; broader G6/G5 and Phase 3 remain open.

## Slice 1: Matching-build ground truth and approved corpus

Owned files: new approved F16/F17 entries only after allocation/approval,
`fixtures/parser-corpus/manifest.md`, the exact fixture allowlist in
`scripts/verify-repository-privacy.mjs` and its existing privacy tests, and a
focused `docs/research/` result. Private recipes, saves, byte inventories,
hashes and GUI captures stay outside Git. Name any further owned file before
changing it. No parser, storage, desktop or client feature changes.

Start from the accepted design's actual merge SHA/tree, retain the preregistered
cases before creation, and qualify in build 26.1.0.5530. Prepare reviewable exact
candidate bytes/expectations/privacy inventory before requesting their required
owner approval. Publish only after that approval. Record no support for any
unqualified zero/slide/step/layout case. Do not repurpose F13's existing approval.

No local Rust/app build or writable Cargo cache is needed for fixture preparation
or a fixture-only PR. Required local checks are documentation, repository
privacy/script/policy tests and independent GUI/hash evidence. Prior CI covers
unchanged product code only; run normal updated-head CI and confirm the actual
approved-corpus parser suite on the added files without invented expectations.
Use the resource rules below before any temporary fixture generation.

## Slice 2: Bounded parser and typed native validation

Owned files: `crates/flp-parser/src/{lib,main,validation}.rs`, a new bounded
note-count decoder and `validation/` module, additive `validation/project_facts.rs`
accessors, focused parser/protocol/descriptor/validation/supervisor tests and
`crates/flp-parser/README.md`, FLP_PARSER.md and the maintained reliability matrix.
The parser package version in `crates/flp-parser/Cargo.toml` and its Cargo.lock
entry change with extraction. No new dependency, executable selection,
renderer permission, stored payload or UI in this slice. Inspect repository
contract tests and name any required descriptor expectation update before editing.

Source boundary: accepted slice-1 merge plus the design; record actual SHA/tree
and all approved hashes before and after. Implement only qualified layouts and
sections, borrowed-slice checked counting, fixed absent/unsupported states,
unchanged existing caps and the proposed new limits. Update full typed validation
and descriptor negotiation; otherwise valid matching-adapter descriptors remain
usable without the new field. Existing exact adapter-version validation stays intact.
No raw note arrays or copied third-party implementation enters the parser.

Local build is required. Run pinned Windows `pnpm check`, parser all-target
warning-denied Clippy/tests and actual supervised protocol tests. Cover approved
positive counts, Playlist reuse, known missing/empty behavior and meaningful
binding/duplicate/malformed/limit cases. Constructed cases verify defenses only.
Require final-head CI, including portable parser supervision. Prior unchanged
CI is provenance context, not proof for changed parser code. Review native
authority against the design table and final code before storage integration.

## Slice 3: Immutable projection and Library display

Owned files: `crates/storage-sqlite/src/metadata/payload.rs` and focused metadata
publication/backward-reading tests; desktop `foundation/project_details.rs`,
its `patterns.rs` projection or a dedicated note-summary projection, relevant
details/native integration tests; client `library/{projectDetails,projectPatterns}.ts`,
`SavedPatterns.tsx`, `ProjectDetailsPanel.tsx` and their fixtures/tests;
explicit additive platform contract mapping/tests only where needed; review docs
and sanitized screenshots. Inspect actual locations before assigning ownership.
Name further files before editing. No migration, new table, snapshot rewrite,
command, sample resolver, global filesystem or audio/process capability.

Source boundary: actual accepted slice-2 merge, with corpus and typed semantics
unchanged; record final source/tree and matching artifacts. Persist only the
optional selected numeric/state projection. Validate stored IDs against patterns
again at the authorized native read and renderer. Missing old fields become
`not_saved`; malformed present data fails safely. Preserve all freshness,
root/source/session/publication/lease fences and last-good results. Refresh
reads results; explicit analysis keeps the existing attempt policy.

Local build and actual installed validation are required. Run full pinned Windows
`pnpm check`, changed `scan-console`/`analysis-jobs` tests and warning-denied
all-target Clippy, storage immutability/stale-result tests, strict renderer
relationship/race tests and actual parser-to-publication-to-details integration
with approved cases. Cover old/missing/unsupported/zero-if-qualified/loading/error,
snapshot refresh and root/source invalidation; hash all approved FLPs before/after.

Exercise a fresh separately identified installed development profile through
analysis, names/counts, refresh-without-analysis, restart and reinstall recovery.
Do not seed/swap/open personal or FoundationSmoke profiles. Review real native
desktop, 390px and 200%-text states, keyboard paging/focus and axe including
contrast. Clearly label constructed malformed/older/limit UI evidence. Record
actual source/binary/package hashes, independent retained files and host lock
ownership. Installed qualification runs alone, after builds are finished.
Require normal final updated-head CI and owner review/manual merge.

## Resource and handoff boundary

Use the inspected reusable source worktree; the private task handoff records its
absolute location and owned file list. Default to the primary development cache
and one reusable validation cache, never one per slice/agent. Existing validation
cache: `bounded-probes`, single owner **Codex `/root`**; its absolute path is
recorded in the private task handoff. The primary checkout's `target/` stays protected.
Future native commands set `CARGO_TARGET_DIR` explicitly to the selected cache.
No cache write or local product build is needed for this design PR.

Before each heavyweight build or fixture generation, run the explicit-source,
build, cache and retained-evidence storage preflight. Charge expected additional
output (initial native-build estimate 6 GiB, increase if justified) and retain
at least **30 GiB free on every used volume after that output**. Inspect paths,
reparses, cache hardlinks, worktree state and active workloads first. The preflight
is read-only and does not prove continuing ownership/exclusivity.

The named cache owner reserves a single heavyweight write window; no concurrent
Cargo/build/test workload may share it. Installed tests use the existing
Foundation Smoke host lock and run with other agent/build/test workloads
quiescent. Fixture GUI qualification runs separately from heavy workloads.
This plan adds no performance target, benchmark waiver or budget acceptance.

Store logs, reports, private fixtures/databases/screenshots and independently
copied evidence binaries outside disposable caches. Record source SHA/tree,
commands, toolchain, timestamps, hashes and exact scope. Never hardlink retained
binaries to writable compiler outputs. Preserve dirty/untracked source, Git
state, dependencies/toolchains, existing corpus, owner databases/profiles and
older evidence. No new permanent multi-gigabyte cache or broad cleanup.

At handoff identify cache ownership, actual free capacity, reusable outputs,
obsolete intermediates if any, retained evidence and cleanup/rebuild cost.
Any cleanup needs verified absolute resolved targets, no unexpected reparses or
active users, an inventory and native `-LiteralPath` operations, then protected
file/state checks and measured reclaimed space. Do not delete whole worktrees,
temporary trees or a directory merely because it is named `target`.

## Review decision and next boundary

Owner review decides whether to accept or revise this scope and ceilings before
product implementation. Exact public fixture approval is a separate decision
about prepared bytes/expectations; it cannot be requested meaningfully now.
The completed design is concrete and reviewable without those future files.
Rollback of this documentation PR is documentation-only. Merging any individual
slice does not close broader G6, accept resources or open Phase 4.

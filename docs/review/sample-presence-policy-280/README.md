# Sample presence policy review

Status: **Slice 1 implementation for owner review; no app probe activation.**
Date: 2026-10-05. Issue:
[#280](https://github.com/guilhermebmichelin-create/fruitboard/issues/280).
Source baseline: `6054790e0a7b2f09ece4644b479f68acf6805b13`, tree
`1f7caa93092d5b6363369a8d029dbcee93bd8273`. This includes reviewed design
[PR279](https://github.com/guilhermebmichelin-create/fruitboard/pull/279) and four
subsequent dependency merges. Design acceptance does not qualify a native port.

## Owner summary

The library makes rules for checking saved sample paths without confusing
"could not check" with "missing". It limits work and prevents slow cancelled
checks from starting overlapping work. The Windows checker and app button
remain separate steps.

## Change boundary

The [library contract](../../../crates/sample-presence/README.md) covers captured
authority, syntax, filesystem-owned root comparison, ordered fixed reports,
metadata-only ports, resource bounds and the single-worker publication fence.
Workspace/lock additions introduce no third-party dependency. Portable CI adds
this crate's Clippy/tests. Desktop/client/parser/storage and existing native
implementations, databases, snapshots, fixtures and activation are unchanged.

## Verification and remaining proof

Recording ports exercise ordinary files/trusted absence, denied/unsafe
observations, forbidden syntax, outside-root paths, case-mode ownership, exact
duplicates, complete 256-slot reports, malformed/older captures, 64/65-component
limits, operation reservation and handle bounds. Fake clocks and blocked work
verify cancellation, stale signals, deadlines, busy admission, late-publication
fencing and shutdown.

Injected stale signals prove policy discards rejected fences; they do not prove
Windows replacement detection or database locking. The metadata-only interface
does not prove syscall rights or no-hydration behavior. These need a real Windows
adapter and adversarial tests in slice 2. Strict IPC, runtime selection fences,
accessibility and installed evidence are slice 3. G5, broader sample resolution
and Phase 3 acceptance stay open.

Required checks: crate Clippy/tests, pinned Windows `pnpm check`, privacy,
Markdown/link/diff checks, approved fixture hashes and normal final-head CI.
Actual receipts, SHA/tree, counts and CI state are recorded in the PR and private
handoff; this document does not label pending checks as passed.

## Build and handoff boundary

Codex `/root` owns the reused review checkout and sole validation cache
whose absolute path is recorded in the private task handoff. The primary cache
is protected. The pinned runner explicitly sets `CARGO_TARGET_DIR`. Sequential
exclusive builds require no other cache user/heavyweight build and a preflight
with explicit source/build/cache/evidence directories, 6 GiB expected output
and at least 30 GiB free afterward on all relevant volumes.

Retain private logs, source/tree provenance, preflight/alias inventories and
handoff outside compiler caches in the private task evidence directory; its
absolute path is recorded in the handoff rather than published here.
No installed test, fixture generation or performance qualification is needed
for this inactive library. Reuse the existing cache; no new permanent cache or
broad cleanup. Preserve other worktrees, profiles/databases, toolchains, approved
corpus and historical evidence binaries.
